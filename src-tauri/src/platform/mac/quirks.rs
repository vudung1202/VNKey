//! Per-app injection quirks and `CGEventFlags` decoding.
//!
//! All pure functions — unit-tested with no tap and no Accessibility grant.

use objc2_core_graphics::CGEventFlags;

/// Apps that need `0x200C` (ZERO WIDTH NON-JOINER) as the "empty" character
/// instead of `0x202F`.
const NICE_SPACE_APPS: &[&str] = &["com.sublimetext.3", "com.sublimetext.2"];

/// Apps that misbehave with Unicode Compound and get browser-style handling.
/// `com.apple.` matches every Apple app by prefix.
const UNICODE_COMPOUND_APPS: &[&str] = &[
    "com.apple.",
    "com.google.Chrome",
    "com.brave.Browser",
    "com.microsoft.edgemac.Dev",
    "com.microsoft.edgemac.Beta",
    "com.microsoft.Edge.Dev",
    "com.microsoft.Edge",
];

/// Apps where the "insert empty char" autocomplete workaround is disabled and
/// replaced by Shift+Left selection.
const RECOMMEND_DISABLED_APPS: &[&str] = &["com.apple.Spotlight"];

/// Resolved quirk flags for the current frontmost app.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AppQuirks {
    /// Use `0x200C` instead of `0x202F` for the empty character.
    pub nice_space: bool,
    /// Prefix match. Gates the double-backspace suppression for Unicode
    /// Compound.
    pub unicode_compound_app_prefix: bool,
    /// Exact match. Gates the (deferred) chromium Shift+Left branch.
    pub unicode_compound_app_exact: bool,
    /// App is in the recommend-workaround-disabled list.
    pub recommend_disabled: bool,
}

/// Resolve quirks for a bundle id. `None` (unknown app) yields all-false,
/// treating an unknown app as an ordinary one.
pub fn resolve_quirks(bundle_id: Option<&str>) -> AppQuirks {
    match bundle_id {
        None => AppQuirks::default(),
        Some(id) => AppQuirks {
            nice_space: NICE_SPACE_APPS.contains(&id),
            // starts_with already covers the exact-equality case too.
            unicode_compound_app_prefix: UNICODE_COMPOUND_APPS.iter().any(|p| id.starts_with(p)),
            unicode_compound_app_exact: UNICODE_COMPOUND_APPS.contains(&id),
            recommend_disabled: RECOMMEND_DISABLED_APPS.contains(&id),
        },
    }
}

/// `caps_status` for `Engine::handle_event`: 1 = Shift held,
/// 2 = Caps Lock (AlphaShift) on, 0 = neither. Takes the raw `CGEventFlags`
/// bits so it is testable without constructing a `CGEventFlags`.
pub fn caps_status_from_flags(flags: u64) -> u8 {
    if flags & CGEventFlags::MaskShift.0 != 0 {
        1
    } else if flags & CGEventFlags::MaskAlphaShift.0 != 0 {
        2
    } else {
        0
    }
}

/// `other_control` for `Engine::handle_event`: any of Command / Control /
/// Option / Fn / NumericPad / Help.
pub fn other_control_from_flags(flags: u64) -> bool {
    const MASK: u64 = CGEventFlags::MaskCommand.0
        | CGEventFlags::MaskControl.0
        | CGEventFlags::MaskAlternate.0
        | CGEventFlags::MaskSecondaryFn.0
        | CGEventFlags::MaskNumericPad.0
        | CGEventFlags::MaskHelp.0;
    flags & MASK != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    // Flag bit values (from objc2-core-graphics CGEventTypes.rs, matching the
    // Carbon `kCGEventFlagMask*` constants).
    const ALPHA_SHIFT: u64 = 0x10000; // Caps Lock
    const SHIFT: u64 = 0x20000;
    const CONTROL: u64 = 0x40000;
    const ALTERNATE: u64 = 0x80000; // Option
    const COMMAND: u64 = 0x100000;
    const NUMERIC_PAD: u64 = 0x200000;
    const HELP: u64 = 0x400000;
    const SECONDARY_FN: u64 = 0x800000;

    #[test]
    fn caps_status_shift_takes_priority_over_capslock() {
        assert_eq!(caps_status_from_flags(SHIFT), 1);
        assert_eq!(caps_status_from_flags(ALPHA_SHIFT), 2);
        assert_eq!(caps_status_from_flags(SHIFT | ALPHA_SHIFT), 1);
        assert_eq!(caps_status_from_flags(0), 0);
        // Non-caps modifiers do not set caps status.
        assert_eq!(caps_status_from_flags(COMMAND), 0);
    }

    #[test]
    fn other_control_detects_each_modifier() {
        for m in [COMMAND, CONTROL, ALTERNATE, SECONDARY_FN, NUMERIC_PAD, HELP] {
            assert!(other_control_from_flags(m), "flag {m:#x} should count");
        }
        assert!(!other_control_from_flags(0));
        // Shift / Caps Lock alone are not "other control" keys.
        assert!(!other_control_from_flags(SHIFT));
        assert!(!other_control_from_flags(ALPHA_SHIFT));
    }

    #[test]
    fn nice_space_apps_matched_exactly() {
        assert!(resolve_quirks(Some("com.sublimetext.3")).nice_space);
        assert!(resolve_quirks(Some("com.sublimetext.2")).nice_space);
        assert!(!resolve_quirks(Some("com.sublimetext.4")).nice_space);
        assert!(!resolve_quirks(Some("com.apple.TextEdit")).nice_space);
    }

    #[test]
    fn unicode_compound_prefix_matches_all_apple_apps() {
        let q = resolve_quirks(Some("com.apple.Safari"));
        assert!(q.unicode_compound_app_prefix);
        // Prefix match, but NOT an exact list entry.
        assert!(!q.unicode_compound_app_exact);

        let chrome = resolve_quirks(Some("com.google.Chrome"));
        assert!(chrome.unicode_compound_app_prefix);
        assert!(chrome.unicode_compound_app_exact);

        let other = resolve_quirks(Some("org.mozilla.firefox"));
        assert!(!other.unicode_compound_app_prefix);
        assert!(!other.unicode_compound_app_exact);
    }

    #[test]
    fn spotlight_is_recommend_disabled() {
        assert!(resolve_quirks(Some("com.apple.Spotlight")).recommend_disabled);
        // Spotlight is also an Apple app by prefix.
        assert!(resolve_quirks(Some("com.apple.Spotlight")).unicode_compound_app_prefix);
        assert!(!resolve_quirks(Some("com.apple.TextEdit")).recommend_disabled);
    }

    #[test]
    fn unknown_app_is_all_false() {
        assert_eq!(resolve_quirks(None), AppQuirks::default());
    }

    #[test]
    fn ordinary_app_has_no_quirks() {
        let q = resolve_quirks(Some("com.sublimetext.4"));
        assert!(!q.nice_space);
        assert!(!q.unicode_compound_app_prefix);
        assert!(!q.recommend_disabled);
    }
}
