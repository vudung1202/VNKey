//! Spelling check core (`Engine.cpp:176-288 checkSpelling`).
//!
//! This ports only what the insert path needs: setting `temp_disable_key`
//! (via `_spellingOK`/`_spellingVowelOK`). The restore-if-wrong-spelling
//! machinery (`checkRestoreIfWrongSpelling`) is Task 5.
//!
//! The C++ function leans hard on file-scope scratch variables (`i, ii, j, k,
//! l`) whose values *escape* their `for` loops and are read afterwards (e.g.
//! `j` after the first-consonant loop feeds `VSI`). To reproduce that
//! faithfully, each C++ `for` loop is transcribed as an explicit `while` loop
//! over a shared local so the post-loop value matches exactly.
//!
//! `if_same_then_else` is allowed: the "ch"/"t" end-consonant limits are two
//! distinct C++ rules that happen to share the body `_spellingOK = false`;
//! merging them would obscure the 1:1 mapping to `Engine.cpp:273-279`.
#![allow(clippy::if_same_then_else)]

use crate::data::{
    CONSONANT_ALLOW_MASK, END_CONSONANT_MASK, MARK1_MASK, MARK5_MASK, MARK_MASK, TONE_MASK,
    TONEW_MASK,
};
use crate::engine::Engine;
use crate::input_method::is_consonant;
use crate::keys::*;
use crate::vietnamese::{CONSONANT_TABLE, END_CONSONANT_TABLE, VOWEL_COMBINE};

/// `END_CONSONANT_MASK`/`CONSONANT_ALLOW_MASK` truncated to `u16`, matching
/// the `Uint16` rows of `_consonantTable`/`_endConsonantTable`.
const EC: u16 = END_CONSONANT_MASK as u16;
const CA: u16 = CONSONANT_ALLOW_MASK as u16;

impl Engine {
    /// `Engine.cpp:176-288 checkSpelling`.
    pub(crate) fn check_spelling(&mut self, force_check_vowel: bool) {
        self.spelling_ok = false;
        self.spelling_vowel_ok = true;
        self.spelling_end_index = self.buf.index;

        if self.buf.index > 0 && self.buf.chr(self.buf.index - 1) == KEY_RIGHT_BRACKET {
            self.spelling_end_index = self.buf.index - 1;
        }

        if self.spelling_end_index > 0 {
            let mut j: i32 = 0;
            // check first consonant
            if is_consonant(self.buf.chr(0)) {
                let mut i: i32 = 0;
                while i < CONSONANT_TABLE.len() as i32 {
                    let row = CONSONANT_TABLE[i as usize];
                    self.spelling_flag = false;
                    if self.spelling_end_index < row.len() as i32 {
                        self.spelling_flag = true;
                    }
                    j = 0;
                    while j < row.len() as i32 {
                        let elem = row[j as usize];
                        let a = elem & !(if self.opts.quick_start_consonant { EC } else { 0 });
                        let b = elem & !(if self.opts.allow_consonant_zfwj { CA } else { 0 });
                        if self.spelling_end_index > j
                            && a != self.buf.chr(j)
                            && b != self.buf.chr(j)
                        {
                            self.spelling_flag = true;
                            break;
                        }
                        j += 1;
                    }
                    if self.spelling_flag {
                        i += 1;
                        continue;
                    }
                    break;
                }
            }

            if j == self.spelling_end_index {
                // for "d" case
                self.spelling_ok = true;
            }

            // check next vowel
            let mut k: i32 = j;
            self.vsi = k;
            // Aug 23 2019: fix case "que't"
            if self.buf.chr(self.vsi) == KEY_U
                && k > 0
                && k < self.spelling_end_index - 1
                && self.buf.chr(self.vsi - 1) == KEY_Q
            {
                k += 1;
                j = k;
                self.vsi = k;
            } else if self.buf.index >= 2
                && self.buf.chr(0) == KEY_G
                && self.buf.chr(1) == KEY_I
                && is_consonant(self.buf.chr(2))
            {
                // Sep 28: fix "gìn"
                self.vsi = 1;
                k = 1;
                j = 1;
            }

            let mut l: i32 = 0;
            while l < 3 {
                if k < self.spelling_end_index && !is_consonant(self.buf.chr(k)) {
                    k += 1;
                    self.vei = k;
                }
                l += 1;
            }

            if k > j {
                // has vowel
                self.spelling_vowel_ok = false;

                // check correct combined vowel
                if k - j > 1 && force_check_vowel {
                    let vowel_set: &[Vec<u32>] =
                        VOWEL_COMBINE.get(&self.buf.chr(j)).map_or(&[], |v| v.as_slice());
                    l = 0;
                    while l < vowel_set.len() as i32 {
                        let combo = &vowel_set[l as usize];
                        self.spelling_flag = false;
                        let mut ii: i32 = 1;
                        while ii < combo.len() as i32 {
                            let idx = j + ii - 1;
                            if idx < self.spelling_end_index
                                && combo[ii as usize]
                                    != (self.buf.chr(idx) as u32
                                        | (self.buf.word[idx as usize] & TONEW_MASK)
                                        | (self.buf.word[idx as usize] & TONE_MASK))
                            {
                                self.spelling_flag = true;
                                break;
                            }
                            ii += 1;
                        }
                        let idx2 = j + ii - 1;
                        if self.spelling_flag
                            || (k < self.spelling_end_index && combo[0] == 0)
                            || (idx2 < self.spelling_end_index && !is_consonant(self.buf.chr(idx2)))
                        {
                            l += 1;
                            continue;
                        }
                        self.spelling_vowel_ok = true;
                        break;
                    }
                } else if !is_consonant(self.buf.chr(j)) {
                    self.spelling_vowel_ok = true;
                }

                // continue check last consonant
                let mut ii: i32 = 0;
                while ii < END_CONSONANT_TABLE.len() as i32 {
                    let row = END_CONSONANT_TABLE[ii as usize];
                    self.spelling_flag = false;
                    j = 0;
                    while j < row.len() as i32 {
                        let elem =
                            row[j as usize] & !(if self.opts.quick_end_consonant { EC } else { 0 });
                        if self.spelling_end_index > k + j && elem != self.buf.chr(k + j) {
                            self.spelling_flag = true;
                            break;
                        }
                        j += 1;
                    }
                    if self.spelling_flag {
                        ii += 1;
                        continue;
                    }
                    if k + j >= self.spelling_end_index {
                        self.spelling_ok = true;
                        break;
                    }
                    ii += 1;
                }

                // limit: end consonant "ch", "t" can not use with "~", "`", "?"
                if self.spelling_ok {
                    let idx = self.buf.index;
                    if idx >= 3
                        && self.buf.chr(idx - 1) == KEY_H
                        && self.buf.chr(idx - 2) == KEY_C
                        && !((self.buf.word[(idx - 3) as usize] & MARK1_MASK != 0)
                            || (self.buf.word[(idx - 3) as usize] & MARK5_MASK != 0)
                            || (self.buf.word[(idx - 3) as usize] & MARK_MASK == 0))
                    {
                        self.spelling_ok = false;
                    } else if idx >= 2
                        && self.buf.chr(idx - 1) == KEY_T
                        && !((self.buf.word[(idx - 2) as usize] & MARK1_MASK != 0)
                            || (self.buf.word[(idx - 2) as usize] & MARK5_MASK != 0)
                            || (self.buf.word[(idx - 2) as usize] & MARK_MASK == 0))
                    {
                        self.spelling_ok = false;
                    }
                }
            }
        } else {
            self.spelling_ok = true;
        }

        self.temp_disable_key = !(self.spelling_ok && self.spelling_vowel_ok);
    }
}
