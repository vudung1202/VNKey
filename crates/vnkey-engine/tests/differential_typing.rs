//! Differential test: drive `vnkey_engine::Engine` and the vendored C++
//! oracle with the *same* random letter/bracket key sequences under the same
//! options, and assert the per-key `HookState` matches on every field the
//! host consumes: `code`, `backspace_count`, `new_char_count`, `ext_code`
//! and `char_data[0..new_char_count]`.
//!
//! `differential_unicode`/`differential_other_code_tables`/
//! `differential_vni_digits` exercise only the normal-key path (keycodes are
//! letters/brackets/digits, never space/delete/word-break) -- the Task 4
//! surface. Macro/smart-switch/temp-off options are held at 0 throughout this
//! file (out of scope through Task 7); `free_mark` is 0.
//!
//! `differential_general` (Task 5) additionally mixes in SPACE, DELETE and a
//! handful of break keys, and samples restore_if_wrong_spelling/
//! quick_start_consonant/quick_end_consonant, to cover the break/SPACE/
//! DELETE branches Task 5 ports.
//!
//! The oracle is a process-wide singleton behind a mutex; every proptest case
//! re-locks it (which runs `ok_reset`, incl. the VNKEY-ORACLE-PATCH static
//! reset) so no state leaks between cases.

use openkey_oracle::{OkOptions, Oracle};
use proptest::prelude::*;
use vnkey_engine::keys::*;
use vnkey_engine::{CodeTable, Engine, EngineOptions, InputType, KeyEvent, KeyEventState, MAX_BUFF};

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

/// The 10 digit keycodes, used only by the dedicated VNI-digit pass below.
const DIGIT_KEYCODES: [u16; 10] = [
    KEY_0, KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9,
];

/// Options strategy forced to `input_type=Vni`, otherwise identical to
/// `options_strategy` (out-of-scope options held at 0/false).
fn vni_options_strategy() -> impl Strategy<Value = EngineOptions> {
    (any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
        |(modern, spell, zfwj, qtelex, upper)| {
            let mut o = EngineOptions {
                input_type: InputType::Vni,
                code_table: CodeTable::Unicode,
                use_modern_orthography: modern,
                check_spelling: spell,
                allow_consonant_zfwj: zfwj,
                quick_telex: qtelex,
                upper_case_first_char: upper,
                free_mark: false,
                ..EngineOptions::default()
            };
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

/// A digit key press. `capsStatus == 1` is excluded (kept at 0 or 2 only):
/// `(IS_NUMBER_KEY(data) && capsStatus == 1)` is one of the OR'd conditions
/// that routes into the still-stubbed (Task 5) word-break branch, which is
/// out of scope for this pass -- it only validates the already-ported
/// normal-key path (`handleMainKey`/`insertKey`).
fn vni_digit_key_strategy() -> impl Strategy<Value = (u16, u8)> {
    let caps = prop_oneof![10 => Just(0u8), 1 => Just(2u8)];
    ((0usize..DIGIT_KEYCODES.len()), caps).prop_map(|(i, c)| (DIGIT_KEYCODES[i], c))
}

/// A letter key press (first 26 entries of `SCAN_KEYCODES`; the bracket
/// entries are excluded from this pass -- see `vni_seq_strategy`).
fn vni_letter_key_strategy() -> impl Strategy<Value = (u16, u8)> {
    let caps = prop_oneof![10 => Just(0u8), 1 => Just(1u8), 1 => Just(2u8)];
    ((0usize..26usize), caps).prop_map(|(i, c)| (SCAN_KEYCODES[i], c))
}

/// A key press weighted ~35% digits / ~65% letters, for the dedicated
/// VNI-digit pass: `SCAN_KEYCODES` has no digit keys, so under
/// `input_type=Vni` a plain `seq_strategy()` run never exercises the VNI
/// `keyForAEO` KEY_6/7/8 paths or the VNI-only `insertW` vei-rescan branch
/// (`Engine.cpp` `handleMainKey`/`insertW`, ~lines 1055-1193 and 871-984).
fn vni_key_strategy() -> impl Strategy<Value = (u16, u8)> {
    prop_oneof![
        35 => vni_digit_key_strategy(),
        65 => vni_letter_key_strategy(),
    ]
}

/// Sequences for the VNI-digit pass always start with a letter (never a
/// digit) and never contain a bracket key. Both restrictions keep `_index`
/// strictly `> 0` for the whole sequence after the first key, so the
/// sequence can never hit `_index == 0 && IS_NUMBER_KEY(data)` -- the other
/// OR'd condition that routes into the still-stubbed word-break branch
/// (`Engine.cpp:1321-1322`; brackets are the only already-ported way `_index`
/// can reset to 0 mid-sequence, via the "case []" block). This isolates the
/// pass to what Task 4 actually ported.
fn vni_seq_strategy() -> impl Strategy<Value = Vec<(u16, u8)>> {
    (
        vni_letter_key_strategy(),
        prop::collection::vec(vni_key_strategy(), 0..=29),
    )
        .prop_map(|(first, mut rest)| {
            let mut seq = Vec::with_capacity(1 + rest.len());
            seq.push(first);
            seq.append(&mut rest);
            seq
        })
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

/// A representative subset of `_breakCode` (`Engine.cpp:21-28`) used by the
/// general pass's "break key" bucket: enough to exercise the break branch's
/// macro/quick-consonant/restore-if-wrong-spelling sub-branches without
/// drawing from all 18 break keys.
const GENERAL_BREAK_KEYCODES: [u16; 5] = [KEY_ENTER, KEY_RETURN, KEY_DOT, KEY_COMMA, KEY_SLASH];

fn general_break_key_strategy() -> impl Strategy<Value = (u16, u8)> {
    let caps = prop_oneof![10 => Just(0u8), 1 => Just(1u8), 1 => Just(2u8)];
    ((0usize..GENERAL_BREAK_KEYCODES.len()), caps).prop_map(|(i, c)| (GENERAL_BREAK_KEYCODES[i], c))
}

/// A digit key press with the full caps distribution. Unlike
/// `vni_digit_key_strategy`, `capsStatus == 1` is included: with the
/// break/SPACE/DELETE branches now ported, `(IS_NUMBER_KEY(data) &&
/// capsStatus == 1)` routing into the break branch is in scope for this pass.
fn general_digit_key_strategy() -> impl Strategy<Value = (u16, u8)> {
    let caps = prop_oneof![10 => Just(0u8), 1 => Just(1u8), 1 => Just(2u8)];
    ((0usize..DIGIT_KEYCODES.len()), caps).prop_map(|(i, c)| (DIGIT_KEYCODES[i], c))
}

/// General-pass key mix (per brief): letters+brackets ~65%, digits ~10%,
/// SPACE ~10%, DELETE ~10%, a handful of break keys ~5%.
fn general_key_strategy() -> impl Strategy<Value = (u16, u8)> {
    prop_oneof![
        65 => key_strategy(),
        10 => general_digit_key_strategy(),
        10 => Just((KEY_SPACE, 0u8)),
        10 => Just((KEY_DELETE, 0u8)),
        5 => general_break_key_strategy(),
    ]
}

/// Sequence length 1-45 (per brief): long enough to reach MAX_BUFF=32 and
/// exercise `_longWordHelper`.
fn general_seq_strategy() -> impl Strategy<Value = Vec<(u16, u8)>> {
    prop::collection::vec(general_key_strategy(), 1..=45)
}

/// Options for the general pass: everything `options_strategy` draws, plus
/// `restore_if_wrong_spelling`/`quick_start_consonant`/`quick_end_consonant`
/// sampled in `{false, true}`. Macro/smart-switch/temp-off stay off (out of
/// scope through Task 7, per brief).
fn general_options_strategy(code_table: CodeTable) -> impl Strategy<Value = EngineOptions> {
    (
        0u8..4,
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            move |(it, modern, spell, zfwj, qtelex, upper, restore_wrong, qsc, qec)| {
                let mut o = EngineOptions {
                    input_type: input_type_of(it),
                    code_table,
                    use_modern_orthography: modern,
                    check_spelling: spell,
                    allow_consonant_zfwj: zfwj,
                    quick_telex: qtelex,
                    upper_case_first_char: upper,
                    restore_if_wrong_spelling: restore_wrong,
                    quick_start_consonant: qsc,
                    quick_end_consonant: qec,
                    free_mark: false,
                    ..EngineOptions::default()
                };
                o.use_macro = false;
                o.use_macro_in_english_mode = false;
                o.auto_caps_macro = false;
                o.use_smart_switch_key = false;
                o.temp_off_spelling = false;
                o.temp_off_openkey = false;
                o
            },
        )
}

/// Run `seq` through both engines and assert the per-key HookState matches.
fn compare_sequence(opts: EngineOptions, seq: &[(u16, u8)]) -> Result<(), TestCaseError> {
    let ok_opts = to_ok_options(&opts);
    let mut session = Oracle::lock(&ok_opts);
    let mut engine = Engine::new(opts);

    for (step, &(keycode, caps)) in seq.iter().enumerate() {
        let hs = engine.handle_event(KeyEvent::Keyboard, KeyEventState::KeyDown, keycode, caps, false);
        // Copy the fields we need out of `hs` (a `&HookState` tied to
        // `engine`'s mutable borrow) so that borrow ends here -- the OOB
        // check below needs its own `&engine` borrow, which can't coexist
        // with `hs` still being live.
        let rust_code = hs.code as u8;
        let bpc = hs.backspace_count;
        let ncc = hs.new_char_count;
        let ext_code = hs.ext_code;
        let char_data = hs.char_data;
        let macro_key_len = hs.macro_key.len();

        session.handle_event(0 /*Keyboard*/, 0 /*KeyDown*/, keycode, caps, 0);
        let st = session.get_state();

        prop_assert_eq!(
            (rust_code, bpc, ncc, ext_code),
            (st.code, st.backspace_count, st.new_char_count, st.ext_code),
            "header mismatch at step {} (keycode={}, caps={}) opts={:?}\n  seq={:?}",
            step, keycode, caps, opts, seq
        );

        // `new_char_count` (`ncc`) can legitimately exceed `MAX_BUFF` (32):
        // e.g. `checkQuickConsonant` (`Engine.cpp:1237`, ported at
        // `engine.rs` `check_quick_consonant`) sets `newCharCount =
        // _index + 1` *before* the `_index < MAX_BUFF-1` growth guard, so
        // once `_index` has saturated at `MAX_BUFF` (both engines pin the
        // typing-word index there on overflow -- see `Buffer::push`), ncc
        // comes out to `MAX_BUFF + 1` (33). Both `charData` in C++ and
        // `char_data` here are fixed `Uint32[MAX_BUFF]`/`[u32; MAX_BUFF]`
        // arrays, so a `newCharCount` beyond `MAX_BUFF` means the real
        // OpenKey app itself would read/write one slot past the end of
        // `charData` -- a latent buffer overflow in upstream OpenKey, not a
        // porting bug. That header value is still asserted equal above
        // (both engines agree on the exact, faithfully-reproduced ncc,
        // OOB-implying or not); only the char_data *contents* comparison
        // must stop at the array boundary, since index `MAX_BUFF` and
        // beyond is unallocated/UB on the C++ side and not comparable.
        // Clamp both sides' slice to the in-bounds prefix.
        let n = (ncc as usize).min(MAX_BUFF);
        // Some char_data cells can be sourced from a C++ `GET`/getCharacterCode
        // call that reads its `_codeTable` row out of bounds -- genuine C++ UB
        // (e.g. a mark key lands on a consonant cell like KEY_D via the same
        // vsi/vowel-position quirks findAndCalculateVowel/handleOldMark
        // faithfully reproduce; KEY_D's Unicode row has only 2 elements, so
        // `KEY_D|MARK3_MASK` computes markElem=5, out of bounds). The oracle's
        // answer for such a cell is whatever adjacent memory happened to hold,
        // not a comparable value, so skip the char_data assertion for this
        // step per the "C++ UB -> exclude + document" divergence rule (see
        // `Engine::debug_char_data_may_be_cpp_oob` / `charset::cpp_index_would_be_oob`).
        if !engine.debug_char_data_may_be_cpp_oob() {
            prop_assert_eq!(
                &char_data[..n],
                &st.char_data[..n],
                "char_data mismatch at step {} (keycode={}, caps={}) opts={:?}\n  seq={:?}",
                step, keycode, caps, opts, seq
            );
        }

        // macro_key length only (content is Task 7 -- macro lookup is a stub);
        // cheap to check and both sides stay 0 while use_macro=0, which every
        // strategy in this file holds fixed.
        prop_assert_eq!(
            macro_key_len,
            st.macro_key_len as usize,
            "macro_key length mismatch at step {} (keycode={}, caps={}) opts={:?}\n  seq={:?}",
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

proptest! {
    #![proptest_config(config(5000))]

    /// Mandatory pre-Task-5 pass (see task-5-brief.md): `input_type=Vni`
    /// only, with sequences weighted ~35% digit keys. Task 4's normal-key
    /// port was fuzzed only through `SCAN_KEYCODES` (letters+brackets, no
    /// digits), so under VNI the `keyForAEO` KEY_6/7/8 branches and the
    /// VNI-only `insertW` vei-rescan were exercised only by the ~5 hand
    /// written cases in `differential_regression_corpus`. This pass
    /// validates that Task 4 foundation is clean on VNI digit input before
    /// Task 5 builds break/space/delete handling on top of it.
    #[test]
    fn differential_vni_digits(
        opts in vni_options_strategy(),
        seq in vni_seq_strategy(),
    ) {
        compare_sequence(opts, &seq)?;
    }
}

proptest! {
    #![proptest_config(config(2000))]

    /// Task 5 general pass: mixed letters/digits/SPACE/DELETE/break-key
    /// sequences (1-45 keys) under the Unicode code table, with
    /// restore_if_wrong_spelling/quick_start_consonant/quick_end_consonant
    /// sampled alongside the existing option axes. Exercises the break/SPACE/
    /// DELETE branches this task ports, incl. `_typingStates` history restore
    /// across word boundaries and `_longWordHelper` for words > MAX_BUFF keys.
    #[test]
    fn differential_general(
        opts in general_options_strategy(CodeTable::Unicode),
        seq in general_seq_strategy(),
    ) {
        compare_sequence(opts, &seq)?;
    }
}
