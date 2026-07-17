//! Differential test: `vnkey_engine::charset::get_character_code` and
//! `key_code_to_character` against the vendored OpenKey C++ engine
//! (`openkey-oracle`), across the full valid cell space x all 5 code
//! tables.
//!
//! Cells where the C++ implementation's `vector<Uint16>` index would fall
//! out of bounds are undefined behavior in C++ (`Engine.cpp:501-558`), so
//! the oracle's answer for them is not trustworthy; they are excluded from
//! comparison via `vnkey_engine::charset::cpp_index_would_be_oob` rather
//! than skipped ad hoc. See that function's doc comment for how the
//! exclusion is derived.

use openkey_oracle::{OkOptions, Oracle};
use vnkey_engine::{
    CAPS_MASK, CodeTable, MARK1_MASK, MARK2_MASK, MARK3_MASK, MARK4_MASK, MARK5_MASK,
    STANDALONE_MASK, TONE_MASK, TONEW_MASK,
};

/// Lives here (vnkey-engine's test code), not in openkey-oracle, so the
/// oracle crate stays independent of vnkey-engine (task-3 brief, step 4).
///
/// A plain function rather than `impl From<&EngineOptions> for OkOptions`:
/// integration tests under `tests/` compile as their own crate, separate
/// from both `vnkey_engine` and `openkey_oracle`, so a `From` impl here
/// would implement a foreign trait for a foreign type with a foreign type
/// parameter -- Rust's orphan rules reject that (E0117) regardless of which
/// file under `tests/` it lives in. A free function has no such
/// restriction.
fn to_ok_options(opts: &vnkey_engine::EngineOptions) -> OkOptions {
    OkOptions {
        language: opts.language as i32,
        input_type: opts.input_type as i32,
        free_mark: opts.free_mark as i32,
        code_table: opts.code_table as i32,
        // EngineOptions has no switch_key_status field (it's runtime
        // switch-hotkey state, not a user-facing option); getCharacterCode
        // and keyCodeToCharacter don't read vSwitchKeyStatus, so 0 is fine.
        switch_key_status: 0,
        check_spelling: opts.check_spelling as i32,
        use_modern_orthography: opts.use_modern_orthography as i32,
        quick_telex: opts.quick_telex as i32,
        restore_if_wrong_spelling: opts.restore_if_wrong_spelling as i32,
        fix_recommend_browser: opts.fix_recommend_browser as i32,
        use_macro: opts.use_macro as i32,
        use_macro_in_english_mode: opts.use_macro_in_english_mode as i32,
        auto_caps_macro: opts.auto_caps_macro as i32,
        use_smart_switch_key: opts.use_smart_switch_key as i32,
        upper_case_first_char: opts.upper_case_first_char as i32,
        temp_off_spelling: opts.temp_off_spelling as i32,
        allow_consonant_zfwj: opts.allow_consonant_zfwj as i32,
        quick_start_consonant: opts.quick_start_consonant as i32,
        quick_end_consonant: opts.quick_end_consonant as i32,
        remember_code: opts.remember_code as i32,
        other_language: opts.other_language as i32,
        temp_off_openkey: opts.temp_off_openkey as i32,
    }
}

const CODE_TABLES: [CodeTable; 5] = [
    CodeTable::Unicode,
    CodeTable::Tcvn3,
    CodeTable::VniWindows,
    CodeTable::UnicodeCompound,
    CodeTable::Cp1258,
];

#[test]
fn get_character_code_matches_cpp_oracle_over_full_cell_space() {
    let engine_opts = vnkey_engine::EngineOptions::default();
    let ok_opts = to_ok_options(&engine_opts);
    let mut session = Oracle::lock(&ok_opts);

    let mut compared: u64 = 0;
    let mut skipped_oob: u64 = 0;

    for keycode in 0u16..=127 {
        for &caps in &[0u32, CAPS_MASK] {
            for &tone in &[0u32, TONE_MASK, TONEW_MASK] {
                for &mark in &[
                    0u32,
                    MARK1_MASK,
                    MARK2_MASK,
                    MARK3_MASK,
                    MARK4_MASK,
                    MARK5_MASK,
                ] {
                    for &standalone in &[0u32, STANDALONE_MASK] {
                        let cell = keycode as u32 | caps | tone | mark | standalone;
                        for &table in &CODE_TABLES {
                            if vnkey_engine::charset::cpp_index_would_be_oob(table, cell) {
                                skipped_oob += 1;
                                continue;
                            }
                            let rust = vnkey_engine::get_character_code(table, cell);
                            let cpp = session.get_character_code(table as i32, cell);
                            assert_eq!(
                                rust, cpp,
                                "mismatch at cell=0x{cell:08X} table={table:?} \
                                 (keycode={keycode}, caps=0x{caps:x}, tone=0x{tone:x}, \
                                 mark=0x{mark:x}, standalone=0x{standalone:x}): \
                                 rust=0x{rust:08X} cpp=0x{cpp:08X}"
                            );
                            compared += 1;
                        }
                    }
                }
            }
        }
    }

    eprintln!(
        "differential_charset: {compared} cells compared, 0 mismatches, \
         {skipped_oob} skipped (C++ index would be out of bounds -- UB)"
    );

    // Sanity bounds so a predicate bug (too broad or too narrow) doesn't
    // silently pass: 128 keycodes x 2 caps x 3 tone x 6 mark x 2 standalone
    // x 5 tables = 46080 total cells. We expect only a handful of UB shapes
    // (KEY_D rows are the sole 2-element rows in the table).
    let total = 128u64 * 2 * 3 * 6 * 2 * 5;
    assert_eq!(compared + skipped_oob, total);
    assert!(
        skipped_oob > 0,
        "expected at least the known KEY_D UB shapes to be skipped"
    );
    assert!(
        skipped_oob < 200,
        "unexpectedly many cells skipped as UB ({skipped_oob}); \
         cpp_index_would_be_oob may be over-broad"
    );
}

#[test]
fn key_code_to_character_matches_cpp_oracle() {
    let engine_opts = vnkey_engine::EngineOptions::default();
    let ok_opts = to_ok_options(&engine_opts);
    let session = Oracle::lock(&ok_opts);

    for keycode in 0u16..=127 {
        for &caps in &[0u32, CAPS_MASK] {
            let cell = keycode as u32 | caps;
            let rust = vnkey_engine::key_code_to_character(cell);
            let cpp = session.key_code_to_character(cell);
            assert_eq!(
                rust, cpp,
                "keyCodeToCharacter mismatch at cell=0x{cell:08X} \
                 (keycode={keycode}, caps=0x{caps:x}): rust={rust} cpp={cpp}"
            );
        }
    }
}
