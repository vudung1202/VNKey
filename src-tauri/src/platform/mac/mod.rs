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
use vnkey_engine::Engine;

use crate::settings::Settings;
use crate::state::{EngineCore, SharedConfig};

/// Initialise the macOS platform layer: load persisted settings, request
/// Accessibility permission if needed, build the shared engine state from
/// those settings, and spawn the event-tap thread.
///
/// Settings are loaded (and, on first run, persisted with defaults) before
/// the engine and tap thread are built, so the very first keystroke sees the
/// user's actual configuration rather than `EngineOptions::default()`.
pub fn start() {
    let settings = Settings::load();
    let opts = settings.to_engine_options();

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

    let mut engine = Engine::new(opts);
    engine.set_macro_table(crate::settings::load_macros());

    let mut core_state = EngineCore::new(engine);
    core_state.smart_switch = crate::settings::load_smart_switch();
    let core = Arc::new(Mutex::new(core_state));

    let config = Arc::new(SharedConfig::new(&opts));
    settings.apply_to_shared_config(&config);

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
