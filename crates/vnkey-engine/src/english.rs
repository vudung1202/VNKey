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
//! actually pressed. The restore that `restore_if_wrong_spelling` adds on its
//! own keeps using `states` (see `check_restore_if_wrong_spelling`), whose
//! character counts are pinned by tests; the restores below read the journal.
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
//!
//! # More signals, all behind `auto_restore_english`
//!
//! * **No marks on an impossible nucleus**: once the vowels typed cannot
//!   start a Vietnamese nucleus (`ei`, `ou`, `ưe`), tone and vowel keys go in
//!   as plain letters, so `their` never shows `thẻi`.
//! * **Mid-word**: once a key makes a word that already carries a Vietnamese
//!   mark impossible as Vietnamese, the raw keys go back on screen right away
//!   (`pềc` + `t` -> `perfect`) instead of waiting for SPACE. A DELETE straight
//!   after undoes it, so a stray key in a Vietnamese word (`việtt`) costs one
//!   backspace. Valid Vietnamese never gets here: every prefix of a correctly
//!   typed syllable passes these checks, or the engine could not compose it.
//! * **End of word**: the keys spell a common English word and the composed
//!   word is not a Vietnamese syllable at all (`more` -> `moẻ`). Any real
//!   syllable wins, however rare: `this` stays `thí`, `test` stays `tét`.
//!   Lists and their licence: `data/README.md`. Words the spelling check
//!   rejects are restored from the journal too, so a double a tone key
//!   swallowed comes back (`message`, not `mesage`).
//!
//! None of this may break the Telex double-key habit: typing a tone key twice
//! to get the bare letter (`tesst` for `test`) gives the same result as
//! before, so the journal is only preferred when the shorter word is not a
//! known word, and a doubled key after an impossible nucleus is swallowed.
//!
//! `tests/golden_typing.rs` measures all of this over whole word lists
//! (`corpus_*`): Vietnamese syllables and the double-key habit must come out
//! unchanged.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::buffer::Buffer;
use crate::charset::key_code_to_character;
use crate::data::{
    CAPS_MASK, CHAR_MASK, HookCode, MARK1_MASK, MARK2_MASK, MARK3_MASK, MARK4_MASK, MARK5_MASK, MARK_MASK,
    MAX_BUFF, TONE_MASK, TONEW_MASK,
};
use crate::engine::Engine;
use crate::input_method::{is_consonant, is_special_key};
use crate::keys::*;
use crate::vietnamese::VOWEL_COMBINE;

/// Raw keystrokes for the word in progress, in typed order, each cell
/// `keycode | CAPS_MASK` - the same encoding `Buffer::states` uses, so the
/// host's raw-keycode decode path applies unchanged.
#[derive(Debug, Default, Clone)]
pub(crate) struct RawJournal {
    keys: Vec<u32>,
    /// The keys no longer describe the word on screen (a DELETE inside a
    /// composed word, or a word rebuilt from history). Restoring from a
    /// diverged journal would replay the wrong text, so every restore refuses.
    diverged: bool,
}

impl RawJournal {
    pub(crate) fn clear(&mut self) {
        self.keys.clear();
        self.diverged = false;
    }

    /// Stop trusting the journal until the next word starts.
    pub(crate) fn invalidate(&mut self) {
        self.keys.clear();
        self.diverged = true;
    }

    pub(crate) fn is_reliable(&self) -> bool {
        !self.diverged
    }

    pub(crate) fn last(&self) -> Option<u32> {
        self.keys.last().copied()
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

/// `word zipf*10` per line, as written by `data/gen_wordlists.py`.
fn load_word_list(text: &'static str) -> HashMap<&'static str, u8> {
    text.lines()
        .filter_map(|line| {
            let (word, zipf) = line.split_once(' ')?;
            Some((word, zipf.parse().ok()?))
        })
        .collect()
}

/// Common English words Telex can transform, keyed by their letters.
pub(crate) static ENGLISH_WORDS: LazyLock<HashMap<&'static str, u8>> =
    LazyLock::new(|| load_word_list(include_str!("../data/en_words.txt")));

/// Every Vietnamese syllable with a diacritic, keyed by canonical Telex
/// (`thế` -> `thees`, tone key last), with its frequency (0 when unknown).
pub(crate) static VIETNAMESE_SYLLABLES: LazyLock<HashMap<&'static str, u8>> =
    LazyLock::new(|| load_word_list(include_str!("../data/vi_syllables.txt")));

/// A c/ch/p/t syllable still missing its tone is kept when its sắc or nặng
/// form reaches this frequency (zipf * 10), see `is_vietnamese`.
const VIETNAMESE_KEEP_ZIPF: u8 = 30;

/// Lowercase ASCII letters for `cells`, or `None` if any is not a letter.
fn letters(cells: impl Iterator<Item = u32>) -> Option<String> {
    cells
        .map(|cell| {
            char::from_u32(key_code_to_character(cell & CHAR_MASK))
                .filter(char::is_ascii_lowercase)
        })
        .collect()
}

/// For a vowel nucleus (buffer cells): `None` when it is not even the start
/// of a Vietnamese nucleus, else whether it can take a final consonant. A hat
/// or horn already typed must match, but one still missing is accepted since
/// it may come later (`ue` before `uê`, while `ưe` is impossible).
fn nucleus_shape(nucleus: &[u32]) -> Option<bool> {
    let mut is_prefix = false;
    let mut takes_final = false;
    for combo in VOWEL_COMBINE.get(&((nucleus[0] & CHAR_MASK) as u16))? {
        let letters = &combo[1..];
        if letters.len() >= nucleus.len()
            && letters.iter().zip(nucleus).all(|(&c, &w)| {
                c & CHAR_MASK == w & CHAR_MASK
                    && (w & TONE_MASK == 0 || c & TONE_MASK != 0)
                    && (w & TONEW_MASK == 0 || c & TONEW_MASK != 0)
            })
        {
            is_prefix = true;
            takes_final |= letters.len() == nucleus.len() && combo[0] == 1;
        }
    }
    is_prefix.then_some(takes_final)
}

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
        if n == 0
            || n > MAX_BUFF
            || self.buf.index < 0
            || self.buf.index as usize > MAX_BUFF
            || !self.raw.is_reliable()
        {
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

    /// At the end of a word: whether to put the raw keys back from the
    /// journal, which unlike `buf.states` still has the keys a doubled tone
    /// key swallowed (`message` shows as `mesage`).
    pub(crate) fn should_restore_raw_at_word_end(&self) -> bool {
        let n = self.raw.len();
        if !self.opts.auto_restore_english
            || !self.raw.is_reliable()
            || !self.buf.long_word_helper.is_empty()
            || !(2..=MAX_BUFF).contains(&n)
            || self.buf.index <= 0
            || self.buf.index as usize > MAX_BUFF
            || !self.transformed_since_raw()
        {
            return false;
        }
        let (Some(typed), Some((composed, tone, marked))) =
            (letters(self.raw.keys().iter().copied()), self.composed_telex())
        else {
            return false;
        };
        match ENGLISH_WORDS.get(typed.as_str()) {
            // No mark left means a doubled key undid one (`pass` -> `pas`).
            // Keep whichever English word is more common, so the Telex escape
            // still wins where it is the likelier intent (`buss` -> `bus`,
            // and the pinned `ass` -> `as`, `off` -> `of`).
            Some(&typed_zipf) if !marked => {
                ENGLISH_WORDS.get(composed.as_str()).is_none_or(|&z| z < typed_zipf)
            }
            Some(&typed_zipf) => !self.is_vietnamese(&composed, tone, typed_zipf),
            // Not a known word: only one the spelling check rejected and the
            // old restore would undo anyway (a vowel mark; `đc` has none and
            // stays), or one with a swallowed double (`buffered`), which the
            // journal still has. The swallowed double may instead be the
            // Telex escape used on purpose, so the old restore stays when the
            // result is a known word (`tesst` for `test`), when the double is
            // at the very end (`tieengss`), or when a key was typed three
            // times (`offfice` for `office`).
            None => {
                self.temp_disable_key
                    && (if marked {
                        self.has_vowel_mark(self.buf.index)
                    } else {
                        !ENGLISH_WORDS.contains_key(composed.as_str())
                    })
                    && !self.ends_with_doubled_transform_key()
                    && !self.has_tripled_key()
            }
        }
    }

    /// Whether the composed word is a Vietnamese syllable, however rare:
    /// `this` stays `thí`, `test` stays `tét` (bánh tét). A c/ch/p/t final
    /// with no tone is not a finished syllable (those take only sắc or nặng):
    /// the tone may come later or have been forgotten. That one is kept when
    /// the toned syllable is common and at least as frequent as the English
    /// word (`toot` stays `tôt` for `tốt`, but `data` is not `dật`).
    fn is_vietnamese(&self, composed: &str, tone: Option<char>, typed_zipf: u8) -> bool {
        let zipf = |t: Option<char>| {
            let key: String = composed.chars().chain(t).collect();
            VIETNAMESE_SYLLABLES.get(key.as_str()).copied()
        };
        if tone.is_none() && composed.ends_with(['c', 'p', 't']) {
            let best = zipf(Some('s')).max(zipf(Some('j'))).unwrap_or(0);
            best >= VIETNAMESE_KEEP_ZIPF && best >= typed_zipf
        } else {
            zipf(tone).is_some()
        }
    }

    fn has_tripled_key(&self) -> bool {
        self.raw
            .keys()
            .windows(3)
            .any(|w| w[0] & CHAR_MASK == w[1] & CHAR_MASK && w[1] & CHAR_MASK == w[2] & CHAR_MASK)
    }

    fn ends_with_doubled_transform_key(&self) -> bool {
        let keys = self.raw.keys();
        let last = (keys[keys.len() - 1] & CHAR_MASK) as u16;
        let before = (keys[keys.len() - 2] & CHAR_MASK) as u16;
        last == before && is_special_key(self.opts.input_type, last)
    }

    /// The word on screen spelled the canonical Telex way minus its tone key
    /// (`thế` -> `thee`; `VIETNAMESE_SYLLABLES` keys add it last, `thees`),
    /// its tone key, and whether it carries any Vietnamese mark at all.
    fn composed_telex(&self) -> Option<(String, Option<char>, bool)> {
        let cells = &self.buf.word[..self.buf.index as usize];
        let mut out = letters(cells.iter().copied())?;
        let mut keys = String::with_capacity(out.len() + 4);
        let mut tone = None;
        for (ch, &cell) in out.chars().zip(cells) {
            keys.push(ch);
            if cell & TONE_MASK != 0 {
                keys.push(ch); // â ê ô, and đ
            }
            if cell & TONEW_MASK != 0 {
                keys.push('w'); // ă ơ ư
            }
            tone = tone.or(match cell & MARK_MASK {
                MARK1_MASK => Some('s'),
                MARK2_MASK => Some('f'),
                MARK3_MASK => Some('r'),
                MARK4_MASK => Some('x'),
                MARK5_MASK => Some('j'),
                _ => None,
            });
        }
        let marked = tone.is_some() || keys.len() != out.len();
        if marked {
            out = keys;
        }
        Some((out, tone, marked))
    }

    /// The engine state before a normal key, for `restore_mid_word`. `None`
    /// when no mid-word restore can follow this key, which also covers a word
    /// that was already broken before it (an earlier key dealt with that).
    pub(crate) fn mid_word_snapshot(&self) -> Option<(Buffer, RawJournal)> {
        (self.opts.auto_restore_english
            && self.spell_check_on
            && !self.temp_disable_key
            && !self.raw_restored
            && self.raw.is_reliable())
        .then(|| (self.buf.clone(), self.raw.clone()))
    }

    /// Called after every normal key. When the key just went in as a plain
    /// letter and left a word with Vietnamese marks that cannot be
    /// Vietnamese any more, put the raw keys back on screen now.
    pub(crate) fn restore_mid_word(&mut self, before: Option<(Buffer, RawJournal)>, data: u16) {
        let Some(before) = before else {
            return;
        };
        let index = self.buf.index;
        let n = self.raw.len();
        if !matches!(self.hook.code, HookCode::DoNothing | HookCode::WillProcess)
            // The key must have added itself as one more letter: inserted
            // plain, possibly with `check_grammar` moving the tone after.
            || index != before.0.index + 1
            || self.buf.chr(index - 1) != data
            || !self.buf.long_word_helper.is_empty()
            || index < 2
            || index as usize >= MAX_BUFF
            || n > MAX_BUFF
            || n < index as usize
        {
            return;
        }
        let spelling_broke = self.temp_disable_key && !(self.spelling_ok && self.spelling_vowel_ok);
        if !(spelling_broke || self.cannot_be_vietnamese()) || !self.has_vowel_mark(index) {
            return;
        }

        // Same shape as the duplicate-mark undo: erase what was shown before
        // this key, type the raw keys before it, and let the host echo it.
        let shown = index - 1;
        self.hook.code = HookCode::Restore;
        self.hook.ext_code = 3; // normal key, whatever `check_grammar` set
        self.hook.backspace_count = shown as u8;
        self.hook.new_char_count = (n - 1) as u8;
        let keys = self.raw.keys();
        for (i, &cell) in keys.iter().enumerate() {
            if i + 1 < n {
                self.hook.char_data[n - 2 - i] = cell;
            }
            self.buf.word[i] = cell;
            self.buf.states[i] = cell;
        }
        self.buf.index = n as i32;
        self.buf.state_index = n as i32;
        if self.opts.use_macro {
            // `macro_key` mirrors the screen, the key just typed included.
            let keep = self.hook.macro_key.len().saturating_sub(index as usize);
            self.hook.macro_key.truncate(keep);
            self.hook.macro_key.extend_from_slice(keys);
        }
        self.raw_restored = true;
        self.restore_undo = Some(before);
    }

    /// DELETE straight after `restore_mid_word`: put back the composed word
    /// exactly as it was before the key that triggered the restore.
    pub(crate) fn undo_mid_word_restore(&mut self, (buf, raw): (Buffer, RawJournal)) {
        let shown = self.buf.index;
        self.buf = buf;
        self.raw = raw;
        self.temp_disable_key = false;
        self.raw_restored = false;

        let index = self.buf.index;
        self.hook.code = HookCode::WillProcess;
        self.hook.ext_code = 2; // delete
        self.hook.backspace_count = shown as u8;
        self.hook.new_char_count = index as u8;
        for i in 0..index {
            self.hook.char_data[(index - 1 - i) as usize] = self.get(self.buf.word[i as usize]);
        }
        if self.opts.use_macro {
            let keep = self.hook.macro_key.len().saturating_sub(shown as usize);
            self.hook.macro_key.truncate(keep);
            self.hook.macro_key.extend_from_slice(&self.buf.word[..index as usize]);
        }
    }

    /// Whether `data` repeats the previous key, which sits plain at the end of
    /// the word. The caller knows that key went in plain only because of
    /// `nucleus_rules_out_vietnamese` (`Engine::mark_key_forced_plain`).
    pub(crate) fn repeats_plain_mark_key(&self, data: u16) -> bool {
        let index = self.buf.index;
        let n = self.raw.len();
        index > 0
            && n >= 2
            && self.raw.keys()[n - 2] & CHAR_MASK == data as u32
            && self.buf.word[index as usize - 1] & !CAPS_MASK == data as u32
    }

    /// Telex habit: typing a tone key twice gives the bare letter (`aerr` ->
    /// `aer`), because the second press undoes the mark. When the first press
    /// already went in plain (`ae` cannot be Vietnamese), drop the second the
    /// way the undo would, so a typist who doubles it out of habit does not
    /// get an extra letter. The journal keeps both keys for ESC.
    pub(crate) fn swallow_repeated_mark_key(&mut self) {
        self.hook.code = HookCode::WillProcess;
        self.hook.backspace_count = 0;
        self.hook.new_char_count = 0;
        self.hook.ext_code = 4; // nothing to send, so no empty-character fix
        // Same bookkeeping as the duplicate-mark undo in `handle_event`.
        self.buf.state_index -= 1;
    }

    /// Whether any vowel among the first `upto` cells carries a mark, tone or
    /// horn, i.e. whether there is anything to restore.
    fn has_vowel_mark(&self, upto: i32) -> bool {
        (0..upto).any(|i| {
            !is_consonant(self.buf.chr(i))
                && self.buf.word[i as usize] & (MARK_MASK | TONE_MASK | TONEW_MASK) != 0
        })
    }

    /// Shapes the incremental spelling check lets through but no Vietnamese
    /// syllable has, whatever is typed next.
    fn cannot_be_vietnamese(&self) -> bool {
        let index = self.buf.index;
        let chr = |i: i32| self.buf.chr(i);
        // c and p finals take only sắc or nặng (`spelling` covers ch and t).
        let stop_final_with_wrong_tone = matches!(chr(index - 1), KEY_C | KEY_P)
            && !is_consonant(chr(index - 2))
            && self.buf.word[..index as usize]
                .iter()
                .any(|&w| w & (MARK2_MASK | MARK3_MASK | MARK4_MASK) != 0);
        stop_final_with_wrong_tone || self.nucleus_rules_out_vietnamese()
    }

    /// Checked BEFORE a key is applied: once the vowels typed so far cannot
    /// start a Vietnamese nucleus (`ei`, `yo`, `ou`), no key can fix that,
    /// so a tone or vowel key goes in as a plain letter and `their` never
    /// shows `thẻi`.
    pub(crate) fn nucleus_rules_out_vietnamese(&self) -> bool {
        let index = self.buf.index;
        if !self.opts.auto_restore_english
            || !self.spell_check_on
            || index < 2
            || index as usize > MAX_BUFF
        {
            return false;
        }
        let chr = |i: i32| self.buf.chr(i);
        // The vowel nucleus, with the i of gi- and the u of qu- left out.
        let mut start = 0;
        while start < index && is_consonant(chr(start)) {
            start += 1;
        }
        if start + 1 < index
            && !is_consonant(chr(start + 1))
            && ((start == 1 && chr(0) == KEY_G && chr(1) == KEY_I)
                || (start >= 1 && chr(start - 1) == KEY_Q && chr(start) == KEY_U))
        {
            start += 1;
        }
        let mut end = start;
        while end < index && !is_consonant(chr(end)) {
            end += 1;
        }
        if end - start < 2 {
            return false;
        }
        match nucleus_shape(&self.buf.word[start as usize..end as usize]) {
            None => true,                                   // `ae`, `ou`, `ei`
            Some(takes_final) => end < index && !takes_final, // `ui` + c
        }
    }

    /// Whether the displayed word differs from the keys that produced it. When
    /// they match, the engine composed nothing and a restore would be a no-op.
    pub(crate) fn transformed_since_raw(&self) -> bool {
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
