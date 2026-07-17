//! Vietnamese linguistic data tables.
//!
//! Ported verbatim from OpenKey `Vietnamese.cpp:19-402` (vowel/consonant
//! tables, quick-typing tables) and `Engine.cpp:21-39` (`_breakCode`'s
//! non-`_WIN32` branch, `ProcessingChar`). No state-machine logic lives
//! here (Task 4+ ports `insertMark`/`checkSpelling`/etc., which consume
//! these tables) -- this module is data only.
//!
//! Every table here is currently unread by this task's public surface
//! (`get_character_code`/`key_code_to_character` in `charset.rs`); their
//! only consumers are the not-yet-ported state-machine functions in
//! `Engine.cpp`. Kept `pub(crate)` per the brief so Task 4+ can use them
//! once ported.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::data::{CONSONANT_ALLOW_MASK, END_CONSONANT_MASK, TONE_MASK, TONEW_MASK};
use crate::keys::*;

/// `END_CONSONANT_MASK` truncated to `u16` -- Vietnamese.cpp's vowel/consonant
/// tables are `vector<Uint16>`/`vector<vector<Uint16>>`, and the mask
/// (`0x4000`) fits within 16 bits, so C++'s `KEY_x | END_CONSONANT_MASK` on a
/// `Uint16` vector element is reproduced with this narrowed constant.
const EC: u16 = END_CONSONANT_MASK as u16;
/// `CONSONANT_ALLOW_MASK` truncated to `u16`, same rationale as `EC`.
const CA: u16 = CONSONANT_ALLOW_MASK as u16;

/// `Vietnamese.cpp:19-97 _vowel`.
fn vowel_table() -> HashMap<u16, Vec<Vec<u16>>> {
    HashMap::from([
        (KEY_A, vec![
            vec![KEY_A, KEY_N, KEY_G],
            vec![KEY_A, KEY_G | EC],
            vec![KEY_A, KEY_N],
            vec![KEY_A, KEY_M],
            vec![KEY_A, KEY_U],
            vec![KEY_A, KEY_Y],
            vec![KEY_A, KEY_T],
            vec![KEY_A, KEY_P],
            vec![KEY_A],
            vec![KEY_A, KEY_C],
        ]),
        (KEY_O, vec![
            vec![KEY_O, KEY_N, KEY_G],
            vec![KEY_O, KEY_G | EC],
            vec![KEY_O, KEY_N],
            vec![KEY_O, KEY_M],
            vec![KEY_O, KEY_I],
            vec![KEY_O, KEY_C],
            vec![KEY_O, KEY_T],
            vec![KEY_O, KEY_P],
            vec![KEY_O],
        ]),
        (KEY_E, vec![
            vec![KEY_E, KEY_N, KEY_H],
            vec![KEY_E, KEY_H | EC],
            vec![KEY_E, KEY_N, KEY_G],
            vec![KEY_E, KEY_G | EC],
            vec![KEY_E, KEY_C, KEY_H],
            vec![KEY_E, KEY_K | EC],
            vec![KEY_E, KEY_C],
            vec![KEY_E, KEY_T],
            vec![KEY_E, KEY_Y],
            vec![KEY_E, KEY_U],
            vec![KEY_E, KEY_P],
            vec![KEY_E, KEY_C],
            vec![KEY_E, KEY_N],
            vec![KEY_E, KEY_M],
            vec![KEY_E],
        ]),
        (KEY_W, vec![
            vec![KEY_O, KEY_N],
            vec![KEY_U, KEY_O, KEY_N, KEY_G],
            vec![KEY_U, KEY_O, KEY_G | EC],
            vec![KEY_U, KEY_O, KEY_N],
            vec![KEY_U, KEY_O, KEY_I],
            vec![KEY_U, KEY_O, KEY_C],
            vec![KEY_O, KEY_I],
            vec![KEY_O, KEY_P],
            vec![KEY_O, KEY_M],
            vec![KEY_O, KEY_A],
            vec![KEY_O, KEY_T],
            vec![KEY_U, KEY_N, KEY_G],
            vec![KEY_U, KEY_G | EC],
            vec![KEY_A, KEY_N, KEY_G],
            vec![KEY_A, KEY_G | EC],
            vec![KEY_U, KEY_N],
            vec![KEY_U, KEY_M],
            vec![KEY_U, KEY_C],
            vec![KEY_U, KEY_A],
            vec![KEY_U, KEY_I],
            vec![KEY_U, KEY_T],
            vec![KEY_U],
            vec![KEY_A, KEY_P],
            vec![KEY_A, KEY_T],
            vec![KEY_A, KEY_M],
            vec![KEY_A, KEY_N],
            vec![KEY_A],
            vec![KEY_A, KEY_C],
            vec![KEY_A, KEY_C, KEY_H],
            vec![KEY_A, KEY_K | EC],
            vec![KEY_O],
            vec![KEY_U, KEY_U],
        ]),
    ])
}

/// `Vietnamese.cpp:244-332 _vowelForMark`.
fn vowel_for_mark_table() -> HashMap<u16, Vec<Vec<u16>>> {
    HashMap::from([
        (KEY_A, vec![
            vec![KEY_A, KEY_N, KEY_G],
            vec![KEY_A, KEY_G | EC],
            vec![KEY_A, KEY_N],
            vec![KEY_A, KEY_N, KEY_H],
            vec![KEY_A, KEY_H | EC],
            vec![KEY_A, KEY_M],
            vec![KEY_A, KEY_U],
            vec![KEY_A, KEY_Y],
            vec![KEY_A, KEY_T],
            vec![KEY_A, KEY_P],
            vec![KEY_A],
            vec![KEY_A, KEY_C],
            vec![KEY_A, KEY_I],
            vec![KEY_A, KEY_O],
            vec![KEY_A, KEY_C, KEY_H],
            vec![KEY_A, KEY_K | EC],
        ]),
        (KEY_O, vec![
            vec![KEY_O, KEY_O, KEY_N, KEY_G],
            vec![KEY_O, KEY_O, KEY_G | EC],
            vec![KEY_O, KEY_N, KEY_G],
            vec![KEY_O, KEY_G | EC],
            vec![KEY_O, KEY_O, KEY_N],
            vec![KEY_O, KEY_O, KEY_C],
            vec![KEY_O, KEY_O],
            vec![KEY_O, KEY_N],
            vec![KEY_O, KEY_M],
            vec![KEY_O, KEY_I],
            vec![KEY_O, KEY_C],
            vec![KEY_O, KEY_T],
            vec![KEY_O, KEY_P],
            vec![KEY_O],
        ]),
        (KEY_E, vec![
            vec![KEY_E, KEY_N, KEY_H],
            vec![KEY_E, KEY_H | EC],
            vec![KEY_E, KEY_N, KEY_G],
            vec![KEY_E, KEY_G | EC],
            vec![KEY_E, KEY_C, KEY_H],
            vec![KEY_E, KEY_K | EC],
            vec![KEY_E, KEY_C],
            vec![KEY_E, KEY_T],
            vec![KEY_E, KEY_Y],
            vec![KEY_E, KEY_U],
            vec![KEY_E, KEY_P],
            vec![KEY_E, KEY_C],
            vec![KEY_E, KEY_N],
            vec![KEY_E, KEY_M],
            vec![KEY_E],
        ]),
        (KEY_I, vec![
            vec![KEY_I, KEY_N, KEY_H],
            vec![KEY_I, KEY_H | EC],
            vec![KEY_I, KEY_C, KEY_H],
            vec![KEY_I, KEY_K | EC],
            vec![KEY_I, KEY_N],
            vec![KEY_I, KEY_T],
            vec![KEY_I, KEY_U],
            vec![KEY_I, KEY_U, KEY_P],
            vec![KEY_I, KEY_N],
            vec![KEY_I, KEY_M],
            vec![KEY_I, KEY_P],
            vec![KEY_I, KEY_A],
            vec![KEY_I, KEY_C],
            vec![KEY_I],
        ]),
        (KEY_U, vec![
            vec![KEY_U, KEY_N, KEY_G],
            vec![KEY_U, KEY_G | EC],
            vec![KEY_U, KEY_I],
            vec![KEY_U, KEY_O],
            vec![KEY_U, KEY_Y],
            vec![KEY_U, KEY_Y, KEY_N],
            vec![KEY_U, KEY_Y, KEY_T],
            vec![KEY_U, KEY_Y, KEY_P],
            vec![KEY_U, KEY_Y, KEY_C, KEY_H],
            vec![KEY_U, KEY_Y, KEY_K | EC],
            vec![KEY_U, KEY_Y, KEY_N, KEY_H],
            vec![KEY_U, KEY_Y, KEY_H | EC],
            vec![KEY_U, KEY_T],
            vec![KEY_U, KEY_U],
            vec![KEY_U, KEY_A],
            vec![KEY_U, KEY_I],
            vec![KEY_U, KEY_C],
            vec![KEY_U, KEY_N],
            vec![KEY_U, KEY_M],
            vec![KEY_U, KEY_P],
            vec![KEY_U],
        ]),
        (KEY_Y, vec![
            vec![KEY_Y],
        ]),
    ])
}

/// `Vietnamese.cpp:99-168 _vowelCombine`.
fn vowel_combine_table() -> HashMap<u16, Vec<Vec<u32>>> {
    HashMap::from([
        (KEY_A, vec![
            vec![0, KEY_A as u32, KEY_I as u32],
            vec![0, KEY_A as u32, KEY_O as u32],
            vec![0, KEY_A as u32, KEY_U as u32],
            vec![0, KEY_A as u32 | TONE_MASK, KEY_U as u32],
            vec![0, KEY_A as u32, KEY_Y as u32],
            vec![0, KEY_A as u32 | TONE_MASK, KEY_Y as u32],
        ]),
        (KEY_E, vec![
            vec![0, KEY_E as u32, KEY_O as u32],
            vec![0, KEY_E as u32 | TONE_MASK, KEY_U as u32],
        ]),
        (KEY_I, vec![
            vec![1, KEY_I as u32, KEY_E as u32 | TONE_MASK, KEY_U as u32],
            vec![0, KEY_I as u32, KEY_A as u32],
            vec![1, KEY_I as u32, KEY_E as u32 | TONE_MASK],
            vec![0, KEY_I as u32, KEY_U as u32],
        ]),
        (KEY_O, vec![
            vec![0, KEY_O as u32, KEY_A as u32, KEY_I as u32],
            vec![0, KEY_O as u32, KEY_A as u32, KEY_O as u32],
            vec![0, KEY_O as u32, KEY_A as u32, KEY_Y as u32],
            vec![0, KEY_O as u32, KEY_E as u32, KEY_O as u32],
            vec![1, KEY_O as u32, KEY_A as u32],
            vec![1, KEY_O as u32, KEY_A as u32 | TONEW_MASK],
            vec![1, KEY_O as u32, KEY_E as u32],
            vec![0, KEY_O as u32, KEY_I as u32],
            vec![0, KEY_O as u32 | TONE_MASK, KEY_I as u32],
            vec![0, KEY_O as u32 | TONEW_MASK, KEY_I as u32],
            vec![1, KEY_O as u32, KEY_O as u32],
            vec![1, KEY_O as u32 | TONE_MASK, KEY_O as u32 | TONE_MASK],
        ]),
        (KEY_U, vec![
            vec![0, KEY_U as u32, KEY_Y as u32, KEY_U as u32],
            vec![1, KEY_U as u32, KEY_Y as u32, KEY_E as u32 | TONE_MASK],
            vec![0, KEY_U as u32, KEY_Y as u32, KEY_A as u32],
            vec![0, KEY_U as u32 | TONEW_MASK, KEY_O as u32 | TONEW_MASK, KEY_U as u32],
            vec![0, KEY_U as u32 | TONEW_MASK, KEY_O as u32 | TONEW_MASK, KEY_I as u32],
            vec![0, KEY_U as u32, KEY_O as u32 | TONE_MASK, KEY_I as u32],
            vec![0, KEY_U as u32, KEY_A as u32 | TONE_MASK, KEY_Y as u32],
            vec![1, KEY_U as u32, KEY_A as u32, KEY_O as u32],
            vec![1, KEY_U as u32, KEY_A as u32],
            vec![1, KEY_U as u32, KEY_A as u32 | TONEW_MASK],
            vec![1, KEY_U as u32, KEY_A as u32 | TONE_MASK],
            vec![0, KEY_U as u32 | TONEW_MASK, KEY_A as u32],
            vec![1, KEY_U as u32, KEY_E as u32 | TONE_MASK],
            vec![0, KEY_U as u32, KEY_I as u32],
            vec![0, KEY_U as u32 | TONEW_MASK, KEY_I as u32],
            vec![1, KEY_U as u32, KEY_O as u32],
            vec![1, KEY_U as u32, KEY_O as u32 | TONE_MASK],
            vec![0, KEY_U as u32, KEY_O as u32 | TONEW_MASK],
            vec![1, KEY_U as u32 | TONEW_MASK, KEY_O as u32 | TONEW_MASK],
            vec![0, KEY_U as u32 | TONEW_MASK, KEY_U as u32],
            vec![1, KEY_U as u32, KEY_Y as u32],
        ]),
        (KEY_Y, vec![
            vec![0, KEY_Y as u32, KEY_E as u32 | TONE_MASK, KEY_U as u32],
            vec![1, KEY_Y as u32, KEY_E as u32 | TONE_MASK],
        ]),
    ])
}

/// `Vietnamese.cpp:170-242 _consonantD` -- D + vowel combinations
/// (D + e/u/i/o/a groups) plus the bare `{KEY_D}` row.
pub(crate) const CONSONANT_D: &[&[u16]] = &[
    &[KEY_D, KEY_E, KEY_N, KEY_H],
    &[KEY_D, KEY_E, KEY_H | EC],
    &[KEY_D, KEY_E, KEY_N, KEY_G],
    &[KEY_D, KEY_E, KEY_G | EC],
    &[KEY_D, KEY_E, KEY_C, KEY_H],
    &[KEY_D, KEY_E, KEY_K | EC],
    &[KEY_D, KEY_E, KEY_N],
    &[KEY_D, KEY_E, KEY_C],
    &[KEY_D, KEY_E, KEY_M],
    &[KEY_D, KEY_E],
    &[KEY_D, KEY_E, KEY_T],
    &[KEY_D, KEY_E, KEY_U],
    &[KEY_D, KEY_E, KEY_O],
    &[KEY_D, KEY_E, KEY_P],
    &[KEY_D, KEY_U, KEY_N, KEY_G],
    &[KEY_D, KEY_U, KEY_G | EC],
    &[KEY_D, KEY_U, KEY_N],
    &[KEY_D, KEY_U, KEY_M],
    &[KEY_D, KEY_U, KEY_C],
    &[KEY_D, KEY_U, KEY_O],
    &[KEY_D, KEY_U, KEY_A],
    &[KEY_D, KEY_U, KEY_O, KEY_I],
    &[KEY_D, KEY_U, KEY_O, KEY_C],
    &[KEY_D, KEY_U, KEY_O, KEY_N],
    &[KEY_D, KEY_U, KEY_O, KEY_N, KEY_G],
    &[KEY_D, KEY_U, KEY_O, KEY_G | EC],
    &[KEY_D, KEY_U],
    &[KEY_D, KEY_U, KEY_P],
    &[KEY_D, KEY_U, KEY_T],
    &[KEY_D, KEY_U, KEY_I],
    &[KEY_D, KEY_I, KEY_C, KEY_H],
    &[KEY_D, KEY_I, KEY_K | EC],
    &[KEY_D, KEY_I, KEY_C],
    &[KEY_D, KEY_I, KEY_N, KEY_H],
    &[KEY_D, KEY_I, KEY_H | EC],
    &[KEY_D, KEY_I, KEY_N],
    &[KEY_D, KEY_I],
    &[KEY_D, KEY_I, KEY_A],
    &[KEY_D, KEY_I, KEY_E],
    &[KEY_D, KEY_I, KEY_E, KEY_C],
    &[KEY_D, KEY_I, KEY_E, KEY_U],
    &[KEY_D, KEY_I, KEY_E, KEY_N],
    &[KEY_D, KEY_I, KEY_E, KEY_M],
    &[KEY_D, KEY_I, KEY_E, KEY_P],
    &[KEY_D, KEY_I, KEY_T],
    &[KEY_D, KEY_O],
    &[KEY_D, KEY_O, KEY_A],
    &[KEY_D, KEY_O, KEY_A, KEY_N],
    &[KEY_D, KEY_O, KEY_A, KEY_N, KEY_G],
    &[KEY_D, KEY_O, KEY_A, KEY_G | EC],
    &[KEY_D, KEY_O, KEY_A, KEY_N, KEY_H],
    &[KEY_D, KEY_O, KEY_A, KEY_H | EC],
    &[KEY_D, KEY_O, KEY_A, KEY_M],
    &[KEY_D, KEY_O, KEY_E],
    &[KEY_D, KEY_O, KEY_I],
    &[KEY_D, KEY_O, KEY_P],
    &[KEY_D, KEY_O, KEY_C],
    &[KEY_D, KEY_O, KEY_N],
    &[KEY_D, KEY_O, KEY_N, KEY_G],
    &[KEY_D, KEY_O, KEY_G | EC],
    &[KEY_D, KEY_O, KEY_M],
    &[KEY_D, KEY_O, KEY_T],
    &[KEY_D, KEY_A],
    &[KEY_D, KEY_A, KEY_T],
    &[KEY_D, KEY_A, KEY_Y],
    &[KEY_D, KEY_A, KEY_U],
    &[KEY_D, KEY_A, KEY_I],
    &[KEY_D, KEY_A, KEY_O],
    &[KEY_D, KEY_A, KEY_P],
    &[KEY_D, KEY_A, KEY_C],
    &[KEY_D, KEY_A, KEY_C, KEY_H],
    &[KEY_D, KEY_A, KEY_K | EC],
    &[KEY_D, KEY_A, KEY_N],
    &[KEY_D, KEY_A, KEY_N, KEY_H],
    &[KEY_D, KEY_A, KEY_H | EC],
    &[KEY_D, KEY_A, KEY_N, KEY_G],
    &[KEY_D, KEY_A, KEY_G | EC],
    &[KEY_D, KEY_A, KEY_M],
    &[KEY_D],
];

/// `Vietnamese.cpp:334-368 _consonantTable`.
pub(crate) const CONSONANT_TABLE: &[&[u16]] = &[
    &[KEY_N, KEY_G, KEY_H],
    &[KEY_P, KEY_H],
    &[KEY_T, KEY_H],
    &[KEY_T, KEY_R],
    &[KEY_G, KEY_I],
    &[KEY_C, KEY_H],
    &[KEY_N, KEY_H],
    &[KEY_N, KEY_G],
    &[KEY_K, KEY_H],
    &[KEY_G, KEY_H],
    &[KEY_G],
    &[KEY_C],
    &[KEY_Q],
    &[KEY_K],
    &[KEY_T],
    &[KEY_R],
    &[KEY_H],
    &[KEY_B],
    &[KEY_M],
    &[KEY_V],
    &[KEY_N],
    &[KEY_L],
    &[KEY_X],
    &[KEY_P],
    &[KEY_S],
    &[KEY_D],
    &[KEY_F | CA],
    &[KEY_W | CA],
    &[KEY_Z | CA],
    &[KEY_J | CA],
    &[KEY_F | EC],
    &[KEY_W | EC],
    &[KEY_J | EC],
];

/// `Vietnamese.cpp:370-374 _endConsonantTable`.
pub(crate) const END_CONSONANT_TABLE: &[&[u16]] = &[
    &[KEY_T],
    &[KEY_P],
    &[KEY_C],
    &[KEY_N],
    &[KEY_M],
    &[KEY_G | EC],
    &[KEY_K | EC],
    &[KEY_H | EC],
    &[KEY_C, KEY_H],
    &[KEY_N, KEY_H],
    &[KEY_N, KEY_G],
];

/// `Vietnamese.cpp:376-378 _standaloneWbad`.
pub(crate) const STANDALONE_WBAD: &[u16] = &[KEY_W, KEY_E, KEY_Y, KEY_F, KEY_J, KEY_K, KEY_Z];

/// `Vietnamese.cpp:380-390 _doubleWAllowed`.
pub(crate) const DOUBLE_W_ALLOWED: &[&[u16]] = &[
    &[KEY_T, KEY_R],
    &[KEY_T, KEY_H],
    &[KEY_C, KEY_H],
    &[KEY_N, KEY_H],
    &[KEY_N, KEY_G],
    &[KEY_K, KEY_H],
    &[KEY_G, KEY_I],
    &[KEY_P, KEY_H],
    &[KEY_G, KEY_H],
];

/// `Vietnamese.cpp:392-396 _quickStartConsonant`.
fn quick_start_consonant_table() -> HashMap<u16, Vec<u16>> {
    HashMap::from([
        (KEY_F, vec![KEY_P, KEY_H]),
        (KEY_J, vec![KEY_G, KEY_I]),
        (KEY_W, vec![KEY_Q, KEY_U]),
    ])
}

/// `Vietnamese.cpp:398-402 _quickEndConsonant`.
fn quick_end_consonant_table() -> HashMap<u16, Vec<u16>> {
    HashMap::from([
        (KEY_G, vec![KEY_N, KEY_G]),
        (KEY_H, vec![KEY_N, KEY_H]),
        (KEY_K, vec![KEY_C, KEY_H]),
    ])
}

/// `Vietnamese.cpp:497-506 _quickTelex`.
fn quick_telex_table() -> HashMap<u32, Vec<u16>> {
    HashMap::from([
        (KEY_C as u32, vec![KEY_C, KEY_H]),
        (KEY_G as u32, vec![KEY_G, KEY_I]),
        (KEY_K as u32, vec![KEY_K, KEY_H]),
        (KEY_N as u32, vec![KEY_N, KEY_G]),
        (KEY_Q as u32, vec![KEY_Q, KEY_U]),
        (KEY_P as u32, vec![KEY_P, KEY_H]),
        (KEY_T as u32, vec![KEY_T, KEY_H]),
        (KEY_U as u32, vec![KEY_U, KEY_U]),
    ])
}

/// `Engine.cpp:21-28 _breakCode`, non-`_WIN32` branch only (per brief).
/// Note the intentional duplicate `KEY_TAB` (first and last entry) --
/// transcribed verbatim from the C++ source, not "cleaned up".
pub(crate) const BREAK_CODE: &[u8] = &[KEY_ESC as u8, KEY_TAB as u8, KEY_ENTER as u8, KEY_RETURN as u8, KEY_LEFT as u8, KEY_RIGHT as u8, KEY_DOWN as u8, KEY_UP as u8, KEY_COMMA as u8, KEY_DOT as u8, KEY_SLASH as u8, KEY_SEMICOLON as u8, KEY_QUOTE as u8, KEY_BACK_SLASH as u8, KEY_MINUS as u8, KEY_EQUALS as u8, KEY_BACKQUOTE as u8, KEY_TAB as u8];

/// `Engine.cpp:15-19 _charKeyCode`.
pub(crate) const CHAR_KEY_CODE: &[u8] = &[
    KEY_BACKQUOTE as u8, KEY_1 as u8, KEY_2 as u8, KEY_3 as u8, KEY_4 as u8, KEY_5 as u8,
    KEY_6 as u8, KEY_7 as u8, KEY_8 as u8, KEY_9 as u8, KEY_0 as u8, KEY_MINUS as u8,
    KEY_EQUALS as u8, KEY_LEFT_BRACKET as u8, KEY_RIGHT_BRACKET as u8, KEY_BACK_SLASH as u8,
    KEY_SEMICOLON as u8, KEY_QUOTE as u8, KEY_COMMA as u8, KEY_DOT as u8, KEY_SLASH as u8,
];

/// `Engine.cpp:30-32 _macroBreakCode`.
pub(crate) const MACRO_BREAK_CODE: &[u8] = &[
    KEY_RETURN as u8, KEY_COMMA as u8, KEY_DOT as u8, KEY_SLASH as u8, KEY_SEMICOLON as u8,
    KEY_QUOTE as u8, KEY_BACK_SLASH as u8, KEY_MINUS as u8, KEY_EQUALS as u8,
];

/// `Engine.cpp:34-39 ProcessingChar[][11]`. Column order: S F R X J A O E
/// W D Z. Note the intentional duplicate `KEY_6` in the VNI row (columns
/// 5 and 6) -- transcribed verbatim from the C++ source, not "fixed".
pub(crate) const PROCESSING_CHAR: [[u16; 11]; 4] = [
    [KEY_S, KEY_F, KEY_R, KEY_X, KEY_J, KEY_A, KEY_O, KEY_E, KEY_W, KEY_D, KEY_Z], // Telex
    [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0], // VNI
    [KEY_S, KEY_F, KEY_R, KEY_X, KEY_J, KEY_A, KEY_O, KEY_E, KEY_W, KEY_D, KEY_Z], // Simple Telex 1
    [KEY_S, KEY_F, KEY_R, KEY_X, KEY_J, KEY_A, KEY_O, KEY_E, KEY_W, KEY_D, KEY_Z], // Simple Telex 2
];

// -- lazily-built map tables (mirrors the C++ `map<...>` globals) --

/// `Vietnamese.cpp:19-97 _vowel`.
pub(crate) static VOWEL: LazyLock<HashMap<u16, Vec<Vec<u16>>>> = LazyLock::new(vowel_table);

/// `Vietnamese.cpp:244-332 _vowelForMark`.
pub(crate) static VOWEL_FOR_MARK: LazyLock<HashMap<u16, Vec<Vec<u16>>>> =
    LazyLock::new(vowel_for_mark_table);

/// `Vietnamese.cpp:99-168 _vowelCombine`. Each row's first `u32` element is
/// the 0/1 "can this cell take an end consonant" flag from the C++ source;
/// the rest are `KEY_x`, optionally OR'd with `TONE_MASK`/`TONEW_MASK`.
pub(crate) static VOWEL_COMBINE: LazyLock<HashMap<u16, Vec<Vec<u32>>>> =
    LazyLock::new(vowel_combine_table);

/// `Vietnamese.cpp:392-396 _quickStartConsonant`.
pub(crate) static QUICK_START_CONSONANT: LazyLock<HashMap<u16, Vec<u16>>> =
    LazyLock::new(quick_start_consonant_table);

/// `Vietnamese.cpp:398-402 _quickEndConsonant`.
pub(crate) static QUICK_END_CONSONANT: LazyLock<HashMap<u16, Vec<u16>>> =
    LazyLock::new(quick_end_consonant_table);

/// `Vietnamese.cpp:497-506 _quickTelex`.
pub(crate) static QUICK_TELEX: LazyLock<HashMap<u32, Vec<u16>>> =
    LazyLock::new(quick_telex_table);

#[cfg(test)]
mod tests {
    use super::*;

    // -- row-count checks (verify against Vietnamese.cpp/Engine.cpp) --

    #[test]
    fn vowel_row_counts_match_source() {
        // Vietnamese.cpp:19-97: KEY_A=10, KEY_O=9, KEY_E=15, KEY_W=32 rows.
        assert_eq!(VOWEL.len(), 4);
        assert_eq!(VOWEL[&KEY_A].len(), 10);
        assert_eq!(VOWEL[&KEY_O].len(), 9);
        assert_eq!(VOWEL[&KEY_E].len(), 15);
        assert_eq!(VOWEL[&KEY_W].len(), 32);
    }

    #[test]
    fn vowel_for_mark_row_counts_match_source() {
        // Vietnamese.cpp:244-332: A=16, O=14, E=15, I=14, U=21, Y=1 rows.
        assert_eq!(VOWEL_FOR_MARK.len(), 6);
        assert_eq!(VOWEL_FOR_MARK[&KEY_A].len(), 16);
        assert_eq!(VOWEL_FOR_MARK[&KEY_O].len(), 14);
        assert_eq!(VOWEL_FOR_MARK[&KEY_E].len(), 15);
        assert_eq!(VOWEL_FOR_MARK[&KEY_I].len(), 14);
        assert_eq!(VOWEL_FOR_MARK[&KEY_U].len(), 21);
        assert_eq!(VOWEL_FOR_MARK[&KEY_Y].len(), 1);
    }

    #[test]
    fn vowel_combine_row_counts_match_source() {
        // Vietnamese.cpp:99-168: A=6, E=2, I=4, O=12, U=21, Y=2 rows.
        assert_eq!(VOWEL_COMBINE.len(), 6);
        assert_eq!(VOWEL_COMBINE[&KEY_A].len(), 6);
        assert_eq!(VOWEL_COMBINE[&KEY_E].len(), 2);
        assert_eq!(VOWEL_COMBINE[&KEY_I].len(), 4);
        assert_eq!(VOWEL_COMBINE[&KEY_O].len(), 12);
        assert_eq!(VOWEL_COMBINE[&KEY_U].len(), 21);
        assert_eq!(VOWEL_COMBINE[&KEY_Y].len(), 2);
    }

    #[test]
    fn flat_table_row_counts_match_source() {
        // Vietnamese.cpp:170-242 (14+16+15+17+16+1), :334-368, :370-374,
        // :376-378, :380-390.
        assert_eq!(CONSONANT_D.len(), 79);
        assert_eq!(CONSONANT_TABLE.len(), 33);
        assert_eq!(END_CONSONANT_TABLE.len(), 11);
        assert_eq!(STANDALONE_WBAD.len(), 7);
        assert_eq!(DOUBLE_W_ALLOWED.len(), 9);
    }

    #[test]
    fn quick_tables_sizes_match_source() {
        // Vietnamese.cpp:392-396 (3), :398-402 (3), :497-506 (8).
        assert_eq!(QUICK_START_CONSONANT.len(), 3);
        assert_eq!(QUICK_END_CONSONANT.len(), 3);
        assert_eq!(QUICK_TELEX.len(), 8);
    }

    #[test]
    fn break_code_has_18_entries_with_intentional_duplicate_tab() {
        // Engine.cpp:21-28 non-_WIN32 branch: 18 entries, KEY_TAB appears at
        // both index 1 and the last index (17) -- verbatim from source.
        assert_eq!(BREAK_CODE.len(), 18);
        assert_eq!(BREAK_CODE[1], KEY_TAB as u8);
        assert_eq!(BREAK_CODE[17], KEY_TAB as u8);
    }

    #[test]
    fn processing_char_vni_row_has_intentional_duplicate_key_6() {
        // Engine.cpp:34-39: VNI row is {KEY_1..KEY_6, KEY_6, KEY_7..KEY_0},
        // i.e. KEY_6 appears at both column 5 and column 6 -- verbatim.
        assert_eq!(PROCESSING_CHAR[1][5], KEY_6);
        assert_eq!(PROCESSING_CHAR[1][6], KEY_6);
        assert_eq!(PROCESSING_CHAR[0][5], KEY_A); // Telex column 5 = 'A'
    }

    // -- spot-check sample rows against Vietnamese.cpp/Engine.cpp hex/values --

    #[test]
    fn vowel_a_first_and_last_rows_match_source() {
        // Vietnamese.cpp:20-21 first row {KEY_A,KEY_N,KEY_G}; line 29 last
        // row {KEY_A,KEY_C}.
        assert_eq!(VOWEL[&KEY_A][0], [KEY_A, KEY_N, KEY_G]);
        assert_eq!(VOWEL[&KEY_A][9], [KEY_A, KEY_C]);
    }

    #[test]
    fn consonant_d_first_and_last_rows_match_source() {
        // Vietnamese.cpp:171 first row {KEY_D,KEY_E,KEY_N,KEY_H};
        // Vietnamese.cpp:241 last row {KEY_D} alone.
        assert_eq!(CONSONANT_D[0], [KEY_D, KEY_E, KEY_N, KEY_H]);
        assert_eq!(CONSONANT_D[78], [KEY_D]);
    }

    #[test]
    fn consonant_table_allow_and_end_mask_rows_match_source() {
        // Vietnamese.cpp:363-367: F/W/Z/J with CONSONANT_ALLOW_MASK, then
        // F/W/J with END_CONSONANT_MASK.
        assert_eq!(CONSONANT_TABLE[26], [KEY_F | CA]);
        assert_eq!(CONSONANT_TABLE[29], [KEY_J | CA]);
        assert_eq!(CONSONANT_TABLE[30], [KEY_F | EC]);
        assert_eq!(CONSONANT_TABLE[32], [KEY_J | EC]);
    }

    #[test]
    fn vowel_combine_u_row_flag_and_masked_entries_match_source() {
        // Vietnamese.cpp:139-140 KEY_U first two rows:
        //   {0, KEY_U, KEY_Y, KEY_U}, {1, KEY_U, KEY_Y, KEY_E|TONE_MASK}
        assert_eq!(
            VOWEL_COMBINE[&KEY_U][0],
            [0, KEY_U as u32, KEY_Y as u32, KEY_U as u32]
        );
        assert_eq!(
            VOWEL_COMBINE[&KEY_U][1],
            [1, KEY_U as u32, KEY_Y as u32, KEY_E as u32 | TONE_MASK]
        );
    }

    #[test]
    fn quick_telex_spot_check() {
        // Vietnamese.cpp:497-506: {KEY_U, {KEY_U, KEY_U}} (uu -> uơ quick key).
        assert_eq!(QUICK_TELEX[&(KEY_U as u32)], [KEY_U, KEY_U]);
    }

    #[test]
    fn ec_and_ca_constants_match_data_masks_truncated() {
        assert_eq!(u32::from(EC), END_CONSONANT_MASK);
        assert_eq!(u32::from(CA), CONSONANT_ALLOW_MASK);
    }

    #[test]
    fn char_key_code_has_21_entries() {
        // Engine.cpp:15-19: backquote, 1-9, 0, -, =, [, ], \, ;, ', ,, ., /.
        assert_eq!(CHAR_KEY_CODE.len(), 21);
        assert_eq!(CHAR_KEY_CODE[0], KEY_BACKQUOTE as u8);
        assert_eq!(CHAR_KEY_CODE[10], KEY_0 as u8);
        assert_eq!(CHAR_KEY_CODE[20], KEY_SLASH as u8);
    }

    #[test]
    fn macro_break_code_has_9_entries() {
        // Engine.cpp:30-32: Return, comma, dot, slash, semicolon, quote,
        // backslash, minus, equals.
        assert_eq!(MACRO_BREAK_CODE.len(), 9);
        assert_eq!(MACRO_BREAK_CODE[0], KEY_RETURN as u8);
        assert_eq!(MACRO_BREAK_CODE[8], KEY_EQUALS as u8);
    }
}
