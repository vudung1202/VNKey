//! Language-switch hotkey detection, the `FlagsChanged` release-edge state
//! machine, and the two temp-off chords.
//!
//! Ports `checkHotKey` (`OpenKey.mm:515-531`), `switchLanguage`
//! (`OpenKey.mm:533-542`), and the hotkey/`FlagsChanged` block inside
//! `OpenKeyCallback` (`OpenKey.mm:614-661`), restricted to `vSwitchKeyStatus`.
//! `OpenKeyCallback` also drives a second, independent hotkey
//! (`convertToolHotKey`, "Quick Convert" -> `[appDelegate onQuickConvert]`)
//! through the exact same `checkHotKey`/`_lastFlag` machinery, interleaved
//! branch-by-branch with the switch-hotkey checks. That feature has no
//! `AppDelegate.onQuickConvert` port and is out of the brief's scope, so its
//! branches are omitted here (not an oversight -- see the Task 9 report).
//!
//! `_lastFlag`/`_hasJustUsedHotKey` are `OpenKeyCallback`'s persisted instance
//! fields (read and written on every callback invocation, from the KeyDown
//! and `FlagsChanged` branches alike). [`HotkeyState`] is their Rust home,
//! owned by the tap thread's `TapContext` and mutated only there -- the tap
//! callback runs on a single dedicated thread, never concurrently with
//! itself, so plain fields (no atomics) are correct.
//!
//! `checkHotKey`'s C++ body reads `_lastFlag` (never the freshly-read
//! `_flag`) in every modifier comparison, at both call sites -- including the
//! KeyDown branch, where `_lastFlag` was last written by a preceding
//! `FlagsChanged` press edge. [`check_hotkey`] mirrors this exactly: its
//! `current_flags` parameter is always fed `HotkeyState::last_flag`, never a
//! raw per-event `CGEventFlags` read.

use vnkey_engine::Engine;

use crate::state::SharedConfig;

/// `EMPTY_HOTKEY` (`OpenKey.mm:22`): the "no hotkey configured" sentinel.
const EMPTY_HOTKEY: u32 = 0xFE0000FE;
/// `GET_SWITCH_KEY(data) == 0xFE` (`Engine.h`/`OpenKey.mm` convention): the
/// packed keycode byte meaning "no physical key -- modifiers only".
const MODIFIER_ONLY_KEYCODE: u16 = 0xFE;
/// `DEFAULT_SWITCH_STATUS` (`AppDelegate.m:35`): Option+Z, no beep.
pub const DEFAULT_SWITCH_KEY_STATUS: u32 = 0x7A00_0206;

/// `GET_SWITCH_KEY(data)` (`Engine.h:30`).
fn get_switch_key(data: u32) -> u16 {
    (data & 0xFF) as u16
}
/// `HAS_CONTROL(data)` (`Engine.h:31`).
fn has_control(data: u32) -> bool {
    data & 0x100 != 0
}
/// `HAS_OPTION(data)` (`Engine.h:32`).
fn has_option(data: u32) -> bool {
    data & 0x200 != 0
}
/// `HAS_COMMAND(data)` (`Engine.h:33`).
fn has_command(data: u32) -> bool {
    data & 0x400 != 0
}
/// `HAS_SHIFT(data)` (`Engine.h:34`).
fn has_shift(data: u32) -> bool {
    data & 0x800 != 0
}
/// `HAS_BEEP(data)` (`Engine.h:36`).
fn has_beep(data: u32) -> bool {
    data & 0x8000 != 0
}

// `kCGEventFlagMask*` bit values (Carbon), duplicated from `quirks.rs`'s test
// constants so this module has no dependency on `objc2_core_graphics` types
// in its pure core -- callers pass plain `u64` flags.
const MASK_SHIFT: u64 = 0x20000;
const MASK_CONTROL: u64 = 0x40000;
const MASK_ALTERNATE: u64 = 0x80000; // Option
const MASK_COMMAND: u64 = 0x100000;

/// `checkHotKey` (`OpenKey.mm:515-531`). XOR-compares each modifier bit of
/// `hotkey_data` against `current_flags`; when `check_keycode` is set, also
/// requires `keycode` to match the hotkey's packed key byte.
pub fn check_hotkey(hotkey_data: u32, current_flags: u64, check_keycode: bool, keycode: u16) -> bool {
    if (hotkey_data & !0x8000) == EMPTY_HOTKEY {
        return false;
    }
    if has_control(hotkey_data) != (current_flags & MASK_CONTROL != 0) {
        return false;
    }
    if has_option(hotkey_data) != (current_flags & MASK_ALTERNATE != 0) {
        return false;
    }
    if has_command(hotkey_data) != (current_flags & MASK_COMMAND != 0) {
        return false;
    }
    if has_shift(hotkey_data) != (current_flags & MASK_SHIFT != 0) {
        return false;
    }
    if check_keycode && get_switch_key(hotkey_data) != keycode {
        return false;
    }
    true
}

/// What [`HotkeyState::on_flags_changed`] decided the caller must do. Kept as
/// plain data (not driven from inside this module) so `hotkey.rs` never needs
/// the engine lock or `SharedConfig` -- easy to unit test with plain values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlagsChangedAction {
    /// The switch hotkey matched on this release edge -- caller must call
    /// [`switch_language`] and swallow the event.
    pub switch: bool,
    /// `vTempOffSpelling` is on and Control was among the modifiers just
    /// released -- caller must call `engine.temp_off_spell_checking()`.
    pub temp_off_spelling: bool,
    /// `vTempOffOpenKey` is on and Command was among the modifiers just
    /// released -- caller must call `engine.temp_off_engine(true)`.
    pub temp_off_openkey: bool,
}

/// `_lastFlag`/`_hasJustUsedHotKey` (`OpenKeyCallback` instance fields):
/// release-edge state for the hotkey/chord machine. One instance lives for
/// the whole tap thread.
#[derive(Debug, Default)]
pub struct HotkeyState {
    last_flag: u64,
    has_just_used_hotkey: bool,
}

impl HotkeyState {
    pub fn new() -> Self {
        Self::default()
    }

    /// KeyDown branch of the hotkey block (`OpenKey.mm:614-631`),
    /// `vSwitchKeyStatus`-only. Returns `true` when the hotkey fired --
    /// caller must call [`switch_language`] and swallow the event (`return
    /// NULL` in the original).
    pub fn on_key_down(&mut self, switch_hotkey: u32, keycode: u16) -> bool {
        let switch_key = get_switch_key(switch_hotkey);
        if switch_key != keycode {
            self.last_flag = 0;
        } else if check_hotkey(
            switch_hotkey,
            self.last_flag,
            switch_key != MODIFIER_ONLY_KEYCODE,
            keycode,
        ) {
            self.last_flag = 0;
            self.has_just_used_hotkey = true;
            return true;
        }
        self.has_just_used_hotkey = self.last_flag != 0;
        false
    }

    /// `FlagsChanged` branch (`OpenKey.mm:634-661`), `vSwitchKeyStatus` +
    /// temp-off-chord subset. `flags` is the event's current
    /// `CGEventGetFlags`; `keycode` is its `kCGKeyboardEventKeycode` (the
    /// specific modifier key that changed -- e.g. 59 for Left Control).
    pub fn on_flags_changed(
        &mut self,
        switch_hotkey: u32,
        flags: u64,
        keycode: u16,
        temp_off_spelling_enabled: bool,
        temp_off_openkey_enabled: bool,
    ) -> FlagsChangedAction {
        let mut action = FlagsChangedAction::default();
        if self.last_flag == 0 || self.last_flag < flags {
            // Press edge (more modifiers held than before): record the peak.
            self.last_flag = flags;
        } else if self.last_flag > flags {
            // Release edge: a modifier was just let go. Check against the
            // peak (`self.last_flag`, pre-release), not the post-release
            // `flags` -- matches `checkHotKey` reading `_lastFlag`.
            let check_keycode = get_switch_key(switch_hotkey) != MODIFIER_ONLY_KEYCODE;
            if check_hotkey(switch_hotkey, self.last_flag, check_keycode, keycode) {
                self.last_flag = 0;
                self.has_just_used_hotkey = true;
                action.switch = true;
                return action;
            }
            if temp_off_spelling_enabled && !self.has_just_used_hotkey && self.last_flag & MASK_CONTROL != 0 {
                action.temp_off_spelling = true;
            }
            if temp_off_openkey_enabled && !self.has_just_used_hotkey && self.last_flag & MASK_COMMAND != 0 {
                action.temp_off_openkey = true;
            }
            self.last_flag = 0;
            self.has_just_used_hotkey = false;
        }
        // `self.last_flag == flags` (neither branch): OpenKey has no `else`
        // here either -- state is left untouched.
        action
    }
}

/// `switchLanguage` (`OpenKey.mm:533-542`): flip `vLanguage`, beep if the
/// hotkey's beep bit is set, start a new engine session. Tray/UI sync
/// (`[appDelegate onImputMethodChanged:YES]`) is the caller's job -- `tap.rs`
/// calls `crate::tray::notify_language_switched` right after this, which
/// reads the same `enabled` atomic this flips. Must not block: only touches
/// the atomic and the engine lock the caller already holds, no IPC.
pub fn switch_language(config: &SharedConfig, engine: &mut Engine, switch_hotkey: u32) {
    use std::sync::atomic::Ordering;

    let new_enabled = !config.enabled.load(Ordering::Relaxed);
    config.enabled.store(new_enabled, Ordering::Relaxed);
    if has_beep(switch_hotkey) {
        // SAFETY: `NSBeep` is a safe extern fn (no args, no return value).
        objc2_app_kit::NSBeep();
    }
    engine.start_new_session();
    eprintln!(
        "VNKey: switched to {}",
        if new_enabled { "Vietnamese" } else { "English" }
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTROL: u64 = MASK_CONTROL;
    const OPTION: u64 = MASK_ALTERNATE;
    const COMMAND: u64 = MASK_COMMAND;
    const SHIFT: u64 = MASK_SHIFT;

    /// Default ⌥Z (`AppDelegate.m:35`): Option + keycode 6 ('z'), no beep.
    const OPT_Z: u32 = DEFAULT_SWITCH_KEY_STATUS;
    /// A modifier-only Ctrl+Shift hotkey (arbitrary, for the FlagsChanged
    /// release-edge tests): keycode byte = 0xFE, Control + Shift bits set.
    const CTRL_SHIFT_MODIFIER_ONLY: u32 = 0xFE | 0x100 | 0x800;
    /// Same combo, with the beep bit also set.
    const CTRL_SHIFT_MODIFIER_ONLY_BEEP: u32 = CTRL_SHIFT_MODIFIER_ONLY | 0x8000;

    // -- packed hotkey decode --------------------------------------------

    #[test]
    fn packed_decode_default_opt_z() {
        assert_eq!(get_switch_key(OPT_Z), 6);
        assert!(!has_control(OPT_Z));
        assert!(has_option(OPT_Z));
        assert!(!has_command(OPT_Z));
        assert!(!has_shift(OPT_Z));
        assert!(!has_beep(OPT_Z));
    }

    #[test]
    fn packed_decode_modifier_only_ctrl_shift() {
        assert_eq!(get_switch_key(CTRL_SHIFT_MODIFIER_ONLY), MODIFIER_ONLY_KEYCODE);
        assert!(has_control(CTRL_SHIFT_MODIFIER_ONLY));
        assert!(!has_option(CTRL_SHIFT_MODIFIER_ONLY));
        assert!(!has_command(CTRL_SHIFT_MODIFIER_ONLY));
        assert!(has_shift(CTRL_SHIFT_MODIFIER_ONLY));
        assert!(!has_beep(CTRL_SHIFT_MODIFIER_ONLY));
        assert!(has_beep(CTRL_SHIFT_MODIFIER_ONLY_BEEP));
    }

    #[test]
    fn empty_hotkey_sentinel_decodes_to_modifier_only_no_bits() {
        assert_eq!(get_switch_key(EMPTY_HOTKEY), MODIFIER_ONLY_KEYCODE);
        assert!(!has_control(EMPTY_HOTKEY));
        assert!(!has_option(EMPTY_HOTKEY));
        assert!(!has_command(EMPTY_HOTKEY));
        assert!(!has_shift(EMPTY_HOTKEY));
    }

    // -- check_hotkey vector table: OPT_Z ---------------------------------

    #[test]
    fn check_hotkey_opt_z_matches_only_option_plus_keycode_6() {
        assert!(check_hotkey(OPT_Z, OPTION, true, 6));
        assert!(!check_hotkey(OPT_Z, OPTION, true, 7)); // wrong key
        assert!(!check_hotkey(OPT_Z, 0, true, 6)); // no Option held
        assert!(!check_hotkey(OPT_Z, OPTION | CONTROL, true, 6)); // extra modifier
        assert!(!check_hotkey(OPT_Z, OPTION | SHIFT, true, 6));
        assert!(!check_hotkey(OPT_Z, CONTROL, true, 6)); // wrong modifier entirely
        // check_keycode = false ignores the keycode requirement.
        assert!(check_hotkey(OPT_Z, OPTION, false, 999));
    }

    #[test]
    fn check_hotkey_empty_hotkey_never_matches() {
        assert!(!check_hotkey(EMPTY_HOTKEY, 0, true, MODIFIER_ONLY_KEYCODE));
        assert!(!check_hotkey(EMPTY_HOTKEY | 0x8000, 0, true, MODIFIER_ONLY_KEYCODE));
    }

    // -- check_hotkey vector table: modifier-only Ctrl+Shift --------------

    #[test]
    fn check_hotkey_ctrl_shift_modifier_only() {
        assert!(check_hotkey(CTRL_SHIFT_MODIFIER_ONLY, CONTROL | SHIFT, false, 0));
        assert!(!check_hotkey(CTRL_SHIFT_MODIFIER_ONLY, CONTROL, false, 0)); // missing Shift
        assert!(!check_hotkey(CTRL_SHIFT_MODIFIER_ONLY, SHIFT, false, 0)); // missing Control
        assert!(!check_hotkey(CTRL_SHIFT_MODIFIER_ONLY, CONTROL | SHIFT | OPTION, false, 0)); // extra
        assert!(!check_hotkey(CTRL_SHIFT_MODIFIER_ONLY, 0, false, 0));
    }

    // -- on_key_down: key-based hotkey (OPT_Z) -----------------------------

    #[test]
    fn key_down_fires_when_modifier_held_via_last_flag() {
        let mut st = HotkeyState::new();
        // A prior FlagsChanged press edge would have set last_flag = OPTION.
        st.last_flag = OPTION;
        assert!(st.on_key_down(OPT_Z, 6));
        // Fired -> state resets.
        assert_eq!(st.last_flag, 0);
        assert!(st.has_just_used_hotkey);
    }

    #[test]
    fn key_down_does_not_fire_without_option_held() {
        let mut st = HotkeyState::new();
        assert!(!st.on_key_down(OPT_Z, 6)); // last_flag == 0, no Option
        assert!(!st.has_just_used_hotkey);
    }

    #[test]
    fn key_down_of_unrelated_key_resets_last_flag() {
        let mut st = HotkeyState::new();
        st.last_flag = CONTROL | SHIFT; // some chord was pending
        assert!(!st.on_key_down(OPT_Z, 99)); // unrelated keycode
        assert_eq!(st.last_flag, 0);
        assert!(!st.has_just_used_hotkey);
    }

    #[test]
    fn key_down_matching_keycode_but_wrong_modifiers_preserves_last_flag() {
        // keycode matches the hotkey's key byte (6, 'z') but Option isn't
        // held -- OpenKey does NOT reset _lastFlag in this branch (only the
        // "keycode doesn't match at all" branch resets it).
        let mut st = HotkeyState::new();
        st.last_flag = CONTROL;
        assert!(!st.on_key_down(OPT_Z, 6));
        assert_eq!(st.last_flag, CONTROL);
        assert!(st.has_just_used_hotkey); // last_flag != 0
    }

    // -- on_flags_changed: release-edge detection for modifier-only chord --

    #[test]
    fn modifier_only_switch_fires_exactly_once_on_release() {
        let mut st = HotkeyState::new();
        // Press Control (keycode 59), then Shift (keycode 56): last_flag climbs.
        let a1 = st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, CONTROL, 59, false, false);
        assert_eq!(a1, FlagsChangedAction::default());
        assert_eq!(st.last_flag, CONTROL);

        let a2 = st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, CONTROL | SHIFT, 56, false, false);
        assert_eq!(a2, FlagsChangedAction::default());
        assert_eq!(st.last_flag, CONTROL | SHIFT);

        // Release Control first (Shift still down): flags drops to SHIFT only
        // -- release edge, checked against the pre-release last_flag (both
        // held) -> fires.
        let a3 = st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, SHIFT, 59, false, false);
        assert!(a3.switch);
        assert_eq!(st.last_flag, 0);
        assert!(st.has_just_used_hotkey);

        // Release Shift too: last_flag is already 0, so this is treated as a
        // fresh press edge (0 == flags is impossible here since flags==0;
        // 0==0 -> press-edge branch, no re-fire). No double-fire.
        let a4 = st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, 0, 56, false, false);
        assert_eq!(a4, FlagsChangedAction::default());
        assert!(!a4.switch);
    }

    #[test]
    fn modifier_only_switch_does_not_fire_when_normal_key_intervenes() {
        let mut st = HotkeyState::new();
        st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, CONTROL, 59, false, false);
        st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, CONTROL | SHIFT, 56, false, false);
        assert_eq!(st.last_flag, CONTROL | SHIFT);

        // A normal key (not the modifier-only hotkey's key portion -- always
        // true here since the key byte is 0xFE) cancels the pending chord.
        assert!(!st.on_key_down(OPT_Z, 0 /* some ordinary keycode */));
        assert_eq!(st.last_flag, 0);

        // Releasing Control now: last_flag was zeroed, so this reads as a
        // fresh press edge (last_flag(0) == 0), not a release -- no fire.
        let a = st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, 0, 59, false, false);
        assert!(!a.switch);
    }

    #[test]
    fn key_based_hotkey_never_fires_from_flags_changed() {
        // GET_SWITCH_KEY(OPT_Z) == 6, which never equals a modifier's own
        // keycode (58 == physical Option key) -- checkHotKey's checkKeyCode
        // branch naturally rejects it.
        let mut st = HotkeyState::new();
        st.on_flags_changed(OPT_Z, OPTION, 58, false, false);
        let a = st.on_flags_changed(OPT_Z, 0, 58, false, false);
        assert!(!a.switch);
    }

    // -- temp-off chords ----------------------------------------------------

    #[test]
    fn ctrl_release_fires_temp_off_spelling_when_enabled() {
        let mut st = HotkeyState::new();
        st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, CONTROL, 59, true, false);
        // Release Control alone (Shift never pressed): last_flag(CONTROL) > flags(0).
        let a = st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, 0, 59, true, false);
        assert!(!a.switch); // Shift bit required by the hotkey isn't held
        assert!(a.temp_off_spelling);
        assert!(!a.temp_off_openkey);
    }

    #[test]
    fn cmd_release_fires_temp_off_openkey_when_enabled() {
        let mut st = HotkeyState::new();
        st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, COMMAND, 55, false, true);
        let a = st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, 0, 55, false, true);
        assert!(!a.switch);
        assert!(!a.temp_off_spelling);
        assert!(a.temp_off_openkey);
    }

    #[test]
    fn chord_does_not_fire_when_disabled() {
        let mut st = HotkeyState::new();
        st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, CONTROL, 59, false, false);
        let a = st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, 0, 59, false, false);
        assert_eq!(a, FlagsChangedAction::default());
    }

    #[test]
    fn chord_and_switch_never_fire_together_in_the_same_release() {
        // Releasing Control while Shift is still held matches the switch
        // hotkey exactly -> the switch branch returns early, so the Control
        // chord (also satisfiable by the same last_flag) never runs.
        let mut st = HotkeyState::new();
        st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, CONTROL, 59, true, false);
        st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, CONTROL | SHIFT, 56, true, false);
        let a = st.on_flags_changed(CTRL_SHIFT_MODIFIER_ONLY, SHIFT, 59, true, false);
        assert!(a.switch);
        assert!(!a.temp_off_spelling);
    }

    #[test]
    fn has_just_used_hotkey_suppresses_immediate_chord_after_switch() {
        // After a switch fires on one release edge, has_just_used_hotkey is
        // true; a *different* HotkeyState instance models "guard active"
        // directly (has_just_used_hotkey gates the chord checks).
        let mut st = HotkeyState::new();
        st.has_just_used_hotkey = true;
        st.last_flag = CONTROL;
        let a = st.on_flags_changed(EMPTY_HOTKEY, 0, 59, true, true);
        assert!(!a.temp_off_spelling);
        assert!(!a.temp_off_openkey);
    }

    // -- switch_language: flag flip -----------------------------------------

    #[test]
    fn has_beep_reads_the_packed_bit() {
        assert!(!has_beep(OPT_Z));
        assert!(has_beep(OPT_Z | 0x8000));
    }

    #[test]
    fn switch_language_flips_enabled_and_starts_new_session() {
        use std::sync::atomic::Ordering;
        use vnkey_engine::{Engine, EngineOptions};

        let opts = EngineOptions::default(); // language == 1 (Vietnamese)
        let config = SharedConfig::new(&opts);
        let mut engine = Engine::new(opts);
        assert!(config.enabled.load(Ordering::Relaxed));

        // OPT_Z has no beep bit -- avoids sounding a real system beep here.
        switch_language(&config, &mut engine, OPT_Z);
        assert!(!config.enabled.load(Ordering::Relaxed));

        switch_language(&config, &mut engine, OPT_Z);
        assert!(config.enabled.load(Ordering::Relaxed));
    }
}
