//! Macro table -- Vietnamese "gõ tắt" text expansion.
//!
//! The C++ keeps one
//! file-scope `map<vector<Uint32>, MacroData>`; here it becomes a
//! [`MacroTable`] the `Engine` owns. A `std::map` keyed by `vector<Uint32>`
//! iterates in ascending lexicographic key order, so [`MacroTable`] uses a
//! [`BTreeMap`] with the identical `Ord` (Rust `Vec<u32>` compares
//! lexicographically like `std::vector`) -- this is what makes `to_blob` /
//! `all` / `export_unikey_txt` byte-exact against the C++ save order.

use std::collections::BTreeMap;

use crate::charset::{
    character_map_get, code_table_cell, find_key_code, get_character_code, key_code_to_character,
};
use crate::data::{CHAR_CODE_MASK, CodeTable, PURE_CHARACTER_MASK};

/// Turn a UTF-8 macro string into the engine's cell vector.
///
/// Each UTF-16 code unit `t` (the C++ uses `wstring` via
/// `std::codecvt_utf8_utf16`, so BMP scalars are one unit; `encode_utf16`
/// reproduces that) maps to:
///   1. its keycode from the character map, if present (ASCII letters/digits/
///      punctuation/space); else
///   2. a re-encoded accented cell: the char is looked up in code table 0
///      (always Unicode) and re-emitted from `code_table` with
///      `CHAR_CODE_MASK` set; else
///   3. `t | PURE_CHARACTER_MASK` ("other" character passed through verbatim).
pub(crate) fn convert(text: &str, code_table: CodeTable) -> Vec<u32> {
    let mut out = Vec::new();
    // C++ `utf8ToWideString(str)` converts `str.c_str()`: an embedded NUL
    // truncates the input there.
    let text = &text[..text.find('\0').unwrap_or(text.len())];
    for unit in text.encode_utf16() {
        let t = unit as u32;
        if let Some(kc) = character_map_get(t) {
            out.push(kc);
            continue;
        }
        // find character which has tone/mark: search code table 0 (Unicode),
        // re-emit from code_table.
        if let Some((key, k)) = find_key_code(CodeTable::Unicode, unit) {
            out.push(code_table_cell(code_table, key, k) as u32 | CHAR_CODE_MASK);
            continue;
        }
        out.push(t | PURE_CHARACTER_MASK);
    }
    out
}

/// ASCII-only uppercase: only `a`-`z` are cased; higher code units pass through.
fn ascii_upper(ch: u32) -> u32 {
    if (b'a' as u32..=b'z' as u32).contains(&ch) {
        ch - 0x20
    } else {
        ch
    }
}

/// One stored macro (mirrors C++ `MacroData`).
#[derive(Debug, Clone)]
struct MacroEntry {
    /// `MacroData.macroText` (e.g. "ms").
    text: String,
    /// `MacroData.macroContent` (e.g. "millisecond").
    content: String,
    /// `MacroData.macroContentCode` -- `convert(content)` under the table in
    /// effect when this entry was (re)built.
    content_code: Vec<u32>,
}

/// The macro table (the `map` plus the active code table).
#[derive(Debug, Clone)]
pub struct MacroTable {
    /// Keyed by `convert(macroText)`; `BTreeMap` reproduces `std::map`'s
    /// ascending-key iteration order.
    map: BTreeMap<Vec<u32>, MacroEntry>,
    /// The code table the keys/content codes were built under -- as seen by
    /// `convert`/`find`. `from_blob` builds under `Unicode` (the default code
    /// table at load time); `add`/`on_table_code_change` update it, matching how
    /// the host sets the code table before those calls.
    code_table: CodeTable,
}

impl Default for MacroTable {
    fn default() -> Self {
        Self::new()
    }
}

impl MacroTable {
    /// Empty table (code table defaults to `Unicode`, the first-run default).
    pub fn new() -> Self {
        MacroTable {
            map: BTreeMap::new(),
            code_table: CodeTable::Unicode,
        }
    }

    /// The active code table (the keys/content codes were built under this).
    pub fn code_table(&self) -> CodeTable {
        self.code_table
    }

    /// Load from the on-disk blob.
    ///
    /// Blob layout: `[u16 count]` then, per macro,
    /// `[u8 textLen][text bytes][u16 contentLen][content bytes]`; all
    /// little-endian. Keys/content codes are built under `Unicode` (the
    /// default code table); call [`on_table_code_change`](Self::on_table_code_change)
    /// afterward if the host's table differs. Truncated/malformed input stops
    /// early rather than reading past the end (the C++ reads unchecked -- UB
    /// on a malformed blob; well-formed blobs are always UTF-8).
    pub fn from_blob(data: &[u8]) -> Self {
        let mut table = MacroTable::new();
        let mut cursor = 0usize;
        let mut count: u16 = 0;
        if data.len() >= 2 {
            count = u16::from_le_bytes([data[0], data[1]]);
            cursor = 2;
        }
        for _ in 0..count {
            if cursor >= data.len() {
                break;
            }
            let text_len = data[cursor] as usize;
            cursor += 1;
            if cursor + text_len > data.len() {
                break;
            }
            let text = String::from_utf8_lossy(&data[cursor..cursor + text_len]).into_owned();
            cursor += text_len;
            if cursor + 2 > data.len() {
                break;
            }
            let content_len = u16::from_le_bytes([data[cursor], data[cursor + 1]]) as usize;
            cursor += 2;
            if cursor + content_len > data.len() {
                break;
            }
            let content = String::from_utf8_lossy(&data[cursor..cursor + content_len]).into_owned();
            cursor += content_len;
            table.insert_entry(text, content);
        }
        table
    }

    /// Serialize back to the blob format.
    /// Byte-exact inverse of [`from_blob`](Self::from_blob) for well-formed
    /// (UTF-8, <=255-byte text, <=65535-byte content) data.
    pub fn to_blob(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let total = self.map.len() as u16;
        out.push(total as u8);
        out.push((total >> 8) as u8);
        for entry in self.map.values() {
            // truncates a >255-byte text (`as u8`), mirroring the source.
            out.push(entry.text.len() as u8);
            out.extend_from_slice(entry.text.as_bytes());
            let content_size = entry.content.len() as u16;
            out.push(content_size as u8);
            out.push((content_size >> 8) as u8);
            out.extend_from_slice(entry.content.as_bytes());
        }
        out
    }

    pub fn has(&self, text: &str) -> bool {
        self.map.contains_key(&convert(text, self.code_table))
    }

    /// `(text, content)` in save order.
    pub fn all(&self) -> Vec<(String, String)> {
        self.map
            .values()
            .map(|e| (e.text.clone(), e.content.clone()))
            .collect()
    }

    /// Add a new macro or replace an existing one's content. `code_table` is
    /// the host's current code table; it becomes the table used for this and
    /// subsequent lookups.
    pub fn add(&mut self, text: &str, content: &str, code_table: CodeTable) -> bool {
        self.code_table = code_table;
        let key = convert(text, code_table);
        let content_code = convert(content, code_table);
        self.map
            .entry(key)
            .and_modify(|e| {
                e.content = content.to_string();
                e.content_code = content_code.clone();
            })
            .or_insert_with(|| MacroEntry {
                text: text.to_string(),
                content: content.to_string(),
                content_code,
            });
        true
    }

    pub fn delete(&mut self, text: &str) -> bool {
        self.map.remove(&convert(text, self.code_table)).is_some()
    }

    /// Rebuild every content code for the new table. Note (like the C++) the
    /// *keys* are NOT rebuilt -- only content codes -- so an accented macro
    /// trigger keeps the key it was created under.
    pub fn on_table_code_change(&mut self, code_table: CodeTable) {
        self.code_table = code_table;
        for entry in self.map.values_mut() {
            entry.content_code = convert(&entry.content, code_table);
        }
    }

    /// Look `key` (the engine's accumulated macro key, i.e. keycodes) up and
    /// return the content codes to emit, or `None`.
    ///
    /// The input keycodes are first run through `get_character_code` in place so
    /// accented keystrokes compare against the stored `convert`'d keys. On an
    /// exact miss with `auto_caps` on, the key is lowered a column at a time
    /// (`modify_case_unicode`) and, if the lowercased key matches, the content
    /// is upper-cased: only its first cell for a Capitalized trigger, or every
    /// cell for an ALL-CAPS trigger.
    pub fn find(&self, key: &[u32], auto_caps: bool) -> Option<Vec<u32>> {
        let mut key: Vec<u32> = key
            .iter()
            .map(|&c| get_character_code(self.code_table, c))
            .collect();

        if let Some(entry) = self.map.get(&key) {
            return Some(entry.content_code.clone());
        }

        if auto_caps {
            let mut macro_flag = false;
            if key.len() > 1 && self.modify_case_unicode(&mut key[1], false) {
                macro_flag = true;
                for cell in &mut key[2..] {
                    self.modify_case_unicode(cell, false);
                }
            }
            // Nested (not collapsed) to mirror the C++ nested-`if` structure
            // verbatim.
            #[allow(clippy::collapsible_if)]
            if !key.is_empty() && self.modify_case_unicode(&mut key[0], false) {
                if let Some(entry) = self.map.get(&key) {
                    let mut content = entry.content_code.clone();
                    for (c, cell) in content.iter_mut().enumerate() {
                        if c == 0 || macro_flag {
                            let k_char = key_code_to_character(*cell);
                            if k_char != 0 {
                                let up = ascii_upper(k_char);
                                // The C++ looks up `toupper(kChar)` with
                                // operator[] (default-inserts 0 for an absent
                                // key; every reachable `up` is present, so
                                // `.unwrap_or(0)` only mirrors that never-taken
                                // default).
                                *cell = character_map_get(up).unwrap_or(0);
                                continue;
                            }
                            if *cell & CHAR_CODE_MASK != 0 {
                                self.modify_case_unicode(cell, true);
                            }
                        }
                    }
                    return Some(content);
                }
            }
        }
        None
    }

    /// Save to a `String` rather than a path. Header then `text:content\n` per
    /// macro in save order.
    ///
    /// The header keeps the UniKey-compatible shape (a leading `;` comment
    /// line, same `*** version=1 ***` suffix) so other tools that read this
    /// format still recognize the file; importers skip line 1 regardless of
    /// its text, as [`Self::import_unikey_txt`] does.
    pub fn export_unikey_txt(&self) -> String {
        let mut out =
            String::from(";Compatible VNKey Macro Data file for UniKey*** version=1 ***\n");
        for entry in self.map.values() {
            out.push_str(&entry.text);
            out.push(':');
            out.push_str(&entry.content);
            out.push('\n');
        }
        out
    }

    /// Load from a `String` rather than a path.
    /// Skips the first (header) line; for each remaining `name:content` line
    /// adds the macro if `name` is non-empty and not already present. The
    /// `while (name == "")` inner loop reattaches leading `:` runs so a macro
    /// *name* may itself contain colons (mirrors the C++ verbatim, including
    /// its use of the outer `line.length()` in the inner `substr`, harmless
    /// because `substr` clamps). Returns the number of macros added.
    pub fn import_unikey_txt(&mut self, content: &str, append: bool, code_table: CodeTable) -> usize {
        self.code_table = code_table;
        if !append {
            self.map.clear();
        }
        let mut added = 0usize;
        for (k, line) in content.split('\n').enumerate() {
            if k == 0 {
                continue; // header line
            }
            if let Some(pos) = line.find(':') {
                let mut name = line[..pos].to_string();
                let mut rest = line[pos + 1..].to_string();
                while name.is_empty() && !rest.is_empty() {
                    if let Some(p2) = rest.find(':') {
                        name.push(':');
                        name.push_str(&rest[..p2]);
                        rest = rest[p2 + 1..].to_string();
                    } else {
                        break;
                    }
                }
                if !name.is_empty() && !self.has(&name) {
                    self.add(&name, &rest, code_table);
                    added += 1;
                }
            }
        }
        added
    }

    /// Insert a `(text, content)` pair, converting both under the current
    /// table. Overwrites on a duplicate key.
    fn insert_entry(&mut self, text: String, content: String) {
        let key = convert(&text, self.code_table);
        let content_code = convert(&content, self.code_table);
        self.map.insert(
            key,
            MacroEntry {
                text,
                content,
                content_code,
            },
        );
    }

    /// Lower/upper a single cell in place, returning whether it changed.
    ///
    /// For a plain keycode cell (no `CHAR_CODE_MASK`) it just flips the caps
    /// bit -- reproducing the C++ `code &= isUpperCase ? CAPS_MASK : ~CAPS_MASK`
    /// exactly, including the quirk that the `isUpperCase` branch masks *down*
    /// to only the caps bit (unreachable in `find`, which only ever calls this
    /// with `isUpperCase == false` on keycodes, and `isUpperCase == true`
    /// only on `CHAR_CODE_MASK` cells). For an accented cell it walks the code
    /// table to find the char's `(key, column)` and moves to the adjacent
    /// caps/lower column.
    fn modify_case_unicode(&self, code: &mut u32, is_upper_case: bool) -> bool {
        let char_buff = *code;
        if *code & CHAR_CODE_MASK == 0 {
            // normal char (keycode)
            if is_upper_case {
                *code &= crate::data::CAPS_MASK;
            } else {
                *code &= !crate::data::CAPS_MASK;
            }
            return *code != char_buff;
        }
        // unicode character: rows are packed [caps, lower, caps, lower, ...].
        let target = *code as u16;
        if let Some((key, k)) = find_key_code(self.code_table, target) {
            let mut k = k;
            if k % 2 == 0 && !is_upper_case {
                k += 1;
            } else if k % 2 != 0 && is_upper_case {
                k -= 1;
            }
            *code = code_table_cell(self.code_table, key, k) as u32 | CHAR_CODE_MASK;
            return *code != char_buff;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::*;

    #[test]
    fn blob_round_trip_is_byte_exact() {
        let mut t = MacroTable::new();
        t.add("ms", "millisecond", CodeTable::Unicode);
        t.add("vn", "Việt Nam", CodeTable::Unicode);
        t.add("btw", "by the way", CodeTable::Unicode);
        let blob = t.to_blob();
        let t2 = MacroTable::from_blob(&blob);
        assert_eq!(blob, t2.to_blob(), "to_blob(from_blob(x)) must equal x");
        assert_eq!(t.all(), t2.all());
    }

    #[test]
    fn convert_ascii_trigger_is_plain_keycodes() {
        // "ms" -> [KEY_M, KEY_S] (via the character map, table-independent).
        assert_eq!(convert("ms", CodeTable::Unicode), vec![KEY_M as u32, KEY_S as u32]);
    }

    #[test]
    fn find_exact_ascii_macro() {
        let mut t = MacroTable::new();
        t.add("ms", "ms!", CodeTable::Unicode);
        // typing "ms" accumulates macroKey [KEY_M, KEY_S].
        let content = t.find(&[KEY_M as u32, KEY_S as u32], false).unwrap();
        assert_eq!(content, convert("ms!", CodeTable::Unicode));
    }

    #[test]
    fn find_miss_returns_none() {
        let t = MacroTable::new();
        assert!(t.find(&[KEY_M as u32, KEY_S as u32], false).is_none());
    }

    #[test]
    fn all_is_in_ascending_key_order() {
        let mut t = MacroTable::new();
        t.add("zz", "z", CodeTable::Unicode);
        t.add("aa", "a", CodeTable::Unicode);
        // keys convert to [KEY_A,KEY_A]=[0,0] < [KEY_Z,KEY_Z]; A sorts first.
        let all = t.all();
        assert_eq!(all[0].0, "aa");
        assert_eq!(all[1].0, "zz");
    }

    #[test]
    fn delete_and_has() {
        let mut t = MacroTable::new();
        t.add("ms", "x", CodeTable::Unicode);
        assert!(t.has("ms"));
        assert!(t.delete("ms"));
        assert!(!t.has("ms"));
        assert!(!t.delete("ms"));
    }

    #[test]
    fn unikey_txt_round_trip_including_colon_in_name() {
        let mut t = MacroTable::new();
        t.add("ms", "millisecond", CodeTable::Unicode);
        t.add("vn", "Việt Nam", CodeTable::Unicode);
        let txt = t.export_unikey_txt();
        assert!(txt.starts_with(";Compatible VNKey Macro Data file for UniKey*** version=1 ***\n"));
        let mut t2 = MacroTable::new();
        t2.import_unikey_txt(&txt, false, CodeTable::Unicode);
        assert_eq!(t.all(), t2.all());
    }

    /// Files exported by other tools (or by older builds) carry a different
    /// header line. Import must still read them: line 1 is skipped whatever it
    /// says, so only the `name:content` lines matter.
    #[test]
    fn import_accepts_any_header_line() {
        let foreign = ";Compatible OpenKey Macro Data file for UniKey*** version=1 ***\nms:millisecond\nvn:Việt Nam\n";
        let mut t = MacroTable::new();
        let added = t.import_unikey_txt(foreign, false, CodeTable::Unicode);

        assert_eq!(added, 2, "both macros must import despite the unfamiliar header");
        // `all()` follows map order, so compare as a sorted set.
        let mut entries = t.all();
        entries.sort();
        assert_eq!(
            entries,
            vec![
                ("ms".to_string(), "millisecond".to_string()),
                ("vn".to_string(), "Việt Nam".to_string()),
            ]
        );
    }
}
