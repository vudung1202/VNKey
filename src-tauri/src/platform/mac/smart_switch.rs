//! Smart-switch-key integration: apply the per-app remembered language/code
//! table on app activation, and remember the user's choice on a manual switch.
//!
//! The [`SmartSwitchTable`] itself (the `map<bundle, packed>` + read cache)
//! lives in the engine crate; this module is the macOS glue that reads the
//! current language/code-table from [`SharedConfig`], runs OpenKey's
//! `OnActiveAppChanged` decision against the table, and applies the result.
//!
//! Ports `OnActiveAppChanged` (`OpenKey.mm:207-226`) and `OnInputMethodChanged`
//! (`OpenKey.mm:237-242`). One deliberate divergence: OpenKey assigns the whole
//! packed value to `vLanguage` (`vLanguage = _languageTemp`), which corrupts
//! `vLanguage` when the stored code-table bits are non-zero (a latent OpenKey
//! bug that never bites because most users keep code table 0). VNKey's `enabled`
//! flag is strictly 0/1, so [`resolve_active_app`] restores only the packed
//! language bit (`_languageTemp & 1`) and restores the code table through the
//! separate remember-code branch — the evident intent.

use std::sync::atomic::Ordering;

use parking_lot::Mutex;
use vnkey_engine::{CodeTable, SmartSwitchTable};

use super::ffi;
use crate::state::{EngineCore, SharedConfig};

/// The `vLanguage | (vCodeTable << 1)` host packing OpenKey stores per app.
fn pack_status(enabled: bool, code_table: CodeTable) -> i32 {
    enabled as i32 | ((code_table as i32) << 1)
}

/// What activating an app should change, decided purely against the table so it
/// can be unit-tested without NSWorkspace.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SmartSwitchDecision {
    /// `Some(enabled)` if the stored language differs from the current one.
    pub new_enabled: Option<bool>,
    /// `Some(table)` if `remember_code` is on and the stored code table differs.
    pub new_code_table: Option<CodeTable>,
    /// The app was unknown and has just been registered with the current
    /// status — the on-disk blob should be re-saved (Task 11 persistence).
    pub registered: bool,
}

/// `OnActiveAppChanged` (`OpenKey.mm:207-226`): look the app up (registering it
/// with the current status on a miss) and decide what to restore. All integer
/// arithmetic mirrors the C++ (including the `-1` miss sentinel's arithmetic
/// right shift), so the language/code-table branches match OpenKey exactly.
pub fn resolve_active_app(
    table: &mut SmartSwitchTable,
    bundle_id: &str,
    current_enabled: bool,
    current_code_table: CodeTable,
    remember_code: bool,
) -> SmartSwitchDecision {
    let current_lang = current_enabled as i32; // 0 or 1
    let current_code = current_code_table as i32;
    let current_packed = current_lang | (current_code << 1);

    // `getAppInputMethodStatus`: stored packed value, or -1 on a miss (which
    // also registers `current_packed` for next time).
    let stored = table.get_status(bundle_id, current_packed);

    let mut decision = SmartSwitchDecision::default();

    // Language: `if ((_languageTemp & 0x01) != vLanguage)`.
    if (stored & 0x01) != current_lang {
        if stored != -1 {
            decision.new_enabled = Some((stored & 0x01) == 1);
        } else {
            decision.registered = true;
        }
    }

    // Code table: `if (vRememberCode && (_languageTemp >> 1) != vCodeTable)`.
    if remember_code && (stored >> 1) != current_code {
        if stored != -1 {
            // `_languageTemp >> 1` is a valid table index for any stored value
            // (max packed is `1 | (4 << 1) = 9`, so the index is 0..=4 and the
            // packed byte is never negative); a corrupt out-of-range index is
            // ignored rather than applied.
            if let Some(code_table) = CodeTable::from_index((stored >> 1) as u8) {
                decision.new_code_table = Some(code_table);
            }
        } else {
            decision.registered = true;
        }
    }

    decision
}

/// Main-thread activation handler (`NSWorkspaceDidActivateApplication`): cache
/// the frontmost bundle id for the tap thread's per-app quirks (Task 8 finding
/// 3), then run smart-switch if enabled.
pub fn on_active_app(engine: &Mutex<EngineCore>, config: &SharedConfig, own_bundle: Option<&str>) {
    // `queryFrontMostApp` — safe here because we are on the main thread.
    let Some(bundle_id) = ffi::frontmost_bundle_id() else {
        return;
    };
    // OpenKey's `!= OPENKEY_BUNDLE` guard: don't let VNKey's own windows
    // (settings) overwrite the target app's cached language.
    if own_bundle == Some(bundle_id.as_str()) {
        return;
    }
    // Cache for the tap thread (closes finding 3: it never calls NSWorkspace).
    config.set_frontmost_bundle_id(Some(bundle_id.clone()));

    if !config.use_smart_switch_key.load(Ordering::Relaxed) {
        return;
    }

    let current_enabled = config.enabled.load(Ordering::Relaxed);
    let current_code_table = config.code_table();
    let remember_code = config.remember_code.load(Ordering::Relaxed);

    let mut core = engine.lock();
    let decision = resolve_active_app(
        &mut core.smart_switch,
        &bundle_id,
        current_enabled,
        current_code_table,
        remember_code,
    );
    if let Some(new_enabled) = decision.new_enabled {
        config.enabled.store(new_enabled, Ordering::Relaxed);
        core.engine.start_new_session();
        // TODO(Task 12): notify the tray/UI that the language changed.
    }
    if let Some(new_code_table) = decision.new_code_table {
        config.set_code_table(new_code_table);
        core.engine.set_code_table(new_code_table);
        // TODO(Task 12): notify the tray/UI that the code table changed.
    }
    drop(core);

    // TODO(Task 11): if `decision.registered`, persist the smart-switch blob
    // (`EngineCore::smart_switch.to_blob()` -> NSUserDefaults). The table is
    // in-memory only this task, so a newly seen app's registration is lost on
    // restart.
    let _ = decision.registered;
}

/// Remember the user's manual language switch for the frontmost app
/// (`OnInputMethodChanged`, `OpenKey.mm:237-242`). Called on the tap thread
/// from the switch-hotkey path, with the engine already locked. Reads the
/// post-switch `enabled` value from `config`.
pub fn record_manual_switch(core: &mut EngineCore, config: &SharedConfig) {
    if !config.use_smart_switch_key.load(Ordering::Relaxed) {
        return;
    }
    // Uses the bundle id cached by the activation observer; `None` only before
    // the first activation (VNKey primes it at startup).
    let Some(bundle_id) = config.frontmost_bundle_id() else {
        return;
    };
    let packed = pack_status(config.enabled.load(Ordering::Relaxed), config.code_table());
    core.smart_switch.set_status(&bundle_id, packed);
    // TODO(Task 11): persist the smart-switch blob after this change.
}

#[cfg(test)]
mod tests {
    use super::*;
    use vnkey_engine::CodeTable::{Cp1258, Unicode};

    #[test]
    fn packed_value_is_never_negative_even_for_code_table_4() {
        // `language | (codeTable << 1)` maxes out at `1 | (4 << 1) = 9`, which
        // is a positive `i8`; only the -1 miss sentinel is negative.
        let packed = pack_status(true, Cp1258);
        assert_eq!(packed, 9);
        assert!(i8::try_from(packed).unwrap() >= 0);
    }

    #[test]
    fn first_activation_registers_current_and_changes_nothing() {
        let mut t = SmartSwitchTable::new();
        // Typing Vietnamese (Unicode) when a brand-new app activates.
        let d = resolve_active_app(&mut t, "com.a", true, Unicode, true);
        assert_eq!(d.new_enabled, None);
        assert_eq!(d.new_code_table, None);
        assert!(d.registered);
        // Re-activating the same app now matches -> still nothing to change.
        let d2 = resolve_active_app(&mut t, "com.a", true, Unicode, true);
        assert_eq!(d2, SmartSwitchDecision::default());
    }

    #[test]
    fn stored_language_differs_switches_language() {
        let mut t = SmartSwitchTable::new();
        t.set_status("com.a", pack_status(false, Unicode)); // remembered English
        // Currently Vietnamese -> activating restores English.
        let d = resolve_active_app(&mut t, "com.a", true, Unicode, true);
        assert_eq!(d.new_enabled, Some(false));
        assert_eq!(d.new_code_table, None);
        assert!(!d.registered);
    }

    #[test]
    fn remember_code_restores_stored_code_table() {
        let mut t = SmartSwitchTable::new();
        t.set_status("com.a", pack_status(false, Cp1258)); // English + CP1258
        let d = resolve_active_app(&mut t, "com.a", true, Unicode, true);
        assert_eq!(d.new_enabled, Some(false));
        assert_eq!(d.new_code_table, Some(Cp1258));
    }

    #[test]
    fn remember_code_off_ignores_code_table() {
        let mut t = SmartSwitchTable::new();
        t.set_status("com.a", pack_status(false, Cp1258));
        let d = resolve_active_app(&mut t, "com.a", true, Unicode, false);
        assert_eq!(d.new_enabled, Some(false));
        assert_eq!(d.new_code_table, None); // code-table branch skipped
    }

    #[test]
    fn packed_val_decodes_language_and_code_table() {
        let mut t = SmartSwitchTable::new();
        t.set_status("com.a", 9); // 1 | (4 << 1): Vietnamese + CP1258
        // Currently English + Unicode.
        let d = resolve_active_app(&mut t, "com.a", false, Unicode, true);
        assert_eq!(d.new_enabled, Some(true));
        assert_eq!(d.new_code_table, Some(Cp1258));
    }

    /// The brief's headline scenario: type VN in app A, switch B to EN, return
    /// to A -> A is remembered as VN.
    #[test]
    fn per_app_memory_round_trip_a_vn_b_en_back_to_a() {
        let mut t = SmartSwitchTable::new();
        // Activate A while typing Vietnamese -> registered as VN.
        resolve_active_app(&mut t, "com.a", true, Unicode, true);
        // Activate B, still Vietnamese -> registered as VN.
        resolve_active_app(&mut t, "com.b", true, Unicode, true);
        // In B, the user manually switches to English (record path).
        t.set_status("com.b", pack_status(false, Unicode));
        // Return to A while currently English (carried over from B).
        let d = resolve_active_app(&mut t, "com.a", false, Unicode, true);
        assert_eq!(d.new_enabled, Some(true), "A should restore Vietnamese");
    }

    #[test]
    fn miss_while_english_current_registers() {
        // Current English, new app: (-1 & 1)=1 != 0 -> language branch, miss.
        let mut t = SmartSwitchTable::new();
        let d = resolve_active_app(&mut t, "com.a", false, Unicode, false);
        assert!(d.registered);
        assert_eq!(d.new_enabled, None);
    }
}
