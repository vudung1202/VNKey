//! Vietnamese character code tables and `get_character_code`.
//!
//! Ported verbatim from OpenKey `Vietnamese.cpp:14,411-577` (code tables,
//! compound-mark table, character map) and `Engine.cpp:501-558`
//! (`getCharacterCode`).

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::data::{
    CAPS_MASK, CHAR_CODE_MASK, CHAR_MASK, CodeTable, MARK_MASK, MARK1_MASK, MARK2_MASK,
    MARK3_MASK, MARK4_MASK, MARK5_MASK, TONE_MASK, TONEW_MASK,
};
use crate::keys::*;

type CodeRow = &'static [u16];

/// `Vietnamese.cpp:14 douKey` -- a -> a-circumflex, e -> e-circumflex.
///
/// Not yet consumed: `douKey` is only read by state-machine logic
/// (`insertAOE`/typing-word handling) that Task 4+ ports, not by this task's
/// `getCharacterCode`/`keyCodeToCharacter` surface. Kept `pub(crate)` per the
/// brief so that logic can use it once ported; allowed dead for now.
#[allow(dead_code)]
pub(crate) const DOU_KEY: [[u16; 2]; 2] = [[KEY_A, 0xE2], [KEY_E, 0xEA]];

/// `Vietnamese.cpp:494 _unicodeCompoundMark` -- sac, huyen, hoi, nga, nang
/// combining diacritics, for Unicode Compound.
///
/// Not yet consumed: only used by Unicode Compound decoding logic that
/// Task 4+ ports. Kept `pub(crate)` per the brief; allowed dead for now.
#[allow(dead_code)]
pub(crate) const UNICODE_COMPOUND_MARK: [u16; 5] = [0x0301, 0x0300, 0x0309, 0x0303, 0x0323];

// `Vietnamese.cpp:411-491 _codeTable[]`.
//
// Row comment in the source:
//   {keyCode, {CAPS_CHAR, NORMAL_CHAR, CAPS_W_CHAR, NORMAL_W_CHAR, <5 marks x caps/lower>}}
// for A/O/U/E/I/Y base rows, and {<5 marks x caps/lower>} directly for the
// A|TONE, A|TONEW, O|TONE, O|TONEW, U|TONEW, E|TONE rows.

fn unicode_table() -> HashMap<u32, CodeRow> {
    HashMap::from([
        (
            KEY_A as u32,
            &[
                0x00C2, 0x00E2, 0x0102, 0x0103, 0x00C1, 0x00E1, 0x00C0, 0x00E0, 0x1EA2, 0x1EA3,
                0x00C3, 0x00E3, 0x1EA0, 0x1EA1,
            ][..],
        ),
        (
            KEY_O as u32,
            &[
                0x00D4, 0x00F4, 0x01A0, 0x01A1, 0x00D3, 0x00F3, 0x00D2, 0x00F2, 0x1ECE, 0x1ECF,
                0x00D5, 0x00F5, 0x1ECC, 0x1ECD,
            ][..],
        ),
        (
            KEY_U as u32,
            &[
                0x0000, 0x0000, 0x01AF, 0x01B0, 0x00DA, 0x00FA, 0x00D9, 0x00F9, 0x1EE6, 0x1EE7,
                0x0168, 0x0169, 0x1EE4, 0x1EE5,
            ][..],
        ),
        (
            KEY_E as u32,
            &[
                0x00CA, 0x00EA, 0x0000, 0x0000, 0x00C9, 0x00E9, 0x00C8, 0x00E8, 0x1EBA, 0x1EBB,
                0x1EBC, 0x1EBD, 0x1EB8, 0x1EB9,
            ][..],
        ),
        (KEY_D as u32, &[0x0110, 0x0111][..]),
        (
            KEY_A as u32 | TONE_MASK,
            &[
                0x1EA4, 0x1EA5, 0x1EA6, 0x1EA7, 0x1EA8, 0x1EA9, 0x1EAA, 0x1EAB, 0x1EAC, 0x1EAD,
            ][..],
        ),
        (
            KEY_A as u32 | TONEW_MASK,
            &[
                0x1EAE, 0x1EAF, 0x1EB0, 0x1EB1, 0x1EB2, 0x1EB3, 0x1EB4, 0x1EB5, 0x1EB6, 0x1EB7,
            ][..],
        ),
        (
            KEY_O as u32 | TONE_MASK,
            &[
                0x1ED0, 0x1ED1, 0x1ED2, 0x1ED3, 0x1ED4, 0x1ED5, 0x1ED6, 0x1ED7, 0x1ED8, 0x1ED9,
            ][..],
        ),
        (
            KEY_O as u32 | TONEW_MASK,
            &[
                0x1EDA, 0x1EDB, 0x1EDC, 0x1EDD, 0x1EDE, 0x1EDF, 0x1EE0, 0x1EE1, 0x1EE2, 0x1EE3,
            ][..],
        ),
        (
            KEY_U as u32 | TONEW_MASK,
            &[
                0x1EE8, 0x1EE9, 0x1EEA, 0x1EEB, 0x1EEC, 0x1EED, 0x1EEE, 0x1EEF, 0x1EF0, 0x1EF1,
            ][..],
        ),
        (
            KEY_E as u32 | TONE_MASK,
            &[
                0x1EBE, 0x1EBF, 0x1EC0, 0x1EC1, 0x1EC2, 0x1EC3, 0x1EC4, 0x1EC5, 0x1EC6, 0x1EC7,
            ][..],
        ),
        (
            KEY_I as u32,
            &[
                0x00CD, 0x00ED, 0x00CC, 0x00EC, 0x1EC8, 0x1EC9, 0x0128, 0x0129, 0x1ECA, 0x1ECB,
            ][..],
        ),
        (
            KEY_Y as u32,
            &[
                0x00DD, 0x00FD, 0x1EF2, 0x1EF3, 0x1EF6, 0x1EF7, 0x1EF8, 0x1EF9, 0x1EF4, 0x1EF5,
            ][..],
        ),
    ])
}

fn tcvn3_table() -> HashMap<u32, CodeRow> {
    HashMap::from([
        (
            KEY_A as u32,
            &[
                0xA2, 0xA9, 0xA1, 0xA8, 0xB8, 0xB8, 0xB5, 0xB5, 0xB6, 0xB6, 0xB7, 0xB7, 0xB9, 0xB9,
            ][..],
        ),
        (
            KEY_O as u32,
            &[
                0xA4, 0xAB, 0xA5, 0xAC, 0xE3, 0xE3, 0xDF, 0xDF, 0xE1, 0xE1, 0xE2, 0xE2, 0xE4, 0xE4,
            ][..],
        ),
        (
            KEY_U as u32,
            &[
                0x00, 0x00, 0xA6, 0xAD, 0xF3, 0xF3, 0xEF, 0xEF, 0xF1, 0xF1, 0xF2, 0xF2, 0xF4, 0xF4,
            ][..],
        ),
        (
            KEY_E as u32,
            &[
                0xA3, 0xAA, 0x00, 0x00, 0xD0, 0xD0, 0xCC, 0xCC, 0xCE, 0xCE, 0xCF, 0xCF, 0xD1, 0xD1,
            ][..],
        ),
        (KEY_D as u32, &[0xA7, 0xAE][..]),
        (
            KEY_A as u32 | TONE_MASK,
            &[0xCA, 0xCA, 0xC7, 0xC7, 0xC8, 0xC8, 0xC9, 0xC9, 0xCB, 0xCB][..],
        ),
        (
            KEY_A as u32 | TONEW_MASK,
            &[0xBE, 0xBE, 0xBB, 0xBB, 0xBC, 0xBC, 0xBD, 0xBD, 0xC6, 0xC6][..],
        ),
        (
            KEY_O as u32 | TONE_MASK,
            &[0xE8, 0xE8, 0xE5, 0xE5, 0xE6, 0xE6, 0xE7, 0xE7, 0xE9, 0xE9][..],
        ),
        (
            KEY_O as u32 | TONEW_MASK,
            &[0xED, 0xED, 0xEA, 0xEA, 0xEB, 0xEB, 0xEC, 0xEC, 0xEE, 0xEE][..],
        ),
        (
            KEY_U as u32 | TONEW_MASK,
            &[0xF8, 0xF8, 0xF5, 0xF5, 0xF6, 0xF6, 0xF7, 0xF7, 0xF9, 0xF9][..],
        ),
        (
            KEY_E as u32 | TONE_MASK,
            &[0xD5, 0xD5, 0xD2, 0xD2, 0xD3, 0xD3, 0xD4, 0xD4, 0xD6, 0xD6][..],
        ),
        (
            KEY_I as u32,
            &[0xDD, 0xDD, 0xD7, 0xD7, 0xD8, 0xD8, 0xDC, 0xDC, 0xDE, 0xDE][..],
        ),
        (
            KEY_Y as u32,
            &[0xFD, 0xFD, 0xFA, 0xFA, 0xFB, 0xFB, 0xFC, 0xFC, 0xFE, 0xFE][..],
        ),
    ])
}

fn vni_windows_table() -> HashMap<u32, CodeRow> {
    HashMap::from([
        (
            KEY_A as u32,
            &[
                0xC241, 0xE261, 0xCA41, 0xEA61, 0xD941, 0xF961, 0xD841, 0xF861, 0xDB41, 0xFB61,
                0xD541, 0xF561, 0xCF41, 0xEF61,
            ][..],
        ),
        (
            KEY_O as u32,
            &[
                0xC24F, 0xE26F, 0x00D4, 0x00F4, 0xD94F, 0xF96F, 0xD84F, 0xF86F, 0xDB4F, 0xFB6F,
                0xD54F, 0xF56F, 0xCF4F, 0xEF6F,
            ][..],
        ),
        (
            KEY_U as u32,
            &[
                0x0000, 0x0000, 0x00D6, 0x00F6, 0xD955, 0xF975, 0xD855, 0xF875, 0xDB55, 0xFB75,
                0xD555, 0xF575, 0xCF55, 0xEF75,
            ][..],
        ),
        (
            KEY_E as u32,
            &[
                0xC245, 0xE265, 0x0000, 0x0000, 0xD945, 0xF965, 0xD845, 0xF865, 0xDB45, 0xFB65,
                0xD545, 0xF565, 0xCF45, 0xEF65,
            ][..],
        ),
        (KEY_D as u32, &[0x00D1, 0x00F1][..]),
        (
            KEY_A as u32 | TONE_MASK,
            &[
                0xC141, 0xE161, 0xC041, 0xE061, 0xC541, 0xE561, 0xC341, 0xE361, 0xC441, 0xE461,
            ][..],
        ),
        (
            KEY_A as u32 | TONEW_MASK,
            &[
                0xC941, 0xE961, 0xC841, 0xE861, 0xDA41, 0xFA61, 0xDC41, 0xFC61, 0xCB41, 0xEB61,
            ][..],
        ),
        (
            KEY_O as u32 | TONE_MASK,
            &[
                0xC14F, 0xE16F, 0xC04F, 0xE06F, 0xC54F, 0xE56F, 0xC34F, 0xE36F, 0xC44F, 0xE46F,
            ][..],
        ),
        (
            KEY_O as u32 | TONEW_MASK,
            &[
                0xD9D4, 0xF9F4, 0xD8D4, 0xF8F4, 0xDBD4, 0xFBF4, 0xD5D4, 0xF5F4, 0xCFD4, 0xEFF4,
            ][..],
        ),
        (
            KEY_U as u32 | TONEW_MASK,
            &[
                0xD9D6, 0xF9F6, 0xD8D6, 0xF8F6, 0xDBD6, 0xFBF6, 0xD5D6, 0xF5F6, 0xCFD6, 0xEFF6,
            ][..],
        ),
        (
            KEY_E as u32 | TONE_MASK,
            &[
                0xC145, 0xE165, 0xC045, 0xE065, 0xC545, 0xE565, 0xC345, 0xE365, 0xC445, 0xE465,
            ][..],
        ),
        (
            KEY_I as u32,
            &[
                0x00CD, 0x00ED, 0x00CC, 0x00EC, 0x00C6, 0x00E6, 0x00D3, 0x00F3, 0x00D2, 0x00F2,
            ][..],
        ),
        (
            KEY_Y as u32,
            &[
                0xD959, 0xF979, 0xD859, 0xF879, 0xDB59, 0xFB79, 0xD559, 0xF579, 0x00CE, 0x00EE,
            ][..],
        ),
    ])
}

fn unicode_compound_table() -> HashMap<u32, CodeRow> {
    HashMap::from([
        (
            KEY_A as u32,
            &[
                0x00C2, 0x00E2, 0x0102, 0x0103, 0x2041, 0x2061, 0x4041, 0x4061, 0x6041, 0x6061,
                0x8041, 0x8061, 0xA041, 0xA061,
            ][..],
        ),
        (
            KEY_O as u32,
            &[
                0x00D4, 0x00F4, 0x01A0, 0x01A1, 0x204F, 0x206F, 0x404F, 0x406F, 0x604F, 0x606F,
                0x804F, 0x806F, 0xA04F, 0xA06F,
            ][..],
        ),
        (
            KEY_U as u32,
            &[
                0x0000, 0x0000, 0x01AF, 0x01B0, 0x2055, 0x2075, 0x4055, 0x4075, 0x6055, 0x6075,
                0x8055, 0x8075, 0xA055, 0xA075,
            ][..],
        ),
        (
            KEY_E as u32,
            &[
                0x00CA, 0x00EA, 0x0000, 0x0000, 0x2045, 0x2065, 0x4045, 0x4065, 0x6045, 0x6065,
                0x8045, 0x8065, 0xA045, 0xA065,
            ][..],
        ),
        (KEY_D as u32, &[0x0110, 0x0111][..]),
        (
            KEY_A as u32 | TONE_MASK,
            &[
                0x20C2, 0x20E2, 0x40C2, 0x40E2, 0x60C2, 0x60E2, 0x80C2, 0x80E2, 0xA0C2, 0xA0E2,
            ][..],
        ),
        (
            KEY_A as u32 | TONEW_MASK,
            &[
                0x2102, 0x2103, 0x4102, 0x4103, 0x6102, 0x6103, 0x8102, 0x8103, 0xA102, 0xA103,
            ][..],
        ),
        (
            KEY_O as u32 | TONE_MASK,
            &[
                0x20D4, 0x20F4, 0x40D4, 0x40F4, 0x60D4, 0x60F4, 0x80D4, 0x80F4, 0xA0D4, 0xA0F4,
            ][..],
        ),
        (
            KEY_O as u32 | TONEW_MASK,
            &[
                0x21A0, 0x21A1, 0x41A0, 0x41A1, 0x61A0, 0x61A1, 0x81A0, 0x81A1, 0xA1A0, 0xA1A1,
            ][..],
        ),
        (
            KEY_U as u32 | TONEW_MASK,
            &[
                0x21AF, 0x21B0, 0x41AF, 0x41B0, 0x61AF, 0x61B0, 0x81AF, 0x81B0, 0xA1AF, 0xA1B0,
            ][..],
        ),
        (
            KEY_E as u32 | TONE_MASK,
            &[
                0x20CA, 0x20EA, 0x40CA, 0x40EA, 0x60CA, 0x60EA, 0x80CA, 0x80EA, 0xA0CA, 0xA0EA,
            ][..],
        ),
        (
            KEY_I as u32,
            &[
                0x2049, 0x2069, 0x4049, 0x4069, 0x6049, 0x6069, 0x8049, 0x8069, 0xA049, 0xA069,
            ][..],
        ),
        (
            KEY_Y as u32,
            &[
                0x2059, 0x2079, 0x4059, 0x4079, 0x6059, 0x6079, 0x8059, 0x8079, 0xA059, 0xA079,
            ][..],
        ),
    ])
}

fn cp1258_table() -> HashMap<u32, CodeRow> {
    HashMap::from([
        (
            KEY_A as u32,
            &[
                0x00C2, 0x00E2, 0x00C3, 0x00E3, 0xEC41, 0xEC61, 0xCC41, 0xCC61, 0xD241, 0xD261,
                0xDE41, 0xDE61, 0xF241, 0xF261,
            ][..],
        ),
        (
            KEY_O as u32,
            &[
                0x00D4, 0x00F4, 0x00D5, 0x00F5, 0xEC4F, 0xEC6F, 0xCC4F, 0xCC6F, 0xD24F, 0xD26F,
                0xDE4F, 0xDE6F, 0xF24F, 0xF26F,
            ][..],
        ),
        (
            KEY_U as u32,
            &[
                0x0000, 0x0000, 0x00DD, 0x00FD, 0xEC55, 0xEC75, 0xCC55, 0xCC75, 0xD255, 0xD275,
                0xDE55, 0xDE75, 0xF255, 0xF275,
            ][..],
        ),
        (
            KEY_E as u32,
            &[
                0x00CA, 0x00EA, 0x0000, 0x0000, 0xEC45, 0xEC65, 0xCC45, 0xCC65, 0xD245, 0xD265,
                0xDE45, 0xDE65, 0xF245, 0xF265,
            ][..],
        ),
        (KEY_D as u32, &[0x00D0, 0x00F0][..]),
        (
            KEY_A as u32 | TONE_MASK,
            &[
                0xECC2, 0xECE2, 0xCCC2, 0xCCE2, 0xD2C2, 0xD2E2, 0xDEC2, 0xDEE2, 0xF2C2, 0xF2E2,
            ][..],
        ),
        (
            KEY_A as u32 | TONEW_MASK,
            &[
                0xECC3, 0xECE3, 0xCCC3, 0xCCE3, 0xD2C3, 0xD2E3, 0xDEC3, 0xDEE3, 0xF2C3, 0xF2E3,
            ][..],
        ),
        (
            KEY_O as u32 | TONE_MASK,
            &[
                0xECD4, 0xECF4, 0xCCD4, 0xCCF4, 0xD2D4, 0xD2F4, 0xDED4, 0xDEF4, 0xF2D4, 0xF2F4,
            ][..],
        ),
        (
            KEY_O as u32 | TONEW_MASK,
            &[
                0xECD5, 0xECF5, 0xCCD5, 0xCCF5, 0xD2D5, 0xD2F5, 0xDED5, 0xDEF5, 0xF2D5, 0xF2F5,
            ][..],
        ),
        (
            KEY_U as u32 | TONEW_MASK,
            &[
                0xECDD, 0xECFD, 0xCCDD, 0xCCFD, 0xD2DD, 0xD2FD, 0xDEDD, 0xDEFD, 0xF2DD, 0xF2FD,
            ][..],
        ),
        (
            KEY_E as u32 | TONE_MASK,
            &[
                0xECCA, 0xECEA, 0xCCCA, 0xCCEA, 0xD2CA, 0xD2EA, 0xDECA, 0xDEEA, 0xF2CA, 0xF2EA,
            ][..],
        ),
        (
            KEY_I as u32,
            &[
                0xEC49, 0xEC69, 0xCC49, 0xCC69, 0xD249, 0xD269, 0xDE49, 0xDE69, 0xF249, 0xF269,
            ][..],
        ),
        (
            KEY_Y as u32,
            &[
                0xEC59, 0xEC79, 0xCC59, 0xCC79, 0xD259, 0xD279, 0xDE59, 0xDE79, 0xF259, 0xF279,
            ][..],
        ),
    ])
}

static CODE_TABLES: LazyLock<[HashMap<u32, CodeRow>; 5]> = LazyLock::new(|| {
    [
        unicode_table(),
        tcvn3_table(),
        vni_windows_table(),
        unicode_compound_table(),
        cp1258_table(),
    ]
});

fn code_table_map(code_table: CodeTable) -> &'static HashMap<u32, CodeRow> {
    &CODE_TABLES[code_table as usize]
}

/// `Vietnamese.cpp:508-556 _characterMap` -- ASCII character to (keycode
/// optionally OR'd with CAPS_MASK).
const CHARACTER_MAP: &[(u32, u32)] = &[
    (b'a' as u32, KEY_A as u32),
    (b'A' as u32, KEY_A as u32 | CAPS_MASK),
    (b'b' as u32, KEY_B as u32),
    (b'B' as u32, KEY_B as u32 | CAPS_MASK),
    (b'c' as u32, KEY_C as u32),
    (b'C' as u32, KEY_C as u32 | CAPS_MASK),
    (b'd' as u32, KEY_D as u32),
    (b'D' as u32, KEY_D as u32 | CAPS_MASK),
    (b'e' as u32, KEY_E as u32),
    (b'E' as u32, KEY_E as u32 | CAPS_MASK),
    (b'f' as u32, KEY_F as u32),
    (b'F' as u32, KEY_F as u32 | CAPS_MASK),
    (b'g' as u32, KEY_G as u32),
    (b'G' as u32, KEY_G as u32 | CAPS_MASK),
    (b'h' as u32, KEY_H as u32),
    (b'H' as u32, KEY_H as u32 | CAPS_MASK),
    (b'i' as u32, KEY_I as u32),
    (b'I' as u32, KEY_I as u32 | CAPS_MASK),
    (b'j' as u32, KEY_J as u32),
    (b'J' as u32, KEY_J as u32 | CAPS_MASK),
    (b'k' as u32, KEY_K as u32),
    (b'K' as u32, KEY_K as u32 | CAPS_MASK),
    (b'l' as u32, KEY_L as u32),
    (b'L' as u32, KEY_L as u32 | CAPS_MASK),
    (b'm' as u32, KEY_M as u32),
    (b'M' as u32, KEY_M as u32 | CAPS_MASK),
    (b'n' as u32, KEY_N as u32),
    (b'N' as u32, KEY_N as u32 | CAPS_MASK),
    (b'o' as u32, KEY_O as u32),
    (b'O' as u32, KEY_O as u32 | CAPS_MASK),
    (b'p' as u32, KEY_P as u32),
    (b'P' as u32, KEY_P as u32 | CAPS_MASK),
    (b'q' as u32, KEY_Q as u32),
    (b'Q' as u32, KEY_Q as u32 | CAPS_MASK),
    (b'r' as u32, KEY_R as u32),
    (b'R' as u32, KEY_R as u32 | CAPS_MASK),
    (b's' as u32, KEY_S as u32),
    (b'S' as u32, KEY_S as u32 | CAPS_MASK),
    (b't' as u32, KEY_T as u32),
    (b'T' as u32, KEY_T as u32 | CAPS_MASK),
    (b'u' as u32, KEY_U as u32),
    (b'U' as u32, KEY_U as u32 | CAPS_MASK),
    (b'v' as u32, KEY_V as u32),
    (b'V' as u32, KEY_V as u32 | CAPS_MASK),
    (b'w' as u32, KEY_W as u32),
    (b'W' as u32, KEY_W as u32 | CAPS_MASK),
    (b'x' as u32, KEY_X as u32),
    (b'X' as u32, KEY_X as u32 | CAPS_MASK),
    (b'y' as u32, KEY_Y as u32),
    (b'Y' as u32, KEY_Y as u32 | CAPS_MASK),
    (b'z' as u32, KEY_Z as u32),
    (b'Z' as u32, KEY_Z as u32 | CAPS_MASK),
    (b'1' as u32, KEY_1 as u32),
    (b'!' as u32, KEY_1 as u32 | CAPS_MASK),
    (b'2' as u32, KEY_2 as u32),
    (b'@' as u32, KEY_2 as u32 | CAPS_MASK),
    (b'3' as u32, KEY_3 as u32),
    (b'#' as u32, KEY_3 as u32 | CAPS_MASK),
    (b'4' as u32, KEY_4 as u32),
    (b'$' as u32, KEY_4 as u32 | CAPS_MASK),
    (b'5' as u32, KEY_5 as u32),
    (b'%' as u32, KEY_5 as u32 | CAPS_MASK),
    (b'6' as u32, KEY_6 as u32),
    (b'^' as u32, KEY_6 as u32 | CAPS_MASK),
    (b'7' as u32, KEY_7 as u32),
    (b'&' as u32, KEY_7 as u32 | CAPS_MASK),
    (b'8' as u32, KEY_8 as u32),
    (b'*' as u32, KEY_8 as u32 | CAPS_MASK),
    (b'9' as u32, KEY_9 as u32),
    (b'(' as u32, KEY_9 as u32 | CAPS_MASK),
    (b'0' as u32, KEY_0 as u32),
    (b')' as u32, KEY_0 as u32 | CAPS_MASK),
    (b'`' as u32, KEY_BACKQUOTE as u32),
    (b'~' as u32, KEY_BACKQUOTE as u32 | CAPS_MASK),
    (b'-' as u32, KEY_MINUS as u32),
    (b'_' as u32, KEY_MINUS as u32 | CAPS_MASK),
    (b'=' as u32, KEY_EQUALS as u32),
    (b'+' as u32, KEY_EQUALS as u32 | CAPS_MASK),
    (b'[' as u32, KEY_LEFT_BRACKET as u32),
    (b'{' as u32, KEY_LEFT_BRACKET as u32 | CAPS_MASK),
    (b']' as u32, KEY_RIGHT_BRACKET as u32),
    (b'}' as u32, KEY_RIGHT_BRACKET as u32 | CAPS_MASK),
    (b'\\' as u32, KEY_BACK_SLASH as u32),
    (b'|' as u32, KEY_BACK_SLASH as u32 | CAPS_MASK),
    (b';' as u32, KEY_SEMICOLON as u32),
    (b':' as u32, KEY_SEMICOLON as u32 | CAPS_MASK),
    (b'\'' as u32, KEY_QUOTE as u32),
    (b'"' as u32, KEY_QUOTE as u32 | CAPS_MASK),
    (b',' as u32, KEY_COMMA as u32),
    (b'<' as u32, KEY_COMMA as u32 | CAPS_MASK),
    (b'.' as u32, KEY_DOT as u32),
    (b'>' as u32, KEY_DOT as u32 | CAPS_MASK),
    (b'/' as u32, KEY_SLASH as u32),
    (b'?' as u32, KEY_SLASH as u32 | CAPS_MASK),
    (b' ' as u32, KEY_SPACE as u32),
];

/// `Vietnamese.cpp:568 keyCodeToCharacter` -- inverse of `_characterMap`,
/// lazily built once like the C++ `_keyCodeToChar` cache.
static KEY_CODE_TO_CHAR: LazyLock<HashMap<u32, u32>> = LazyLock::new(|| {
    CHARACTER_MAP
        .iter()
        .map(|&(ch, code)| (code, ch))
        .collect()
});

/// `Engine.cpp:501-558 getCharacterCode`.
///
/// Returns `data` unchanged when the key/mark combination has no entry in
/// the code table (mirrors every "not found" early-return in the C++).
pub fn get_character_code(code_table: CodeTable, data: u32) -> u32 {
    let caps_elem: i32 = if data & CAPS_MASK != 0 { 0 } else { 1 };
    let mut key: u32 = data & CHAR_MASK;
    let table = code_table_map(code_table);

    if data & MARK_MASK != 0 {
        let mut mark_elem: i32 = match data & MARK_MASK {
            MARK1_MASK => 0,
            MARK2_MASK => 2,
            MARK3_MASK => 4,
            MARK4_MASK => 6,
            MARK5_MASK => 8,
            _ => -2, // matches no C++ switch case: markElem stays -2 (UB in C++)
        };
        mark_elem += caps_elem;

        if matches!(key as u16, KEY_A | KEY_O | KEY_U | KEY_E)
            && (data & TONE_MASK) == 0
            && (data & TONEW_MASK) == 0
        {
            mark_elem += 4;
        }

        if data & TONE_MASK != 0 {
            key |= TONE_MASK;
        } else if data & TONEW_MASK != 0 {
            key |= TONEW_MASK;
        }

        let Some(row) = table.get(&key) else {
            return data; // not found
        };
        // C++ indexes the vector with a possibly negative/OOB markElem here,
        // which is undefined behavior (only reachable with an invalid,
        // multi-bit MARK_MASK combination that real key handling never
        // produces). We deviate deliberately and fall back to `data`.
        match usize::try_from(mark_elem).ok().and_then(|i| row.get(i)) {
            Some(&code) => code as u32 | CHAR_CODE_MASK,
            None => data,
        }
    } else {
        let Some(row) = table.get(&key) else {
            return data; // not found
        };
        if data & TONE_MASK != 0 {
            row[caps_elem as usize] as u32 | CHAR_CODE_MASK
        } else if data & TONEW_MASK != 0 {
            row[(caps_elem + 2) as usize] as u32 | CHAR_CODE_MASK
        } else {
            data // not found
        }
    }
}

/// `Vietnamese.cpp:568 keyCodeToCharacter`. Returns 0 for an unknown code,
/// like the C++ version.
pub fn key_code_to_character(key_code: u32) -> u32 {
    *KEY_CODE_TO_CHAR.get(&key_code).unwrap_or(&0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- get_character_code: hand-computed cases --
    //
    // getCharacterCode (Engine.cpp:501-558) algorithm:
    //   capsElem = (data & CAPS_MASK) ? 0 : 1
    //   key = data & CHAR_MASK   (strips CAPS/TONE/TONEW/MARK bits, all >0xFFFF)
    //   if data has a MARK bit:
    //     markElem = 0/2/4/6/8 for MARK1..MARK5, += capsElem
    //     if key in {A,O,U,E} and neither TONE_MASK nor TONEW_MASK set: markElem += 4
    //       (because those rows pack {caps,^caps,breve-caps,breve} at 0..3 first)
    //     if TONE_MASK set: key |= TONE_MASK; elif TONEW_MASK set: key |= TONEW_MASK
    //     row = codeTable[table][key]; return row[markElem] | CHAR_CODE_MASK (or data if key absent)
    //   else (no mark bit):
    //     row = codeTable[table][key] (or return data if absent)
    //     if TONE_MASK set: return row[capsElem] | CHAR_CODE_MASK
    //     elif TONEW_MASK set: return row[capsElem+2] | CHAR_CODE_MASK
    //     else: return data unchanged

    #[test]
    fn e_tone_mark5_lowercase_is_e_dot_below_circumflex() {
        // KEY_E|TONE_MASK|MARK5_MASK: capsElem=1(no caps), markElem=8+1=1... wait 8+1=9.
        // key=E is in {A,O,U,E} but TONE_MASK is set so no +4. key|=TONE_MASK.
        // Unicode row for KEY_E|TONE_MASK (Vietnamese.cpp:427):
        //   {0x1EBE,0x1EBF,0x1EC0,0x1EC1,0x1EC2,0x1EC3,0x1EC4,0x1EC5,0x1EC6,0x1EC7}
        // index 9 = 0x1EC7 = 'ệ'
        let data = KEY_E as u32 | TONE_MASK | MARK5_MASK;
        assert_eq!(
            get_character_code(CodeTable::Unicode, data),
            0x1EC7 | CHAR_CODE_MASK
        );
    }

    #[test]
    fn e_tone_mark5_uppercase_is_e_dot_below_circumflex_caps() {
        // Same as above but CAPS_MASK set: capsElem=0, markElem=8+0=8 -> row[8]=0x1EC6='Ệ'.
        let data = KEY_E as u32 | CAPS_MASK | TONE_MASK | MARK5_MASK;
        assert_eq!(
            get_character_code(CodeTable::Unicode, data),
            0x1EC6 | CHAR_CODE_MASK
        );
    }

    #[test]
    fn a_tonew_no_mark_lowercase_is_a_breve() {
        // No MARK bit. TONEW_MASK set, capsElem=1 -> row[capsElem+2] = row[3].
        // Unicode KEY_A row (Vietnamese.cpp:415): index3 = 0x0103 = 'ă'.
        let data = KEY_A as u32 | TONEW_MASK;
        assert_eq!(
            get_character_code(CodeTable::Unicode, data),
            0x0103 | CHAR_CODE_MASK
        );
    }

    #[test]
    fn o_tonew_mark2_lowercase_is_o_horn_grave() {
        // MARK2 (huyen) -> markElem base 2, +capsElem(1) = 3. TONEW_MASK set -> no +4,
        // key |= TONEW_MASK. Unicode row KEY_O|TONEW_MASK (Vietnamese.cpp:425):
        //   {0x1EDA,0x1EDB,0x1EDC,0x1EDD,0x1EDE,0x1EDF,0x1EE0,0x1EE1,0x1EE2,0x1EE3}
        // index 3 = 0x1EDD = 'ờ'
        let data = KEY_O as u32 | TONEW_MASK | MARK2_MASK;
        assert_eq!(
            get_character_code(CodeTable::Unicode, data),
            0x1EDD | CHAR_CODE_MASK
        );
    }

    #[test]
    fn d_with_tone_mask_lowercase_is_dd_stroke() {
        // D has no MARK1-5 entries in the table; OpenKey's insertD() toggles TONE_MASK
        // on the D cell to flip d <-> đ (Engine.cpp insertD, ~line 813-823).
        // No MARK bit -> TONE_MASK branch -> row[capsElem].
        // Unicode KEY_D row (Vietnamese.cpp:419): {0x0110 (Đ), 0x0111 (đ)}.
        // capsElem=1 (no caps) -> row[1] = 0x0111 = 'đ'.
        let data = KEY_D as u32 | TONE_MASK;
        assert_eq!(
            get_character_code(CodeTable::Unicode, data),
            0x0111 | CHAR_CODE_MASK
        );
    }

    #[test]
    fn d_with_tone_mask_uppercase_is_dd_stroke_caps() {
        // capsElem=0 -> row[0] = 0x0110 = 'Đ'.
        let data = KEY_D as u32 | CAPS_MASK | TONE_MASK;
        assert_eq!(
            get_character_code(CodeTable::Unicode, data),
            0x0110 | CHAR_CODE_MASK
        );
    }

    // Same cell (KEY_A|TONE_MASK|MARK1_MASK, sac, no caps -> 'a-circumflex-acute')
    // across every code table. markElem = 0(mark1) + 1(capsElem) = 1.
    #[test]
    fn a_tone_mark1_unicode_is_a_circumflex_acute() {
        // Vietnamese.cpp:421 KEY_A|TONE_MASK row: index1 = 0x1EA5 = 'ấ'.
        let data = KEY_A as u32 | TONE_MASK | MARK1_MASK;
        assert_eq!(
            get_character_code(CodeTable::Unicode, data),
            0x1EA5 | CHAR_CODE_MASK
        );
    }

    #[test]
    fn a_tone_mark1_tcvn3() {
        // Vietnamese.cpp:437 KEY_A|TONE_MASK row: {0xCA,0xCA,0xC7,0xC7,0xC8,0xC8,0xC9,0xC9,0xCB,0xCB}
        // index1 = 0xCA.
        let data = KEY_A as u32 | TONE_MASK | MARK1_MASK;
        assert_eq!(
            get_character_code(CodeTable::Tcvn3, data),
            0xCA | CHAR_CODE_MASK
        );
    }

    #[test]
    fn a_tone_mark1_vni_windows_two_byte() {
        // Vietnamese.cpp:452 KEY_A|TONE_MASK row:
        //   {0xC141,0xE161,0xC041,0xE061,0xC541,0xE561,0xC341,0xE361,0xC441,0xE461}
        // index1 = 0xE161 (2-byte packed hi/lo).
        let data = KEY_A as u32 | TONE_MASK | MARK1_MASK;
        assert_eq!(
            get_character_code(CodeTable::VniWindows, data),
            0xE161 | CHAR_CODE_MASK
        );
    }

    #[test]
    fn a_tone_mark1_unicode_compound_packed() {
        // Vietnamese.cpp:467 KEY_A|TONE_MASK row:
        //   {0x20C2,0x20E2,0x40C2,0x40E2,0x60C2,0x60E2,0x80C2,0x80E2,0xA0C2,0xA0E2}
        // index1 = 0x20E2 (base char + packed mark index, decoded elsewhere).
        let data = KEY_A as u32 | TONE_MASK | MARK1_MASK;
        assert_eq!(
            get_character_code(CodeTable::UnicodeCompound, data),
            0x20E2 | CHAR_CODE_MASK
        );
    }

    #[test]
    fn a_tone_mark1_cp1258() {
        // Vietnamese.cpp:482 KEY_A|TONE_MASK row:
        //   {0xECC2,0xECE2,0xCCC2,0xCCE2,0xD2C2,0xD2E2,0xDEC2,0xDEE2,0xF2C2,0xF2E2}
        // index1 = 0xECE2.
        let data = KEY_A as u32 | TONE_MASK | MARK1_MASK;
        assert_eq!(
            get_character_code(CodeTable::Cp1258, data),
            0xECE2 | CHAR_CODE_MASK
        );
    }

    #[test]
    fn a_mark1_no_tone_hits_plus4_offset_lowercase() {
        // Neither TONE_MASK nor TONEW_MASK set, key==A -> markElem += 4.
        // markElem = 0(mark1) + 1(capsElem) + 4 = 5.
        // Unicode KEY_A row (Vietnamese.cpp:415), index5 = 0x00E1 = 'á'.
        let data = KEY_A as u32 | MARK1_MASK;
        assert_eq!(
            get_character_code(CodeTable::Unicode, data),
            0x00E1 | CHAR_CODE_MASK
        );
    }

    #[test]
    fn a_mark1_no_tone_hits_plus4_offset_uppercase() {
        // markElem = 0 + 0(capsElem) + 4 = 4. row[4] = 0x00C1 = 'Á'.
        let data = KEY_A as u32 | CAPS_MASK | MARK1_MASK;
        assert_eq!(
            get_character_code(CodeTable::Unicode, data),
            0x00C1 | CHAR_CODE_MASK
        );
    }

    #[test]
    fn i_mark3_not_in_aoue_switch_no_plus4_offset() {
        // KEY_I is not in the {A,O,U,E} switch, so no +4 even without TONE/TONEW.
        // markElem = 4(mark3=hoi) + 1(capsElem) = 5.
        // Unicode KEY_I row (Vietnamese.cpp:428):
        //   {0x00CD,0x00ED,0x00CC,0x00EC,0x1EC8,0x1EC9,0x128,0x129,0x1ECA,0x1ECB}
        // index5 = 0x1EC9 = 'ỉ'.
        let data = KEY_I as u32 | MARK3_MASK;
        assert_eq!(
            get_character_code(CodeTable::Unicode, data),
            0x1EC9 | CHAR_CODE_MASK
        );
    }

    #[test]
    fn consonant_not_in_table_returns_input_unchanged() {
        // KEY_B has no entry in _codeTable -> "not found", return data as-is.
        let data = KEY_B as u32 | CAPS_MASK;
        assert_eq!(get_character_code(CodeTable::Unicode, data), data);
    }

    #[test]
    fn mark_on_key_absent_from_table_returns_input_unchanged() {
        // MARK_MASK set but KEY_B still absent from the table -> data unchanged.
        let data = KEY_B as u32 | MARK1_MASK;
        assert_eq!(get_character_code(CodeTable::Unicode, data), data);
    }

    #[test]
    fn found_key_without_tone_or_tonew_flag_returns_input_unchanged() {
        // KEY_D is in the table, but with no MARK bit and neither TONE_MASK nor
        // TONEW_MASK set, C++ falls through to `return data` (Engine.cpp:552-553).
        let data = KEY_D as u32 | CAPS_MASK;
        assert_eq!(get_character_code(CodeTable::Unicode, data), data);
    }

    #[test]
    fn invalid_multi_mark_bits_does_not_panic_and_is_safe_fallback() {
        // Two MARK bits set simultaneously never happens from real key handling
        // (Engine.cpp only ever ORs in a single MARKx_MASK at a time). In C++
        // this hits `markElem = -2` (no switch case matches `data & MARK_MASK`
        // exactly), += capsElem(1) = -1. KEY_I is not in the {A,O,U,E} switch,
        // so the "+4 no-tone offset" never rescues the index back to >= 0, and
        // C++ then indexes a std::vector with a negative index: undefined
        // behavior. We deliberately deviate: return `data` unchanged instead of
        // indexing out of bounds or panicking.
        let data = KEY_I as u32 | MARK1_MASK | MARK2_MASK;
        assert_eq!(get_character_code(CodeTable::Unicode, data), data);
    }

    // -- key_code_to_character round-trip (Vietnamese.cpp:508-576) --

    #[test]
    fn key_code_to_character_round_trips_letters() {
        assert_eq!(key_code_to_character(KEY_A as u32), b'a' as u32);
        assert_eq!(key_code_to_character(KEY_A as u32 | CAPS_MASK), b'A' as u32);
        assert_eq!(key_code_to_character(KEY_Z as u32), b'z' as u32);
    }

    #[test]
    fn key_code_to_character_round_trips_shifted_digit() {
        // {'1', KEY_1}, {'!', KEY_1|CAPS_MASK}
        assert_eq!(key_code_to_character(KEY_1 as u32 | CAPS_MASK), b'!' as u32);
    }

    #[test]
    fn key_code_to_character_round_trips_space() {
        assert_eq!(key_code_to_character(KEY_SPACE as u32), b' ' as u32);
    }

    #[test]
    fn key_code_to_character_unknown_code_returns_zero() {
        assert_eq!(key_code_to_character(0xDEAD_u32), 0);
    }

    // -- table size / spot-check --

    #[test]
    fn all_five_code_tables_have_13_rows() {
        // Vietnamese.cpp:411-491: each of the 5 _codeTable[] entries has exactly
        // 13 rows: A, O, U, E, D, A|TONE, A|TONEW, O|TONE, O|TONEW, U|TONEW,
        // E|TONE, I, Y.
        for ct in [
            CodeTable::Unicode,
            CodeTable::Tcvn3,
            CodeTable::VniWindows,
            CodeTable::UnicodeCompound,
            CodeTable::Cp1258,
        ] {
            assert_eq!(code_table_map(ct).len(), 13, "{ct:?} should have 13 rows");
        }
    }

    #[test]
    fn unicode_a_row_has_14_entries_and_tone_rows_have_10() {
        // Vietnamese.cpp:415 KEY_A row has 14 entries (no-tone base row packs
        // caps/breve variants first); Vietnamese.cpp:421 KEY_A|TONE_MASK row
        // has 10 (5 marks x caps/lower only).
        assert_eq!(code_table_map(CodeTable::Unicode)[&(KEY_A as u32)].len(), 14);
        assert_eq!(
            code_table_map(CodeTable::Unicode)[&(KEY_A as u32 | TONE_MASK)].len(),
            10
        );
        assert_eq!(code_table_map(CodeTable::Unicode)[&(KEY_D as u32)].len(), 2);
    }

    #[test]
    fn tcvn3_spot_check_hex_values() {
        // Vietnamese.cpp:432 KEY_A row: {0xA2,0xA9,0xA1,0xA8,0xB8,0xB8,0xB5,0xB5,0xB6,0xB6,0xB7,0xB7,0xB9,0xB9}
        let row = &code_table_map(CodeTable::Tcvn3)[&(KEY_A as u32)];
        assert_eq!(row[0], 0xA2);
        assert_eq!(row[13], 0xB9);
    }

    #[test]
    fn vni_windows_spot_check_hex_values() {
        // Vietnamese.cpp:459 KEY_Y row:
        //   {0xD959,0xF979,0xD859,0xF879,0xDB59,0xFB79,0xD559,0xF579,0x00CE,0x00EE}
        let row = &code_table_map(CodeTable::VniWindows)[&(KEY_Y as u32)];
        assert_eq!(row[0], 0xD959);
        assert_eq!(row[8], 0x00CE);
    }

    #[test]
    fn unicode_compound_mark_table_matches_source() {
        // Vietnamese.cpp:494: sac, huyen, hoi, nga, nang.
        assert_eq!(
            UNICODE_COMPOUND_MARK,
            [0x0301, 0x0300, 0x0309, 0x0303, 0x0323]
        );
    }

    #[test]
    fn dou_key_matches_source() {
        // Vietnamese.cpp:14: a -> a-circumflex, e -> e-circumflex.
        assert_eq!(DOU_KEY, [[KEY_A, 0xE2], [KEY_E, 0xEA]]);
    }
}
