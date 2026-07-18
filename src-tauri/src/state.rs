//! Process-wide shared state for the platform layer.
//!
//! Two pieces of state are shared between the Tauri main thread and the
//! dedicated CGEventTap thread:
//!
//! * [`EngineCore`] behind an `Arc<Mutex<_>>` — the Vietnamese engine plus the
//!   `_syncKey` stack ported from OpenKey. Only ever touched under the lock.
//! * [`SharedConfig`] — atomics the tap callback reads on the hot path without
//!   locking (engine on/off + a couple of injection-quirk toggles).

use std::sync::atomic::AtomicBool;

use vnkey_engine::Engine;

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
}

impl SharedConfig {
    pub fn new(opts: &vnkey_engine::EngineOptions) -> Self {
        Self {
            enabled: AtomicBool::new(opts.language == 1),
            send_key_step_by_step: AtomicBool::new(false),
            fix_recommend_browser: AtomicBool::new(opts.fix_recommend_browser),
            fix_chromium_browser: AtomicBool::new(false),
        }
    }
}
