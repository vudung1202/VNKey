//! macOS platform layer: CGEventTap keystroke interception + Vietnamese text
//! injection.

mod ffi;
mod inject;
mod quirks;
mod tap;

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

    tap::spawn(core, config, opts.code_table);
}
