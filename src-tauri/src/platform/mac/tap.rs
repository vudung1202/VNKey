//! The dedicated CGEventTap thread and its C callback.
//!
//! A single background thread owns the tap: it creates a private event source
//! and an active session tap, attaches the mach port to *its own* run loop, and
//! blocks in `CFRunLoopRun`. The callback runs on this thread for every
//! observed event. This keeps event handling off Tauri's main run loop.
//!
//! Callback pipeline (port of `OpenKeyCallback`, `OpenKey.mm:601-794`):
//!   0. re-enable the tap if it was disabled (the fix over OpenKey — OpenKey
//!      never handles `kCGEventTapDisabledBy*`, so a slow callback silently
//!      kills its own tap);
//!   1. skip VNKey's own injected events (private source match), and also
//!      pass everything through untouched while the Control Panel's
//!      HotkeyRecorder is capturing a new hotkey (`hotkey_recording`, Task
//!      13);
//!   2. layout-compat keycode convert (`vPerformLayoutCompat`, KeyDown only —
//!      `hotkey.rs`/`layout.rs`, `OpenKey.mm:614-617`);
//!   3. hotkey detect: key-based switch hotkey on KeyDown, modifier-only
//!      switch hotkey + temp-off chords on FlagsChanged (`hotkey.rs`,
//!      `OpenKey.mm:618-661`);
//!   4. mouse down/drag -> start a new word, pass through;
//!   5. only KeyDown reaches the engine (KeyUp/FlagsChanged/other pass
//!      through);
//!   6. engine off (`vLanguage == 0`) -> pass through. This runs BEFORE the
//!      other-language TIS check so English mode never pays a
//!      `TISCopyCurrentKeyboardInputSource` per key (folds Task 9's
//!      callback-reorder review; OpenKey early-returns on `vLanguage == 0` at
//!      `OpenKey.mm:672/684` before the other-language block at `:693`);
//!   7. other-language passthrough (`vOtherLanguage`, `tis.rs`,
//!      `OpenKey.mm:694-710`);
//!   8. run the engine under the lock, snapshot its decision, drop the lock;
//!   9. inject (lock-free) and swallow/pass per the decision.

use core::ffi::c_void;
use core::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use objc2_core_foundation::{CFMachPort, CFRetained};
use objc2_core_graphics::{CGEvent, CGEventSource, CGEventTapProxy, CGEventType};
use parking_lot::Mutex;
use tauri::AppHandle;
use vnkey_engine::{KeyEvent, KeyEventState};

use super::hotkey::HotkeyState;
use super::inject::{self, CgSink, HookSnapshot, InjectCtx};
use super::{ffi, hotkey, layout, quirks, smart_switch, tis};
use crate::state::{EngineCore, SharedConfig};

/// Cross-thread handle to the tap's mach port, so the main-thread sleep/wake
/// observer can disable the tap while the machine sleeps and re-enable it on
/// wake (the enable-toggle lifecycle; see the module-level note below and the
/// Task 10 report). The tap thread installs the port once, after creating the
/// tap; the observer only ever calls [`set_enabled`](TapControl::set_enabled).
pub struct TapControl {
    port: Mutex<Option<SharedTapPort>>,
}

/// Wraps the tap's `CFMachPort` so it can be moved to / shared with the
/// main-thread observer.
///
/// `objc2` marks `CFMachPort` `!Send`/`!Sync` (its conservative default). The
/// wrapper is `Send`/`Sync` because the ONLY cross-thread operation performed
/// on the port is `CGEventTapEnable` (via [`ffi::set_tap_enabled`]), which
/// flips the tap's enabled bit through a mach message; it does not mutate the
/// `CFMachPort`'s user-space object, and CF retain/release (the only refcount
/// ops) are atomic. The port stays retained for the whole process by the tap
/// thread's leaked `TapContext`, so the reference can never dangle. Toggling a
/// tap from a thread other than the one servicing its run loop is an accepted
/// CoreGraphics pattern — OpenKey itself calls `CGEventTapEnable`/`stopEventTap`
/// from the main thread while the callback runs on its run loop.
struct SharedTapPort(CFRetained<CFMachPort>);

// SAFETY: see the type-level comment on `SharedTapPort`.
unsafe impl Send for SharedTapPort {}
// SAFETY: see the type-level comment on `SharedTapPort`.
unsafe impl Sync for SharedTapPort {}

impl TapControl {
    pub fn new() -> Self {
        Self {
            port: Mutex::new(None),
        }
    }

    /// Store the port after the tap thread has created it.
    fn install(&self, port: CFRetained<CFMachPort>) {
        *self.port.lock() = Some(SharedTapPort(port));
    }

    /// Enable or disable the tap from the sleep/wake observer. A no-op until
    /// the tap thread has installed the port (or if tap creation failed).
    pub fn set_enabled(&self, enabled: bool) {
        if let Some(port) = self.port.lock().as_ref() {
            ffi::set_tap_enabled(&port.0, enabled);
        }
    }
}

/// Shared, process-lifetime context handed to the C callback via a leaked
/// `*mut c_void`. Created and mutated only on the tap thread.
struct TapContext {
    app_handle: AppHandle,
    engine: Arc<Mutex<EngineCore>>,
    config: Arc<SharedConfig>,
    /// Private source, kept alive for the process; also used to recognise our
    /// own injected events.
    source: CFRetained<CGEventSource>,
    source_state_id: i64,
    /// The tap, stored after creation so the callback can re-enable it. Written
    /// once (before the run loop starts); read-only thereafter, single-threaded.
    tap: Option<CFRetained<objc2_core_foundation::CFMachPort>>,
    /// `_lastFlag`/`_hasJustUsedHotKey` — release-edge state for the
    /// language-switch hotkey and the temp-off chords. Mutated only here, on
    /// the tap thread.
    hotkey_state: HotkeyState,
}

/// Spawn the event-tap thread. Moves the shared state in; the thread lives for
/// the whole process. `tap_control` receives the tap's mach port once created,
/// so the sleep/wake observer can toggle it. `app_handle` lets the callback
/// post a tray/UI sync to the main thread after a hotkey language switch
/// (`crate::tray::notify_language_switched`) without calling AppKit itself.
pub fn spawn(
    app_handle: AppHandle,
    engine: Arc<Mutex<EngineCore>>,
    config: Arc<SharedConfig>,
    tap_control: Arc<TapControl>,
) {
    std::thread::Builder::new()
        .name("vnkey-event-tap".into())
        .spawn(move || run_tap_thread(app_handle, engine, config, tap_control))
        .expect("VNKey: failed to spawn event-tap thread");
}

fn run_tap_thread(
    app_handle: AppHandle,
    engine: Arc<Mutex<EngineCore>>,
    config: Arc<SharedConfig>,
    tap_control: Arc<TapControl>,
) {
    let Some(source) = ffi::create_private_source() else {
        eprintln!("VNKey: failed to create private event source");
        return;
    };
    let source_state_id = ffi::source_state_id(&source);

    // The context lives for the whole process. `Box::into_raw` intentionally
    // leaks it: the tap thread never stops, so there is no point at which the
    // callback could still fire after a free — no double-free is possible.
    let ctx_ptr = Box::into_raw(Box::new(TapContext {
        app_handle,
        engine,
        config,
        source,
        source_state_id,
        tap: None,
        hotkey_state: HotkeyState::new(),
    }));

    let mask = ffi::tap_event_mask();
    // SAFETY: `tap_callback` is a correct C callback; `ctx_ptr` is valid for the
    // process lifetime.
    let tap = unsafe { ffi::create_tap(mask, Some(tap_callback), ctx_ptr.cast::<c_void>()) };
    let Some(tap) = tap else {
        eprintln!(
            "VNKey: failed to create event tap. Grant Accessibility permission \
             (System Settings > Privacy & Security > Accessibility) and relaunch."
        );
        return;
    };

    // Share the port with the sleep/wake observer (a retained clone; CF
    // retain/release is atomic). The tap thread keeps its own reference in the
    // context below for the callback's timeout re-enable.
    tap_control.install(tap.clone());

    // Store the tap so the callback can re-enable it after a timeout.
    // SAFETY: single-threaded; nothing else references the context yet.
    unsafe { (*ctx_ptr).tap = Some(tap) };
    // SAFETY: `tap` field was just set; no concurrent access on this thread.
    let tap_ref = unsafe { (*ctx_ptr).tap.as_ref().unwrap() };
    ffi::enable_tap(tap_ref);
    // Blocks forever (process-lifetime tap thread).
    ffi::run_tap_loop(tap_ref);
}

/// The C tap callback. Matches `CGEventTapCallBack`'s signature exactly.
unsafe extern "C-unwind" fn tap_callback(
    proxy: CGEventTapProxy,
    ty: CGEventType,
    event: NonNull<CGEvent>,
    refcon: *mut c_void,
) -> *mut CGEvent {
    // SAFETY: `refcon` is the `TapContext` leaked in `run_tap_thread`; it is
    // alive for the whole process and only ever dereferenced on this thread.
    // `&mut`: `hotkey_state` is mutated below (single-threaded tap callback).
    let ctx = unsafe { &mut *refcon.cast::<TapContext>() };

    // 0. Re-enable a tap the system disabled (fix over OpenKey).
    if ty == CGEventType::TapDisabledByTimeout || ty == CGEventType::TapDisabledByUserInput {
        if let Some(tap) = &ctx.tap {
            ffi::enable_tap(tap);
        }
        return event.as_ptr();
    }

    // SAFETY: inside the callback the event is live.
    let event_ref = unsafe { event.as_ref() };

    // 1. Skip events VNKey itself generated.
    // SAFETY: live event.
    if unsafe { ffi::event_source_state_id(event_ref) } == ctx.source_state_id {
        return event.as_ptr();
    }

    // 1.5. The Control Panel's HotkeyRecorder is capturing input (Task 13):
    // pass every event through untouched so it sees the raw chord instead of
    // the tap swallowing/rewriting it.
    if ctx.config.hotkey_recording.load(Ordering::Relaxed) {
        return event.as_ptr();
    }

    // SAFETY: live event. Read once, up front, for every event type --
    // `OpenKeyCallback` reads `_flag`/`_keycode` unconditionally before
    // branching on type (`OpenKey.mm:609-612`).
    let flags = unsafe { ffi::event_flags(event_ref) }.0;
    // SAFETY: live event.
    let mut keycode = unsafe { ffi::event_keycode(event_ref) };

    // 2. Layout-compat keycode convert (`vPerformLayoutCompat`, KeyDown only,
    // `OpenKey.mm:614-617`).
    if ty == CGEventType::KeyDown && ctx.config.perform_layout_compat.load(Ordering::Relaxed) {
        keycode = layout::convert_event_to_layout_compat_keycode(event_ref, keycode);
    }

    // 3. Hotkey detect: key-based switch hotkey on KeyDown, modifier-only
    // switch hotkey + temp-off chords on FlagsChanged (`OpenKey.mm:618-661`).
    let switch_hotkey = ctx.config.switch_key_status.load(Ordering::Relaxed);
    if ty == CGEventType::KeyDown {
        if ctx.hotkey_state.on_key_down(switch_hotkey, keycode) {
            let mut core = ctx.engine.lock();
            hotkey::switch_language(&ctx.config, &mut core.engine, switch_hotkey);
            // Remember the user's choice for the frontmost app (OpenKey
            // `OnInputMethodChanged`).
            smart_switch::record_manual_switch(&mut core, &ctx.config);
            drop(core);
            // Fire-and-forget: post the tray/UI sync and the smart-switch
            // persist to the main thread. Must not block this callback (see
            // `notify_language_switched`'s and `persist_after_manual_switch`'s
            // doc comments).
            crate::tray::notify_language_switched(&ctx.app_handle, &ctx.config);
            smart_switch::persist_after_manual_switch(&ctx.app_handle, &ctx.engine);
            return core::ptr::null_mut();
        }
    } else if ty == CGEventType::FlagsChanged {
        let action = ctx.hotkey_state.on_flags_changed(
            switch_hotkey,
            flags,
            keycode,
            ctx.config.temp_off_spelling.load(Ordering::Relaxed),
            ctx.config.temp_off_openkey.load(Ordering::Relaxed),
        );
        if action.switch {
            let mut core = ctx.engine.lock();
            hotkey::switch_language(&ctx.config, &mut core.engine, switch_hotkey);
            smart_switch::record_manual_switch(&mut core, &ctx.config);
            drop(core);
            crate::tray::notify_language_switched(&ctx.app_handle, &ctx.config);
            smart_switch::persist_after_manual_switch(&ctx.app_handle, &ctx.engine);
            return core::ptr::null_mut();
        }
        if action.temp_off_spelling || action.temp_off_openkey {
            let mut core = ctx.engine.lock();
            if action.temp_off_spelling {
                core.engine.temp_off_spell_checking();
            }
            if action.temp_off_openkey {
                core.engine.temp_off_engine(true);
            }
        }
    }

    // 4. Mouse -> full session reset (OpenKey `RequestNewSession()`,
    // OpenKey.mm:147-154). See `EngineCore::request_new_session` for why this
    // drives a Mouse event through the engine (and ignores the `HookState`)
    // rather than calling `start_new_session()` directly. Pass the click
    // through. Not part of the decode/encode dispatch below, so the code
    // table is read fresh here rather than threaded in from step 8.
    if ty == CGEventType::LeftMouseDown
        || ty == CGEventType::RightMouseDown
        || ty == CGEventType::LeftMouseDragged
        || ty == CGEventType::RightMouseDragged
    {
        ctx.engine.lock().request_new_session(ctx.config.code_table());
        return event.as_ptr();
    }

    // 5. Only KeyDown reaches the engine.
    if ty != CGEventType::KeyDown {
        return event.as_ptr();
    }

    // 6. Engine off (`vLanguage == 0`) -> pass through. Checked BEFORE the
    // other-language TIS query so English mode never pays a
    // `TISCopyCurrentKeyboardInputSource` per key (Task 9 review fold; OpenKey
    // early-returns on `vLanguage == 0` at OpenKey.mm:672/684, ahead of the
    // other-language block at :693).
    if !ctx.config.enabled.load(Ordering::Relaxed) {
        return event.as_ptr();
    }

    // 7. Other-language passthrough (`vOtherLanguage`, `OpenKey.mm:694-710`).
    if ctx.config.other_language.load(Ordering::Relaxed) && !tis::current_input_is_english() {
        return event.as_ptr();
    }

    let caps_status = quirks::caps_status_from_flags(flags);
    let other_control = quirks::other_control_from_flags(flags);

    // 8. Engine under the lock; snapshot the decision; take `_syncKey` out so
    // injection runs lock-free (posting events must not re-enter the engine
    // while the lock is held). The code table is also snapshotted here, under
    // the same lock that guards the engine's own encode step, instead of
    // being read separately before/after -- so a concurrent smart-switch or
    // settings change (which now also updates the engine and `SharedConfig`
    // together under this same lock; see `commands.rs::apply_settings_core`)
    // can never be observed half-applied: decode (this snapshot, used by
    // `InjectCtx` below) and encode (inside `handle_event`) always agree on
    // which table produced `snap`/`sync_key` (final review fix).
    let (snap, mut sync_key, code_table) = {
        let mut core = ctx.engine.lock();
        let code_table = ctx.config.code_table();
        let hook = core.engine.handle_event(
            KeyEvent::Keyboard,
            KeyEventState::KeyDown,
            keycode,
            caps_status,
            other_control,
        );
        let snap = HookSnapshot {
            code: hook.code,
            backspace_count: hook.backspace_count,
            new_char_count: hook.new_char_count,
            ext_code: hook.ext_code,
            char_data: hook.char_data,
            macro_data: hook.macro_data.clone(),
            keycode,
            caps: caps_status == 1 || caps_status == 2,
            shift: caps_status == 1,
        };
        let sk = std::mem::take(&mut core.sync_key);
        (snap, sk, code_table)
    };

    // Read the frontmost bundle id from the cache the main-thread NSWorkspace
    // observer maintains -- the tap thread no longer calls NSWorkspace itself
    // (closes Task 8 finding 3). `InjectCtx::quirks` resolves this to
    // `AppQuirks` lazily, only when a dispatch branch that reads quirks runs.
    let bundle_id = ctx.config.frontmost_bundle_id();

    let mut sink = CgSink::new(&ctx.source, proxy);
    let mut ictx = InjectCtx {
        code_table,
        sync_key: &mut sync_key,
        bundle_id: bundle_id.as_deref(),
        quirks_cache: None,
        fix_recommend_browser: ctx.config.fix_recommend_browser.load(Ordering::Relaxed),
        fix_chromium_browser: ctx.config.fix_chromium_browser.load(Ordering::Relaxed),
        send_step_by_step: ctx.config.send_key_step_by_step.load(Ordering::Relaxed),
        spotlight_visible: false, // TODO(Task 9/10): CGWindowList Spotlight check.
    };
    let result = inject::dispatch(&mut sink, &mut ictx, &snap);

    // 9. Put `_syncKey` back; run any deferred start-new-session.
    {
        let mut core = ctx.engine.lock();
        core.sync_key = sync_key;
        if result.start_new_session {
            core.engine.start_new_session();
        }
    }

    if result.swallow {
        core::ptr::null_mut()
    } else {
        event.as_ptr()
    }
}
