//! Typing-word and key-state buffers.
//!
//! Mirrors the C++ globals `TypingWord[MAX_BUFF]`/`_index`,
//! `KeyStates[MAX_BUFF]`/`_stateIndex` and `_longWordHelper`
//! (`Engine.cpp:83-93`), plus the low-level buffer operations `setKeyData`
//! (`Engine.cpp:165-169`), the write half of `insertKey`
//! (`Engine.cpp:349-359`) and `insertState` (`Engine.cpp:369-379`).
//!
//! `index`/`state_index` are `i32`, not `usize`: the C++ code is riddled with
//! `_index - 1`, `_index - 2`, `_index - 3` reads whose signedness matters
//! (`_index` is a `Byte` promoted to `int` in every expression, so `0 - 1` is
//! `-1`, and guards like `_index - 2 >= 0` rely on that). Reproducing those
//! semantics with `usize` would underflow-panic, so signed arithmetic is the
//! faithful choice.

use crate::data::{CAPS_MASK, CHAR_MASK, MAX_BUFF};

pub(crate) struct Buffer {
    /// `Engine.cpp:83 TypingWord[MAX_BUFF]`.
    pub word: [u32; MAX_BUFF],
    /// `Engine.cpp:84 _index`.
    pub index: i32,
    /// `Engine.cpp:92 KeyStates[MAX_BUFF]`.
    pub states: [u32; MAX_BUFF],
    /// `Engine.cpp:93 _stateIndex`.
    pub state_index: i32,
    /// `Engine.cpp:85 _longWordHelper` -- overflow spill for words longer
    /// than `MAX_BUFF`.
    pub long_word_helper: Vec<u32>,
}

impl Buffer {
    pub(crate) fn new() -> Self {
        Buffer {
            word: [0; MAX_BUFF],
            index: 0,
            states: [0; MAX_BUFF],
            state_index: 0,
            long_word_helper: Vec::new(),
        }
    }

    /// `Engine.cpp:132 CHR(index)` -- `(Uint16)TypingWord[index]`, i.e. the
    /// low 16 bits (keycode; the CAPS/TONE/MARK flags all sit above bit 15).
    /// Caller must guarantee `index >= 0` (every C++ `CHR` read is reached
    /// only under a guard that ensures that; those guards are preserved at
    /// each call site).
    #[inline]
    pub(crate) fn chr(&self, index: i32) -> u16 {
        (self.word[index as usize] & CHAR_MASK) as u16
    }

    /// `Engine.cpp:165-169 setKeyData`.
    pub(crate) fn set_key_data(&mut self, index: i32, key_code: u16, is_caps: bool) {
        if index < 0 || index >= MAX_BUFF as i32 {
            return;
        }
        self.word[index as usize] = key_code as u32 | if is_caps { CAPS_MASK } else { 0 };
    }

    /// Write half of `insertKey` (`Engine.cpp:350-359`): append a key to the
    /// typing word, left-shifting into `_longWordHelper` on overflow. The
    /// spelling/`tempDisableKey` half of `insertKey` stays on `Engine` since
    /// it needs engine options + `checkSpelling`.
    pub(crate) fn push_word(&mut self, key_code: u16, is_caps: bool) {
        if self.index >= MAX_BUFF as i32 {
            self.long_word_helper.push(self.word[0]); // save long word
            for iii in 0..MAX_BUFF - 1 {
                self.word[iii] = self.word[iii + 1];
            }
            self.set_key_data(self.index - 1, key_code, is_caps);
        } else {
            self.set_key_data(self.index, key_code, is_caps);
            self.index += 1;
        }
    }

    /// `Engine.cpp:369-379 insertState`.
    pub(crate) fn push_state(&mut self, key_code: u16, is_caps: bool) {
        let caps = if is_caps { CAPS_MASK } else { 0 };
        if self.state_index >= MAX_BUFF as i32 {
            for iii in 0..MAX_BUFF - 1 {
                self.states[iii] = self.states[iii + 1];
            }
            // mirror C++ quirk: _stateIndex is NOT incremented on overflow, so
            // the write lands at states[_stateIndex - 1] and stays pinned there.
            self.states[(self.state_index - 1) as usize] = key_code as u32 | caps;
        } else {
            self.states[self.state_index as usize] = key_code as u32 | caps;
            self.state_index += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_word_increments_index_and_sets_caps() {
        let mut b = Buffer::new();
        b.push_word(5, false);
        b.push_word(6, true);
        assert_eq!(b.index, 2);
        assert_eq!(b.word[0], 5);
        assert_eq!(b.word[1], 6 | CAPS_MASK);
        assert_eq!(b.chr(1), 6);
    }

    #[test]
    fn push_word_overflow_left_shifts_into_long_word_helper() {
        let mut b = Buffer::new();
        for n in 0..MAX_BUFF as u16 {
            b.push_word(n, false);
        }
        assert_eq!(b.index, MAX_BUFF as i32);
        // One more overflows: word[0] (=0) is spilled, buffer shifts left.
        b.push_word(99, false);
        assert_eq!(b.long_word_helper, vec![0]);
        assert_eq!(b.word[0], 1); // was word[1]
        assert_eq!(b.word[MAX_BUFF - 1], 99);
        assert_eq!(b.index, MAX_BUFF as i32); // stays pinned at MAX_BUFF
    }

    #[test]
    fn push_state_overflow_pins_at_last_slot() {
        let mut b = Buffer::new();
        for n in 0..MAX_BUFF as u16 {
            b.push_state(n, false);
        }
        assert_eq!(b.state_index, MAX_BUFF as i32);
        b.push_state(42, false);
        assert_eq!(b.states[MAX_BUFF - 1], 42);
        assert_eq!(b.state_index, MAX_BUFF as i32);
    }
}
