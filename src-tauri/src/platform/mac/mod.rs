//! macOS platform layer: CGEventTap keystroke interception + Vietnamese text
//! injection.

mod ffi;
mod inject;
pub(crate) mod hotkey;
mod layout;
mod quirks;
mod smart_switch;
mod tap;
mod tis;
mod workspace;

use std::sync::Arc;

use parking_lot::Mutex;
use vnkey_engine::{Engine, EngineOptions};

use crate::state::{EngineCore, SharedConfig};

/// Initialise the macOS platform layer: request Accessibility permission if
/// needed, build the shared engine state with default options, and spawn the
/// event-tap thread.
///
/// Engine options are hardcoded to `EngineOptions::default()` for this task
/// (Telex, Unicode, Vietnamese on); the settings UI wiring is a later task.
pub fn start() {
    let opts = EngineOptions::default();

    // Accessibility: prompt once if not yet trusted. We still spawn the tap —
    // it fails softly if permission is missing, and the user can grant it and
    // relaunch.
    if !ffi::is_trusted() {
        eprintln!(
            "VNKey: Accessibility permission not granted. Enable VNKey under \
             System Settings > Privacy & Security > Accessibility, then relaunch."
        );
        ffi::prompt_for_trust();
    }

    let engine = Engine::new(opts);
    let core = Arc::new(Mutex::new(EngineCore::new(engine)));
    let config = Arc::new(SharedConfig::new(&opts));
    let tap_control = Arc::new(tap::TapControl::new());

    // Prime the frontmost bundle-id cache on the main thread so the tap thread
    // has a value before the first activation notification (and never calls
    // NSWorkspace itself — Task 8 finding 3).
    config.set_frontmost_bundle_id(ffi::frontmost_bundle_id());

    // Register NSWorkspace lifecycle observers on the main thread (AppKit
    // requirement). This runs inside Tauri's `setup`, on the main run loop.
    workspace::register(core.clone(), config.clone(), tap_control.clone());

    tap::spawn(core, config, tap_control);
}
