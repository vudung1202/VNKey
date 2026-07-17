//! Golden typing tests: drive `Engine::handle_event` with letter/bracket key
//! sequences and apply the resulting `HookState` to a virtual text field,
//! asserting the final Unicode string.
//!
//! The virtual field mirrors what a host application does with a `HookState`
//! (per the Task 4 brief):
//!   * `DoNothing`  -> append the originally typed character.
//!   * `WillProcess`/`Restore` -> delete `backspace_count` chars, then append
//!     `char_data[new_char_count-1 ..= 0]` (the buffer is stored reversed);
//!     `Restore` additionally re-appends the original key.
//!
//! Every expected string here was cross-checked against the C++ oracle while
//! developing the port (the exhaustive check lives in
//! `differential_typing.rs`). Where the brief's shorthand differs from real
//! OpenKey behavior, the oracle-verified value is used and noted.

use vnkey_engine::keys::*;
use vnkey_engine::{
    CAPS_MASK, CHAR_CODE_MASK, CHAR_MASK, Engine, EngineOptions, HookState, InputType, KeyEvent,
    KeyEventState, key_code_to_character,
};

/// Map an ASCII char to `(keycode, caps_status)`. Uppercase -> shift (1).
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

fn apply(field: &mut Vec<char>, orig: char, hs: &HookState) {
    use vnkey_engine::HookCode::*;
    match hs.code {
        DoNothing => field.push(orig),
        WillProcess | Restore => {
            for _ in 0..hs.backspace_count {
                field.pop();
            }
            for i in (0..hs.new_char_count as usize).rev() {
                field.push(render_cell(hs.char_data[i]));
            }
            if hs.code == Restore {
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
        apply(&mut field, orig, hs);
    }
    field.into_iter().collect()
}

fn telex() -> EngineOptions {
    EngineOptions::default()
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
