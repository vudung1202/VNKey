//! The dedicated CGEventTap thread and its C callback.
//!
//! A single background thread owns the tap: it creates a private event source
//! and an active session tap, attaches the mach port to *its own* run loop, and
//! blocks in `CFRunLoopRun`. The callback runs on this thread for every
//! observed event. This keeps event handling off Tauri's main run loop.
//!
//! Callback pipeline (port of `OpenKeyCallback`, `OpenKey.mm:601-794`, reduced
//! to what this task needs):
//!   0. re-enable the tap if it was disabled (the fix over OpenKey — OpenKey
//!      never handles `kCGEventTapDisabledBy*`, so a slow callback silently
//!      kills its own tap);
//!   1. skip VNKey's own injected events (private source match);
//!   2. mouse down/drag -> start a new word, pass through;
//!   3. only KeyDown is transformed (KeyUp/FlagsChanged pass through — full
//!      hotkey/FlagsChanged handling is Task 9);
//!   4. engine off -> pass through;
//!   5. run the engine under the lock, snapshot its decision, drop the lock;
//!   6. inject (lock-free) and swallow/pass per the decision.

use core::ffi::c_void;
use core::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use objc2_core_foundation::CFRetained;
use objc2_core_graphics::{CGEvent, CGEventSource, CGEventTapProxy, CGEventType};
use parking_lot::Mutex;
use vnkey_engine::{CodeTable, KeyEvent, KeyEventState, is_double_code};

use super::inject::{self, CgSink, HookSnapshot, InjectCtx};
use super::{ffi, quirks};
use crate::state::{EngineCore, SharedConfig};

/// Shared, process-lifetime context handed to the C callback via a leaked
/// `*mut c_void`. Created and mutated only on the tap thread.
struct TapContext {
    engine: Arc<Mutex<EngineCore>>,
    config: Arc<SharedConfig>,
    /// Private source, kept alive for the process; also used to recognise our
    /// own injected events.
    source: CFRetained<CGEventSource>,
    source_state_id: i64,
    code_table: CodeTable,
    /// The tap, stored after creation so the callback can re-enable it. Written
    /// once (before the run loop starts); read-only thereafter, single-threaded.
    tap: Option<CFRetained<objc2_core_foundation::CFMachPort>>,
}

/// Spawn the event-tap thread. Moves the shared state in; the thread lives for
/// the whole process.
pub fn spawn(engine: Arc<Mutex<EngineCore>>, config: Arc<SharedConfig>, code_table: CodeTable) {
    std::thread::Builder::new()
        .name("vnkey-event-tap".into())
        .spawn(move || run_tap_thread(engine, config, code_table))
        .expect("VNKey: failed to spawn event-tap thread");
}

fn run_tap_thread(
    engine: Arc<Mutex<EngineCore>>,
    config: Arc<SharedConfig>,
    code_table: CodeTable,
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
        engine,
        config,
        source,
        source_state_id,
        code_table,
        tap: None,
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
    let ctx = unsafe { &*refcon.cast::<TapContext>() };

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

    // 2. Mouse -> start a new word, pass the click through.
    if ty == CGEventType::LeftMouseDown
        || ty == CGEventType::RightMouseDown
        || ty == CGEventType::LeftMouseDragged
        || ty == CGEventType::RightMouseDragged
    {
        let mut core = ctx.engine.lock();
        core.engine.start_new_session();
        if is_double_code(ctx.code_table) {
            core.sync_key.clear();
        }
        return event.as_ptr();
    }

    // 3. Only KeyDown is transformed.
    if ty != CGEventType::KeyDown {
        return event.as_ptr();
    }

    // 4. Engine off -> pass through.
    if !ctx.config.enabled.load(Ordering::Relaxed) {
        return event.as_ptr();
    }

    // SAFETY: live keyboard event.
    let flags = unsafe { ffi::event_flags(event_ref) }.0;
    // SAFETY: live keyboard event.
    let keycode = unsafe { ffi::event_keycode(event_ref) };
    let caps_status = quirks::caps_status_from_flags(flags);
    let other_control = quirks::other_control_from_flags(flags);

    // 5. Engine under the lock; snapshot the decision; take `_syncKey` out so
    // injection runs lock-free (posting events must not re-enter the engine
    // while the lock is held).
    let (snap, mut sync_key) = {
        let mut core = ctx.engine.lock();
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
        (snap, sk)
    };

    // Resolve per-app quirks (queries NSWorkspace on this thread).
    let quirks = quirks::resolve_quirks(ffi::frontmost_bundle_id().as_deref());

    let mut sink = CgSink::new(&ctx.source, proxy);
    let mut ictx = InjectCtx {
        code_table: ctx.code_table,
        sync_key: &mut sync_key,
        quirks,
        fix_recommend_browser: ctx.config.fix_recommend_browser.load(Ordering::Relaxed),
        fix_chromium_browser: ctx.config.fix_chromium_browser.load(Ordering::Relaxed),
        send_step_by_step: ctx.config.send_key_step_by_step.load(Ordering::Relaxed),
        spotlight_visible: false, // TODO(Task 9/10): CGWindowList Spotlight check.
    };
    let result = inject::dispatch(&mut sink, &mut ictx, &snap);

    // 6. Put `_syncKey` back; run any deferred start-new-session.
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
