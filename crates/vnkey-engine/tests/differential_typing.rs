//! Differential test: drive `vnkey_engine::Engine` and the vendored C++
//! oracle with the *same* random letter/bracket key sequences under the same
//! options, and assert the per-key `HookState` matches on every field the
//! host consumes: `code`, `backspace_count`, `new_char_count`, `ext_code`
//! and `char_data[0..new_char_count]`.
//!
//! Only the normal-key path is exercised (keycodes are the 26 letters plus
//! `[`/`]`, never space/delete/numbers/word-break), which is exactly the
//! surface Task 4 ports. Macro/smart-switch/restore-if-wrong/temp-off options
//! are held at 0 (out of scope); `free_mark` is 0.
//!
//! The oracle is a process-wide singleton behind a mutex; every proptest case
//! re-locks it (which runs `ok_reset`, incl. the VNKEY-ORACLE-PATCH static
//! reset) so no state leaks between cases.

use openkey_oracle::{OkOptions, Oracle};
use proptest::prelude::*;
use vnkey_engine::keys::*;
use vnkey_engine::{CodeTable, Engine, EngineOptions, InputType, KeyEvent, KeyEventState};

/// The 28 keycodes the scan draws from: 26 letters (in `_characterMap` order)
/// plus the two bracket keys.
const SCAN_KEYCODES: [u16; 28] = [
    KEY_A, KEY_B, KEY_C, KEY_D, KEY_E, KEY_F, KEY_G, KEY_H, KEY_I, KEY_J, KEY_K, KEY_L, KEY_M,
    KEY_N, KEY_O, KEY_P, KEY_Q, KEY_R, KEY_S, KEY_T, KEY_U, KEY_V, KEY_W, KEY_X, KEY_Y, KEY_Z,
    KEY_LEFT_BRACKET, KEY_RIGHT_BRACKET,
];

fn to_ok_options(o: &EngineOptions) -> OkOptions {
    OkOptions {
        language: o.language as i32,
        input_type: o.input_type as i32,
        free_mark: o.free_mark as i32,
        code_table: o.code_table as i32,
        switch_key_status: 0,
        check_spelling: o.check_spelling as i32,
        use_modern_orthography: o.use_modern_orthography as i32,
        quick_telex: o.quick_telex as i32,
        restore_if_wrong_spelling: o.restore_if_wrong_spelling as i32,
        fix_recommend_browser: o.fix_recommend_browser as i32,
        use_macro: o.use_macro as i32,
        use_macro_in_english_mode: o.use_macro_in_english_mode as i32,
        auto_caps_macro: o.auto_caps_macro as i32,
        use_smart_switch_key: o.use_smart_switch_key as i32,
        upper_case_first_char: o.upper_case_first_char as i32,
        temp_off_spelling: o.temp_off_spelling as i32,
        allow_consonant_zfwj: o.allow_consonant_zfwj as i32,
        quick_start_consonant: o.quick_start_consonant as i32,
        quick_end_consonant: o.quick_end_consonant as i32,
        remember_code: o.remember_code as i32,
        other_language: o.other_language as i32,
        temp_off_openkey: o.temp_off_openkey as i32,
    }
}

fn input_type_of(i: u8) -> InputType {
    match i {
        0 => InputType::Telex,
        1 => InputType::Vni,
        2 => InputType::SimpleTelex1,
        _ => InputType::SimpleTelex2,
    }
}

/// Options strategy: 4 input types x {modern, check_spelling, allow_zfwj,
/// quick_telex, upper_case_first_char} bools, everything else fixed. Macro /
/// smart-switch / restore-if-wrong / temp-off stay off (out of scope).
fn options_strategy(code_table: CodeTable) -> impl Strategy<Value = EngineOptions> {
    (0u8..4, any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
        move |(it, modern, spell, zfwj, qtelex, upper)| {
            let mut o = EngineOptions {
                input_type: input_type_of(it),
                code_table,
                use_modern_orthography: modern,
                check_spelling: spell,
                allow_consonant_zfwj: zfwj,
                quick_telex: qtelex,
                upper_case_first_char: upper,
                free_mark: false,
                ..EngineOptions::default()
            };
            // Force everything out-of-scope off so the two engines only differ
            // (if at all) on the ported normal-key logic.
            o.use_macro = false;
            o.use_macro_in_english_mode = false;
            o.auto_caps_macro = false;
            o.use_smart_switch_key = false;
            o.restore_if_wrong_spelling = false;
            o.quick_start_consonant = false;
            o.quick_end_consonant = false;
            o.temp_off_spelling = false;
            o.temp_off_openkey = false;
            o
        },
    )
}

/// A key press: `(keycode, caps_status)` with caps weighted toward 0 (none).
fn key_strategy() -> impl Strategy<Value = (u16, u8)> {
    let caps = prop_oneof![10 => Just(0u8), 1 => Just(1u8), 1 => Just(2u8)];
    ((0usize..SCAN_KEYCODES.len()), caps).prop_map(|(i, c)| (SCAN_KEYCODES[i], c))
}

fn seq_strategy() -> impl Strategy<Value = Vec<(u16, u8)>> {
    prop::collection::vec(key_strategy(), 1..=30)
}

/// Full options draw over the four non-Unicode tables (incl. the double-byte
/// VNI / Unicode-Compound / CP1258 encodings that pack two bytes per cell).
fn other_tables_options_strategy() -> impl Strategy<Value = EngineOptions> {
    (0usize..4).prop_flat_map(|idx| {
        let table = [
            CodeTable::Tcvn3,
            CodeTable::VniWindows,
            CodeTable::UnicodeCompound,
            CodeTable::Cp1258,
        ][idx];
        options_strategy(table)
    })
}

/// Run `seq` through both engines and assert the per-key HookState matches.
fn compare_sequence(opts: EngineOptions, seq: &[(u16, u8)]) -> Result<(), TestCaseError> {
    let ok_opts = to_ok_options(&opts);
    let mut session = Oracle::lock(&ok_opts);
    let mut engine = Engine::new(opts);

    for (step, &(keycode, caps)) in seq.iter().enumerate() {
        let hs = engine.handle_event(KeyEvent::Keyboard, KeyEventState::KeyDown, keycode, caps, false);
        session.handle_event(0 /*Keyboard*/, 0 /*KeyDown*/, keycode, caps, 0);
        let st = session.get_state();

        let rust_code = hs.code as u8;
        let ncc = hs.new_char_count;
        prop_assert_eq!(
            (rust_code, hs.backspace_count, hs.new_char_count, hs.ext_code),
            (st.code, st.backspace_count, st.new_char_count, st.ext_code),
            "header mismatch at step {} (keycode={}, caps={}) opts={:?}\n  seq={:?}",
            step, keycode, caps, opts, seq
        );

        let n = ncc as usize;
        prop_assert_eq!(
            &hs.char_data[..n],
            &st.char_data[..n],
            "char_data mismatch at step {} (keycode={}, caps={}) opts={:?}\n  seq={:?}",
            step, keycode, caps, opts, seq
        );
    }
    Ok(())
}

/// Map an ASCII char to `(keycode, caps_status)` (uppercase -> shift).
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
        other => panic!("regression corpus: unmapped char {other:?}"),
    };
    (code, if up { 1 } else { 0 })
}

fn seq_of(s: &str) -> Vec<(u16, u8)> {
    s.chars().map(key_of).collect()
}

/// Deterministic regression corpus: concrete sequences that stress the dense
/// tone/mark-placement paths (modern vs old mark, insertW two-vowel branches,
/// checkGrammar re-placement, quick-telex, brackets, VNI 6/7/8, uppercase).
/// These always run, complementing the randomized passes, and pin the exact
/// inputs used while developing the port.
#[test]
fn differential_regression_corpus() {
    let telex = EngineOptions {
        use_macro: false,
        use_smart_switch_key: false,
        ..EngineOptions::default()
    };
    let vni = EngineOptions { input_type: InputType::Vni, ..telex };
    let st1 = EngineOptions { input_type: InputType::SimpleTelex1, ..telex };
    let st2 = EngineOptions { input_type: InputType::SimpleTelex2, ..telex };
    let modern = EngineOptions { use_modern_orthography: true, ..telex };
    let qtelex = EngineOptions { quick_telex: true, ..telex };
    let nospell = EngineOptions { check_spelling: false, ..telex };
    let upper = EngineOptions { upper_case_first_char: true, ..telex };

    let cases: &[(EngineOptions, &str)] = &[
        (telex, "vieejt"),
        (telex, "ddaay"),
        (telex, "hoaf"),
        (modern, "hoaf"),
        (telex, "uwowng"),
        (telex, "w"),
        (telex, "xoong"),
        (telex, "thueej"),
        (telex, "ss"),
        (telex, "ass"),
        (telex, "asz"),
        (telex, "zz"),
        (telex, "["),
        (telex, "]"),
        (telex, "quww"),      // qu + w standalone edge
        (telex, "nguoiwf"),   // "người" old mark on triphthong-ish
        (telex, "thuowng"),   // "thương"
        (telex, "khuyees"),   // uyê + sac
        (telex, "hoaaf"),     // oa + double-a
        (telex, "giaf"),      // gi + a + huyen (gia)
        (telex, "quangr"),    // qu + ang + hoi
        (telex, "ddoongf"),   // đ + oo + ng + huyen
        (modern, "khuyees"),
        (modern, "hoaif"),
        (telex, "AsDDF"),     // uppercase mixes
        (telex, "Ddaay"),
        (vni, "vie65t"),
        (vni, "d9a6y"),
        (vni, "nguo72i"),     // VNI 7 horn
        (vni, "uo8ng"),       // VNI 8 horn
        (vni, "a821"),        // VNI edge digits
        (st1, "w"),
        (st1, "aw"),
        (st2, "w"),
        (st2, "uwow"),
        (qtelex, "cc"),
        (qtelex, "ggi"),
        (qtelex, "uu"),
        (nospell, "zzz"),
        (nospell, "qjxw"),
        (upper, "abc"),
        (telex, "[]w"),
        (telex, "o]"),
    ];

    for (opts, s) in cases {
        compare_sequence(*opts, &seq_of(s)).unwrap_or_else(|e| panic!("{s:?}: {e:?}"));
    }
}

/// Read `PROPTEST_CASES` (proptest's own env knob) so CI / local runs can
/// scale the case count; default per the brief is >= 2000 for the main pass.
fn config(default_cases: u32) -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(default_cases);
    ProptestConfig::with_cases(cases)
}

proptest! {
    #![proptest_config(config(2000))]

    /// Main pass: Unicode code table (single-byte cells).
    #[test]
    fn differential_unicode(
        opts in options_strategy(CodeTable::Unicode),
        seq in seq_strategy(),
    ) {
        compare_sequence(opts, &seq)?;
    }
}

proptest! {
    #![proptest_config(config(500))]

    /// Secondary pass over the other four code tables, incl. the double-byte
    /// VNI / Unicode-Compound / CP1258 encodings that pack two bytes per cell.
    #[test]
    fn differential_other_code_tables(
        opts in other_tables_options_strategy(),
        seq in seq_strategy(),
    ) {
        compare_sequence(opts, &seq)?;
    }
}
