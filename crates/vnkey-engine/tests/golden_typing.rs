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
        '\u{8}' => KEY_DELETE, '\n' => KEY_ENTER, '\u{1b}' => KEY_ESC,
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
            } else if orig != '\0' {
                // A key with no printable form (ESC) reaches the app but
                // inserts nothing.
                field.push(orig);
            }
        }
        WillProcess | Restore | RestoreAndStartNewSession | RestoreAndSwallow => {
            for _ in 0..hs.backspace_count {
                field.pop();
            }
            for i in (0..hs.new_char_count as usize).rev() {
                field.push(render_cell(hs.char_data[i]));
            }
            // `RestoreAndSwallow` (the ESC undo) deliberately does not echo its
            // trigger; the host suppresses it the same way.
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
    type_into(&mut e, &mut field, s);
    field.into_iter().collect()
}

/// Type `s` into an existing engine/field pair, so a caller can inspect the
/// field between keystrokes.
fn type_into(e: &mut Engine, field: &mut Vec<char>, s: &str) {
    for c in s.chars() {
        let (code, caps) = key_of(c);
        let hs = e.handle_event(KeyEvent::Keyboard, KeyEventState::KeyDown, code, caps, false);
        let orig = char::from_u32(key_code_to_character(
            code as u32 | if caps == 1 { CAPS_MASK } else { 0 },
        ))
        .unwrap();
        apply(field, orig, code == KEY_DELETE, hs);
    }
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

/// English words that Telex composes into something that cannot be Vietnamese
/// come back exactly as typed once the word ends.
#[test]
fn auto_restore_english_returns_words_as_typed() {
    for word in [
        "text ", "expect ", "user ", "push ", "sort ", "search ", "design ",
        "with ", "will ", "would ", "what ", "which ", "when ", "your ",
        "year ", "their ", "work ", "want ", "well ", "way ", "good ", "look ",
    ] {
        assert_eq!(type_str(telex(), word), word, "typing {word:?}");
    }
    // Same via a break key rather than SPACE.
    assert_eq!(type_str(telex(), "text."), "text.");
}

/// The six two-letter words the `english` heuristic is aimed at. Each is also
/// the Telex spelling of a real syllable, so this is the frequency bet the
/// module documents, not a linguistic rule.
#[test]
fn auto_restore_english_handles_the_two_letter_list() {
    for word in ["is ", "us ", "as ", "of ", "if ", "or "] {
        assert_eq!(type_str(telex(), word), word, "typing {word:?}");
    }
}

/// English that composes into a real Vietnamese syllable stays Vietnamese,
/// however rare the syllable (`tét` as in bánh tét). Pinned so a future
/// improvement is a deliberate change rather than an accident.
#[test]
fn auto_restore_english_cannot_detect_valid_vietnamese_syllables() {
    assert_eq!(type_str(telex(), "mix "), "mĩ ");
    assert_eq!(type_str(telex(), "box "), "bõ ");
    assert_eq!(type_str(telex(), "this "), "thí ");
    assert_eq!(type_str(telex(), "been "), "bên ");
    assert_eq!(type_str(telex(), "did "), "đi ");
    assert_eq!(type_str(telex(), "best "), "bét ");
    assert_eq!(type_str(telex(), "these "), "thế ");
    assert_eq!(type_str(telex(), "test "), "tét ");
    assert_eq!(type_str(telex(), "docs "), "dóc ");
}

/// A common English word whose Telex result is not a Vietnamese syllable
/// comes back at the end of the word, including the letters a doubled tone
/// key swallowed.
#[test]
fn auto_restore_english_uses_the_dictionary_at_word_end() {
    for word in ["more ", "down ", "data ", "law ", "message ", "office ", "coffee ", "error "] {
        assert_eq!(type_str(telex(), word), word, "typing {word:?}");
    }
    assert_eq!(type_str(telex(), "message."), "message.");
    // A swallowed double inside a word the dictionary does not know.
    assert_eq!(type_str(telex(), "deferring "), "deferring ");
    // The Telex escape keeps working where it is the likelier intent, and a
    // key typed three times to get a double is left alone.
    assert_eq!(type_str(telex(), "buss "), "bus ");
    assert_eq!(type_str(telex(), "tesst "), "test ");
    assert_eq!(type_str(telex(), "tesst."), "test.");
    assert_eq!(type_str(telex(), "theirr"), "their");
    assert_eq!(type_str(telex(), "aerrial "), "aerial ");
    assert_eq!(type_str(telex(), "offfice "), "office ");
    assert_eq!(type_str(telex(), "ass "), "as ");
    assert_eq!(type_str(telex(), "off "), "of ");
    // A c/ch/p/t syllable still missing its tone may get it later, so it stays
    // when the toned syllable is the more common word (`tốt`, `bột`).
    assert_eq!(type_str(telex(), "toot "), "tôt ");
    assert_eq!(type_str(telex(), "boot "), "bôt ");
    assert_eq!(type_str(telex(), "loop "), "loop ");
    // Off switch.
    let mut o = telex();
    o.auto_restore_english = false;
    assert_eq!(type_str(o, "down "), "dơn ");
}

/// Once a key makes a word with Vietnamese marks impossible as Vietnamese, the
/// raw keys come back immediately rather than at SPACE, and the rest of the
/// word is typed as is.
#[test]
fn auto_restore_english_restores_mid_word() {
    for word in [
        "perfect", "perform", "server", "terminal", "export", "case", "music", "their", "your",
        "various", "cause", "we", "west", "wife",
    ] {
        assert_eq!(type_str(telex(), word), word, "typing {word:?}");
    }
    // c/p finals take only sac or nang, so `pềc` already gives up.
    assert_eq!(type_str(telex(), "perfec"), "perfec");
    // `đc` is a common Vietnamese abbreviation, and a stroke is not a mark.
    assert_eq!(type_str(telex(), "ddc "), "đc ");
    assert_eq!(type_str(telex(), "ddc."), "đc.");
    // Valid Vietnamese is never touched on the way.
    assert_eq!(type_str(telex(), "nguwowfi"), "người");
    assert_eq!(type_str(telex(), "giowf"), "giờ");
    assert_eq!(type_str(telex(), "quyeets"), "quyết");
}

/// DELETE straight after a mid-word restore undoes it, so a stray key in a
/// Vietnamese word costs one backspace. Any other key in between commits the
/// restore.
#[test]
fn delete_right_after_mid_word_restore_undoes_it() {
    assert_eq!(type_str(telex(), "vieejtt"), "vieejtt");
    assert_eq!(type_str(telex(), "vieejtt\u{8}"), "việt");
    assert_eq!(type_str(telex(), "vieejtt\u{8}s"), "viết");
    assert_eq!(type_str(telex(), "vieejtt\u{8} "), "việt ");
    assert_eq!(type_str(telex(), "perfecti\u{8}"), "perfect");
    // Exact even when the triggering vowel made `check_grammar` move the tone.
    assert_eq!(type_str(telex(), "cause\u{8}"), "cáu");
}

/// ESC replays the raw keys only while they still describe the word: after a
/// DELETE erased a composed letter they do not, so ESC passes through.
#[test]
fn esc_after_deleting_a_composed_letter_passes_through() {
    assert_eq!(type_str(telex(), "vieejt\u{8}\u{1b}"), "vieej");
    assert_eq!(type_str(telex(), "tex\u{8}\u{1b}"), "t");
}

/// ESC undoes the composition of the word in progress, including for a valid
/// Vietnamese word -- that is the escape hatch for the case above. The ESC
/// itself is swallowed, so nothing extra lands in the text.
#[test]
fn esc_restores_the_word_in_progress() {
    assert_eq!(type_str(telex(), "mix\u{1b}"), "mix");
    assert_eq!(type_str(telex(), "this\u{1b}"), "this");
    assert_eq!(type_str(telex(), "vieejt\u{1b}"), "vieejt");
    // Recovers a keystroke the duplicate-mark undo consumed, which the raw
    // journal exists for: `off` composes to `of` and only the journal still
    // knows about the second `f`.
    assert_eq!(type_str(telex(), "off\u{1b}"), "off");
}

/// ESC with nothing to undo must reach the application untouched, or it would
/// stop closing dialogs and leaving Vim's insert mode.
#[test]
fn esc_passes_through_when_there_is_nothing_to_restore() {
    let mut e = Engine::new(telex());
    let hs = e.handle_event(KeyEvent::Keyboard, KeyEventState::KeyDown, KEY_ESC, 0, false);
    assert_eq!(hs.code, HookCode::DoNothing, "empty buffer");

    // A word that was typed but never composed: still nothing to undo.
    assert_eq!(type_str(telex(), "things\u{1b}"), "things");
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
    //
    // `auto_restore_english` is off to keep this the oracle-verified path; with
    // it on, the `d` that breaks the spelling restores the raw keys instead.
    let long = "vieejtddaayuwowngthueejnguoiwfkhuyees";
    assert_eq!(long.len(), 37);
    let mut o = telex();
    o.auto_restore_english = false;
    assert_eq!(type_str(o, long), "việtđaayuwowngthueejnguoiwfkhuyees");
    assert_eq!(type_str(telex(), long), long);
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

// -- English auto-restore corpus ---------------------------------------------
//
// Ratchets for the English auto-restore heuristics, measured over whole word
// lists rather than hand-picked cases (see `data/README.md` for the source).
// The English counts may only go down and the Vietnamese one must stay 0: a
// change that improves English by breaking Vietnamese fails here.
//
// `en_heldout.txt` is English the dictionary does NOT contain, so its counts
// show what the dictionary-free rules achieve on their own.

/// The settings that matter for restore, in both tone orthographies.
fn corpus_configs() -> [EngineOptions; 2] {
    let old = telex();
    let mut modern = telex();
    modern.use_modern_orthography = true;
    [old, modern]
}

/// `(shown wrong right before SPACE, still wrong after SPACE, sample)` over a
/// word list, summed over both orthographies.
fn english_misses(words: &str) -> (usize, usize, Vec<String>) {
    let (mut before, mut after, mut sample) = (0, 0, Vec::new());
    for opts in corpus_configs() {
        for line in words.lines() {
            let word = line.split(' ').next().unwrap();
            let mut e = Engine::new(opts);
            let mut field = Vec::new();
            type_into(&mut e, &mut field, word);
            if field.iter().collect::<String>() != word {
                before += 1;
            }
            type_into(&mut e, &mut field, " ");
            let done: String = field.iter().collect();
            if done.trim_end() != word {
                after += 1;
                if sample.len() < 60 {
                    sample.push(format!("{word}->{}", done.trim_end()));
                }
            }
        }
    }
    (before, after, sample)
}

#[test]
fn corpus_vietnamese_is_never_restored() {
    // Restored on purpose by the two-letter heuristic in `english.rs`.
    let two_letter = ["as", "if", "is", "us", "of", "or"];
    let mut changed = Vec::new();
    for opts in corpus_configs() {
        let mut plain = opts;
        plain.auto_restore_english = false;
        plain.restore_if_wrong_spelling = false;
        let mut strict = opts;
        strict.restore_if_wrong_spelling = true;
        for line in include_str!("data/vi_typing.txt").lines() {
            for keys in line.split('\t').filter(|k| !two_letter.contains(k)) {
                let typed = format!("{keys} ");
                let want = type_str(plain, &typed);
                for o in [opts, strict] {
                    let got = type_str(o, &typed);
                    if got != want {
                        changed.push(format!("{keys}: {want:?} -> {got:?}"));
                    }
                }
            }
        }
    }
    changed.dedup();
    assert!(changed.is_empty(), "{} changed: {:?}", changed.len(), changed);
}

#[test]
fn corpus_english_ratchet() {
    let (before, after, sample) = english_misses(include_str!("../data/en_words.txt"));
    println!("dictionary words: wrong before space={before} after space={after}");
    println!("  {}", sample.join(" "));
    let (h_before, h_after, h_sample) = english_misses(include_str!("data/en_heldout.txt"));
    println!("held-out words: wrong before space={h_before} after space={h_after}");
    println!("  {}", h_sample.join(" "));
    // Before this work: 17348 / 3992 and 2316 / 454. Keeping the Telex
    // double-key habit working (below) costs a few words with a real double
    // right after an impossible nucleus a letter until SPACE.
    assert!(before <= 3832 && after <= 1216, "dictionary words got worse");
    assert!(h_before <= 425 && h_after <= 224, "held-out words got worse");
}

/// How a Telex user types English today: doubling a tone key where a single
/// one would compose (`tesst` for `test`, `aerrial` for `aerial`). Each line
/// was produced by the engine as it was before English detection (commit
/// be221e6), so it is the muscle memory the new rules must not break. Only
/// the words where the doubled spelling is itself the more common word
/// (`less` typed for `les`) may come out differently.
#[test]
fn corpus_telex_double_key_habit_still_works() {
    let (mut before, mut after, mut sample) = (0, 0, Vec::new());
    for line in include_str!("data/en_escape_typing.txt").lines() {
        let (typed, word) = line.split_once('\t').unwrap();
        if type_str(telex(), typed) != word {
            before += 1;
        }
        let done = type_str(telex(), &format!("{typed} "));
        if done.trim_end() != word {
            after += 1;
            sample.push(format!("{typed}->{}", done.trim_end()));
        }
    }
    assert_eq!(before, 0, "the double-key habit broke while typing");
    assert!(after <= 17, "{after} broke after SPACE: {sample:?}");
}






