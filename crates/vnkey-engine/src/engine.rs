//! The typing engine: the normal-key path plus main-key handling, key/state
//! insertion, vowel finding, quick-telex, the standalone-`[`/`]`/`w` helpers
//! and upper-casing the first character.
//!
//! Every C++ file-scope global becomes a private `Engine` field; the scratch
//! loop variables `i, ii, iii, j, k, kk, l` become locals. All C++ index
//! arithmetic is reproduced with `i32` (the C++ `Byte`s are promoted to
//! `int` in every expression, and negative intermediates / `>= 0` guards are
//! load-bearing).
//!
//! Scope: break-code / SPACE / DELETE (Task 5) plus the normal-key path
//! (Task 4). Macro lookup, English mode, and the temp-off-engine/spelling
//! session methods were added in Task 6/7.
//!
//! Clippy lints are allowed where "simplifying" would break the 1:1 mapping to
//! the source: `int_plus_one` keeps the C++ `x - 1 >= 0` index guards
//! verbatim, and `collapsible_if` keeps the C++ nested-`if` gates intact.
#![allow(clippy::int_plus_one, clippy::collapsible_if)]

use crate::buffer::Buffer;
use crate::charset::get_character_code;
use crate::data::{
    CAPS_MASK, CodeTable, HookCode, HookState, InputType, KeyEvent, KeyEventState, MARK1_MASK,
    MARK2_MASK, MARK3_MASK, MARK4_MASK, MARK5_MASK, MARK_MASK, MAX_BUFF, STANDALONE_MASK, TONE_MASK,
    TONEW_MASK,
};
use crate::input_method::{
    is_bracket_key, is_consonant, is_key_d, is_key_double, is_key_f, is_key_j, is_key_r, is_key_s,
    is_key_w, is_key_x, is_key_z, is_mark_key, is_number_key, is_quick_telex_key, is_special_key,
};
use crate::keys::*;
use crate::macros::MacroTable;
use crate::options::EngineOptions;
use crate::vietnamese::{
    BREAK_CODE, CHAR_KEY_CODE, CONSONANT_D, MACRO_BREAK_CODE, QUICK_END_CONSONANT,
    QUICK_START_CONSONANT, QUICK_TELEX, STANDALONE_WBAD, VOWEL, VOWEL_FOR_MARK,
};

pub struct Engine {
    pub(crate) opts: EngineOptions,
    /// An option, but mutated at runtime by the Ctrl spell-toggle; kept
    /// separate from `opts.check_spelling`.
    pub(crate) spell_check_on: bool,
    pub(crate) use_spell_checking_before: bool,

    pub(crate) buf: Buffer,

    // engine flags / vowel-position scratch (mirror the like-named C++ statics)
    pub(crate) temp_disable_key: bool,
    pub(crate) is_corect: bool,
    pub(crate) is_changed: bool,
    pub(crate) vowel_count: i32,
    /// Vowel start index (VSI).
    pub(crate) vsi: i32,
    /// Vowel end index (VEI).
    pub(crate) vei: i32,
    /// Vowel-will-set-mark index (VWSM); `insert_mark` may override it to VEI.
    pub(crate) vwsm: i32,
    pub(crate) is_restored_w: bool,
    pub(crate) key_for_aeo: u16,

    // break/space/upper-case tracking + typing-state history
    pub(crate) space_count: i32,
    pub(crate) upper_case_status: u8,
    pub(crate) special_char: Vec<u32>,
    /// Stack of saved words/space-runs/special-char runs, used to restore
    /// across word boundaries on DELETE.
    pub(crate) typing_states: Vec<Vec<u32>>,
    pub(crate) will_temp_off_engine: bool,
    pub(crate) has_handled_macro: bool,
    pub(crate) has_handle_quick_consonant: bool,

    // check_spelling scratch
    pub(crate) spelling_ok: bool,
    pub(crate) spelling_flag: bool,
    pub(crate) spelling_vowel_ok: bool,
    pub(crate) spelling_end_index: i32,

    /// The owned macro table (plus its active code table), consulted by
    /// `find_macro` and managed via `macro_*` accessors. Kept in sync with
    /// `opts.code_table` by `new`/`set_options`.
    pub(crate) macros: MacroTable,

    pub(crate) hook: HookState,
}

impl Engine {
    /// Apply options and start a fresh engine.
    pub fn new(opts: EngineOptions) -> Self {
        Engine {
            opts,
            spell_check_on: opts.check_spelling,
            use_spell_checking_before: opts.check_spelling,
            buf: Buffer::new(),
            temp_disable_key: false,
            is_corect: false,
            is_changed: false,
            vowel_count: 0,
            vsi: 0,
            vei: 0,
            vwsm: 0,
            is_restored_w: false,
            key_for_aeo: 0,
            space_count: 0,
            upper_case_status: 0,
            special_char: Vec::new(),
            typing_states: Vec::new(),
            will_temp_off_engine: false,
            has_handled_macro: false,
            has_handle_quick_consonant: false,
            spelling_ok: false,
            spelling_flag: false,
            spelling_vowel_ok: false,
            spelling_end_index: 0,
            macros: {
                let mut m = MacroTable::new();
                m.on_table_code_change(opts.code_table);
                m
            },
            hook: HookState::default(),
        }
    }

    /// Re-apply options: update the runtime spelling flag from the new options,
    /// leave the typing buffer intact.
    ///
    /// Changing `code_table` also runs `MacroTable::on_table_code_change` (the
    /// host would re-encode after switching tables, so a loaded macro table's
    /// content codes track the active table).
    pub fn set_options(&mut self, opts: EngineOptions) {
        if self.macros.code_table() != opts.code_table {
            self.macros.on_table_code_change(opts.code_table);
        }
        self.opts = opts;
        self.spell_check_on = opts.check_spelling;
    }

    /// Switch the active code table. Re-encodes the macro table's content codes
    /// for the new table (the same re-encode `set_options` models) and updates
    /// the option. Used by the smart-switch remember-code branch to restore a
    /// per-app code table.
    pub fn set_code_table(&mut self, code_table: CodeTable) {
        if self.opts.code_table != code_table {
            self.macros.on_table_code_change(code_table);
            self.opts.code_table = code_table;
        }
    }

    /// Shared read access to the macro table (for the host/IPC to list
    /// macros, export blobs, etc.).
    pub fn macro_table(&self) -> &MacroTable {
        &self.macros
    }

    /// Mutable access to the macro table (for the host/IPC to add/delete
    /// macros or load a blob at startup). Keeps the table's code-table field
    /// in sync with the engine's current option.
    pub fn macro_table_mut(&mut self) -> &mut MacroTable {
        &mut self.macros
    }

    /// Replace the macro table wholesale (e.g. a blob loaded at startup),
    /// re-syncing its content codes to the engine's active code table.
    pub fn set_macro_table(&mut self, mut table: MacroTable) {
        if table.code_table() != self.opts.code_table {
            table.on_table_code_change(self.opts.code_table);
        }
        self.macros = table;
    }

    /// Add a macro to the owned table, under the engine's active code table.
    pub fn macro_add(&mut self, text: &str, content: &str) -> bool {
        self.macros.add(text, content, self.opts.code_table)
    }

    /// Delete a macro from the owned table.
    pub fn macro_delete(&mut self, text: &str) -> bool {
        self.macros.delete(text)
    }

    /// All macros on the owned table -- `(text, content)` in save order.
    pub fn macro_list(&self) -> Vec<(String, String)> {
        self.macros.all()
    }

    pub fn start_new_session(&mut self) {
        self.buf.index = 0;
        self.hook.backspace_count = 0;
        self.hook.new_char_count = 0;
        self.temp_disable_key = false;
        self.buf.state_index = 0;
        self.has_handled_macro = false;
        self.has_handle_quick_consonant = false;
        self.buf.long_word_helper.clear();
    }

    /// Only flips the *current* spelling flag (`spell_check_on`) when
    /// `use_spell_checking_before` (the "original" value saved by
    /// `reset_check_spelling`) is set -- so calling this repeatedly toggles the
    /// runtime flag on/off around that saved baseline instead of
    /// unconditionally flipping it.
    pub fn temp_off_spell_checking(&mut self) {
        if self.use_spell_checking_before {
            self.spell_check_on = !self.spell_check_on;
        }
    }

    /// Snapshots the current runtime spelling flag as the new "original"
    /// baseline that `temp_off_spell_checking` toggles around.
    pub fn reset_check_spelling(&mut self) {
        self.use_spell_checking_before = self.spell_check_on;
    }

    pub fn temp_off_engine(&mut self, off: bool) {
        self.will_temp_off_engine = off;
    }

    /// `get_character_code` against the active table.
    #[inline]
    pub(crate) fn get(&self, data: u32) -> u32 {
        get_character_code(self.opts.code_table, data)
    }

    #[inline]
    pub(crate) fn input_type(&self) -> InputType {
        self.opts.input_type
    }

    fn is_word_break(event: KeyEvent, data: u16) -> bool {
        if event == KeyEvent::Mouse {
            return true;
        }
        BREAK_CODE.iter().any(|&b| b as u16 == data)
    }

    fn is_macro_break_code(data: u16) -> bool {
        MACRO_BREAK_CODE.iter().any(|&b| b as u16 == data)
    }

    /// Membership test against `CHAR_KEY_CODE`.
    fn is_char_key_code(data: u16) -> bool {
        CHAR_KEY_CODE.iter().any(|&b| b as u16 == data)
    }

    /// Look the accumulated `hook.macro_key` up in the owned `MacroTable` and,
    /// on a hit, fill `hook.macro_data` with the content codes and return true.
    /// On a miss, leaves `hook.macro_data` untouched (only clears/assigns the
    /// content codes on a match). The break/SPACE/english-mode call sites set
    /// `hook.code = ReplaceMacro` / `hook.backspace_count = macro_key.len()`
    /// themselves, exactly like the source.
    fn find_macro(&mut self) -> bool {
        match self.macros.find(&self.hook.macro_key, self.opts.auto_caps_macro) {
            Some(content) => {
                self.hook.macro_data = content;
                true
            }
            None => false,
        }
    }

    /// Save the current word (no-arg overload).
    fn save_word(&mut self) {
        if self.hook.code != HookCode::ReplaceMacro {
            if self.buf.index > 0 {
                if !self.buf.long_word_helper.is_empty() {
                    // save long word first
                    let mut data: Vec<u32> = Vec::new();
                    for (i, &w) in self.buf.long_word_helper.iter().enumerate() {
                        if i != 0 && i % MAX_BUFF == 0 {
                            // save if overflow
                            self.typing_states.push(std::mem::take(&mut data));
                        }
                        data.push(w);
                    }
                    self.typing_states.push(data);
                    self.buf.long_word_helper.clear();
                }

                // save current word
                let data: Vec<u32> = self.buf.word[..self.buf.index as usize].to_vec();
                self.typing_states.push(data);
            }
        } else {
            // save macro words
            let mut data: Vec<u32> = Vec::new();
            for (i, &w) in self.hook.macro_data.iter().enumerate() {
                if i != 0 && i % MAX_BUFF == 0 {
                    // break if overflow
                    self.typing_states.push(std::mem::take(&mut data));
                }
                data.push(w);
            }
            self.typing_states.push(data);
        }
    }

    /// Save a repeated key `count` times (2-arg overload). Every call site
    /// guards `count > 0` before calling, matching the C++ loop
    /// (`for (i = 0; i < count; i++)`), which never runs for `count <= 0`.
    fn save_word_repeated(&mut self, key_code: u32, count: i32) {
        self.typing_states.push(vec![key_code; count as usize]);
    }

    fn restore_last_typing_state(&mut self) {
        if let Some(data) = self.typing_states.pop() {
            if !data.is_empty() {
                // Compare the full, untruncated u32 element against KEY_SPACE
                // (no cast).
                if data[0] == KEY_SPACE as u32 {
                    self.space_count = data.len() as i32;
                    self.buf.index = 0;
                } else if CHAR_KEY_CODE.iter().any(|&b| b as u16 == data[0] as u16) {
                    // Truncate to `u16` before the lookup.
                    self.buf.index = 0;
                    self.special_char = data;
                    self.check_spelling(false);
                } else {
                    for (i, &w) in data.iter().enumerate() {
                        self.buf.word[i] = w;
                    }
                    // Truncate the size to 8 bits (`as u8`).
                    self.buf.index = (data.len() as u8) as i32;
                }
            }
        }
    }

    fn check_restore_if_wrong_spelling(&mut self, handle_code: HookCode) -> bool {
        let mut ii = 0i32;
        while ii < self.buf.index {
            let w = self.buf.word[ii as usize];
            if !is_consonant(self.buf.chr(ii))
                && (w & MARK_MASK != 0 || w & TONE_MASK != 0 || w & TONEW_MASK != 0)
            {
                self.hook.code = handle_code;
                self.hook.backspace_count = self.buf.index as u8;
                self.hook.new_char_count = self.buf.state_index as u8;
                let mut i = 0i32;
                while i < self.buf.state_index {
                    self.buf.word[i as usize] = self.buf.states[i as usize];
                    // mirror the source quirk: char_data is filled with the RAW
                    // word[i] value (post-restore), not get(word[i]).
                    self.hook.char_data[(self.buf.state_index - 1 - i) as usize] =
                        self.buf.word[i as usize];
                    i += 1;
                }
                self.buf.index = self.buf.state_index;
                return true;
            }
            ii += 1;
        }
        false
    }

    fn check_quick_consonant(&mut self) -> bool {
        if self.buf.index <= 1 {
            return false;
        }
        let mut l = 0;
        if self.buf.index > 0 {
            if self.opts.quick_start_consonant {
                if let Some(row) = QUICK_START_CONSONANT.get(&self.buf.chr(0)) {
                    self.hook.code = HookCode::Restore;
                    self.hook.backspace_count = self.buf.index as u8;
                    self.hook.new_char_count = (self.buf.index + 1) as u8;
                    if self.buf.index < MAX_BUFF as i32 - 1 {
                        self.buf.index += 1;
                    }
                    // right shift
                    let mut i = self.buf.index - 1;
                    while i >= 2 {
                        self.buf.word[i as usize] = self.buf.word[(i - 1) as usize];
                        i -= 1;
                    }
                    let caps0 = self.buf.word[0] & CAPS_MASK != 0;
                    let caps2 = self.buf.word[2] & CAPS_MASK != 0;
                    self.buf.word[1] = row[1] as u32 | if caps0 && caps2 { CAPS_MASK } else { 0 };
                    self.buf.word[0] = row[0] as u32 | if caps0 { CAPS_MASK } else { 0 };
                    l = 1;
                }
            }
            if self.opts.quick_end_consonant
                && self.buf.index - 2 >= 0
                && !is_consonant(self.buf.chr(self.buf.index - 2))
                && QUICK_END_CONSONANT.contains_key(&self.buf.chr(self.buf.index - 1))
            {
                self.hook.code = HookCode::Restore;
                if l == 1 {
                    self.hook.new_char_count += 1;
                } else {
                    self.hook.backspace_count = 1;
                    self.hook.new_char_count = 2;
                }
                if self.buf.index < MAX_BUFF as i32 - 1 {
                    self.buf.index += 1;
                }
                // mirror the source quirk / UB: the source re-looks-up
                // `QUICK_END_CONSONANT[chr(index-2)]` (operator[], keyed by the
                // index AFTER the conditional increment above) instead of
                // reusing this guard's `chr(index-1)` (pre-increment) key.
                // When `index` was already MAX_BUFF-1 at guard time, the
                // increment above is skipped, so the re-lookup key differs
                // from the guard's validated key -- if that different
                // character is not itself a `QUICK_END_CONSONANT` key, the C++
                // `operator[]` default-constructs an empty vector and the
                // `[0]`/`[1]` indexing below is an out-of-bounds read (genuine
                // UB, not a defined quirk, only reachable at
                // `index == MAX_BUFF-1`). We look the key up fresh (matching
                // the source's statement order/re-lookup exactly) and, on a
                // miss, skip the mutation as a safe no-op instead of indexing
                // into nothing.
                if let Some(row2) = QUICK_END_CONSONANT.get(&self.buf.chr(self.buf.index - 2)) {
                    let caps = self.buf.word[(self.buf.index - 2) as usize] & CAPS_MASK != 0;
                    self.buf.word[(self.buf.index - 1) as usize] =
                        row2[1] as u32 | if caps { CAPS_MASK } else { 0 };
                    let caps2 = self.buf.word[(self.buf.index - 2) as usize] & CAPS_MASK != 0;
                    self.buf.word[(self.buf.index - 2) as usize] =
                        row2[0] as u32 | if caps2 { CAPS_MASK } else { 0 };
                }
                l = 1;
            }
            if l == 1 {
                self.has_handle_quick_consonant = true;
                let mut i = self.buf.index - 1;
                while i >= 0 {
                    self.hook.char_data[(self.buf.index - 1 - i) as usize] =
                        self.get(self.buf.word[i as usize]);
                    i -= 1;
                }
                return true;
            }
        }
        false
    }

    pub(crate) fn insert_key(&mut self, key_code: u16, is_caps: bool, is_check_spelling: bool) {
        self.buf.push_word(key_code, is_caps);

        if self.spell_check_on && is_check_spelling {
            self.check_spelling(false);
        }

        // allow d after consonant
        if key_code == KEY_D
            && self.buf.index - 2 >= 0
            && is_consonant(self.buf.chr(self.buf.index - 2))
        {
            self.temp_disable_key = false;
        }
    }

    pub(crate) fn find_and_calculate_vowel(&mut self, for_grammar: bool) {
        self.vowel_count = 0;
        self.vsi = 0;
        self.vei = 0;
        let mut iii = self.buf.index - 1;
        while iii >= 0 {
            if is_consonant(self.buf.chr(iii)) {
                if self.vowel_count > 0 {
                    break;
                }
            } else {
                // is vowel
                if self.vowel_count == 0 {
                    self.vei = iii;
                }
                if !for_grammar
                    && ((iii - 1 >= 0
                        && self.buf.chr(iii) == KEY_I
                        && self.buf.chr(iii - 1) == KEY_G)
                        || (iii - 1 >= 0
                            && self.buf.chr(iii) == KEY_U
                            && self.buf.chr(iii - 1) == KEY_Q))
                {
                    break;
                }
                self.vsi = iii;
                self.vowel_count += 1;
            }
            iii -= 1;
        }
        // Aug 26 2019: don't count "u" of "qu" as a vowel.
        if self.vsi - 1 >= 0
            && self.buf.chr(self.vsi) == KEY_U
            && self.buf.chr(self.vsi - 1) == KEY_Q
        {
            self.vsi += 1;
            self.vowel_count -= 1;
        }
    }

    fn reverse_last_standalone_char(&mut self, key_code: u16, is_caps: bool) {
        self.hook.code = HookCode::WillProcess;
        self.hook.backspace_count = 0;
        self.hook.new_char_count = 1;
        self.hook.ext_code = 4;
        let idx = self.buf.index - 1;
        self.buf.word[idx as usize] =
            key_code as u32 | TONEW_MASK | STANDALONE_MASK | if is_caps { CAPS_MASK } else { 0 };
        self.hook.char_data[0] = self.get(self.buf.word[idx as usize]);
    }

    fn check_for_standalone_char(&mut self, data: u16, is_caps: bool, key_will_reverse: u16) {
        // mirror the source quirk: the source reads chr(index-1)/word[index-1]
        // here unguarded, which is an out-of-bounds read when index == 0 (UB).
        // After a fresh reset that read is non-TONEW, so the branch is never
        // taken; we guard `index > 0` to get the same effect without the OOB
        // read.
        if self.buf.index > 0
            && self.buf.chr(self.buf.index - 1) == key_will_reverse
            && (self.buf.word[(self.buf.index - 1) as usize] & TONEW_MASK) != 0
        {
            self.hook.code = HookCode::WillProcess;
            self.hook.backspace_count = 1;
            self.hook.new_char_count = 1;
            let idx = (self.buf.index - 1) as usize;
            self.buf.word[idx] = data as u32 | if is_caps { CAPS_MASK } else { 0 };
            self.hook.char_data[0] = self.get(self.buf.word[idx]);
            return;
        }

        // check standalone w -> ư
        if self.buf.index > 0
            && self.buf.chr(self.buf.index - 1) == KEY_U
            && key_will_reverse == KEY_O
        {
            self.insert_key(key_will_reverse, is_caps, true);
            self.reverse_last_standalone_char(key_will_reverse, is_caps);
            return;
        }

        if self.buf.index == 0 {
            self.insert_key(data, is_caps, false);
            self.reverse_last_standalone_char(key_will_reverse, is_caps);
        } else if self.buf.index == 1 {
            for &bad in STANDALONE_WBAD {
                if self.buf.chr(0) == bad {
                    self.insert_key(data, is_caps, true);
                    return;
                }
            }
            self.insert_key(data, is_caps, false);
            self.reverse_last_standalone_char(key_will_reverse, is_caps);
        } else if self.buf.index == 2 {
            for pair in crate::vietnamese::DOUBLE_W_ALLOWED {
                if self.buf.chr(0) == pair[0] && self.buf.chr(1) == pair[1] {
                    self.insert_key(data, is_caps, false);
                    self.reverse_last_standalone_char(key_will_reverse, is_caps);
                    return;
                }
            }
            self.insert_key(data, is_caps, true);
        } else {
            self.insert_key(data, is_caps, true);
        }
    }

    fn upper_case_first_character(&mut self) {
        if self.buf.word[0] & CAPS_MASK == 0 {
            self.hook.code = HookCode::WillProcess;
            self.hook.backspace_count = 0;
            self.hook.new_char_count = 1;
            self.buf.word[0] |= CAPS_MASK;
            self.hook.char_data[0] = self.get(self.buf.word[0]);
            self.upper_case_status = 0;
            // The source indexes macro_key[0] unconditionally under use_macro;
            // guard non-empty (only reachable in states our tests never produce).
            if self.opts.use_macro {
                if let Some(first) = self.hook.macro_key.first_mut() {
                    *first |= CAPS_MASK;
                }
            }
        }
    }

    fn handle_quick_telex(&mut self, data: u16, is_caps: bool) {
        let pair = &QUICK_TELEX[&(data as u32)];
        let caps = if is_caps { CAPS_MASK } else { 0 };
        self.hook.code = HookCode::WillProcess;
        self.hook.backspace_count = 1;
        self.hook.new_char_count = 2;
        self.hook.char_data[1] = pair[0] as u32 | caps;
        self.hook.char_data[0] = pair[1] as u32 | caps;
        self.insert_key(pair[1], is_caps, false);
    }

    fn handle_main_key(&mut self, data: u16, is_caps: bool) {
        let it = self.input_type();

        // Z key -> remove mark
        if is_key_z(it, data) {
            self.remove_mark();
            if !self.is_changed {
                self.insert_key(data, is_caps, true);
            }
            return;
        }

        if data == KEY_LEFT_BRACKET {
            self.check_for_standalone_char(data, is_caps, KEY_O);
            return;
        }
        if data == KEY_RIGHT_BRACKET {
            self.check_for_standalone_char(data, is_caps, KEY_U);
            return;
        }

        // D key
        if is_key_d(it, data) {
            self.is_corect = false;
            self.is_changed = false;
            for row in CONSONANT_D {
                if self.buf.index < row.len() as i32 {
                    continue;
                }
                self.is_corect = true;
                self.check_correct_vowel(row, data);

                // allow d after consonant
                if !self.is_corect
                    && self.buf.index - 2 >= 0
                    && self.buf.chr(self.buf.index - 1) == KEY_D
                    && is_consonant(self.buf.chr(self.buf.index - 2))
                {
                    self.is_corect = true;
                }
                if self.is_corect {
                    self.is_changed = true;
                    self.insert_d(data, is_caps);
                    break;
                }
            }
            if !self.is_changed {
                self.insert_key(data, is_caps, true);
            }
            return;
        }

        // mark key
        if is_mark_key(it, data) {
            // std::map iterates keys ascending; reproduce that order.
            let mut keys: Vec<u16> = VOWEL_FOR_MARK.keys().copied().collect();
            keys.sort_unstable();
            for key in keys {
                let charset = &VOWEL_FOR_MARK[&key];
                self.is_corect = false;
                self.is_changed = false;
                for row in charset {
                    if self.buf.index < row.len() as i32 {
                        continue;
                    }
                    self.is_corect = true;
                    self.check_correct_vowel(row, data);

                    if self.is_corect {
                        self.is_changed = true;
                        if is_key_s(it, data) {
                            self.insert_mark(MARK1_MASK, true);
                        } else if is_key_f(it, data) {
                            self.insert_mark(MARK2_MASK, true);
                        } else if is_key_r(it, data) {
                            self.insert_mark(MARK3_MASK, true);
                        } else if is_key_x(it, data) {
                            self.insert_mark(MARK4_MASK, true);
                        } else if is_key_j(it, data) {
                            self.insert_mark(MARK5_MASK, true);
                        }
                        break;
                    }
                }
                if self.is_corect {
                    break;
                }
            }
            if !self.is_changed {
                self.insert_key(data, is_caps, true);
            }
            return;
        }

        // check vowel
        if it == InputType::Vni {
            let mut i = self.buf.index - 1;
            while i >= 0 {
                let c = self.buf.chr(i);
                if c == KEY_O || c == KEY_A || c == KEY_E {
                    self.vei = i;
                    break;
                }
                i -= 1;
            }
        }

        self.key_for_aeo = if it != InputType::Vni {
            data
        } else if data == KEY_7 || data == KEY_8 {
            KEY_W
        } else if data == KEY_6 {
            self.buf.chr(self.vei)
        } else {
            data
        };

        // VOWEL[key_for_aeo]: a missing key is an empty row set (C++ map[]).
        let charset: &[Vec<u16>] = VOWEL.get(&self.key_for_aeo).map_or(&[], |v| v.as_slice());
        self.is_corect = false;
        self.is_changed = false;
        for row in charset {
            if self.buf.index < row.len() as i32 {
                continue;
            }
            self.is_corect = true;
            self.check_correct_vowel(row, data);

            if self.is_corect {
                self.is_changed = true;
                if is_key_double(it, data) {
                    self.insert_aoe(self.key_for_aeo, is_caps);
                } else if is_key_w(it, data) {
                    if it == InputType::Vni {
                        let mut j = self.buf.index - 1;
                        while j >= 0 {
                            let c = self.buf.chr(j);
                            if c == KEY_O || c == KEY_U || c == KEY_A || c == KEY_E {
                                self.vei = j;
                                break;
                            }
                            j -= 1;
                        }
                        let vei_ok_a = data == KEY_7
                            && self.buf.chr(self.vei) == KEY_A
                            && (if self.vei - 1 >= 0 {
                                self.buf.chr(self.vei - 1) != KEY_U
                            } else {
                                true
                            });
                        let vei_ok_ou = data == KEY_8
                            && (self.buf.chr(self.vei) == KEY_O || self.buf.chr(self.vei) == KEY_U);
                        if vei_ok_a || vei_ok_ou {
                            break;
                        }
                    }
                    self.insert_w(self.key_for_aeo, is_caps);
                }
                break;
            }
        }

        if !self.is_changed {
            if data == KEY_W && it != InputType::SimpleTelex1 {
                self.check_for_standalone_char(data, is_caps, KEY_U);
            } else {
                self.insert_key(data, is_caps, true);
            }
        }
    }

    /// The main event handler (normal-key path).
    pub fn handle_event(
        &mut self,
        event: KeyEvent,
        state: KeyEventState,
        keycode: u16,
        caps_status: u8,
        other_control: bool,
    ) -> &HookState {
        let data = keycode;
        let is_caps = caps_status == 1 || caps_status == 2;

        if (is_number_key(data) && caps_status == 1)
            || other_control
            || Self::is_word_break(event, data)
            || (self.buf.index == 0 && is_number_key(data))
        {
            // Break/word-break path.
            self.hook.code = HookCode::DoNothing;
            self.hook.backspace_count = 0;
            self.hook.new_char_count = 0;
            self.hook.ext_code = 1; // word break

            // check macro feature
            if self.opts.use_macro
                && Self::is_macro_break_code(data)
                && !self.has_handled_macro
                && self.find_macro()
            {
                self.hook.code = HookCode::ReplaceMacro;
                self.hook.backspace_count = self.hook.macro_key.len() as u8;
                self.has_handled_macro = true;
            } else if (self.opts.quick_start_consonant || self.opts.quick_end_consonant)
                && !self.temp_disable_key
                && Self::is_macro_break_code(data)
            {
                self.check_quick_consonant();
            } else if self.opts.restore_if_wrong_spelling && Self::is_word_break(event, data) {
                // restore key if wrong spelling with break-key
                if !self.temp_disable_key && self.spell_check_on {
                    self.check_spelling(true); // force check spelling
                }
                if self.temp_disable_key
                    && !self.check_restore_if_wrong_spelling(HookCode::RestoreAndStartNewSession)
                {
                    self.hook.code = HookCode::DoNothing;
                }
            }

            let is_char_key_code = state == KeyEventState::KeyDown && Self::is_char_key_code(data);
            if !is_char_key_code {
                // clear all line cache
                self.special_char.clear();
                self.typing_states.clear();
            } else {
                // check and save current word
                if self.space_count > 0 {
                    self.save_word_repeated(KEY_SPACE as u32, self.space_count);
                    self.space_count = 0;
                } else {
                    self.save_word();
                }
                self.special_char.push(data as u32 | if is_caps { CAPS_MASK } else { 0 });
                self.hook.ext_code = 3; // normal word
            }

            if self.hook.code == HookCode::DoNothing {
                self.start_new_session();
                self.spell_check_on = self.use_spell_checking_before;
                self.will_temp_off_engine = false;
            } else if self.hook.code == HookCode::ReplaceMacro || self.has_handle_quick_consonant {
                self.buf.index = 0;
            }

            // insert key for macro function
            if self.opts.use_macro {
                if is_char_key_code {
                    self.hook.macro_key.push(data as u32 | if is_caps { CAPS_MASK } else { 0 });
                } else {
                    self.hook.macro_key.clear();
                }
            }

            if self.opts.upper_case_first_char {
                if data == KEY_DOT {
                    self.upper_case_status = 1;
                } else if data == KEY_ENTER || data == KEY_RETURN {
                    self.upper_case_status = 2;
                } else {
                    self.upper_case_status = 0;
                }
            }

            return &self.hook;
        } else if data == KEY_SPACE {
            // SPACE path. `hook.code` is NOT reset at branch entry -- it stays
            // whatever it was from the previous call unless one of the
            // sub-branches below explicitly sets it.
            if !self.temp_disable_key && self.spell_check_on {
                self.check_spelling(true); // force check spelling
            }
            if self.opts.use_macro && !self.has_handled_macro && self.find_macro() {
                // macro
                self.hook.code = HookCode::ReplaceMacro;
                self.hook.backspace_count = self.hook.macro_key.len() as u8;
                self.space_count += 1;
                self.has_handled_macro = true;
            } else if (self.opts.quick_start_consonant || self.opts.quick_end_consonant)
                && !self.temp_disable_key
                && self.check_quick_consonant()
            {
                self.space_count += 1;
            } else if self.opts.restore_if_wrong_spelling
                && self.temp_disable_key
                && !self.has_handled_macro
            {
                // restore key if wrong spelling
                if !self.check_restore_if_wrong_spelling(HookCode::Restore) {
                    self.hook.code = HookCode::DoNothing;
                }
                self.space_count += 1;
            } else {
                // do nothing with SPACE KEY
                self.hook.code = HookCode::DoNothing;
                self.space_count += 1;
            }
            if self.opts.use_macro {
                self.hook.macro_key.clear();
            }
            if self.opts.upper_case_first_char && self.upper_case_status == 1 {
                self.upper_case_status = 2;
            }
            // save word
            if self.space_count == 1 {
                if !self.special_char.is_empty() {
                    self.save_special_char();
                } else {
                    self.save_word();
                }
            }
            self.spell_check_on = self.use_spell_checking_before;
            self.will_temp_off_engine = false;
            return &self.hook;
        } else if data == KEY_DELETE {
            // DELETE path.
            self.hook.code = HookCode::DoNothing;
            self.hook.ext_code = 2; // delete

            if !self.special_char.is_empty() {
                self.special_char.pop();
                if self.special_char.is_empty() {
                    self.restore_last_typing_state();
                }
            } else if self.space_count > 0 {
                // previous char is space
                self.space_count -= 1;
                if self.space_count == 0 {
                    // restore word
                    self.restore_last_typing_state();
                }
            } else {
                if self.buf.state_index > 0 {
                    self.buf.state_index -= 1;
                }
                if self.buf.index > 0 {
                    self.buf.index -= 1;
                    self.buf.restore_long_word_helper();
                    if self.spell_check_on {
                        self.check_spelling(false);
                    }
                }
                if self.opts.use_macro && !self.hook.macro_key.is_empty() {
                    self.hook.macro_key.pop();
                }

                self.hook.backspace_count = 0;
                self.hook.new_char_count = 0;
                self.hook.ext_code = 2; // delete key
                if self.buf.index == 0 {
                    self.start_new_session();
                    self.special_char.clear();
                    self.restore_last_typing_state();
                } else {
                    // August 23rd continue check grammar
                    self.check_grammar(1);
                }
            }

            return &self.hook;
        }

        // -- normal key --
        if self.will_temp_off_engine {
            self.hook.code = HookCode::DoNothing;
            self.hook.ext_code = 3;
            return &self.hook;
        }
        if self.space_count > 0 {
            self.hook.backspace_count = 0;
            self.hook.new_char_count = 0;
            self.hook.ext_code = 0;
            self.start_new_session();
            // continue save space
            self.save_word_repeated(KEY_SPACE as u32, self.space_count);
            self.space_count = 0;
        } else if !self.special_char.is_empty() {
            self.save_special_char();
        }

        self.buf.push_state(data, is_caps);

        if !is_special_key(self.input_type(), data) || self.temp_disable_key {
            if self.opts.quick_telex
                && is_quick_telex_key(
                    self.buf.index,
                    if self.buf.index > 0 {
                        self.buf.chr(self.buf.index - 1)
                    } else {
                        0
                    },
                    data,
                )
            {
                self.handle_quick_telex(data, is_caps);
                return &self.hook;
            } else {
                self.hook.code = HookCode::DoNothing;
                self.hook.backspace_count = 0;
                self.hook.new_char_count = 0;
                self.hook.ext_code = 3; // normal key
                self.insert_key(data, is_caps, true);
            }
        } else {
            self.hook.code = HookCode::DoNothing;
            self.hook.ext_code = 3; // normal key
            self.handle_main_key(data, is_caps);
        }

        if !self.opts.free_mark && !is_key_d(self.input_type(), data) {
            if self.hook.code == HookCode::DoNothing {
                self.check_grammar(-1);
            } else {
                self.check_grammar(0);
            }
        }

        if self.hook.code == HookCode::Restore {
            self.insert_key(data, is_caps, true);
            self.buf.state_index -= 1;
        }

        // macro key maintenance (only mutates the uncompared macroKey vector)
        if self.opts.use_macro {
            if self.hook.code == HookCode::DoNothing {
                self.hook.macro_key.push(data as u32 | if is_caps { CAPS_MASK } else { 0 });
            } else if self.hook.code == HookCode::WillProcess
                || self.hook.code == HookCode::Restore
            {
                let bpc = self.hook.backspace_count as i32;
                let ncc = self.hook.new_char_count as i32;
                for _ in 0..bpc {
                    self.hook.macro_key.pop();
                }
                let start = self.buf.index - bpc;
                let mut i = start;
                while i < ncc + start {
                    if i >= 0 && (i as usize) < crate::data::MAX_BUFF {
                        self.hook.macro_key.push(self.buf.word[i as usize]);
                    }
                    i += 1;
                }
            }
        }

        if self.opts.upper_case_first_char {
            if self.buf.index == 1 && self.upper_case_status == 2 {
                self.upper_case_first_character();
            }
            self.upper_case_status = 0;
        }

        // case [ ]
        if is_bracket_key(data)
            && (is_bracket_key((self.hook.char_data[0] & 0xFFFF) as u16)
                || self.input_type() == InputType::SimpleTelex1
                || self.input_type() == InputType::SimpleTelex2)
        {
            let sub = if self.hook.code == HookCode::WillProcess {
                self.hook.backspace_count as i32
            } else {
                0
            };
            if self.buf.index - sub > 0 {
                self.buf.index -= 1;
                self.save_word();
            }
            self.buf.index = 0;
            self.temp_disable_key = false;
            self.buf.state_index = 0;
            self.hook.ext_code = 3;
            self.special_char.push(data as u32 | if is_caps { CAPS_MASK } else { 0 });
        }

        &self.hook
    }

    /// English mode never transforms keys (`hook.code` stays `DoNothing`); it
    /// only accumulates/clears `hook.macro_key` for the macro lookup
    /// (`find_macro`) on SPACE, and tracks `will_temp_off_engine` the same way
    /// the normal-key path does.
    pub fn english_mode(
        &mut self,
        st: KeyEventState,
        keycode: u16,
        caps: bool,
        other_control: bool,
    ) -> &HookState {
        let data = keycode;
        self.hook.code = HookCode::DoNothing;
        if st == KeyEventState::MouseDown || (other_control && !caps) {
            self.hook.macro_key.clear();
            self.will_temp_off_engine = false;
        } else if data == KEY_SPACE {
            if !self.has_handled_macro && self.find_macro() {
                self.hook.code = HookCode::ReplaceMacro;
                self.hook.backspace_count = self.hook.macro_key.len() as u8;
            }
            self.hook.macro_key.clear();
            self.will_temp_off_engine = false;
        } else if data == KEY_DELETE {
            if !self.hook.macro_key.is_empty() {
                self.hook.macro_key.pop();
            } else {
                self.will_temp_off_engine = false;
            }
        } else {
            // mirror the source quirk: is_word_break is always called with
            // KeyEvent::Keyboard hardcoded here, not whatever event the real
            // caller saw.
            if Self::is_word_break(KeyEvent::Keyboard, data) && !Self::is_char_key_code(data) {
                self.hook.macro_key.clear();
                self.will_temp_off_engine = false;
            } else if !self.will_temp_off_engine {
                self.hook.macro_key.push(data as u32 | if caps { CAPS_MASK } else { 0 });
            }
        }
        &self.hook
    }

    fn save_special_char(&mut self) {
        self.typing_states.push(self.special_char.clone());
        self.special_char.clear();
    }
}
