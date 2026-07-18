//! Audited FFI wrappers over CoreGraphics (event tap + synthetic events),
//! CoreFoundation (run loop / mach port), AppKit (frontmost app) and the
//! Accessibility trust API.
//!
//! Every function here is a thin shim so the rest of the platform layer stays
//! free of `unsafe`. All of the CoreGraphics/CoreFoundation entry points come
//! from the `objc2-core-graphics` / `objc2-core-foundation` crates (verified
//! against the pinned 0.3.2 sources). The only symbols that are NOT provided by
//! any objc2 crate are the three Accessibility ones, declared by hand at the
//! bottom of this file with a `#[link]` to the `ApplicationServices` framework.

use core::ffi::{c_ulong, c_void};

use objc2_core_foundation::{CFMachPort, CFRetained, CFRunLoop, kCFRunLoopCommonModes};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFlags, CGEventMask, CGEventSource, CGEventSourceStateID,
    CGEventTapCallBack, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventTapProxy,
    CGEventType,
};

/// macOS virtual keycode for Backspace/Delete-left.
pub const KEYCODE_BACKSPACE: u16 = 51;
/// macOS virtual keycode for the Left arrow.
pub const KEYCODE_LEFT_ARROW: u16 = 123;

/// Create the private event source used to synthesise VNKey's own events.
///
/// Using `kCGEventSourceStatePrivate` (`= -1`) lets the callback recognise and
/// skip the events VNKey itself injects (see [`event_source_state_id`]).
pub fn create_private_source() -> Option<CFRetained<CGEventSource>> {
    CGEventSource::new(CGEventSourceStateID::Private)
}

/// The state id of a source, used to tag/recognise self-generated events.
pub fn source_state_id(source: &CGEventSource) -> i64 {
    CGEventSource::source_state_id(Some(source)).0 as i64
}

/// `kCGEventSourceStateID` of an incoming event: matches our private source for
/// events VNKey generated, so the callback can pass them straight through.
///
/// # Safety
/// `event` must be a live event (it is, inside the tap callback).
pub unsafe fn event_source_state_id(event: &CGEvent) -> i64 {
    CGEvent::integer_value_field(Some(event), CGEventField::EventSourceStateID)
}

/// The virtual keycode of a keyboard event.
///
/// # Safety
/// `event` must be a live keyboard event.
pub unsafe fn event_keycode(event: &CGEvent) -> u16 {
    CGEvent::integer_value_field(Some(event), CGEventField::KeyboardEventKeycode) as u16
}

/// The modifier flags of an event.
///
/// # Safety
/// `event` must be a live event.
pub unsafe fn event_flags(event: &CGEvent) -> CGEventFlags {
    CGEvent::flags(Some(event))
}

/// Post a raw keyboard keystroke (down then up) from the private source.
///
/// Mirrors OpenKey's `SendKeyCode` / `SendBackspace` / `SendShiftAndLeftArrow`
/// event construction: force `NonCoalesced` and set/clear the Shift flag.
///
/// # Safety
/// `proxy` must be the proxy passed to the currently executing tap callback.
pub unsafe fn post_key(
    source: &CGEventSource,
    proxy: CGEventTapProxy,
    keycode: u16,
    shift: bool,
) {
    for down in [true, false] {
        let Some(ev) = CGEvent::new_keyboard_event(Some(source), keycode, down) else {
            continue;
        };
        let mut flags = CGEvent::flags(Some(&ev));
        if shift {
            flags |= CGEventFlags::MaskShift;
        } else {
            flags &= !CGEventFlags::MaskShift;
        }
        flags |= CGEventFlags::MaskNonCoalesced;
        CGEvent::set_flags(Some(&ev), flags);
        // SAFETY: `proxy` is the live callback proxy; `ev` is a valid event.
        unsafe { CGEvent::tap_post_event(proxy, Some(&ev)) };
    }
}

/// Post a UTF-16 string (down then up) as a synthetic keyboard event, mirroring
/// OpenKey's `CGEventKeyboardSetUnicodeString` usage.
///
/// # Safety
/// `proxy` must be the proxy passed to the currently executing tap callback.
pub unsafe fn post_unicode(source: &CGEventSource, proxy: CGEventTapProxy, units: &[u16]) {
    if units.is_empty() {
        return;
    }
    for down in [true, false] {
        let Some(ev) = CGEvent::new_keyboard_event(Some(source), 0, down) else {
            continue;
        };
        // SAFETY: `units` is a valid, non-empty `&[u16]`; length matches ptr.
        unsafe {
            CGEvent::keyboard_set_unicode_string(
                Some(&ev),
                units.len() as c_ulong,
                units.as_ptr(),
            );
        }
        // SAFETY: `proxy` is the live callback proxy; `ev` is a valid event.
        unsafe { CGEvent::tap_post_event(proxy, Some(&ev)) };
    }
}

/// Event mask for the events VNKey's tap observes (keyboard + mouse-down/drag,
/// matching OpenKey's `initEventTap`).
pub fn tap_event_mask() -> CGEventMask {
    let bit = |t: CGEventType| 1u64 << t.0;
    bit(CGEventType::KeyDown)
        | bit(CGEventType::KeyUp)
        | bit(CGEventType::FlagsChanged)
        | bit(CGEventType::LeftMouseDown)
        | bit(CGEventType::RightMouseDown)
        | bit(CGEventType::LeftMouseDragged)
        | bit(CGEventType::RightMouseDragged)
}

/// Create an active session event tap, head-inserted, with the given callback
/// and opaque context pointer. Returns `None` when Accessibility permission is
/// missing (the OS refuses to create the tap).
///
/// # Safety
/// `callback` must be a valid C callback and `context` a pointer that stays
/// valid for the tap's lifetime (VNKey leaks it for the whole process).
pub unsafe fn create_tap(
    mask: CGEventMask,
    callback: CGEventTapCallBack,
    context: *mut c_void,
) -> Option<CFRetained<CFMachPort>> {
    // SAFETY: forwarded per this function's contract.
    unsafe {
        CGEvent::tap_create(
            CGEventTapLocation::SessionEventTap,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default, // active tap: may modify/swallow events
            mask,
            callback,
            context,
        )
    }
}

/// (Re-)enable a tap. Called at startup and after `kCGEventTapDisabledBy*`.
pub fn enable_tap(tap: &CFMachPort) {
    CGEvent::tap_enable(tap, true);
}

/// Attach `tap`'s mach port to the current thread's run loop and block on it.
/// Returns only if the run loop stops (it never does for the process-lifetime
/// tap thread). Returns `false` if the run-loop source could not be created.
pub fn run_tap_loop(tap: &CFMachPort) -> bool {
    let Some(source) = CFMachPort::new_run_loop_source(None, Some(tap), 0) else {
        return false;
    };
    let Some(run_loop) = CFRunLoop::current() else {
        return false;
    };
    // SAFETY: reading the framework-provided run-loop mode constant.
    let mode = unsafe { kCFRunLoopCommonModes };
    run_loop.add_source(Some(&source), mode);
    CFRunLoop::run();
    true
}

/// The frontmost application's bundle identifier (OpenKey's `queryFrontMostApp`
/// / `FRONT_APP`), used for per-app injection quirks.
pub fn frontmost_bundle_id() -> Option<String> {
    use objc2_app_kit::NSWorkspace;
    let ws = NSWorkspace::sharedWorkspace();
    let app = ws.frontmostApplication()?;
    let bid = app.bundleIdentifier()?;
    Some(bid.to_string())
}

// -- Accessibility trust API ------------------------------------------------
//
// These three symbols live in the ApplicationServices umbrella framework
// (HIServices). No objc2 crate in the dependency tree exposes them, so they are
// declared by hand. The C ABI is stable; `Boolean` is `unsigned char`, mapped
// to `u8` (compared `!= 0`) to avoid any invalid-`bool` concerns.

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> u8;
    static kAXTrustedCheckOptionPrompt: *const objc2_foundation::NSString;
}

/// Whether this process currently has Accessibility (input monitoring) trust.
pub fn is_trusted() -> bool {
    // SAFETY: nullary C function, always safe to call.
    unsafe { AXIsProcessTrusted() != 0 }
}

/// Check trust and, if missing, show the system Accessibility prompt (builds
/// `{kAXTrustedCheckOptionPrompt: @YES}`). Returns the current trust state.
pub fn prompt_for_trust() -> bool {
    use objc2_foundation::{NSDictionary, NSNumber, NSString};

    // SAFETY: reading the framework's CFStringRef constant.
    let key_ptr = unsafe { kAXTrustedCheckOptionPrompt };
    if key_ptr.is_null() {
        return is_trusted();
    }
    // SAFETY: the constant is a non-null immortal CFString (toll-free bridged
    // to NSString); borrowing it for the duration of the call is sound.
    let key: &NSString = unsafe { &*key_ptr };
    let value = NSNumber::numberWithBool(true);
    let dict: objc2::rc::Retained<NSDictionary<NSString, NSNumber>> =
        NSDictionary::from_slices(&[key], &[&*value]);
    // NSDictionary is toll-free bridged to CFDictionaryRef.
    let opts = (&*dict as *const NSDictionary<NSString, NSNumber>).cast::<c_void>();
    // SAFETY: `opts` points to a valid (retained) dictionary for this call.
    unsafe { AXIsProcessTrustedWithOptions(opts) != 0 }
}
