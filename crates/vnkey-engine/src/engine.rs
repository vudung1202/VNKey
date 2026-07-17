//! The typing engine: the normal-key branch of `vKeyHandleEvent`
//! (`Engine.cpp:1314-1552`) plus `handleMainKey` (`Engine.cpp:1055-1193`),
//! `insertKey`/`insertState`, `findAndCalculateVowel`, `handleQuickTelex`,
//! the standalone-`[`/`]`/`w` helpers and `upperCaseFirstCharacter`.
//!
//! Every C++ file-scope global becomes a private `Engine` field; the scratch
//! loop variables `i, ii, iii, j, k, kk, l` become locals. All C++ index
//! arithmetic is reproduced with `i32` (the C++ `Byte`s are promoted to
//! `int` in every expression, and negative intermediates / `>= 0` guards are
//! load-bearing).
//!
//! Scope: this task ports the normal-key path only. The space / delete /
//! word-break / macro / restore-if-wrong-spelling branches are stubbed with
//! `// TODO Task 5/6` markers and are never reached by letter/bracket input.
//!
//! Clippy lints are allowed where "simplifying" would break the 1:1 mapping to
//! `Engine.cpp`: `int_plus_one` keeps the C++ `x - 1 >= 0` index guards
//! verbatim, and `collapsible_if` keeps the C++ nested-`if` gates intact.
#![allow(clippy::int_plus_one, clippy::collapsible_if)]

use crate::buffer::Buffer;
use crate::charset::get_character_code;
use crate::data::{
    CAPS_MASK, HookCode, HookState, InputType, KeyEvent, KeyEventState, MARK1_MASK, MARK2_MASK,
    MARK3_MASK, MARK4_MASK, MARK5_MASK, STANDALONE_MASK, TONEW_MASK,
};
use crate::input_method::{
    is_bracket_key, is_consonant, is_key_d, is_key_double, is_key_f, is_key_j, is_key_r, is_key_s,
    is_key_w, is_key_x, is_key_z, is_mark_key, is_number_key, is_quick_telex_key, is_special_key,
};
use crate::keys::*;
use crate::options::EngineOptions;
use crate::vietnamese::{BREAK_CODE, CONSONANT_D, QUICK_TELEX, STANDALONE_WBAD, VOWEL, VOWEL_FOR_MARK};

pub struct Engine {
    pub(crate) opts: EngineOptions,
    /// `vCheckSpelling` -- an option, but mutated at runtime by the Ctrl
    /// spell-toggle; kept separate from `opts.check_spelling`.
    pub(crate) spell_check_on: bool,
    /// `_useSpellCheckingBefore`.
    pub(crate) use_spell_checking_before: bool,

    pub(crate) buf: Buffer,

    // engine flags / vowel-position scratch (mirror the like-named C++ statics)
    pub(crate) temp_disable_key: bool,
    pub(crate) is_corect: bool,
    pub(crate) is_changed: bool,
    pub(crate) vowel_count: i32,
    /// `vowelStartIndex` (VSI).
    pub(crate) vsi: i32,
    /// `vowelEndIndex` (VEI).
    pub(crate) vei: i32,
    /// `vowelWillSetMark` (VWSM) -- the same variable the C++ `VWSM` macro
    /// aliases; `insertMark` may override it to `VEI`.
    pub(crate) vwsm: i32,
    pub(crate) is_restored_w: bool,
    pub(crate) key_for_aeo: u16,

    // break/space/upper-case tracking (mostly Task 6; read by the normal path)
    pub(crate) space_count: i32,
    pub(crate) upper_case_status: u8,
    pub(crate) special_char: Vec<u32>,
    pub(crate) will_temp_off_engine: bool,
    pub(crate) has_handled_macro: bool,
    pub(crate) has_handle_quick_consonant: bool,

    // checkSpelling scratch (Engine.cpp:171-174)
    pub(crate) spelling_ok: bool,
    pub(crate) spelling_flag: bool,
    pub(crate) spelling_vowel_ok: bool,
    pub(crate) spelling_end_index: i32,

    pub(crate) hook: HookState,
}

impl Engine {
    /// Mirrors the oracle's `ok_reset`: apply options + a fresh engine.
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
            will_temp_off_engine: false,
            has_handled_macro: false,
            has_handle_quick_consonant: false,
            spelling_ok: false,
            spelling_flag: false,
            spelling_vowel_ok: false,
            spelling_end_index: 0,
            hook: HookState::default(),
        }
    }

    /// Re-apply options (mirrors the oracle shim's `applyOptions`): update
    /// `vCheckSpelling` from the new options, leave the typing buffer intact.
    pub fn set_options(&mut self, opts: EngineOptions) {
        self.opts = opts;
        self.spell_check_on = opts.check_spelling;
    }

    /// `Engine.cpp:457-466 startNewSession`.
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

    /// `Engine.cpp:65 GET` -- `getCharacterCode` against the active table.
    #[inline]
    pub(crate) fn get(&self, data: u32) -> u32 {
        get_character_code(self.opts.code_table, data)
    }

    #[inline]
    pub(crate) fn input_type(&self) -> InputType {
        self.opts.input_type
    }

    /// `Engine.cpp:145-154 isWordBreak`.
    fn is_word_break(event: KeyEvent, data: u16) -> bool {
        if event == KeyEvent::Mouse {
            return true;
        }
        BREAK_CODE.iter().any(|&b| b as u16 == data)
    }

    /// `Engine.cpp:349-367 insertKey`.
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

    /// `Engine.cpp:560-585 findAndCalculateVowel`.
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

    /// `Engine.cpp:986-993 reverseLastStandaloneChar`.
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

    /// `Engine.cpp:995-1040 checkForStandaloneChar`.
    fn check_for_standalone_char(&mut self, data: u16, is_caps: bool, key_will_reverse: u16) {
        // mirror C++ quirk: C++ reads CHR(_index-1)/TypingWord[_index-1] here
        // unguarded, which is an out-of-bounds read when _index == 0 (UB). After
        // a fresh reset that read is non-TONEW, so the branch is never taken;
        // we guard `index > 0` to get the same effect without the OOB read.
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

    /// `Engine.cpp:1042-1053 upperCaseFirstCharacter`.
    fn upper_case_first_character(&mut self) {
        if self.buf.word[0] & CAPS_MASK == 0 {
            self.hook.code = HookCode::WillProcess;
            self.hook.backspace_count = 0;
            self.hook.new_char_count = 1;
            self.buf.word[0] |= CAPS_MASK;
            self.hook.char_data[0] = self.get(self.buf.word[0]);
            self.upper_case_status = 0;
            // C++ indexes hMacroKey[0] unconditionally under vUseMacro; guard
            // non-empty (only reachable in states our tests never produce).
            if self.opts.use_macro {
                if let Some(first) = self.hook.macro_key.first_mut() {
                    *first |= CAPS_MASK;
                }
            }
        }
    }

    /// `Engine.cpp:1195-1202 handleQuickTelex`.
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

    /// `Engine.cpp:1055-1193 handleMainKey`.
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

        // _vowel[keyForAEO]: a missing key is an empty row set (C++ map[]).
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

    /// `Engine.cpp:1314-1552 vKeyHandleEvent`, normal-key path.
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
        // `state` is only read by the Task 6 word-break / delete branches
        // (`_isCharKeyCode = state == KeyDown && ...`); unused on the normal path.
        let _ = state;

        if (is_number_key(data) && caps_status == 1)
            || other_control
            || Self::is_word_break(event, data)
            || (self.buf.index == 0 && is_number_key(data))
        {
            // TODO Task 6: word-break / macro / restore-if-wrong-spelling path.
            // Never reached by letter/bracket input (the only input this task's
            // tests exercise); emit the C++ default header for the branch.
            self.hook.code = HookCode::DoNothing;
            self.hook.backspace_count = 0;
            self.hook.new_char_count = 0;
            self.hook.ext_code = 1; // word break
            self.start_new_session();
            self.spell_check_on = self.use_spell_checking_before;
            self.will_temp_off_engine = false;
            return &self.hook;
        } else if data == KEY_SPACE {
            // TODO Task 6: space handling (macro / spell / save word).
            self.hook.code = HookCode::DoNothing;
            self.space_count += 1;
            self.spell_check_on = self.use_spell_checking_before;
            self.will_temp_off_engine = false;
            return &self.hook;
        } else if data == KEY_DELETE {
            // TODO Task 6: delete handling.
            self.hook.code = HookCode::DoNothing;
            self.hook.ext_code = 2;
            return &self.hook;
        }

        // -- normal key (Engine.cpp:1466-1552) --
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
            // TODO Task 6: saveWord(KEY_SPACE, _spaceCount) history.
            self.space_count = 0;
        } else if !self.special_char.is_empty() {
            self.save_special_char();
        }

        self.buf.push_state(data, is_caps); // insertState

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
                // TODO Task 6: saveWord() history.
            }
            self.buf.index = 0;
            self.temp_disable_key = false;
            self.buf.state_index = 0;
            self.hook.ext_code = 3;
            self.special_char.push(data as u32 | if is_caps { CAPS_MASK } else { 0 });
        }

        &self.hook
    }

    /// `Engine.cpp:426-433 saveSpecialChar`. History bookkeeping (the pushed
    /// `_typingStates` list is Task 6 / restore-only); for the normal-key
    /// path only the `_specialChar.clear()` side effect matters.
    fn save_special_char(&mut self) {
        // TODO Task 6: push self.special_char to the typing-state history.
        self.special_char.clear();
    }
}
