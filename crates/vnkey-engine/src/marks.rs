//! Tone/mark placement.
//!
//! Fidelity over elegance: several C++ conditions are self-contradictory
//! (e.g. `CHR(x) == KEY_I && CHR(x) == KEY_A`) yet deterministic. They are
//! reproduced verbatim and flagged `// mirror C++ quirk:`.
//!
//! Clippy lints are allowed where "simplifying" would break the 1:1 mapping to
//! the source: `int_plus_one` keeps the C++ `x - 1 >= 0` guards, and
//! `collapsible_if`/`if_same_then_else` keep the C++ nested gates and the
//! distinct-rule/identical-body ladders (e.g. the "ch"/"t" limits) intact.
#![allow(
    clippy::int_plus_one,
    clippy::collapsible_if,
    clippy::if_same_then_else
)]

use crate::data::{
    CAPS_MASK, END_CONSONANT_MASK, HookCode, MARK_MASK, MAX_BUFF, STANDALONE_MASK, TONE_MASK,
    TONEW_MASK,
};
use crate::engine::Engine;
use crate::input_method::{is_consonant, is_key_d, is_key_f, is_key_r, is_key_x};
use crate::keys::*;
use crate::vietnamese::VOWEL_COMBINE;

const EC: u16 = END_CONSONANT_MASK as u16;

impl Engine {
    /// Only ever uses a single row in the C++, so the row is passed directly.
    /// Updates `is_corect` (caller sets it `true` before calling).
    pub(crate) fn check_correct_vowel(&mut self, row: &[u16], mark_key: u16) {
        let it = self.input_type();

        // ignore "qu" case
        if self.buf.index >= 2
            && self.buf.chr(self.buf.index - 1) == KEY_U
            && self.buf.chr(self.buf.index - 2) == KEY_Q
        {
            self.is_corect = false;
            return;
        }

        let mut k = self.buf.index - 1;
        let mut j = row.len() as i32 - 1;
        while j >= 0 {
            let elem = row[j as usize] & !(if self.opts.quick_end_consonant { EC } else { 0 });
            if elem != self.buf.chr(k) {
                self.is_corect = false;
                return;
            }
            k -= 1;
            if k < 0 {
                break;
            }
            j -= 1;
        }

        // limit mark for end consonant "C", "T"
        if self.is_corect
            && row.len() > 1
            && (is_key_f(it, mark_key) || is_key_x(it, mark_key) || is_key_r(it, mark_key))
        {
            if row[1] == KEY_C || row[1] == KEY_T {
                self.is_corect = false;
            } else if row.len() > 2 && row[2] == KEY_T {
                self.is_corect = false;
            }
        }

        if self.is_corect && k >= 0 && self.buf.chr(k) == self.buf.chr(k + 1) {
            self.is_corect = false;
        }
    }

    pub(crate) fn remove_mark(&mut self) {
        self.find_and_calculate_vowel(true);
        self.is_changed = false;
        if self.buf.index > 0 {
            let mut i = self.vsi;
            while i <= self.vei {
                if self.buf.word[i as usize] & MARK_MASK != 0 {
                    self.buf.word[i as usize] &= !MARK_MASK;
                    self.is_changed = true;
                }
                i += 1;
            }
        }
        if self.is_changed {
            self.hook.code = HookCode::WillProcess;
            let mut bpc = 0i32;
            let mut i = self.buf.index - 1;
            while i >= self.vsi {
                bpc += 1;
                self.hook.char_data[(self.buf.index - 1 - i) as usize] =
                    self.get(self.buf.word[i as usize]);
                i -= 1;
            }
            self.hook.backspace_count = bpc as u8;
            self.hook.new_char_count = bpc as u8;
        } else {
            self.hook.code = HookCode::DoNothing;
        }
    }

    pub(crate) fn can_has_end_consonant(&self) -> bool {
        let vo: &[Vec<u32>] = VOWEL_COMBINE
            .get(&self.buf.chr(self.vsi))
            .map_or(&[], |v| v.as_slice());
        let mut ii = 0i32;
        while ii < vo.len() as i32 {
            let combo = &vo[ii as usize];
            let mut kk = self.vsi;
            let mut iii = 1i32;
            while iii < combo.len() as i32 {
                if kk > self.vei
                    || (self.buf.chr(kk) as u32
                        | (self.buf.word[kk as usize] & TONE_MASK)
                        | (self.buf.word[kk as usize] & TONEW_MASK))
                        != combo[iii as usize]
                {
                    break;
                }
                kk += 1;
                iii += 1;
            }
            if iii >= combo.len() as i32 {
                return combo[0] == 1;
            }
            ii += 1;
        }
        false
    }

    fn handle_old_mark(&mut self) {
        let idx = self.buf.index;
        if self.vowel_count == 0 && self.buf.chr(self.vei) == KEY_I {
            self.vwsm = self.vei;
        } else {
            self.vwsm = self.vsi;
        }
        self.hook.backspace_count = (idx - self.vwsm) as u8;

        // rule 2
        if self.vowel_count == 3
            || (self.vei + 1 < idx
                && is_consonant(self.buf.chr(self.vei + 1))
                && self.can_has_end_consonant())
        {
            self.vwsm = self.vsi + 1;
            self.hook.backspace_count = (idx - self.vwsm) as u8;
        }

        // rule 3
        let mut ii = self.vsi;
        while ii <= self.vei {
            if (self.buf.chr(ii) == KEY_E && self.buf.word[ii as usize] & TONE_MASK != 0)
                || (self.buf.chr(ii) == KEY_O && self.buf.word[ii as usize] & TONEW_MASK != 0)
            {
                self.vwsm = ii;
                self.hook.backspace_count = (idx - self.vwsm) as u8;
                break;
            }
            ii += 1;
        }

        self.hook.new_char_count = self.hook.backspace_count;
    }

    fn handle_modern_mark(&mut self) {
        let vsi = self.vsi;
        let vei = self.vei;
        let vc = self.vowel_count;
        let idx = self.buf.index;

        // default
        self.vwsm = vei;
        self.hook.backspace_count = (idx - vei) as u8;

        // rule 2
        if vc == 3
            && ((self.buf.chr(vsi) == KEY_O
                && self.buf.chr(vsi + 1) == KEY_A
                && self.buf.chr(vsi + 2) == KEY_I)
                || (self.buf.chr(vsi) == KEY_U
                    && self.buf.chr(vsi + 1) == KEY_Y
                    && self.buf.chr(vsi + 2) == KEY_U)
                || (self.buf.chr(vsi) == KEY_O
                    && self.buf.chr(vsi + 1) == KEY_E
                    && self.buf.chr(vsi + 2) == KEY_O)
                || (self.buf.chr(vsi) == KEY_U
                    && self.buf.chr(vsi + 1) == KEY_Y
                    && self.buf.chr(vsi + 2) == KEY_A))
        {
            self.vwsm = vsi + 1;
            self.hook.backspace_count = (idx - self.vwsm) as u8;
        } else if (self.buf.chr(vsi) == KEY_O && self.buf.chr(vsi + 1) == KEY_I)
            || (self.buf.chr(vsi) == KEY_A && self.buf.chr(vsi + 1) == KEY_I)
            || (self.buf.chr(vsi) == KEY_U && self.buf.chr(vsi + 1) == KEY_I)
        {
            self.vwsm = vsi;
            self.hook.backspace_count = (idx - self.vwsm) as u8;
        } else if self.buf.chr(vei - 1) == KEY_A && self.buf.chr(vei) == KEY_Y {
            self.vwsm = vei - 1;
            self.hook.backspace_count = ((idx - vei) + 1) as u8;
        } else if self.buf.chr(vsi) == KEY_U && self.buf.chr(vsi + 1) == KEY_O {
            self.vwsm = vsi + 1;
            self.hook.backspace_count = (idx - self.vwsm) as u8;
        } else if self.buf.chr(vsi + 1) == KEY_O || self.buf.chr(vsi + 1) == KEY_U {
            self.vwsm = vei - 1;
            self.hook.backspace_count = ((idx - vei) + 1) as u8;
        } else if self.buf.chr(vsi) == KEY_O || self.buf.chr(vsi) == KEY_U {
            self.vwsm = vei;
            self.hook.backspace_count = (idx - vei) as u8;
        }

        // rule 3.1
        let w = |e: &Engine, i: i32| e.buf.word[i as usize];
        if (self.buf.chr(vsi) == KEY_I && (w(self, vsi + 1) & (KEY_E as u32 | TONE_MASK)) != 0)
            || (self.buf.chr(vsi) == KEY_Y && (w(self, vsi + 1) & (KEY_E as u32 | TONE_MASK)) != 0)
            || (self.buf.chr(vsi) == KEY_U && w(self, vsi + 1) == (KEY_O as u32 | TONE_MASK))
            || (w(self, vsi) == (KEY_U as u32 | TONEW_MASK)
                && w(self, vsi + 1) == (KEY_O as u32 | TONEW_MASK))
        {
            if vsi + 2 < idx {
                let c2 = self.buf.chr(vsi + 2);
                // mirror C++ quirk: the CH/NH/NG sub-checks read CHR(vsi+2)
                // against two different letters at the SAME index, so they are
                // always false; transcribed verbatim.
                if c2 == KEY_P
                    || c2 == KEY_T
                    || c2 == KEY_M
                    || c2 == KEY_N
                    || c2 == KEY_O
                    || c2 == KEY_U
                    || c2 == KEY_I
                    || c2 == KEY_C
                    || (vsi + 3 < idx && c2 == KEY_C && c2 == KEY_H)
                    || (vsi + 3 < idx && c2 == KEY_N && c2 == KEY_H)
                    || (vsi + 3 < idx && c2 == KEY_N && c2 == KEY_G)
                {
                    self.vwsm = vsi + 1;
                    self.hook.backspace_count = (idx - self.vwsm) as u8;
                } else {
                    self.vwsm = vsi;
                    self.hook.backspace_count = (idx - self.vwsm) as u8;
                }
            } else {
                self.vwsm = vsi;
                self.hook.backspace_count = (idx - self.vwsm) as u8;
            }
        }
        // rule 3.2
        // mirror C++ quirk: each `CHR(vsi)==X && CHR(vsi)==KEY_A` term compares
        // one index to two letters, so the first three terms are always false.
        else if (self.buf.chr(vsi) == KEY_I && self.buf.chr(vsi) == KEY_A)
            || (self.buf.chr(vsi) == KEY_Y && self.buf.chr(vsi) == KEY_A)
            || (self.buf.chr(vsi) == KEY_U && self.buf.chr(vsi) == KEY_A)
            || (self.buf.chr(vsi) == KEY_U && w(self, vsi + 1) == (KEY_U as u32 | TONEW_MASK))
        {
            self.vwsm = vsi;
            self.hook.backspace_count = (idx - self.vwsm) as u8;
        }

        // rule 4
        if vc == 2 {
            if (self.buf.chr(vsi) == KEY_I && self.buf.chr(vsi + 1) == KEY_A)
                || (self.buf.chr(vsi) == KEY_I && self.buf.chr(vsi + 1) == KEY_U)
                || (self.buf.chr(vsi) == KEY_I && self.buf.chr(vsi + 1) == KEY_O)
            {
                if vsi == 0 || self.buf.chr(vsi - 1) != KEY_G {
                    self.vwsm = vsi;
                    self.hook.backspace_count = (idx - self.vwsm) as u8;
                } else {
                    self.vwsm = vsi + 1;
                    self.hook.backspace_count = (idx - self.vwsm) as u8;
                }
            } else if self.buf.chr(vsi) == KEY_U && self.buf.chr(vsi + 1) == KEY_A {
                if vsi == 0 || self.buf.chr(vsi - 1) != KEY_Q {
                    if vei + 1 >= idx || !self.can_has_end_consonant() {
                        self.vwsm = vsi;
                        self.hook.backspace_count = (idx - self.vwsm) as u8;
                    }
                } else {
                    self.vwsm = vsi + 1;
                    self.hook.backspace_count = (idx - self.vwsm) as u8;
                }
            } else if self.buf.chr(vsi) == KEY_O && self.buf.chr(vsi + 1) == KEY_O {
                self.vwsm = vei;
                self.hook.backspace_count = (idx - self.vwsm) as u8;
            }
        }
    }

    pub(crate) fn insert_mark(&mut self, mark_mask: u32, can_modify_flag: bool) {
        self.vowel_count = 0;
        if can_modify_flag {
            self.hook.code = HookCode::WillProcess;
        }
        self.hook.backspace_count = 0;
        self.hook.new_char_count = 0;

        self.find_and_calculate_vowel(false);
        self.vwsm = 0;

        // detect mark position
        if self.vowel_count == 1 {
            self.vwsm = self.vei;
            self.hook.backspace_count = (self.buf.index - self.vei) as u8;
        } else {
            if !self.opts.use_modern_orthography {
                self.handle_old_mark();
            } else {
                self.handle_modern_mark();
            }
            if self.buf.word[self.vei as usize] & TONE_MASK != 0
                || self.buf.word[self.vei as usize] & TONEW_MASK != 0
            {
                self.vwsm = self.vei;
            }
        }

        // send data
        let mut kk = self.buf.index - 1 - self.vsi;
        if self.buf.word[self.vwsm as usize] & mark_mask != 0 {
            // duplicate same mark -> restore
            self.buf.word[self.vwsm as usize] &= !MARK_MASK;
            if can_modify_flag {
                self.hook.code = HookCode::Restore;
            }
            let mut ii = self.vsi;
            while ii < self.buf.index {
                self.buf.word[ii as usize] &= !MARK_MASK;
                self.hook.char_data[kk as usize] = self.get(self.buf.word[ii as usize]);
                kk -= 1;
                ii += 1;
            }
            self.temp_disable_key = true;
        } else {
            // remove other mark, add mark
            self.buf.word[self.vwsm as usize] &= !MARK_MASK;
            self.buf.word[self.vwsm as usize] |= mark_mask;
            let mut ii = self.vsi;
            while ii < self.buf.index {
                if ii != self.vwsm {
                    self.buf.word[ii as usize] &= !MARK_MASK;
                }
                self.hook.char_data[kk as usize] = self.get(self.buf.word[ii as usize]);
                kk -= 1;
                ii += 1;
            }
            self.hook.backspace_count = (self.buf.index - self.vsi) as u8;
        }
        self.hook.new_char_count = self.hook.backspace_count;
    }

    pub(crate) fn insert_d(&mut self, _data: u16, _is_caps: bool) {
        self.hook.code = HookCode::WillProcess;
        let mut bpc = 0i32;
        let mut ii = self.buf.index - 1;
        while ii >= 0 {
            bpc += 1;
            if self.buf.chr(ii) == KEY_D {
                if self.buf.word[ii as usize] & TONE_MASK != 0 {
                    // restore and disable temporary
                    self.hook.code = HookCode::Restore;
                    self.buf.word[ii as usize] &= !TONE_MASK;
                    self.hook.char_data[(self.buf.index - 1 - ii) as usize] =
                        self.buf.word[ii as usize];
                    self.temp_disable_key = true;
                    break;
                } else {
                    self.buf.word[ii as usize] |= TONE_MASK;
                    self.hook.char_data[(self.buf.index - 1 - ii) as usize] =
                        self.get(self.buf.word[ii as usize]);
                }
                break;
            } else {
                self.hook.char_data[(self.buf.index - 1 - ii) as usize] =
                    self.get(self.buf.word[ii as usize]);
            }
            ii -= 1;
        }
        self.hook.backspace_count = bpc as u8;
        self.hook.new_char_count = bpc as u8;
    }

    pub(crate) fn insert_aoe(&mut self, data: u16, _is_caps: bool) {
        let it = self.input_type();
        self.find_and_calculate_vowel(false);

        // remove W tone
        let mut ii = self.vsi;
        while ii <= self.vei {
            self.buf.word[ii as usize] &= !TONEW_MASK;
            ii += 1;
        }

        self.hook.code = HookCode::WillProcess;
        let mut bpc = 0i32;
        ii = self.buf.index - 1;
        while ii >= 0 {
            bpc += 1;
            if self.buf.chr(ii) == data {
                if self.buf.word[ii as usize] & TONE_MASK != 0 {
                    self.hook.code = HookCode::Restore;
                    self.buf.word[ii as usize] &= !TONE_MASK;
                    self.hook.char_data[(self.buf.index - 1 - ii) as usize] =
                        self.buf.word[ii as usize];
                    if data != KEY_O {
                        // case "thoòng"
                        self.temp_disable_key = true;
                    }
                    break;
                } else {
                    self.buf.word[ii as usize] |= TONE_MASK;
                    if !is_key_d(it, data) {
                        self.buf.word[ii as usize] &= !TONEW_MASK;
                    }
                    self.hook.char_data[(self.buf.index - 1 - ii) as usize] =
                        self.get(self.buf.word[ii as usize]);
                }
                break;
            } else {
                self.hook.char_data[(self.buf.index - 1 - ii) as usize] =
                    self.get(self.buf.word[ii as usize]);
            }
            ii -= 1;
        }
        self.hook.backspace_count = bpc as u8;
        self.hook.new_char_count = bpc as u8;
    }

    pub(crate) fn insert_w(&mut self, _data: u16, _is_caps: bool) {
        self.is_restored_w = false;
        self.find_and_calculate_vowel(false);

        // remove ^ tone
        let mut ii = self.vsi;
        while ii <= self.vei {
            self.buf.word[ii as usize] &= !TONE_MASK;
            ii += 1;
        }

        if self.vowel_count > 1 {
            let vsi = self.vsi;
            self.hook.backspace_count = (self.buf.index - vsi) as u8;
            self.hook.new_char_count = self.hook.backspace_count;

            let w0 = self.buf.word[vsi as usize];
            let w1 = self.buf.word[(vsi + 1) as usize];
            let restore = ((w0 & TONEW_MASK != 0) && (w1 & TONEW_MASK != 0))
                || ((w0 & TONEW_MASK != 0) && self.buf.chr(vsi + 1) == KEY_I)
                || ((w0 & TONEW_MASK != 0) && self.buf.chr(vsi + 1) == KEY_A);

            if restore {
                self.hook.code = HookCode::Restore;
                let mut ii = self.vsi;
                while ii < self.buf.index {
                    self.buf.word[ii as usize] &= !TONEW_MASK;
                    self.hook.char_data[(self.buf.index - 1 - ii) as usize] =
                        self.get(self.buf.word[ii as usize]) & !STANDALONE_MASK;
                    ii += 1;
                }
                self.is_restored_w = true;
                self.temp_disable_key = true;
            } else {
                self.hook.code = HookCode::WillProcess;

                if self.buf.chr(vsi) == KEY_U && self.buf.chr(vsi + 1) == KEY_O {
                    if vsi - 2 >= 0
                        && self.buf.word[(vsi - 2) as usize] == KEY_T as u32
                        && self.buf.word[(vsi - 1) as usize] == KEY_H as u32
                    {
                        self.buf.word[(vsi + 1) as usize] |= TONEW_MASK;
                        if vsi + 2 < self.buf.index && self.buf.chr(vsi + 2) == KEY_N {
                            self.buf.word[vsi as usize] |= TONEW_MASK;
                        }
                    } else if vsi - 1 >= 0 && self.buf.word[(vsi - 1) as usize] == KEY_Q as u32 {
                        self.buf.word[(vsi + 1) as usize] |= TONEW_MASK;
                    } else {
                        self.buf.word[vsi as usize] |= TONEW_MASK;
                        self.buf.word[(vsi + 1) as usize] |= TONEW_MASK;
                    }
                } else if (self.buf.chr(vsi) == KEY_U && self.buf.chr(vsi + 1) == KEY_A)
                    || (self.buf.chr(vsi) == KEY_U && self.buf.chr(vsi + 1) == KEY_I)
                    || (self.buf.chr(vsi) == KEY_U && self.buf.chr(vsi + 1) == KEY_U)
                    || (self.buf.chr(vsi) == KEY_O && self.buf.chr(vsi + 1) == KEY_I)
                {
                    self.buf.word[vsi as usize] |= TONEW_MASK;
                } else if (self.buf.chr(vsi) == KEY_I && self.buf.chr(vsi + 1) == KEY_O)
                    || (self.buf.chr(vsi) == KEY_O && self.buf.chr(vsi + 1) == KEY_A)
                {
                    self.buf.word[(vsi + 1) as usize] |= TONEW_MASK;
                } else {
                    // don't do anything
                    self.temp_disable_key = true;
                    self.is_changed = false;
                    self.hook.code = HookCode::DoNothing;
                }

                let mut ii = self.vsi;
                while ii < self.buf.index {
                    self.hook.char_data[(self.buf.index - 1 - ii) as usize] =
                        self.get(self.buf.word[ii as usize]);
                    ii += 1;
                }
            }
            return;
        }

        // vowelCount <= 1
        self.hook.code = HookCode::WillProcess;
        let mut bpc = 0i32;
        let mut ii = self.buf.index - 1;
        while ii >= 0 {
            if ii < self.vsi {
                break;
            }
            bpc += 1;
            let c = self.buf.chr(ii);
            if c == KEY_A || c == KEY_U || c == KEY_O {
                if self.buf.word[ii as usize] & TONEW_MASK != 0 {
                    if self.buf.word[ii as usize] & STANDALONE_MASK != 0 {
                        self.hook.code = HookCode::WillProcess;
                        if self.buf.chr(ii) == KEY_U {
                            self.buf.word[ii as usize] =
                                KEY_W as u32 | (self.buf.word[ii as usize] & CAPS_MASK);
                        } else if self.buf.chr(ii) == KEY_O {
                            self.hook.code = HookCode::Restore;
                            self.buf.word[ii as usize] =
                                KEY_O as u32 | (self.buf.word[ii as usize] & CAPS_MASK);
                            self.is_restored_w = true;
                        }
                        self.hook.char_data[(self.buf.index - 1 - ii) as usize] =
                            self.buf.word[ii as usize];
                    } else {
                        self.hook.code = HookCode::Restore;
                        self.buf.word[ii as usize] &= !TONEW_MASK;
                        self.hook.char_data[(self.buf.index - 1 - ii) as usize] =
                            self.buf.word[ii as usize];
                        self.is_restored_w = true;
                    }
                    self.temp_disable_key = true;
                } else {
                    self.buf.word[ii as usize] |= TONEW_MASK;
                    self.buf.word[ii as usize] &= !TONE_MASK;
                    self.hook.char_data[(self.buf.index - 1 - ii) as usize] =
                        self.get(self.buf.word[ii as usize]);
                }
            } else {
                self.hook.char_data[(self.buf.index - 1 - ii) as usize] =
                    self.get(self.buf.word[ii as usize]);
            }
            ii -= 1;
        }
        self.hook.backspace_count = bpc as u8;
        self.hook.new_char_count = bpc as u8;
    }

    pub(crate) fn check_grammar(&mut self, delta_back_space: i32) {
        if self.buf.index <= 1 || self.buf.index >= MAX_BUFF as i32 {
            return;
        }

        self.find_and_calculate_vowel(true);
        if self.vowel_count == 0 {
            return;
        }

        let mut is_checked_grammar = false;
        let l = self.vsi;

        // if N key for case: "thuơn", "ưoi", "ưom", "ưoc"
        if self.buf.index >= 3 {
            let mut i = self.buf.index - 1;
            while i >= 0 {
                let c = self.buf.chr(i);
                if c == KEY_N || c == KEY_C || c == KEY_I || c == KEY_M || c == KEY_P || c == KEY_T {
                    if i - 2 >= 0
                        && self.buf.chr(i - 1) == KEY_O
                        && self.buf.chr(i - 2) == KEY_U
                        && ((self.buf.word[(i - 1) as usize] & TONEW_MASK)
                            ^ (self.buf.word[(i - 2) as usize] & TONEW_MASK))
                            != 0
                    {
                        self.buf.word[(i - 2) as usize] |= TONEW_MASK;
                        self.buf.word[(i - 1) as usize] |= TONEW_MASK;
                        is_checked_grammar = true;
                        break;
                    }
                }
                i -= 1;
            }
        }

        // check mark
        if self.buf.index >= 2 {
            let mut i = l;
            while i <= self.vei {
                if self.buf.word[i as usize] & MARK_MASK != 0 {
                    let mark = self.buf.word[i as usize] & MARK_MASK;
                    self.buf.word[i as usize] &= !MARK_MASK;
                    self.insert_mark(mark, false);
                    if i != self.vwsm {
                        is_checked_grammar = true;
                    }
                    break;
                }
                i += 1;
            }
        }

        // re-arrange data to sendback
        if is_checked_grammar {
            if self.hook.code == HookCode::DoNothing {
                self.hook.code = HookCode::WillProcess;
            }
            let mut bpc = 0i32;
            let mut i = self.buf.index - 1;
            while i >= l {
                bpc += 1;
                self.hook.char_data[(self.buf.index - 1 - i) as usize] =
                    self.get(self.buf.word[i as usize]);
                i -= 1;
            }
            self.hook.new_char_count = bpc as u8;
            self.hook.backspace_count = (bpc + delta_back_space) as u8;
            self.hook.ext_code = 4;
        }
    }
}
