//! FFI bindings to the vendored OpenKey C++ engine (`vendor/`), used as a
//! differential test oracle against `vnkey-engine`.
//!
//! The C++ engine is a pile of global mutable state (option globals,
//! typing-word buffer, macro map, ...), so every call must go through the
//! process-wide [`ORACLE_LOCK`] mutex. [`Oracle::lock`] is the only way to
//! get at the engine: it acquires the lock, resets the C++ globals, and
//! hands back an [`OracleSession`] that borrows the guard. All engine
//! operations live on `OracleSession`, not `Oracle`, so a caller cannot
//! forget to lock and reset before talking to the engine.
//!
//! Dev-only: this crate wraps GPL-licensed vendored C++ and must never ship
//! in the VNKey release build (`publish = false` in Cargo.toml; only ever a
//! dev-dependency).

use std::sync::{Mutex, MutexGuard};

/// Mirrors the C `OkOptions` struct in `shim/oracle_shim.cpp` field-for-field
/// (all `int32_t`): the ~21 option globals from `vendor/Engine.h` plus
/// `switch_key_status`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OkOptions {
    pub language: i32,
    pub input_type: i32,
    pub free_mark: i32,
    pub code_table: i32,
    pub switch_key_status: i32,
    pub check_spelling: i32,
    pub use_modern_orthography: i32,
    pub quick_telex: i32,
    pub restore_if_wrong_spelling: i32,
    pub fix_recommend_browser: i32,
    pub use_macro: i32,
    pub use_macro_in_english_mode: i32,
    pub auto_caps_macro: i32,
    pub use_smart_switch_key: i32,
    pub upper_case_first_char: i32,
    pub temp_off_spelling: i32,
    pub allow_consonant_zfwj: i32,
    pub quick_start_consonant: i32,
    pub quick_end_consonant: i32,
    pub remember_code: i32,
    pub other_language: i32,
    pub temp_off_openkey: i32,
}

/// Mirrors the C `OkState` struct in `shim/oracle_shim.cpp` field-for-field:
/// a flattened, capacity-clamped copy of the C++ engine's `vKeyHookState`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OkState {
    pub code: u8,
    pub backspace_count: u8,
    pub new_char_count: u8,
    pub ext_code: u8,
    pub char_data: [u32; 32],
    pub macro_key_len: u32,
    pub macro_key: [u32; 256],
    pub macro_data_len: u32,
    pub macro_data: [u32; 1024],
}

impl Default for OkState {
    fn default() -> Self {
        OkState {
            code: 0,
            backspace_count: 0,
            new_char_count: 0,
            ext_code: 0,
            char_data: [0; 32],
            macro_key_len: 0,
            macro_key: [0; 256],
            macro_data_len: 0,
            macro_data: [0; 1024],
        }
    }
}

unsafe extern "C" {
    fn ok_reset(opts: *const OkOptions);
    fn ok_set_options(opts: *const OkOptions);
    fn ok_handle_event(event: u8, state: u8, keycode: u16, caps: u8, other_control: u8);
    fn ok_english_mode(state: u8, keycode: u16, caps: u8, other_control: u8);
    fn ok_start_new_session();
    fn ok_temp_off_spell_checking();
    fn ok_set_check_spelling();
    fn ok_temp_off_engine(off: u8);
    fn ok_get_state(out: *mut OkState);
    fn ok_get_character_code(code_table: i32, cell: u32) -> u32;
    fn ok_key_code_to_character(key_code: u32) -> u32;
    fn ok_set_macro_blob(data: *const u8, len: u32);
    fn ok_get_macro_blob(out: *mut u8, cap: u32) -> u32;
    fn ok_add_macro(text: *const u8, tlen: u32, content: *const u8, clen: u32);
    fn ok_macro_on_table_change(code_table: i32);
    fn ok_find_macro(
        key: *const u32,
        key_len: u32,
        code_table: i32,
        auto_caps: i32,
        out: *mut u32,
        cap: u32,
    ) -> u32;
    fn ok_smart_set_blob(data: *const u8, len: u32);
    fn ok_smart_get_blob(out: *mut u8, cap: u32) -> u32;
    fn ok_smart_get(bundle: *const u8, len: u32, current: i32) -> i32;
    fn ok_smart_set(bundle: *const u8, len: u32, language: i32);
    fn ok_convert_util(
        src: *const u8,
        len: u32,
        opts: *const OkConvertOpts,
        out: *mut u8,
        cap: u32,
    ) -> u32;
}

/// Mirrors the C `OkConvertOpts` struct in `shim/oracle_shim.cpp`: the 7
/// `convertTool*` globals that affect `convertUtil` (the two host-only knobs
/// `convertToolHotKey`/`convertToolDontAlertWhenCompleted` are omitted).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OkConvertOpts {
    pub from_code: i32,
    pub to_code: i32,
    pub remove_mark: i32,
    pub to_all_caps: i32,
    pub to_all_non_caps: i32,
    pub to_caps_first_letter: i32,
    pub to_caps_each_word: i32,
}

/// Process-wide lock serializing all access to the C++ engine's global
/// state. Poisoning (a panic while holding the lock) does not invalidate
/// the engine's C++ globals, so a poisoned lock is recovered rather than
/// propagated.
static ORACLE_LOCK: Mutex<()> = Mutex::new(());

/// Entry point to the oracle. See [`Oracle::lock`].
pub struct Oracle;

impl Oracle {
    /// Acquires the global oracle lock, resets the C++ engine with `options`
    /// (equivalent to a fresh process: all option globals set, `vKeyInit()`
    /// called, macro map and smart-switch-key data cleared), and returns a
    /// session for interacting with it.
    pub fn lock(options: &OkOptions) -> OracleSession {
        let guard = ORACLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { ok_reset(options) };
        OracleSession { _guard: guard }
    }
}

/// A locked, reset handle to the C++ engine. Holding this value is proof
/// that the oracle's global mutex is held and `ok_reset` has run, so every
/// engine-interaction method lives here rather than on [`Oracle`].
pub struct OracleSession {
    _guard: MutexGuard<'static, ()>,
}

impl OracleSession {
    /// `vKeyHandleEvent` -- main per-key entry point.
    pub fn handle_event(&mut self, event: u8, state: u8, keycode: u16, caps: u8, other_control: u8) {
        unsafe { ok_handle_event(event, state, keycode, caps, other_control) }
    }

    /// `vEnglishMode`.
    pub fn english_mode(&mut self, state: u8, keycode: u16, caps: u8, other_control: u8) {
        unsafe { ok_english_mode(state, keycode, caps, other_control) }
    }

    /// `startNewSession`.
    pub fn start_new_session(&mut self) {
        unsafe { ok_start_new_session() }
    }

    /// `vTempOffSpellChecking`.
    pub fn temp_off_spell_checking(&mut self) {
        unsafe { ok_temp_off_spell_checking() }
    }

    /// `vSetCheckSpelling`.
    pub fn set_check_spelling(&mut self) {
        unsafe { ok_set_check_spelling() }
    }

    /// `vTempOffEngine`.
    pub fn temp_off_engine(&mut self, off: bool) {
        unsafe { ok_temp_off_engine(off as u8) }
    }

    /// Re-applies option globals without resetting the typing buffer/macro
    /// map/`vKeyInit()` (`ok_set_options` in the shim), for tests that need
    /// to change an option mid-session.
    pub fn set_options(&mut self, options: &OkOptions) {
        unsafe { ok_set_options(options) }
    }

    /// Snapshot of the engine's `vKeyHookState` (host-facing output buffer).
    pub fn get_state(&self) -> OkState {
        let mut out = OkState::default();
        unsafe { ok_get_state(&mut out) };
        out
    }

    /// `getCharacterCode`, after pointing the engine at `code_table`
    /// (`vnkey_engine::CodeTable as i32`).
    pub fn get_character_code(&mut self, code_table: i32, cell: u32) -> u32 {
        unsafe { ok_get_character_code(code_table, cell) }
    }

    /// `keyCodeToCharacter`.
    pub fn key_code_to_character(&self, key_code: u32) -> u32 {
        unsafe { ok_key_code_to_character(key_code) }
    }

    /// `initMacroMap` with a serialized macro blob.
    pub fn set_macro_blob(&mut self, data: &[u8]) {
        unsafe { ok_set_macro_blob(data.as_ptr(), data.len() as u32) }
    }

    /// `getMacroSaveData`. Returns the serialized macro blob, truncated to
    /// `cap` bytes if it doesn't fit (mirrors the shim's clamp behavior).
    pub fn get_macro_blob(&self, cap: usize) -> Vec<u8> {
        let mut buf = vec![0u8; cap];
        let cap_u32 = u32::try_from(cap).unwrap_or(u32::MAX);
        let len = unsafe { ok_get_macro_blob(buf.as_mut_ptr(), cap_u32) };
        buf.truncate((len as usize).min(cap));
        buf
    }

    /// `addMacro(text, content)` under the current `vCodeTable`.
    pub fn add_macro(&mut self, text: &str, content: &str) {
        unsafe {
            ok_add_macro(
                text.as_ptr(),
                text.len() as u32,
                content.as_ptr(),
                content.len() as u32,
            )
        }
    }

    /// `vCodeTable = code_table; onTableCodeChange()`.
    pub fn macro_on_table_change(&mut self, code_table: i32) {
        unsafe { ok_macro_on_table_change(code_table) }
    }

    /// `findMacro(key, out)` under `vCodeTable = code_table` /
    /// `vAutoCapsMacro = auto_caps`. Returns `Some(content_codes)` on a match
    /// (possibly empty), `None` on a miss.
    pub fn find_macro(&mut self, key: &[u32], code_table: i32, auto_caps: bool) -> Option<Vec<u32>> {
        let mut out = vec![0u32; 1024];
        let total = unsafe {
            ok_find_macro(
                key.as_ptr(),
                key.len() as u32,
                code_table,
                auto_caps as i32,
                out.as_mut_ptr(),
                out.len() as u32,
            )
        };
        if total == 0xFFFF_FFFF {
            return None;
        }
        out.truncate((total as usize).min(out.len()));
        Some(out)
    }

    /// `initSmartSwitchKey` with a serialized smart-switch blob.
    pub fn set_smart_blob(&mut self, data: &[u8]) {
        unsafe { ok_smart_set_blob(data.as_ptr(), data.len() as u32) }
    }

    /// `getSmartSwitchKeySaveData`. Returns the serialized blob, truncated to
    /// `cap` bytes if it doesn't fit.
    pub fn get_smart_blob(&self, cap: usize) -> Vec<u8> {
        let mut buf = vec![0u8; cap];
        let cap_u32 = u32::try_from(cap).unwrap_or(u32::MAX);
        let len = unsafe { ok_smart_get_blob(buf.as_mut_ptr(), cap_u32) };
        buf.truncate((len as usize).min(cap));
        buf
    }

    /// `getAppInputMethodStatus(bundle_id, current)`.
    pub fn smart_get(&mut self, bundle_id: &str, current: i32) -> i32 {
        unsafe { ok_smart_get(bundle_id.as_ptr(), bundle_id.len() as u32, current) }
    }

    /// `setAppInputMethodStatus(bundle_id, language)`.
    pub fn smart_set(&mut self, bundle_id: &str, language: i32) {
        unsafe { ok_smart_set(bundle_id.as_ptr(), bundle_id.len() as u32, language) }
    }

    /// `convertUtil(src)` after setting the 7 `convertTool*` option globals.
    ///
    /// Returns `None` when the C++ `convertUtil` threw (a `std::range_error`
    /// from `wstring_convert` on a lone-surrogate output -- an OpenKey crash
    /// bug the shim catches). Differential callers exclude those inputs.
    pub fn convert_util(&mut self, src: &str, opts: &OkConvertOpts) -> Option<Vec<u8>> {
        let mut out = vec![0u8; src.len() * 8 + 16];
        let total = unsafe {
            ok_convert_util(
                src.as_ptr(),
                src.len() as u32,
                opts,
                out.as_mut_ptr(),
                out.len() as u32,
            )
        };
        if total == u32::MAX {
            return None; // C++ convertUtil threw
        }
        // Grow and retry if the output didn't fit (defensive; the 8x + 16
        // headroom is generous for the widest table-to-table expansion).
        if total as usize > out.len() {
            out = vec![0u8; total as usize];
            unsafe {
                ok_convert_util(
                    src.as_ptr(),
                    src.len() as u32,
                    opts,
                    out.as_mut_ptr(),
                    out.len() as u32,
                )
            };
        }
        out.truncate(total as usize);
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Raw literals, not vnkey-engine constants: this crate must stay
    // independent of vnkey-engine (see module docs).
    const KEY_A: u32 = 0;
    const CAPS_MASK: u32 = 0x10000;
    const TONE_MASK: u32 = 0x20000;
    const MARK1_MASK: u32 = 0x80000;
    const CHAR_CODE_MASK: u32 = 0x2000000;
    const UNICODE_TABLE: i32 = 0;

    #[test]
    fn get_character_code_matches_hand_verified_unicode_cell() {
        // KEY_A|TONE_MASK|MARK1_MASK, Unicode table -> 'ấ' (0x1EA5), the
        // same cell vnkey-engine's own unit tests hand-verify.
        let opts = OkOptions::default();
        let mut session = Oracle::lock(&opts);
        let cell = KEY_A | TONE_MASK | MARK1_MASK;
        assert_eq!(
            session.get_character_code(UNICODE_TABLE, cell),
            0x1EA5 | CHAR_CODE_MASK
        );
    }

    #[test]
    fn key_code_to_character_round_trips_letter_a() {
        let opts = OkOptions::default();
        let session = Oracle::lock(&opts);
        assert_eq!(session.key_code_to_character(KEY_A), b'a' as u32);
        assert_eq!(session.key_code_to_character(KEY_A | CAPS_MASK), b'A' as u32);
    }

    // Raw mac keycodes (vendor/platforms/mac.h) used by the reset test below.
    const K_V: u16 = 9;
    const K_I: u16 = 34;
    const K_E: u16 = 14;
    const K_J: u16 = 38;
    const K_T: u16 = 17;
    const K_A: u16 = 0;
    // vKeyEvent::Keyboard = 0, vKeyEventState::KeyDown = 0.
    const EV_KB: u8 = 0;
    const ST_DOWN: u8 = 0;

    fn state_tuple(s: &OkState) -> (u8, u8, u8, u8, Vec<u32>) {
        (
            s.code,
            s.backspace_count,
            s.new_char_count,
            s.ext_code,
            s.char_data[..s.new_char_count as usize].to_vec(),
        )
    }

    /// Coverage proof for the VNKEY-ORACLE-PATCH reset: a session that leaves
    /// the engine dirty (tempDisableKey / TypingWord / marks / HookState all
    /// mutated) must not bleed into the next locked session. We type the same
    /// probe key `a` (1) from a pristine engine and (2) right after a polluting
    /// session; the resulting HookState must be identical. Before the reset fix
    /// this failed because vKeyInit() left tempDisableKey, TypingWord, marks,
    /// _upperCaseStatus, _specialChar, etc. untouched.
    #[test]
    fn ok_reset_clears_leaked_static_state_between_sessions() {
        let opts = OkOptions {
            check_spelling: 1,
            ..OkOptions::default()
        };

        // (1) pristine engine, type `a`.
        let baseline = {
            let mut s = Oracle::lock(&opts);
            s.handle_event(EV_KB, ST_DOWN, K_A, 0, 0);
            state_tuple(&s.get_state())
        };

        // (2) pollute the engine with a full word incl. a tone mark ("việt"),
        // which leaves TypingWord populated, marks set, and tempDisableKey in a
        // non-default state; then drop the lock without cleaning up.
        {
            let mut s = Oracle::lock(&opts);
            for &k in &[K_V, K_I, K_E, K_E, K_J, K_T] {
                s.handle_event(EV_KB, ST_DOWN, k, 0, 0);
            }
        }

        // (3) fresh lock (=> ok_reset) then the same probe key `a`.
        let after_pollution = {
            let mut s = Oracle::lock(&opts);
            s.handle_event(EV_KB, ST_DOWN, K_A, 0, 0);
            state_tuple(&s.get_state())
        };

        assert_eq!(
            baseline, after_pollution,
            "ok_reset must fully clear leaked static state between sessions"
        );
    }

    #[test]
    fn lock_can_be_reacquired_sequentially() {
        // Regression check for the mutex-guard pattern: acquiring, dropping,
        // and reacquiring the oracle lock must not deadlock or panic.
        let opts = OkOptions::default();
        {
            let _session = Oracle::lock(&opts);
        }
        let session = Oracle::lock(&opts);
        assert_eq!(session.key_code_to_character(KEY_A), b'a' as u32);
    }
}
