//! Process-wide shared state for the platform layer.
//!
//! Two pieces of state are shared between the Tauri main thread and the
//! dedicated CGEventTap thread:
//!
//! * [`EngineCore`] behind an `Arc<Mutex<_>>` — the Vietnamese engine plus the
//!   `_syncKey` stack ported from OpenKey. Only ever touched under the lock.
//! * [`SharedConfig`] — atomics the tap callback reads on the hot path without
//!   locking (engine on/off + a couple of injection-quirk toggles).

use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};

use parking_lot::Mutex;
use vnkey_engine::{CodeTable, Engine, KeyEvent, KeyEventState, SmartSwitchTable, is_double_code};

use crate::platform::mac::hotkey::DEFAULT_SWITCH_KEY_STATUS;

/// Engine state guarded by the mutex.
///
/// `sync_key` is OpenKey's `_syncKey` (`OpenKey.mm:77`): a stack of the UTF-16
/// unit-length of every character VNKey has injected for the 2-byte code tables
/// (VNI Windows / Unicode Compound), used to compute how many real backspaces a
/// logical backspace needs. It lives beside the engine because it is logically
/// part of the "what have we already sent" bookkeeping, but it is only ever read
/// or written by the tap thread.
pub struct EngineCore {
    pub engine: Engine,
    pub sync_key: Vec<u8>,
    /// Per-app (bundle-id) input-language memory (`SmartSwitchKey.cpp`). Lives
    /// under the engine lock because both the main-thread NSWorkspace observer
    /// (on app activation) and the tap thread (on a manual language switch)
    /// mutate it. In-memory only this task; Task 11 loads/saves the on-disk
    /// blob via [`SmartSwitchTable::from_blob`]/`to_blob` through this field.
    pub smart_switch: SmartSwitchTable,
}

impl EngineCore {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine,
            sync_key: Vec::new(),
            smart_switch: SmartSwitchTable::new(),
        }
    }

    /// OpenKey `RequestNewSession()` (`OpenKey.mm:147-154`): drive a
    /// `Mouse`/`MouseDown` through the engine (the unconditional word-break
    /// path, which also clears `special_char`/`typing_states`) and, for the
    /// 2-byte code tables, clear the `_syncKey` stack. Used by the tap thread's
    /// mouse branch and by the active-space-change observer.
    pub fn request_new_session(&mut self, code_table: CodeTable) {
        let _ = self
            .engine
            .handle_event(KeyEvent::Mouse, KeyEventState::MouseDown, 0, 0, false);
        if is_double_code(code_table) {
            self.sync_key.clear();
        }
    }
}

/// Flags the tap callback reads on every keystroke without taking the engine
/// lock. Set once at startup in this task; later tasks flip them from the UI.
pub struct SharedConfig {
    /// Master engine on/off (OpenKey's `vLanguage == 1`). When `false` the tap
    /// passes every key through untouched.
    pub enabled: AtomicBool,
    /// `vSendKeyStepByStep` — send each character as its own keyboard event
    /// instead of one batched `SetUnicodeString`.
    pub send_key_step_by_step: AtomicBool,
    /// `vFixRecommendBrowser` — insert an empty character before backspacing to
    /// defeat autocomplete popups (Safari/Chrome address bars, etc.).
    pub fix_recommend_browser: AtomicBool,
    /// `vFixChromiumBrowser` — use Shift+Left selection instead of backspace in
    /// Chromium apps. Wiring of the actual Shift+Left branch is deferred (see
    /// `inject.rs`); the flag exists so the default (off) is explicit.
    pub fix_chromium_browser: AtomicBool,
    /// `vSwitchKeyStatus` — the packed language-switch hotkey (see
    /// `platform::mac::hotkey`). No settings UI yet (Task 13), so this starts
    /// at OpenKey's default (⌥Z, `DEFAULT_SWITCH_KEY_STATUS`).
    pub switch_key_status: AtomicU32,
    /// `vTempOffSpelling` — releasing Control alone temporarily disables spell
    /// checking for the in-progress word.
    pub temp_off_spelling: AtomicBool,
    /// `vTempOffOpenKey` — releasing Command alone temporarily disables the
    /// whole engine for the in-progress word.
    pub temp_off_openkey: AtomicBool,
    /// `vOtherLanguage` — when on, pass every key through untouched while the
    /// system's current keyboard input source isn't English.
    pub other_language: AtomicBool,
    /// `vPerformLayoutCompat` — translate physical keycodes through the
    /// current (non-QWERTY) keyboard layout before handing them to the
    /// engine. No settings UI yet (Task 13), so this starts off.
    pub perform_layout_compat: AtomicBool,
    /// `vUseSmartSwitchKey` — restore the per-app language/code table on app
    /// activation and remember it on a manual switch.
    pub use_smart_switch_key: AtomicBool,
    /// `vRememberCode` — smart-switch also restores/remembers the code table,
    /// not just the language.
    pub remember_code: AtomicBool,
    /// `vCodeTable` — the active code table, as its enum discriminant. Lives
    /// here (not only in the engine, whose options are private, and not as a
    /// fixed tap-thread field) so the main-thread smart-switch observer can
    /// change it and the tap thread reads the current value each keystroke.
    code_table: AtomicU8,
    /// Frontmost app bundle id, cached by the main-thread NSWorkspace observer
    /// (`workspace.rs`). The tap thread reads it for per-app injection quirks
    /// instead of calling `NSWorkspace.frontmostApplication` off-thread — this
    /// closes Task 8 finding 3. `None` until the first activation notification.
    frontmost_bundle_id: Mutex<Option<String>>,
    /// Set by the `set_hotkey_recording` IPC command while the Control
    /// Panel's `HotkeyRecorder` has focus (Task 13). The tap callback checks
    /// this immediately after the self-event filter and passes every event
    /// through untouched while it's `true`, so the recorder sees the raw
    /// chord instead of the tap swallowing/rewriting it.
    pub hotkey_recording: AtomicBool,
}

impl SharedConfig {
    pub fn new(opts: &vnkey_engine::EngineOptions) -> Self {
        Self {
            enabled: AtomicBool::new(opts.language == 1),
            send_key_step_by_step: AtomicBool::new(false),
            fix_recommend_browser: AtomicBool::new(opts.fix_recommend_browser),
            fix_chromium_browser: AtomicBool::new(false),
            switch_key_status: AtomicU32::new(DEFAULT_SWITCH_KEY_STATUS),
            temp_off_spelling: AtomicBool::new(opts.temp_off_spelling),
            temp_off_openkey: AtomicBool::new(opts.temp_off_openkey),
            other_language: AtomicBool::new(opts.other_language),
            perform_layout_compat: AtomicBool::new(false),
            use_smart_switch_key: AtomicBool::new(opts.use_smart_switch_key),
            remember_code: AtomicBool::new(opts.remember_code),
            code_table: AtomicU8::new(opts.code_table as u8),
            frontmost_bundle_id: Mutex::new(None),
            hotkey_recording: AtomicBool::new(false),
        }
    }

    /// The active code table (`vCodeTable`).
    pub fn code_table(&self) -> CodeTable {
        CodeTable::from_index(self.code_table.load(Ordering::Relaxed)).unwrap_or(CodeTable::Unicode)
    }

    /// Set the active code table (smart-switch remember-code branch).
    pub fn set_code_table(&self, code_table: CodeTable) {
        self.code_table.store(code_table as u8, Ordering::Relaxed);
    }

    /// The cached frontmost bundle id (a clone of the string the observer
    /// stored). `None` before the first app-activation notification.
    pub fn frontmost_bundle_id(&self) -> Option<String> {
        self.frontmost_bundle_id.lock().clone()
    }

    /// Cache the frontmost bundle id (called by the main-thread observer).
    pub fn set_frontmost_bundle_id(&self, bundle_id: Option<String>) {
        *self.frontmost_bundle_id.lock() = bundle_id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vnkey_engine::EngineOptions;

    #[test]
    fn bundle_id_cache_round_trips() {
        // Models the main-thread observer writing and the tap thread reading.
        let config = SharedConfig::new(&EngineOptions::default());
        assert_eq!(config.frontmost_bundle_id(), None);

        config.set_frontmost_bundle_id(Some("com.apple.TextEdit".into()));
        assert_eq!(
            config.frontmost_bundle_id().as_deref(),
            Some("com.apple.TextEdit")
        );

        config.set_frontmost_bundle_id(Some("com.google.Chrome".into()));
        assert_eq!(config.frontmost_bundle_id().as_deref(), Some("com.google.Chrome"));

        config.set_frontmost_bundle_id(None);
        assert_eq!(config.frontmost_bundle_id(), None);
    }

    #[test]
    fn code_table_defaults_from_options_and_updates() {
        let config = SharedConfig::new(&EngineOptions::default());
        assert_eq!(config.code_table(), CodeTable::Unicode);

        config.set_code_table(CodeTable::Cp1258);
        assert_eq!(config.code_table(), CodeTable::Cp1258);

        config.set_code_table(CodeTable::VniWindows);
        assert_eq!(config.code_table(), CodeTable::VniWindows);
    }

    #[test]
    fn smart_switch_flags_track_options() {
        let config = SharedConfig::new(&EngineOptions::default());
        // OpenKey defaults: both on.
        assert!(config.use_smart_switch_key.load(Ordering::Relaxed));
        assert!(config.remember_code.load(Ordering::Relaxed));
    }

    /// Task 13: the HotkeyRecorder's tap-passthrough flag must start off (the
    /// tap must process keys normally until the UI explicitly starts
    /// recording) and reflect whatever the `set_hotkey_recording` command
    /// last stored.
    #[test]
    fn hotkey_recording_defaults_false_and_toggles() {
        let config = SharedConfig::new(&EngineOptions::default());
        assert!(!config.hotkey_recording.load(Ordering::Relaxed));

        config.hotkey_recording.store(true, Ordering::Relaxed);
        assert!(config.hotkey_recording.load(Ordering::Relaxed));

        config.hotkey_recording.store(false, Ordering::Relaxed);
        assert!(!config.hotkey_recording.load(Ordering::Relaxed));
    }
}
