//! Process-wide shared state for the platform layer.
//!
//! Two pieces of state are shared between the Tauri main thread and the
//! dedicated CGEventTap thread:
//!
//! * [`EngineCore`] behind an `Arc<Mutex<_>>` — the Vietnamese engine plus the
//!   `_syncKey` stack ported from OpenKey. Only ever touched under the lock.
//! * [`SharedConfig`] — atomics the tap callback reads on the hot path without
//!   locking (engine on/off + a couple of injection-quirk toggles).

use std::sync::atomic::{AtomicBool, AtomicU32};

use vnkey_engine::Engine;

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
}

impl EngineCore {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine,
            sync_key: Vec::new(),
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
        }
    }
}
