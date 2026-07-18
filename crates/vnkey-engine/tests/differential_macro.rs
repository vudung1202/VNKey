//! Differential test: `vnkey_engine`'s macro machinery vs the vendored C++
//! `Macro.cpp`, across five surfaces:
//!
//! 1. blob round-trip -- `MacroTable::to_blob` re-serialized by the oracle
//!    (`getMacroSaveData`) byte-for-byte;
//! 2. direct `findMacro` -- random keycode vectors against a fixed macro set,
//!    over each code table and `auto_caps` in {0, 1} (`ok_find_macro`);
//! 3. typing-path -- `use_macro=1` key sequences driven through both engines,
//!    comparing the full `HookState` *including `macro_data` content* on every
//!    `ReplaceMacro` step;
//! 4. english-mode -- macro expansion on SPACE through `vEnglishMode`;
//! 5. `onTableCodeChange` -- content codes after switching tables.
//!
//! Plus a `use_macro=1` wiring pass with an *empty* macro table, proving the
//! now-live `find_macro` call sites did not perturb the Task 4/5/6 typing path
//! (Task 6's grand differential held `use_macro=0`).

use openkey_oracle::{OkOptions, Oracle};
use proptest::prelude::*;
use vnkey_engine::keys::*;
use vnkey_engine::{CodeTable, Engine, EngineOptions, InputType, KeyEvent, KeyEventState, MacroTable, MAX_BUFF};

// -- shared option marshaling ------------------------------------------------

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

const TABLES: [CodeTable; 5] = [
    CodeTable::Unicode,
    CodeTable::Tcvn3,
    CodeTable::VniWindows,
    CodeTable::UnicodeCompound,
    CodeTable::Cp1258,
];

/// Fixed macro set with ASCII triggers (table-independent keys) and mixed
/// ASCII / accented content. Lowercase triggers so the auto-caps path fires on
/// upper-cased input.
const MACROS: &[(&str, &str)] = &[
    ("ms", "millisecond"),
    ("ko", "không"),
    ("vn", "Việt Nam"),
    ("hn", "Hà Nội"),
    ("tv", "tiếng việt"),
    ("btn", "buổi tối nay"),
    ("a", "á"),
    ("bb", "bye bye"),
];

fn build_macro_table(code_table: CodeTable) -> MacroTable {
    let mut t = MacroTable::new();
    for (text, content) in MACROS {
        t.add(text, content, code_table);
    }
    t
}

fn config(default_cases: u32) -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(default_cases);
    ProptestConfig::with_cases(cases)
}

// -- 1. blob round-trip ------------------------------------------------------

#[test]
fn macro_blob_round_trip_matches_oracle() {
    let t = build_macro_table(CodeTable::Unicode);
    let blob = t.to_blob();

    let opts = OkOptions::default(); // code_table = 0 (Unicode)
    let session = {
        let mut s = Oracle::lock(&opts);
        s.set_macro_blob(&blob);
        s
    };
    let oracle_blob = session.get_macro_blob(65536);
    assert_eq!(blob, oracle_blob, "macro blob must be byte-exact with OpenKey");

    // Rust re-parses its own blob identically.
    let t2 = MacroTable::from_blob(&blob);
    assert_eq!(blob, t2.to_blob());
    assert_eq!(t.all(), t2.all());
}

// -- 2. direct findMacro differential ---------------------------------------

/// Keycode pool for random `find` keys: the trigger letters plus a few others,
/// so hits (exact + auto-caps) and misses are both common.
const KEY_POOL: [u16; 12] = [
    KEY_M, KEY_S, KEY_K, KEY_O, KEY_V, KEY_N, KEY_H, KEY_A, KEY_B, KEY_T, KEY_I, KEY_E,
];

fn find_key_strategy() -> impl Strategy<Value = Vec<u32>> {
    let cell = (0usize..KEY_POOL.len(), prop_oneof![3 => Just(0u32), 1 => Just(CAPS)])
        .prop_map(|(i, caps)| KEY_POOL[i] as u32 | caps);
    prop::collection::vec(cell, 0..=4)
}

const CAPS: u32 = 0x10000;

proptest! {
    #![proptest_config(config(5000))]

    #[test]
    fn find_macro_matches_oracle(
        table_idx in 0usize..5,
        auto_caps in any::<bool>(),
        key in find_key_strategy(),
    ) {
        let code_table = TABLES[table_idx];
        let ct = code_table as i32;

        // Build the identical macro set in both engines under `code_table`.
        let rust = build_macro_table(code_table);
        let opts = OkOptions { code_table: ct, auto_caps_macro: auto_caps as i32, ..OkOptions::default() };
        let mut session = Oracle::lock(&opts);
        for (text, content) in MACROS {
            session.add_macro(text, content);
        }

        let rust_res = rust.find(&key, auto_caps);
        let oracle_res = session.find_macro(&key, ct, auto_caps);
        prop_assert_eq!(
            rust_res, oracle_res,
            "find mismatch: table={:?} auto_caps={} key={:?}",
            code_table, auto_caps, key
        );
    }
}

// -- 3. typing-path macro_data differential ---------------------------------

/// Compare a `use_macro` typing sequence through both engines, asserting the
/// full `HookState` (incl. `macro_data` content on `ReplaceMacro`) matches.
fn compare_macro_typing(
    opts: EngineOptions,
    blob: &[u8],
    seq: &[(u16, u8)],
) -> Result<(), TestCaseError> {
    let ok_opts = to_ok_options(&opts);
    let mut session = Oracle::lock(&ok_opts);
    session.set_macro_blob(blob);

    let mut engine = Engine::new(opts);
    engine.set_macro_table(MacroTable::from_blob(blob));

    const REPLACE_MACRO: u8 = 4;

    for (step, &(keycode, caps)) in seq.iter().enumerate() {
        let hs = engine.handle_event(KeyEvent::Keyboard, KeyEventState::KeyDown, keycode, caps, false);
        let rust_code = hs.code as u8;
        let bpc = hs.backspace_count;
        let ncc = hs.new_char_count;
        let ext_code = hs.ext_code;
        let char_data = hs.char_data;
        let macro_key_len = hs.macro_key.len();
        let macro_data = hs.macro_data.clone();

        session.handle_event(0, 0, keycode, caps, 0);
        let st = session.get_state();

        prop_assert_eq!(
            (rust_code, bpc, ncc, ext_code),
            (st.code, st.backspace_count, st.new_char_count, st.ext_code),
            "header mismatch at step {} (keycode={}, caps={}) opts={:?}\n  seq={:?}",
            step, keycode, caps, opts, seq
        );

        let n = (ncc as usize).min(MAX_BUFF);
        if !engine.debug_char_data_may_be_cpp_oob() {
            prop_assert_eq!(
                &char_data[..n],
                &st.char_data[..n],
                "char_data mismatch at step {} (keycode={}, caps={}) opts={:?}\n  seq={:?}",
                step, keycode, caps, opts, seq
            );
        }

        prop_assert_eq!(
            macro_key_len,
            st.macro_key_len as usize,
            "macro_key length mismatch at step {} opts={:?}\n  seq={:?}",
            step, opts, seq
        );

        // On a macro replacement, compare the emitted content codes.
        if rust_code == REPLACE_MACRO {
            let oracle_data = &st.macro_data[..st.macro_data_len as usize];
            prop_assert_eq!(
                macro_data.as_slice(),
                oracle_data,
                "macro_data mismatch at step {} (keycode={}, caps={}) opts={:?}\n  seq={:?}",
                step, keycode, caps, opts, seq
            );
        }
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
        'z' => KEY_Z, ' ' => KEY_SPACE, '.' => KEY_DOT, ',' => KEY_COMMA,
        other => panic!("unmapped char {other:?}"),
    };
    (code, if up { 1 } else { 0 })
}

fn seq_of(s: &str) -> Vec<(u16, u8)> {
    s.chars().map(key_of).collect()
}

fn macro_opts(code_table: CodeTable, input_type: InputType, auto_caps: bool) -> EngineOptions {
    EngineOptions {
        input_type,
        code_table,
        use_macro: true,
        auto_caps_macro: auto_caps,
        use_smart_switch_key: false,
        ..EngineOptions::default()
    }
}

#[test]
fn macro_typing_hand_cases() {
    let blob = build_macro_table(CodeTable::Unicode).to_blob();
    // (auto_caps, typed string): trigger followed by a break/space that fires
    // the macro. Includes auto-caps upper/capitalized triggers.
    let cases: &[(bool, &str)] = &[
        (false, "ms "),
        (false, "ko "),
        (false, "vn "),
        (false, "hn."),
        (false, "bb "),
        (false, "a "),
        (true, "ko "),
        (true, "Ko "),
        (true, "KO "),
        (true, "VN "),
        (true, "Vn "),
        (false, "xyz "), // non-trigger: no macro, still must match
        (false, "ms ko "), // two macros in a row
    ];
    for &(auto_caps, s) in cases {
        let opts = macro_opts(CodeTable::Unicode, InputType::Telex, auto_caps);
        compare_macro_typing(opts, &blob, &seq_of(s))
            .unwrap_or_else(|e| panic!("auto_caps={auto_caps} {s:?}: {e:?}"));
    }
}

/// Key mix biased toward the trigger letters + spaces/breaks so macros fire.
fn macro_key_strategy() -> impl Strategy<Value = (u16, u8)> {
    let trigger_letters = [KEY_M, KEY_S, KEY_K, KEY_O, KEY_V, KEY_N, KEY_H, KEY_A, KEY_B, KEY_T];
    let letter = (0usize..trigger_letters.len(), prop_oneof![6 => Just(0u8), 1 => Just(1u8), 1 => Just(2u8)])
        .prop_map(move |(i, c)| (trigger_letters[i], c));
    prop_oneof![
        70 => letter,
        15 => Just((KEY_SPACE, 0u8)),
        8 => Just((KEY_DOT, 0u8)),
        7 => Just((KEY_RETURN, 0u8)),
    ]
}

fn macro_seq_strategy() -> impl Strategy<Value = Vec<(u16, u8)>> {
    prop::collection::vec(macro_key_strategy(), 1..=30)
}

fn input_type_of(i: u8) -> InputType {
    match i {
        0 => InputType::Telex,
        1 => InputType::Vni,
        2 => InputType::SimpleTelex1,
        _ => InputType::SimpleTelex2,
    }
}

proptest! {
    #![proptest_config(config(5000))]

    /// use_macro=1 typing over all 5 tables x 4 input types x auto_caps{0,1},
    /// comparing macro_data content when a macro fires.
    #[test]
    fn macro_typing_differential(
        table_idx in 0usize..5,
        it in 0u8..4,
        auto_caps in any::<bool>(),
        seq in macro_seq_strategy(),
    ) {
        let code_table = TABLES[table_idx];
        let opts = macro_opts(code_table, input_type_of(it), auto_caps);
        let blob = build_macro_table(code_table).to_blob();
        compare_macro_typing(opts, &blob, &seq)?;
    }
}

// -- 4. english-mode macro differential -------------------------------------

proptest! {
    #![proptest_config(config(2000))]

    /// vEnglishMode fires findMacro on SPACE; drive both with a loaded macro
    /// set and compare code/backspace_count/macro_data.
    #[test]
    fn english_mode_macro_differential(
        auto_caps in any::<bool>(),
        seq in macro_seq_strategy(),
    ) {
        let mut opts = macro_opts(CodeTable::Unicode, InputType::Telex, auto_caps);
        opts.language = 0;
        opts.use_macro_in_english_mode = true;
        let ok_opts = to_ok_options(&opts);
        let blob = build_macro_table(CodeTable::Unicode).to_blob();

        let mut session = Oracle::lock(&ok_opts);
        session.set_macro_blob(&blob);
        let mut engine = Engine::new(opts);
        engine.set_macro_table(MacroTable::from_blob(&blob));

        const REPLACE_MACRO: u8 = 4;
        for (step, &(keycode, caps)) in seq.iter().enumerate() {
            let is_caps = caps == 1 || caps == 2;
            let hs = engine.english_mode(KeyEventState::KeyDown, keycode, is_caps, false);
            let rust_code = hs.code as u8;
            let bpc = hs.backspace_count;
            let macro_key_len = hs.macro_key.len();
            let macro_data = hs.macro_data.clone();

            session.english_mode(0, keycode, caps, 0);
            let st = session.get_state();

            prop_assert_eq!(
                (rust_code, bpc),
                (st.code, st.backspace_count),
                "english_mode header mismatch at step {} (keycode={}) auto_caps={}\n  seq={:?}",
                step, keycode, auto_caps, seq
            );
            prop_assert_eq!(macro_key_len, st.macro_key_len as usize,
                "english_mode macro_key len mismatch at step {}\n  seq={:?}", step, seq);
            if rust_code == REPLACE_MACRO {
                prop_assert_eq!(
                    macro_data.as_slice(),
                    &st.macro_data[..st.macro_data_len as usize],
                    "english_mode macro_data mismatch at step {} (keycode={})\n  seq={:?}",
                    step, keycode, seq
                );
            }
        }
    }
}

// -- 5. onTableCodeChange differential --------------------------------------

#[test]
fn on_table_code_change_matches_oracle() {
    // Add macros under Unicode, then switch to each other table on both sides
    // and confirm the recomputed content codes agree (via findMacro on the
    // ASCII trigger, which stays a table-independent key).
    for &to in &TABLES {
        let mut rust = build_macro_table(CodeTable::Unicode);
        rust.on_table_code_change(to);

        let opts = OkOptions::default(); // Unicode
        let mut session = Oracle::lock(&opts);
        for (text, content) in MACROS {
            session.add_macro(text, content);
        }
        session.macro_on_table_change(to as i32);

        for (text, _content) in MACROS {
            // The accumulated macroKey for an ASCII trigger is its keycodes.
            let key: Vec<u32> = text
                .chars()
                .map(|c| key_of(c).0 as u32)
                .collect();
            let rust_res = rust.find(&key, false);
            let oracle_res = session.find_macro(&key, to as i32, false);
            assert_eq!(
                rust_res, oracle_res,
                "onTableCodeChange content mismatch for {text:?} -> table {to:?}"
            );
        }
    }
}

// -- 6. use_macro=1 wiring pass (empty macro table) -------------------------

/// General key mix (letters/brackets/digits/SPACE/DELETE/breaks) -- same shape
/// as the Task 5/6 general strategy, so this pass re-exercises the whole typing
/// path with `use_macro=1` and an empty macro table.
const SCAN_KEYCODES: [u16; 28] = [
    KEY_A, KEY_B, KEY_C, KEY_D, KEY_E, KEY_F, KEY_G, KEY_H, KEY_I, KEY_J, KEY_K, KEY_L, KEY_M,
    KEY_N, KEY_O, KEY_P, KEY_Q, KEY_R, KEY_S, KEY_T, KEY_U, KEY_V, KEY_W, KEY_X, KEY_Y, KEY_Z,
    KEY_LEFT_BRACKET, KEY_RIGHT_BRACKET,
];

fn wiring_key_strategy() -> impl Strategy<Value = (u16, u8)> {
    let caps = prop_oneof![10 => Just(0u8), 1 => Just(1u8), 1 => Just(2u8)];
    let letter = ((0usize..SCAN_KEYCODES.len()), caps).prop_map(|(i, c)| (SCAN_KEYCODES[i], c));
    prop_oneof![
        65 => letter,
        15 => Just((KEY_SPACE, 0u8)),
        12 => Just((KEY_DELETE, 0u8)),
        8 => Just((KEY_DOT, 0u8)),
    ]
}

proptest! {
    #![proptest_config(config(5000))]

    /// Wiring proof: `use_macro=1`, empty macro table (find always misses), so
    /// the only behavior beyond Task 6's `use_macro=0` grand pass is
    /// `macro_key` accumulation -- asserted equal here alongside the full
    /// HookState. Confirms the live `find_macro` call sites did not break the
    /// ported typing path.
    #[test]
    fn use_macro_wiring_empty_table(
        table_idx in 0usize..5,
        it in 0u8..4,
        auto_caps in any::<bool>(),
        seq in prop::collection::vec(wiring_key_strategy(), 1..=45),
    ) {
        let opts = macro_opts(TABLES[table_idx], input_type_of(it), auto_caps);
        // Empty blob (count=0): both engines start with no macros.
        let blob: Vec<u8> = vec![0u8, 0u8];
        compare_macro_typing(opts, &blob, &seq)?;
    }
}

// -- 7. UniKey .txt round-trip ----------------------------------------------

#[test]
fn unikey_txt_export_import_round_trip() {
    let t = build_macro_table(CodeTable::Unicode);
    let txt = t.export_unikey_txt();
    assert!(txt.starts_with(
        ";Compatible OpenKey Macro Data file for UniKey*** version=1 ***\n"
    ));
    let mut t2 = MacroTable::new();
    let added = t2.import_unikey_txt(&txt, false, CodeTable::Unicode);
    assert_eq!(added, MACROS.len());
    assert_eq!(t.all(), t2.all());
    // Blob (and thus the converted keys/content) also match.
    assert_eq!(t.to_blob(), t2.to_blob());
}

#[test]
fn unikey_txt_handles_colon_in_macro_name() {
    // A macro whose name itself contains a colon: readFromFile reattaches the
    // leading ':' run (its `while (name == "")` loop).
    let mut t = MacroTable::new();
    let txt = ";header line\n:cc:content here\n";
    t.import_unikey_txt(txt, false, CodeTable::Unicode);
    // The name becomes ":cc" (leading empty split reattached with ':').
    assert!(t.has(":cc"));
    let all = t.all();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].0, ":cc");
    assert_eq!(all[0].1, "content here");
}
