//! Golden typing tests: drive `Engine::handle_event` with letter/bracket/
//! SPACE/DELETE/break-key sequences and apply the resulting `HookState` to a
//! virtual text field, asserting the final Unicode string.
//!
//! The virtual field mirrors what a host application does with a `HookState`
//! (per the Task 4/5 briefs and the real OpenKey macOS host, `OpenKey.mm`):
//!   * `DoNothing` -> pass the raw keystroke through: append the originally
//!     typed character, except for DELETE, where it means "let the native OS
//!     backspace happen" (pop one char).
//!   * `WillProcess`/`Restore`/`RestoreAndStartNewSession` -> delete
//!     `backspace_count` chars, then append `char_data[new_char_count-1..=0]`
//!     (the buffer is stored reversed); `Restore` and
//!     `RestoreAndStartNewSession` additionally re-append the original key
//!     (both behave identically for field mutation -- the "AndStartNewSession"
//!     half only resets engine-internal state, not the visible field).
//!
//! Every expected string here was cross-checked against the C++ oracle while
//! developing the port (the exhaustive check lives in
//! `differential_typing.rs`). Where the brief's shorthand differs from real
//! OpenKey behavior, the oracle-verified value is used and noted.

use vnkey_engine::keys::*;
use vnkey_engine::{
    CAPS_MASK, CHAR_CODE_MASK, CHAR_MASK, Engine, EngineOptions, HookCode, HookState, InputType,
    KeyEvent, KeyEventState, key_code_to_character,
};

/// Map an ASCII char to `(keycode, caps_status)`. Uppercase -> shift (1).
/// `'\u{8}'` (backspace) and `'\n'` (enter) are sentinels for DELETE/ENTER,
/// which have no printable ASCII form of their own.
fn key_of(c: char) -> (u16, u8) {
    let up = c.is_ascii_uppercase();
    let code = match c.to_ascii_lowercase() {
        'a' => KEY_A, 'b' => KEY_B, 'c' => KEY_C, 'd' => KEY_D, 'e' => KEY_E,
        'f' => KEY_F, 'g' => KEY_G, 'h' => KEY_H, 'i' => KEY_I, 'j' => KEY_J,
        'k' => KEY_K, 'l' => KEY_L, 'm' => KEY_M, 'n' => KEY_N, 'o' => KEY_O,
        'p' => KEY_P, 'q' => KEY_Q, 'r' => KEY_R, 's' => KEY_S, 't' => KEY_T,
        'u' => KEY_U, 'v' => KEY_V, 'w' => KEY_W, 'x' => KEY_X, 'y' => KEY_Y,
        'z' => KEY_Z,
        '0' => KEY_0, '1' => KEY_1, '2' => KEY_2, '3' => KEY_3, '4' => KEY_4,
        '5' => KEY_5, '6' => KEY_6, '7' => KEY_7, '8' => KEY_8, '9' => KEY_9,
        '[' => KEY_LEFT_BRACKET, ']' => KEY_RIGHT_BRACKET,
        ' ' => KEY_SPACE, '.' => KEY_DOT, ',' => KEY_COMMA,
        '\u{8}' => KEY_DELETE, '\n' => KEY_ENTER,
        other => panic!("golden test: unmapped char {other:?}"),
    };
    (code, if up { 1 } else { 0 })
}

/// Render one `char_data` cell to a Unicode scalar. A cell with
/// `CHAR_CODE_MASK` carries a character code (low 16 bits, for the Unicode
/// table); otherwise it is a keyboard code to be mapped back to ASCII.
fn render_cell(cell: u32) -> char {
    if cell & CHAR_CODE_MASK != 0 {
        char::from_u32(cell & 0xFFFF).unwrap_or('\u{FFFD}')
    } else {
        char::from_u32(key_code_to_character(cell & (CHAR_MASK | CAPS_MASK))).unwrap_or('\u{FFFD}')
    }
}

/// `is_delete` is whether the just-pressed key was DELETE: a `DoNothing` for
/// DELETE means "let the native OS backspace happen" (pop one char), while a
/// `DoNothing` for every other key means "insert the raw character".
fn apply(field: &mut Vec<char>, orig: char, is_delete: bool, hs: &HookState) {
    use vnkey_engine::HookCode::*;
    match hs.code {
        DoNothing => {
            if is_delete {
                field.pop();
            } else {
                field.push(orig);
            }
        }
        WillProcess | Restore | RestoreAndStartNewSession => {
            for _ in 0..hs.backspace_count {
                field.pop();
            }
            for i in (0..hs.new_char_count as usize).rev() {
                field.push(render_cell(hs.char_data[i]));
            }
            if hs.code == Restore || hs.code == RestoreAndStartNewSession {
                field.push(orig);
            }
        }
        _ => {}
    }
}

/// Type `s` through a fresh engine and return the resulting virtual-field text.
fn type_str(opts: EngineOptions, s: &str) -> String {
    let mut e = Engine::new(opts);
    let mut field: Vec<char> = Vec::new();
    for c in s.chars() {
        let (code, caps) = key_of(c);
        let hs = e.handle_event(KeyEvent::Keyboard, KeyEventState::KeyDown, code, caps, false);
        let orig = char::from_u32(key_code_to_character(
            code as u32 | if caps == 1 { CAPS_MASK } else { 0 },
        ))
        .unwrap();
        apply(&mut field, orig, code == KEY_DELETE, hs);
    }
    field.into_iter().collect()
}

fn telex() -> EngineOptions {
    EngineOptions::default()
}

/// Like `type_str`, but calls `action` on the engine immediately before
/// processing the character at index `action_at` (0-based). Used for the
/// Task 6 session-method golden cases, which need to call
/// `temp_off_spell_checking`/`temp_off_engine` partway through a sequence.
fn type_str_with_action(
    opts: EngineOptions,
    s: &str,
    action_at: usize,
    action: impl FnOnce(&mut Engine),
) -> String {
    let mut e = Engine::new(opts);
    let mut field: Vec<char> = Vec::new();
    let mut action = Some(action);
    for (i, c) in s.chars().enumerate() {
        if i == action_at
            && let Some(f) = action.take()
        {
            f(&mut e);
        }
        let (code, caps) = key_of(c);
        let hs = e.handle_event(KeyEvent::Keyboard, KeyEventState::KeyDown, code, caps, false);
        let orig = char::from_u32(key_code_to_character(
            code as u32 | if caps == 1 { CAPS_MASK } else { 0 },
        ))
        .unwrap();
        apply(&mut field, orig, code == KEY_DELETE, hs);
    }
    field.into_iter().collect()
}

#[test]
fn telex_vieejt_gives_viet() {
    assert_eq!(type_str(telex(), "vieejt"), "việt");
}

#[test]
fn telex_ddaay_gives_day() {
    assert_eq!(type_str(telex(), "ddaay"), "đây");
}

#[test]
fn telex_hoaf_old_orthography_is_hoa_grave_on_o() {
    // modern = 0 (default): the grave sits on the first vowel -> "hòa".
    assert_eq!(type_str(telex(), "hoaf"), "hòa");
}

#[test]
fn telex_hoaf_modern_orthography_is_hoa_grave_on_a() {
    let mut o = telex();
    o.use_modern_orthography = true;
    assert_eq!(type_str(o, "hoaf"), "hoà");
}

#[test]
fn telex_uwowng_gives_uong_horns() {
    assert_eq!(type_str(telex(), "uwowng"), "ương");
}

#[test]
fn telex_standalone_w_gives_u_horn() {
    assert_eq!(type_str(telex(), "w"), "ư");
}

#[test]
fn telex_xoong_gives_xong() {
    // The brief lists "xoong" as its own shorthand, but real OpenKey (and this
    // port, verified against the oracle) folds the double-o into a circumflex:
    // x-o-o -> "xô", then n,g -> "xông".
    assert_eq!(type_str(telex(), "xoong"), "xông");
}

#[test]
fn telex_thueej_gives_thue_dot() {
    assert_eq!(type_str(telex(), "thueej"), "thuệ");
}

#[test]
fn telex_double_mark_key_without_vowel_inserts_both() {
    // "ss" has no vowel to mark, so each 's' is a plain insert (matches oracle).
    assert_eq!(type_str(telex(), "ss"), "ss");
}

#[test]
fn telex_repeated_mark_key_toggles_mark_off() {
    // "as" -> "á"; the second 's' re-presses the same (sac) mark -> revert to "as".
    assert_eq!(type_str(telex(), "ass"), "as");
}

#[test]
fn telex_z_key_removes_mark() {
    // "as" -> "á"; 'z' removes the mark -> "a".
    assert_eq!(type_str(telex(), "asz"), "a");
}

#[test]
fn telex_zz_without_mark_inserts_both() {
    assert_eq!(type_str(telex(), "zz"), "zz");
}

#[test]
fn telex_left_bracket_gives_o_horn() {
    assert_eq!(type_str(telex(), "["), "ơ");
}

#[test]
fn telex_right_bracket_gives_u_horn() {
    assert_eq!(type_str(telex(), "]"), "ư");
}

#[test]
fn vni_vie65t_gives_viet() {
    let mut o = telex();
    o.input_type = InputType::Vni;
    assert_eq!(type_str(o, "vie65t"), "việt");
}

#[test]
fn vni_d9a6y_gives_day() {
    let mut o = telex();
    o.input_type = InputType::Vni;
    assert_eq!(type_str(o, "d9a6y"), "đây");
}

#[test]
fn simple_telex1_w_is_not_standalone_u_horn() {
    // In SimpleTelex1, 'w' is not a standalone key -> it stays a literal 'w'.
    let mut o = telex();
    o.input_type = InputType::SimpleTelex1;
    assert_eq!(type_str(o, "w"), "w");
}

#[test]
fn quick_telex_cc_starts_word_with_ch() {
    let mut o = telex();
    o.quick_telex = true;
    assert_eq!(type_str(o, "cc"), "ch");
}

// -- Task 5: break/SPACE/DELETE/restore golden cases -----------------------
//
// The brief's "Golden tests" section (task-5-brief.md) specifies these 7
// cases. Every expected value below was obtained by running the exact
// sequence through the C++ oracle (`openkey-oracle`'s `Oracle::lock` +
// `OracleSession`, the same helper `differential_typing.rs` uses) -- see
// task-5-report.md's "Review fix" section for the oracle output. Where the
// oracle disagreed with the brief's literal wording, the oracle wins (same
// rule `telex_xoong_gives_xong` above follows) and the divergence is noted.

#[test]
fn restore_if_wrong_spelling_restores_raw_keys_on_space() {
    // The brief's literal example (`tieengs` + SPACE) does NOT trigger a
    // restore: "tiếng" is valid Vietnamese spelling, so the oracle just types
    // it normally. Substituted `nghasl` (ends in the consonant 'l', which is
    // not a valid Vietnamese ending) to actually exercise
    // checkRestoreIfWrongSpelling: 'l' makes checkSpelling flag the word
    // invalid, so SPACE restores the buffer to its raw keystrokes ("nghasl")
    // and then appends the space, giving back exactly what was typed.
    let mut o = telex();
    o.restore_if_wrong_spelling = true;
    assert_eq!(type_str(o, "nghasl "), "nghasl ");
}

/// The other restore path: a break key (here `.`) rather than SPACE, which
/// goes through `RestoreAndStartNewSession` instead of `Restore`. Pinned
/// because that branch had no coverage at all.
#[test]
fn restore_if_wrong_spelling_restores_raw_keys_on_break_key() {
    let mut o = telex();
    o.restore_if_wrong_spelling = true;
    assert_eq!(type_str(o, "nghasl."), "nghasl.");
}

/// Turning restore on must not disturb ordinary Vietnamese. Every entry here
/// produces byte-identical output with the option off and on; these are the
/// regression tripwire for the auto-restore work.
#[test]
fn restore_if_wrong_spelling_leaves_valid_vietnamese_untouched() {
    let mut on = telex();
    on.restore_if_wrong_spelling = true;
    let cases = [
        ("vieejt ", "việt "),
        ("ddaay ", "đây "),
        ("uwowng ", "ương "),
        ("thueej ", "thuệ "),
        ("hoaf ", "hòa "),
        ("nghieeng ", "nghiêng "),
        ("dduwowcj ", "được "),
        ("ruwowuj ", "rượu "),
        ("af ", "à "),
        ("oof ", "ồ "),
        ("bee ", "bê "),
        ("mee ", "mê "),
        ("quaf ", "quà "),
        ("gifn ", "gìn "),
        ("khoong ", "không "),
    ];
    for (typed, expected) in cases {
        assert_eq!(type_str(on, typed), expected, "with restore ON: {typed:?}");
        assert_eq!(type_str(telex(), typed), expected, "with restore OFF: {typed:?}");
    }
}

#[test]
fn backspace_then_retype_repositions_mark_via_check_grammar() {
    // "vieejt" -> "việt". Two backspaces strip back to "vi", then retyping
    // "ejt" (e, then nang-mark j, then t) re-derives the nang mark -- but
    // checkGrammar's post-delete re-placement (Engine.cpp "August 23rd
    // continue check grammar") makes it land on the *first* vowel (i) rather
    // than where it sat before the deletes (e), giving "vịet", not "việt".
    // This is real, oracle-verified OpenKey behavior, not a porting bug.
    assert_eq!(type_str(telex(), "vieejt\u{8}\u{8}ejt"), "vịet");
}

#[test]
fn backspace_crosses_word_boundary_back_into_first_word() {
    // "vieejt" + SPACE + "an" -> "việt an". Four backspaces delete "n", "a",
    // the space (restoring the `_typingStates` entry for "việt" from
    // history), and finally one more char off "việt" itself, ending on
    // "việ" -- demonstrating the typing-state history restore across the
    // word boundary that DELETE/space_count/_typingStates coordinate.
    assert_eq!(type_str(telex(), "vieejt an\u{8}\u{8}\u{8}\u{8}"), "việ");
}

#[test]
fn quick_end_consonant_hag_space_gives_hang() {
    let mut o = telex();
    o.quick_end_consonant = true;
    assert_eq!(type_str(o, "hag "), "hang ");
}

#[test]
fn quick_start_consonant_fanh_space_gives_phanh() {
    let mut o = telex();
    o.quick_start_consonant = true;
    assert_eq!(type_str(o, "fanh "), "phanh ");
}

#[test]
fn upper_case_first_char_after_dot_space_capitalizes_next_word() {
    // DOT sets upper_case_status=1, SPACE upgrades it to 2, and the first
    // letter of the next word ('v') triggers upperCaseFirstCharacter. The
    // full virtual field also carries the literal ". " that was typed (a
    // real text field shows those keystrokes too) -- the brief's "Viet"
    // shorthand refers to the capitalized word, not the whole field content.
    let mut o = telex();
    o.upper_case_first_char = true;
    assert_eq!(type_str(o, ". viet"), ". Viet");
}

#[test]
fn word_longer_than_max_buff_exercises_long_word_helper() {
    // 37 raw keystrokes, one word (no break), which overflows MAX_BUFF (32)
    // and exercises `_longWordHelper`'s overflow bookkeeping. The first
    // syllable ("vieejt" -> "việt") still gets full diacritic handling, but
    // partway through "d" (a would-be đ/d-stroke trigger) checkSpelling
    // flags the ever-growing buffer as invalid Vietnamese spelling, which
    // sets temp_disable_key and routes every subsequent key through the
    // plain-insert path -- so the tail types out completely literally. This
    // is the real, oracle-verified behavior for an over-length word.
    let long = "vieejtddaayuwowngthueejnguoiwfkhuyees";
    assert_eq!(long.len(), 37);
    assert_eq!(
        type_str(telex(), long),
        "việtđaayuwowngthueejnguoiwfkhuyees"
    );
}

// -- Task 6: temp-off-spelling / temp-off-engine / english_mode ------------
//
// Every expected value below was obtained the same way as the Task 5 cases
// above: run the exact sequence through the C++ oracle via a throwaway
// `tests/zz_oracle_explore.rs` scratch harness (deleted before commit, not
// part of the diff) and compare per-key `(code, backspace_count,
// new_char_count, ext_code[, char_data])` field by field before pinning the
// final string/length here.

#[test]
fn temp_off_spelling_round_trip() {
    // check_spelling=1, restore_if_wrong_spelling=1. `temp_off_spell_checking`
    // is called once, before the FIRST word only -- no `reset_check_spelling`.
    //
    // Oracle trace, word 1 ("nghasl "): spelling is off (vCheckSpelling
    // flipped false), so checkSpelling never runs; 'n','g','h','a' insert
    // plain, 's' places a sac mark on 'a' -> "nghá", 'l' inserts plain ->
    // "nghál", and SPACE does NOT restore (temp_disable_key never got set) --
    // contrast with `restore_if_wrong_spelling_restores_raw_keys_on_space`
    // above, the same "nghasl " input WITHOUT temp-off, which restores to
    // raw keys instead.
    //
    // Word-break auto-restores the baseline (`spell_check_on =
    // use_spell_checking_before`, ported in Task 5) back to ON, since
    // `reset_check_spelling` was never called to make the "off" state stick.
    // So word 2 (second "nghasl "): checkSpelling flags the invalid 'l'
    // ending again and SPACE fully restores to raw keys, exactly like the
    // no-temp-off case -- completing the round trip.
    let mut o = telex();
    o.check_spelling = true;
    o.restore_if_wrong_spelling = true;
    let out = type_str_with_action(o, "nghasl nghasl ", 0, |e| e.temp_off_spell_checking());
    assert_eq!(out, "nghál nghasl ");
}

#[test]
fn temp_off_engine_passes_keys_through_until_word_break() {
    // `temp_off_engine(true)` called right before the third key ('e', the
    // first of the double-e in "vieejt"). From that point on every key is
    // pure passthrough (`hCode` stays DoNothing, no mark/vowel transform),
    // so the first word types out literally as "vieejt" instead of
    // transforming to "việt". SPACE resets `will_temp_off_engine` (already
    // ported in Task 5's break/space branches), so the second "vieejt"
    // transforms normally again -> "việt".
    let out = type_str_with_action(telex(), "vieejt vieejt", 2, |e| e.temp_off_engine(true));
    assert_eq!(out, "vieejt việt");
}

#[test]
fn english_mode_accumulates_macro_key_and_clears_on_space() {
    // language=0 (English mode), no macro table (use_macro stays false, the
    // same Task-7 carve-out as the rest of this port). `english_mode` never
    // transforms a key -- `hCode` stays DoNothing for every key -- it only
    // accumulates one `macro_key` entry per non-break key and clears it on
    // SPACE (`findMacro` is the Task 7 stub, always a miss).
    let mut e = Engine::new(telex());
    let mut lens = Vec::new();
    for c in "btw ".chars() {
        let (code, caps) = key_of(c);
        let hs = e.english_mode(KeyEventState::KeyDown, code, caps == 1, false);
        assert_eq!(hs.code, HookCode::DoNothing);
        lens.push(hs.macro_key.len());
    }
    assert_eq!(lens, vec![1, 2, 3, 0]);
}
