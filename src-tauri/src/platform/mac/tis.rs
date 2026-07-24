//! System keyboard input-source check: detect whether the
//! current macOS keyboard input source is English, so the tap callback can
//! yield (passthrough, no engine) when the user has switched to some other
//! system input source (Japanese, Korean, ...).
//!
//! Carbon's Text Input Source Services has no `objc2` binding in this
//! dependency tree, so the three symbols are declared by hand, matching the
//! established pattern in `ffi.rs`'s Accessibility bindings.
//!
//! # CoreFoundation memory-management fix
//! `TISCopyCurrentKeyboardInputSource` is a "Copy" function (owned, +1
//! retain -- caller must `CFRelease`). `TISGetInputSourceProperty` and
//! `CFArrayGetValueAtIndex` are "Get" accessors (unowned/borrowed -- must
//! NOT be released). So: release `isource` exactly once, whenever non-null,
//! on every path (including the early non-English return); never release
//! `languages`/`lang_ref`, which are unowned. Getting this wrong leaks
//! `isource` or over-releases an unowned array element -- Rust cannot tolerate
//! either the way Objective-C's reference counting silently absorbs it.

use core::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use objc2_foundation::NSString;

use crate::state::SharedConfig;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn TISCopyCurrentKeyboardInputSource() -> *const c_void;
    fn TISGetInputSourceProperty(input_source: *const c_void, property_key: *const c_void) -> *const c_void;
    static kTISPropertyInputSourceLanguages: *const c_void;
    /// Distributed-notification name posted when the selected keyboard input
    /// source changes.
    static kTISNotifySelectedKeyboardInputSourceChanged: *const c_void;
}

/// `CFNotificationCallback`: fired by CoreFoundation on the run loop of the
/// thread that registered the observer (the main thread here).
type CfNotificationCallback =
    extern "C" fn(*const c_void, *mut c_void, *const c_void, *const c_void, *const c_void);

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFArrayGetCount(array: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(array: *const c_void, idx: isize) -> *const c_void;
    fn CFRelease(cf: *const c_void);
    fn CFNotificationCenterGetDistributedCenter() -> *const c_void;
    fn CFNotificationCenterAddObserver(
        center: *const c_void,
        observer: *const c_void,
        callback: CfNotificationCallback,
        name: *const c_void,
        object: *const c_void,
        suspension_behavior: isize,
    );
}

/// The original language check compared the input source's language against
/// `"en"` with an `isLike:` selector that has no discoverable definition
/// anywhere in the reference source tree (grepped the whole checkout -- only
/// the one call site exists, no category/extension declares it). Interpreted
/// here as a BCP-47
/// primary-subtag prefix match (`"en"`, `"en-US"`, `"en-GB"`, ... all match;
/// `"ja"`, `"vi"`, ... do not), which is the practical intent of comparing a
/// TIS input-source language code against the literal `"en"`.
pub fn is_english_language_code(lang: &str) -> bool {
    lang.starts_with("en")
}

/// `true` when the current macOS keyboard input source's first declared
/// language is English, or when the check is inconclusive (no current input
/// source, no language property, empty language list) -- in all of those
/// cases the engine keeps typing Vietnamese, never forcing passthrough on an
/// inconclusive read.
pub fn current_input_is_english() -> bool {
    // SAFETY: nullary Carbon call, always safe to invoke.
    let isource = unsafe { TISCopyCurrentKeyboardInputSource() };
    if isource.is_null() {
        return true;
    }
    // SAFETY: `isource` is non-null (just checked); `kTISPropertyInputSourceLanguages`
    // is a live, immortal CFStringRef constant provided by the framework.
    let languages = unsafe { TISGetInputSourceProperty(isource, kTISPropertyInputSourceLanguages) };
    let result = if languages.is_null() {
        true
    } else {
        // SAFETY: `languages` is non-null and, being a property of `isource`
        // (still alive, not yet released), valid for this call.
        let count = unsafe { CFArrayGetCount(languages) };
        if count <= 0 {
            true
        } else {
            // SAFETY: `count > 0`, so index 0 is in bounds.
            let lang_ref = unsafe { CFArrayGetValueAtIndex(languages, 0) };
            if lang_ref.is_null() {
                true
            } else {
                // Toll-free bridging: CFStringRef and NSString* share layout
                // (same pattern as `ffi.rs`'s `kAXTrustedCheckOptionPrompt`).
                // SAFETY: `lang_ref` is a live, non-null CFStringRef, an
                // unowned array element borrowed only for this call (never
                // released here -- the array owns it).
                let s: &NSString = unsafe { &*lang_ref.cast::<NSString>() };
                is_english_language_code(&s.to_string())
            }
        }
    };
    // SAFETY: `isource` is non-null and owned by this call (`Copy`
    // semantics) -- released exactly once, on every path, after every read
    // of the unowned `languages`/`lang_ref` derived from it.
    unsafe { CFRelease(isource) };
    result
}

/// Distributed-notification callback: refreshes the cached English flag on the
/// main thread (where the TIS query is safe). Registered by
/// [`install_input_source_observer`].
extern "C" fn on_input_source_changed(
    _center: *const c_void,
    observer: *mut c_void,
    _name: *const c_void,
    _object: *const c_void,
    _user_info: *const c_void,
) {
    if observer.is_null() {
        return;
    }
    // SAFETY: `observer` is the `*const SharedConfig` leaked in
    // `install_input_source_observer`; it lives for the whole process.
    let config = unsafe { &*(observer as *const SharedConfig) };
    config.input_is_english.store(current_input_is_english(), Ordering::Relaxed);
}

/// Seed [`SharedConfig::input_is_english`] and keep it fresh: read the current
/// input source once, then observe the distributed input-source-change
/// notification and re-read on each change. MUST run on the main thread -- both
/// the initial read and every callback query TIS, which crashes off the main
/// thread on macOS 15 (`dispatch_assert_queue` in TSM). The tap thread then
/// only ever reads the cached atomic, never TIS.
pub fn install_input_source_observer(config: &Arc<SharedConfig>) {
    config.input_is_english.store(current_input_is_english(), Ordering::Relaxed);

    // Leak one Arc ref as the observer pointer: the observer is never removed
    // (it lives for the whole process, like the NSWorkspace observers).
    let observer = Arc::into_raw(config.clone()) as *const c_void;
    // SAFETY: distributed center handle + framework notification-name constant;
    // the callback and the leaked observer pointer both outlive registration.
    unsafe {
        CFNotificationCenterAddObserver(
            CFNotificationCenterGetDistributedCenter(),
            observer,
            on_input_source_changed,
            kTISNotifySelectedKeyboardInputSourceChanged,
            core::ptr::null(),
            4, // kCFNotificationSuspensionBehaviorDeliverImmediately
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_and_english_region_variants_match() {
        assert!(is_english_language_code("en"));
        assert!(is_english_language_code("en-US"));
        assert!(is_english_language_code("en-GB"));
        assert!(is_english_language_code("en_US"));
    }

    #[test]
    fn other_languages_do_not_match() {
        assert!(!is_english_language_code("ja"));
        assert!(!is_english_language_code("vi"));
        assert!(!is_english_language_code("zh-Hans"));
        assert!(!is_english_language_code(""));
    }
}
