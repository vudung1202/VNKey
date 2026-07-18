//! Rust -> UI events.
//!
//! One thin wrapper per event name the brief specifies, so every emit site
//! goes through a typed helper instead of hand-typing an event-name string
//! and payload shape. Emit failures (e.g. no webview yet listening) are
//! swallowed -- there is nothing a caller on the hot paths (menu click,
//! hotkey switch) could usefully do about them.

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::settings::Settings;

/// Payload for `status-changed`: just the fields the tray and the UI's
/// status indicator need, not the whole [`Settings`].
#[derive(Debug, Clone, Copy, Serialize)]
pub struct StatusChanged {
    pub language: u8,
    pub input_type: u8,
    pub code_table: u8,
}

/// Emit `status-changed` (language/input-type/code-table changed).
pub fn status_changed(app: &AppHandle, settings: &Settings) {
    let payload = StatusChanged {
        language: settings.language,
        input_type: settings.input_type,
        code_table: settings.code_table,
    };
    let _ = app.emit("status-changed", payload);
}

/// Emit `settings-changed` (the full, current [`Settings`]).
pub fn settings_changed(app: &AppHandle, settings: &Settings) {
    let _ = app.emit("settings-changed", *settings);
}

/// Emit `macros-changed` (the macro table was added to / deleted from /
/// imported).
pub fn macros_changed(app: &AppHandle) {
    let _ = app.emit("macros-changed", ());
}

/// Emit `permission-changed` (Accessibility trust status). No caller in this
/// task's scope pushes this yet -- there is no AppKit/TCC notification for an
/// Accessibility grant change to hook it to, so the UI currently learns the
/// status by calling `get_permission_status` (e.g. after the user returns
/// from System Settings). Kept as the documented helper the brief specifies,
/// ready for a future polling/notification caller.
#[allow(dead_code)]
pub fn permission_changed(app: &AppHandle, granted: bool) {
    let _ = app.emit("permission-changed", granted);
}
