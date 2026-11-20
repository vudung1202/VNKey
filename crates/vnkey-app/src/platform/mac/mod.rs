//! macOS platform layer: CGEventTap keystroke interception + Vietnamese text
//! injection.

pub(crate) mod autostart;
pub(crate) mod ffi;
mod inject;
pub(crate) mod hotkey;
pub(crate) mod layout;
mod quirks;
pub(crate) mod smart_switch;
mod tap;
mod tis;
mod workspace;

use std::sync::Arc;

use parking_lot::Mutex;
use vnkey_engine::Engine;

use crate::app::AppSender;
use crate::settings::Settings;
use crate::state::{EngineCore, SharedConfig};

/// The shared engine/config handles `app.rs` stores in the `Core` global, so
/// the UI and tray actions in `commands.rs` can reach them.
pub struct PlatformHandles {
    pub engine: Arc<Mutex<EngineCore>>,
    pub config: Arc<SharedConfig>,
}

/// Initialise the macOS platform layer: load persisted settings, request
/// Accessibility permission if needed, build the shared engine state from
/// those settings, and spawn the event-tap thread.
///
/// Settings are loaded (and, on first run, persisted with defaults) before
/// the engine and tap thread are built, so the very first keystroke sees the
/// user's actual configuration rather than `EngineOptions::default()`.
///
/// `events` is threaded into the NSWorkspace observers and the tap thread so
/// they can post tray/UI sync back to the main thread (`crate::app`) after a
/// smart-switch or hotkey language change, without either of those paths
/// touching AppKit or GPUI state themselves.
pub fn start(events: AppSender) -> PlatformHandles {
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
    // requirement). This runs inside GPUI's launch callback, on the main run loop.
    workspace::register(events.clone(), core.clone(), config.clone(), tap_control.clone());

    tap::spawn(events, core.clone(), config.clone(), tap_control);

    PlatformHandles { engine: core, config }
}
