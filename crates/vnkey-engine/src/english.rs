//! English-word recovery: a raw-keystroke journal plus the one heuristic that
//! earns its keep.
//!
//! # Why a second journal instead of `Buffer::states`
//!
//! `Buffer::states` is the engine's own raw-key array, but it is lossy for our
//! purposes in two ways:
//!
//! * the duplicate-key undo path decrements `state_index` after re-inserting a
//!   key (`engine.rs`, the `HookCode::Restore` arm of the normal-key branch),
//!   so typing `off` leaves only `of` recorded - the second `f` is gone and no
//!   amount of widening a restore guard can bring it back;
//! * `push_state` stops advancing past `MAX_BUFF`, pinning writes to the last
//!   slot, so words longer than the buffer lose their tail.
//!
//! [`RawJournal`] records every printable keystroke exactly as typed and is
//! never rewound by a transform, so ESC can always reproduce what the user
//! actually pressed. The automatic restore path deliberately keeps using
//! `states` (see `check_restore_if_wrong_spelling`) because its character
//! counts are load-bearing for behaviour that is already pinned by tests.
//!
//! # Why the heuristic is a short list, not a shape rule
//!
//! The obvious rule - "a single vowel carrying a mark and no final consonant
//! is probably English" - cannot work. In Telex the English word IS the
//! Vietnamese spelling: you type `is` to get `í`, `of` to get `ò`, `as` to get
//! `á`. Output shape carries no information that separates them, so a
//! keep-list over shapes would either miss every English word or eat real
//! Vietnamese.
//!
//! What does carry information is frequency, and that is a bet rather than a
//! rule. So the bet is made explicit and kept as small as possible: an exact
//! list of two-letter raw-key sequences. Anything longer, or not on the list,
//! is untouched - `à` (`af`), `ừ` (`uwf`), `ồ` (`oof`), `ớ` (`ows`) can never
//! be affected. The cost is that standalone `í ú á ò ì ỏ` are un-composed when
//! typed alone; ESC restores them, and the whole heuristic sits behind
//! `EngineOptions::auto_restore_english`.

use crate::data::{HookCode, MARK_MASK, MAX_BUFF};
use crate::engine::Engine;
use crate::keys::*;

/// Raw keystrokes for the word in progress, in typed order, each cell
/// `keycode | CAPS_MASK` - the same encoding `Buffer::states` uses, so the
/// host's raw-keycode decode path applies unchanged.
#[derive(Debug, Default)]
pub(crate) struct RawJournal {
    keys: Vec<u32>,
}

impl RawJournal {
    pub(crate) fn clear(&mut self) {
        self.keys.clear();
    }

    pub(crate) fn push(&mut self, cell: u32) {
        self.keys.push(cell);
    }

    /// Drop the last keystroke (DELETE).
    pub(crate) fn pop(&mut self) {
        self.keys.pop();
    }

    pub(crate) fn len(&self) -> usize {
        self.keys.len()
    }

    pub(crate) fn keys(&self) -> &[u32] {
        &self.keys
    }
}

/// Two-letter raw-key sequences that are far more often English words than
/// standalone Vietnamese syllables. Each one is also the Telex spelling of the
/// Vietnamese word in the comment, which is exactly what makes this a
/// frequency judgement rather than a linguistic rule.
const ENGLISH_TWO_LETTER: &[(u16, u16)] = &[
    (KEY_I, KEY_S), // vs. í
    (KEY_U, KEY_S), // vs. ú
    (KEY_A, KEY_S), // vs. á
    (KEY_O, KEY_F), // vs. ò
    (KEY_I, KEY_F), // vs. ì
    (KEY_O, KEY_R), // vs. ỏ
];

impl Engine {
    /// `true` when the word in progress is one of the listed two-letter
    /// English words that Telex has composed into a single marked vowel.
    ///
    /// Intentionally narrow: it only ever fires on a two-keystroke word whose
    /// output is one marked vowel, so no longer word and no unmarked word can
    /// reach it.
    pub(crate) fn looks_like_english_word(&self) -> bool {
        if self.raw.len() != 2 || self.buf.index != 1 {
            return false;
        }
        // The single displayed character must actually carry a mark; without
        // one nothing was composed and there is nothing to undo.
        if self.buf.word[0] & MARK_MASK == 0 {
            return false;
        }
        let first = (self.raw.keys()[0] & 0xFFFF) as u16;
        let second = (self.raw.keys()[1] & 0xFFFF) as u16;
        ENGLISH_TWO_LETTER.iter().any(|&(a, b)| a == first && b == second)
    }

    /// Replace the displayed word with the raw keystrokes from the journal.
    /// Returns `false` (leaving the hook untouched) when there is nothing to
    /// undo, so the caller can let the triggering key through.
    ///
    /// `char_data` is filled in reverse, holding bare keycode cells, matching
    /// what `check_restore_if_wrong_spelling` emits: the host decodes cells
    /// without `CHAR_CODE_MASK` through `key_code_to_character`.
    pub(crate) fn restore_from_raw(&mut self, handle_code: HookCode) -> bool {
        let n = self.raw.len();
        // `new_char_count`/`backspace_count` are u8 and `char_data` is
        // MAX_BUFF wide; bail rather than truncate or overflow.
        if n == 0 || n > MAX_BUFF || self.buf.index < 0 || self.buf.index as usize > MAX_BUFF {
            return false;
        }
        if !self.transformed_since_raw() {
            return false;
        }

        self.hook.code = handle_code;
        self.hook.backspace_count = self.buf.index as u8;
        self.hook.new_char_count = n as u8;
        for (i, &cell) in self.raw.keys().iter().enumerate() {
            self.hook.char_data[n - 1 - i] = cell;
            self.buf.word[i] = cell;
        }
        self.buf.index = n as i32;
        self.buf.state_index = n as i32;
        true
    }

    /// Whether the displayed word differs from the keys that produced it. When
    /// they match, the engine composed nothing and a restore would be a no-op.
    fn transformed_since_raw(&self) -> bool {
        if self.buf.index as usize != self.raw.len() {
            return true;
        }
        self.raw
            .keys()
            .iter()
            .enumerate()
            .any(|(i, &cell)| self.buf.word[i] != cell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_records_pops_and_clears() {
        let mut j = RawJournal::default();
        assert_eq!(j.len(), 0);

        j.push(KEY_O as u32);
        j.push(KEY_F as u32);
        j.push(KEY_F as u32);
        assert_eq!(j.len(), 3, "every keystroke is kept, including a repeat");
        assert_eq!(j.keys(), &[KEY_O as u32, KEY_F as u32, KEY_F as u32]);

        j.pop();
        assert_eq!(j.keys(), &[KEY_O as u32, KEY_F as u32]);

        j.clear();
        assert_eq!(j.len(), 0);
    }

    /// The journal is not bounded by `MAX_BUFF`, which is the point: it is the
    /// only source that survives a word longer than the engine's buffer.
    #[test]
    fn journal_grows_past_max_buff() {
        let mut j = RawJournal::default();
        for _ in 0..(MAX_BUFF + 8) {
            j.push(KEY_A as u32);
        }
        assert_eq!(j.len(), MAX_BUFF + 8);
    }

    #[test]
    fn english_list_holds_only_two_letter_pairs_and_no_duplicates() {
        for (i, &(a, b)) in ENGLISH_TWO_LETTER.iter().enumerate() {
            assert!(
                !ENGLISH_TWO_LETTER[i + 1..].iter().any(|&(c, d)| a == c && b == d),
                "duplicate entry in ENGLISH_TWO_LETTER"
            );
        }
        // Every second key is a Telex tone key; a pair whose second key is a
        // plain letter could never have produced a mark in the first place.
        for &(_, b) in ENGLISH_TWO_LETTER {
            assert!(
                matches!(b, KEY_S | KEY_F | KEY_R | KEY_X | KEY_J),
                "second key must be a tone key"
            );
        }
    }
}
