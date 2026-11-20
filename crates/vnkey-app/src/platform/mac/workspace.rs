//! Main-thread NSWorkspace lifecycle observers.
//!
//! AppKit requires these observers to be registered on (and delivered to) the
//! main thread, so [`register`] is called from GPUI's launch callback, not the
//! tap thread. Observes the following workspace notifications:
//!
//! * `NSWorkspaceDidActivateApplication` -> cache the frontmost bundle id (for
//!   the tap thread's per-app quirks - closes Task 8 finding 3) + smart-switch;
//! * `NSWorkspaceWillSleep` / `NSWorkspaceDidWake` -> disable / enable the tap
//!   (see [`super::tap::TapControl`] and the Task 10 report for the
//!   enable-toggle vs. thread-restart decision);
//! * `NSWorkspaceActiveSpaceDidChange` -> start a new typing session.
//!
//! Each observer is a block that runs synchronously on the posting (main)
//! thread (`queue: None`); touching the engine locks the shared mutex, which is
//! safe from here.

use core::ptr::NonNull;
use std::sync::Arc;

use block2::{DynBlock, RcBlock};
use objc2_app_kit::{
    NSWorkspace, NSWorkspaceActiveSpaceDidChangeNotification,
    NSWorkspaceDidActivateApplicationNotification, NSWorkspaceDidWakeNotification,
    NSWorkspaceWillSleepNotification,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSNotificationName};
use parking_lot::Mutex;

use super::tap::TapControl;
use super::{ffi, smart_switch, tis};
use crate::app::AppSender;
use crate::state::{EngineCore, SharedConfig};

/// Register the four NSWorkspace lifecycle observers. Must run on the main
/// thread. The observer tokens are leaked (the observers live for the whole
/// process), so nothing needs to be kept by the caller. `events` lets the
/// activation observer sync the tray and windows after a smart-switch
/// language/code-table change (`smart_switch::on_active_app`).
pub fn register(
    events: AppSender,
    engine: Arc<Mutex<EngineCore>>,
    config: Arc<SharedConfig>,
    tap_control: Arc<TapControl>,
) {
    let center = NSWorkspace::sharedWorkspace().notificationCenter();
    let own_bundle = ffi::own_bundle_id();

    // App activation: cache bundle id + smart-switch.
    {
        let engine = engine.clone();
        let config = config.clone();
        let own_bundle = own_bundle.clone();
        let block = RcBlock::new(move |_notif: NonNull<NSNotification>| {
            smart_switch::on_active_app(&events, &engine, &config, own_bundle.as_deref());
        });
        // SAFETY: framework notification-name constant.
        add_observer(&center, unsafe { NSWorkspaceDidActivateApplicationNotification }, &block);
    }

    // Will sleep: pause the tap.
    {
        let tap_control = tap_control.clone();
        let block = RcBlock::new(move |_notif: NonNull<NSNotification>| {
            tap_control.set_enabled(false);
        });
        // SAFETY: framework notification-name constant.
        add_observer(&center, unsafe { NSWorkspaceWillSleepNotification }, &block);
    }

    // Did wake: resume the tap. Belt-and-suspenders with the callback's
    // `kCGEventTapDisabledBy*` auto-re-enable (Task 8).
    {
        let tap_control = tap_control.clone();
        let block = RcBlock::new(move |_notif: NonNull<NSNotification>| {
            tap_control.set_enabled(true);
        });
        // SAFETY: framework notification-name constant.
        add_observer(&center, unsafe { NSWorkspaceDidWakeNotification }, &block);
    }

    // Active space change: start a clean word (RequestNewSession).
    {
        let engine = engine.clone();
        let config = config.clone();
        let block = RcBlock::new(move |_notif: NonNull<NSNotification>| {
            engine.lock().request_new_session(config.code_table());
        });
        // SAFETY: framework notification-name constant.
        add_observer(&center, unsafe { NSWorkspaceActiveSpaceDidChangeNotification }, &block);
    }

    // Seed + observe the system input source on the main thread, so the tap
    // thread reads a cached flag instead of calling TIS off-thread (which
    // crashes on macOS 15). Runs here because `register` is already main-thread.
    tis::install_input_source_observer(&config);
}

/// Add one block-based observer for `name` and leak the returned token so the
/// observer stays registered for the whole process.
fn add_observer(
    center: &NSNotificationCenter,
    name: &NSNotificationName,
    block: &DynBlock<dyn Fn(NonNull<NSNotification>)>,
) {
    // SAFETY: `name` is a valid notification-name constant; `queue: None`
    // delivers the block synchronously on the posting (main) thread; the block
    // captures only `Send`/`Sync` shared state and outlives registration (the
    // returned token is forgotten, so the notification center keeps it alive).
    let token = unsafe {
        center.addObserverForName_object_queue_usingBlock(Some(name), None, None, block)
    };
    core::mem::forget(token);
}
