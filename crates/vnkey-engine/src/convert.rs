//! Convert tool -- batch re-encode a whole string between the 5 code tables,
//! with optional mark removal and case transforms.
//!
//! Pure-Rust port of OpenKey `ConvertTool.cpp`/`.h`. The C++ drives
//! `convertUtil` off 9 `convertTool*` globals; here they are gathered into
//! [`ConvertOptions`] (the two host-only knobs -- `convertToolHotKey`,
//! `convertToolDontAlertWhenCompleted` -- are omitted, they never affect
//! `convertUtil`'s output).
//!
//! The C++ works on a `wstring` obtained via `std::codecvt_utf8_utf16`, i.e. a
//! sequence of UTF-16 code units; [`str::encode_utf16`] reproduces that, and
//! the output is rebuilt from the emitted code units with [`String::from_utf16`]
//! (mirroring `wideStringToUtf8`). Every VNI/TCVN3/CP1258 byte the C++ pushes
//! into its `wchar_t` buffer becomes a single UTF-16 unit in the 0..=255 range,
//! so it round-trips through UTF-16 exactly as the C++ round-trips through its
//! `wchar_t` pipeline.

use crate::charset::{code_table_cell, find_key_code, key_code_to_character};
use crate::charset::UNICODE_COMPOUND_MARK;
use crate::data::CodeTable;

/// The `convertTool*` option globals that affect `convertUtil`.
#[derive(Debug, Clone, Copy)]
pub struct ConvertOptions {
    /// `convertToolFromCode`.
    pub from_code: CodeTable,
    /// `convertToolToCode`.
    pub to_code: CodeTable,
    /// `convertToolRemoveMark`.
    pub remove_mark: bool,
    /// `convertToolToAllCaps`.
    pub to_all_caps: bool,
    /// `convertToolToAllNonCaps`.
    pub to_all_non_caps: bool,
    /// `convertToolToCapsFirstLetter`.
    pub to_caps_first_letter: bool,
    /// `convertToolToCapsEachWord`.
    pub to_caps_each_word: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        ConvertOptions {
            from_code: CodeTable::Unicode,
            to_code: CodeTable::Unicode,
            remove_mark: false,
            to_all_caps: false,
            to_all_non_caps: false,
            to_caps_first_letter: false,
            to_caps_each_word: false,
        }
    }
}

/// ASCII-only `towupper` (the oracle runs in `"C"` locale -- only ASCII cases).
fn ascii_upper(c: u16) -> u16 {
    if (b'a' as u16..=b'z' as u16).contains(&c) {
        c - 0x20
    } else {
        c
    }
}

/// ASCII-only `towlower`.
fn ascii_lower(c: u16) -> u16 {
    if (b'A' as u16..=b'Z' as u16).contains(&c) {
        c + 0x20
    } else {
        c
    }
}

/// `ConvertTool.cpp:42 getUnicodeCompoundMarkIndex` -- the packed high bits
/// `((i + 1) << 13)` for a Unicode-Compound combining mark, or 0.
fn get_unicode_compound_mark_index(mark: u16) -> u16 {
    for (i, &m) in UNICODE_COMPOUND_MARK.iter().enumerate() {
        if mark == m {
            return (i as u16 + 1) << 13;
        }
    }
    0
}

/// Push `target` into the output code-unit buffer, encoded per `to_code`
/// (`ConvertTool.cpp:107-123`): Unicode/TCVN3 push the raw cell; VNI/CP1258
/// split into low/high bytes (high byte only when `HIBYTE > 32`);
/// Unicode-Compound splits a packed cell into base + combining mark.
fn emit(temp: &mut Vec<u16>, to_code: CodeTable, target: u16) {
    match to_code {
        CodeTable::Unicode | CodeTable::Tcvn3 => temp.push(target),
        CodeTable::VniWindows | CodeTable::Cp1258 => {
            if ((target >> 8) & 0xFF) > 32 {
                temp.push(target & 0xFF);
                temp.push(target >> 8);
            } else {
                temp.push(target & 0xFF);
            }
        }
        CodeTable::UnicodeCompound => {
            if (target >> 13) > 0 {
                temp.push(target & 0x1FFF);
                temp.push(UNICODE_COMPOUND_MARK[((target >> 13) - 1) as usize]);
            } else {
                temp.push(target);
            }
        }
    }
}

/// Re-emit a matched `(key, k)` cell from `to_code`, applying the caps toggle
/// and mark removal (`ConvertTool.cpp:90-105` / `133-148`, identical in both
/// the two-byte-lookahead and single-byte branches).
fn reencode(opts: &ConvertOptions, should_upper_case: bool, key: u32, k: usize) -> u16 {
    // Rows are packed [caps, lower, caps, lower, ...]: an even column is a
    // caps glyph, an odd column its lowercase pair.
    let mut target = code_table_cell(opts.to_code, key, k);
    if (opts.to_all_caps || should_upper_case) && !k.is_multiple_of(2) {
        target = code_table_cell(opts.to_code, key, k - 1);
    } else if (opts.to_all_non_caps || !should_upper_case) && k.is_multiple_of(2) {
        target = code_table_cell(opts.to_code, key, k + 1);
    }

    if opts.remove_mark {
        // keyCodeToCharacter((Uint8)j): the low byte of the key drops the
        // TONE/TONEW bits, so e.g. 'ấ' (KEY_A|TONE) removes to 'a'.
        target = key_code_to_character(key & 0xFF) as u16;
        if opts.to_all_caps {
            target = ascii_upper(target);
        } else if opts.to_all_non_caps {
            target = ascii_lower(target);
        }
    }
    target
}

/// `ConvertTool.cpp:51 convertUtil` -- re-encode `src` per `opts`.
pub fn convert_util(src: &str, opts: &ConvertOptions) -> String {
    // C++ `utf8ToWideString(str)` converts `str.c_str()`, so any embedded NUL
    // truncates the input there. Mirror that before encoding.
    let src = &src[..src.find('\0').unwrap_or(src.len())];
    let data: Vec<u16> = src.encode_utf16().collect();
    let n = data.len();

    let mut temp: Vec<u16> = Vec::new();
    let mut has_break = false;
    let mut should_upper_case = opts.to_caps_first_letter || opts.to_caps_each_word;
    if opts.to_all_non_caps {
        should_upper_case = false;
    }
    const BREAK_CODE: [u16; 3] = [b'.' as u16, b'?' as u16, b'!' as u16];

    let mut i = 0usize;
    while i < n {
        let mut p = 0usize;
        // find char with tone/mark (two-unit lookahead), C++ `i < size - 1`.
        if i + 1 < n {
            let t: u16 = match opts.from_code {
                CodeTable::VniWindows | CodeTable::Cp1258 => {
                    p = 1;
                    data[i] | (data[i + 1] << 8)
                }
                CodeTable::UnicodeCompound => {
                    let target = get_unicode_compound_mark_index(data[i + 1]);
                    if target > 0 {
                        p = 1;
                        data[i] | target
                    } else {
                        data[i]
                    }
                }
                _ => data[i],
            };

            if let Some((key, k)) = find_key_code(opts.from_code, t) {
                i += p;
                let target = reencode(opts, should_upper_case, key, k);
                emit(&mut temp, opts.to_code, target);
                should_upper_case = false;
                has_break = false;
                i += 1;
                continue;
            }
        }

        // find primary keycode first (single unit). Note (mirror C++ quirk):
        // this block ends in a bare `_temp.push_back(target)` -- it does NOT
        // run the `to_code`-aware split `emit`, which only exists in the
        // two-unit block above. So a char matched here (a string's last char,
        // or one whose multi-byte lookahead failed) is emitted as the raw
        // `target` code unit regardless of `to_code`.
        let t: u16 = data[i];
        if let Some((key, k)) = find_key_code(opts.from_code, t) {
            let target = reencode(opts, should_upper_case, key, k);
            temp.push(target);
            should_upper_case = false;
            has_break = false;
            i += 1;
            continue;
        }

        // if we don't find => normal char.
        if opts.to_all_caps || should_upper_case {
            temp.push(ascii_upper(data[i]));
        } else if opts.to_all_non_caps || !should_upper_case {
            temp.push(ascii_lower(data[i]));
        } else {
            temp.push(data[i]);
        }

        // sentence-case state machine (C++ compares the single-unit `t`, which
        // in this fallback equals `data[i]`).
        if t == b'\n' as u16 || (has_break && t == b' ' as u16) {
            if opts.to_caps_first_letter || opts.to_caps_each_word {
                should_upper_case = true;
            }
        } else if t == b' ' as u16 && opts.to_caps_each_word {
            should_upper_case = true;
        } else if BREAK_CODE.contains(&t) {
            has_break = true;
        } else {
            should_upper_case = false;
            has_break = false;
        }

        i += 1;
    }

    // C++ appends a trailing 0 then converts `str.c_str()` (stops at the first
    // NUL). Reproduce by truncating at the first 0 code unit before decoding.
    if let Some(zero) = temp.iter().position(|&u| u == 0) {
        temp.truncate(zero);
    }
    // All emitted units are scalar BMP values or 0..=255 bytes -- never lone
    // surrogates -- so `from_utf16` succeeds; the lossy fallback is defensive.
    String::from_utf16(&temp).unwrap_or_else(|_| String::from_utf16_lossy(&temp))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(from: CodeTable, to: CodeTable) -> ConvertOptions {
        ConvertOptions {
            from_code: from,
            to_code: to,
            ..ConvertOptions::default()
        }
    }

    #[test]
    fn default_options_lowercase_everything() {
        // Quirk: with no case option set, `shouldUpperCase` stays false and the
        // `else if (... || !shouldUpperCase)` branch lowercases every char (the
        // C++ `else { push data[i] }` branch is dead code). Verified against the
        // oracle by the differential test.
        let s = "Tiếng Việt";
        assert_eq!(
            convert_util(s, &opts(CodeTable::Unicode, CodeTable::Unicode)),
            "tiếng việt"
        );
    }

    #[test]
    fn remove_mark_strips_diacritics() {
        let o = ConvertOptions {
            remove_mark: true,
            ..opts(CodeTable::Unicode, CodeTable::Unicode)
        };
        // 'ế' -> 'e', 'ệ' -> 'e' (keyCodeToCharacter of the base key); default
        // (no caps option) also lowercases the leading 'T'.
        assert_eq!(convert_util("Tiếng Việt", &o), "tieng viet");
    }

    #[test]
    fn to_all_caps_uppercases_ascii_and_accents() {
        let o = ConvertOptions {
            to_all_caps: true,
            ..opts(CodeTable::Unicode, CodeTable::Unicode)
        };
        assert_eq!(convert_util("việt", &o), "VIỆT");
    }

    #[test]
    fn to_all_non_caps_lowercases() {
        let o = ConvertOptions {
            to_all_non_caps: true,
            ..opts(CodeTable::Unicode, CodeTable::Unicode)
        };
        assert_eq!(convert_util("VIỆT nam", &o), "việt nam");
    }

    #[test]
    fn surrogate_producing_conversion_does_not_panic() {
        // A string ENDING in a char whose CP1258 cell lands in the UTF-16
        // surrogate range (e.g. 'ã' -> 0xDE61) goes through the single-unit
        // emit branch, which pushes the whole cell unsplit. The vendored C++
        // crashes here (wstring_convert throws); we intentionally do NOT
        // reproduce the crash -- `from_utf16` falls back to replacement chars.
        // (Non-last such chars are fine: the two-unit branch splits them.)
        let o = ConvertOptions {
            from_code: CodeTable::Unicode,
            to_code: CodeTable::Cp1258,
            ..ConvertOptions::default()
        };
        let out = convert_util("ã", &o); // must not panic
        assert!(!out.is_empty());
    }

    #[test]
    fn lowercase_round_trip_through_vni() {
        // An already-lowercase string survives Unicode -> VNI -> Unicode intact
        // (the default lowercasing is then a no-op).
        let s = "tiếng việt nam";
        let to_vni = convert_util(s, &opts(CodeTable::Unicode, CodeTable::VniWindows));
        let back = convert_util(&to_vni, &opts(CodeTable::VniWindows, CodeTable::Unicode));
        assert_eq!(back, s);
    }
}
