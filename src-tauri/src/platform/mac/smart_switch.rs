//! Smart-switch-key integration: apply the per-app remembered language/code
//! table on app activation, and remember the user's choice on a manual switch.
//!
//! The [`SmartSwitchTable`] itself (the `map<bundle, packed>` + read cache)
//! lives in the engine crate; this module is the macOS glue that reads the
//! current language/code-table from [`SharedConfig`], runs the active-app
//! decision against the table, and applies the result.
//!
//! One deliberate divergence from the original: it assigns the whole packed
//! value to the language field, which corrupts it when the stored code-table
//! bits are non-zero (a latent bug that never bites because most users keep
//! code table 0). VNKey's `enabled` flag is strictly 0/1, so
//! [`resolve_active_app`] restores only the packed language bit (`& 1`) and
//! restores the code table through the separate remember-code branch - the
//! evident intent.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use parking_lot::Mutex;
use tauri::{AppHandle, Manager};
use vnkey_engine::{CodeTable, SmartSwitchTable};

use super::ffi;
use crate::state::{EngineCore, SharedConfig};

/// The `language | (code_table << 1)` host packing stored per app.
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

/// Look the app up (registering it with the current status on a miss) and
/// decide what to restore. All integer arithmetic mirrors the C++ (including
/// the `-1` miss sentinel's arithmetic right shift), so the language/code-table
/// branches match the original exactly.
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

    // Language: does the stored language bit differ from the current language?
    if (stored & 0x01) != current_lang {
        if stored != -1 {
            decision.new_enabled = Some((stored & 0x01) == 1);
        } else {
            decision.registered = true;
        }
    }

    // Code table: is remember-code on and the stored code-table bits differ?
    if remember_code && (stored >> 1) != current_code {
        if stored != -1 {
            // `stored >> 1` is a valid table index for any stored value
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
/// 3), then run smart-switch if enabled. `app` is used to sync the tray/UI
/// after a language or code-table change; this handler already runs on the
/// main thread (the observer that calls it is registered there), so it calls
/// [`crate::tray::apply_tray_state`] directly rather than posting through
/// [`crate::tray::update_tray`].
pub fn on_active_app(app: &AppHandle, engine: &Mutex<EngineCore>, config: &SharedConfig, own_bundle: Option<&str>) {
    // Safe here because we are on the main thread.
    let Some(bundle_id) = ffi::frontmost_bundle_id() else {
        return;
    };
    // Skip VNKey's own bundle: don't let VNKey's own windows (settings)
    // overwrite the target app's cached language.
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
    let mut state_changed = false;
    if let Some(new_enabled) = decision.new_enabled {
        config.enabled.store(new_enabled, Ordering::Relaxed);
        core.engine.start_new_session();
        state_changed = true;
    }
    if let Some(new_code_table) = decision.new_code_table {
        config.set_code_table(new_code_table);
        core.engine.set_code_table(new_code_table);
        state_changed = true;
    }
    // A newly seen app was just registered in the table (`resolve_active_app`
    // -> `get_status`'s miss branch): snapshot it now, under the lock (cheap,
    // in-memory clone), so the disk write below runs after the lock is
    // dropped rather than making the tap thread's next keystroke wait on it.
    let table_snapshot = decision.registered.then(|| core.smart_switch.clone());
    drop(core);

    // Sync the tray + emit `status-changed` for the UI. Called at most once
    // per activation, after both possible changes above, since it reads the
    // resulting language/code-table straight off `config` rather than taking
    // the individual deltas.
    if state_changed {
        crate::tray::sync_after_live_change(app, config);
    }

    // Persist the newly registered app so it survives restart. Safe to save
    // directly here: this handler already runs on the main thread (the
    // NSWorkspace activation observer), and the lock was dropped above, so
    // this doesn't add AppKit/disk I/O to the tap thread or hold the shared
    // engine lock during the write.
    if let Some(table) = table_snapshot {
        let _ = crate::settings::save_smart_switch(&table);
    }
}

/// Remember the user's manual language switch for the frontmost app. Called on
/// the tap thread from the switch-hotkey path, with the engine already locked.
/// Reads the post-switch `enabled` value from `config`. Persistence happens
/// separately -- `tap.rs` calls [`persist_after_manual_switch`] right after
/// this (fire-and-forget), since disk I/O must never run on the tap thread.
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
}

/// Remember a language choice made from the status-bar menu or the Control
/// Panel, exactly as the switch hotkey does. Without it those choices only
/// lasted until the next app switch, which restored the old remembered value.
/// The choice goes to the app the user was in: VNKey's own windows never
/// become the cached frontmost app (see [`on_active_app`]).
pub(crate) fn remember_language_choice(app: &AppHandle) {
    let engine = app.state::<Arc<Mutex<EngineCore>>>();
    let config = app.state::<Arc<SharedConfig>>();
    record_manual_switch(&mut engine.lock(), &config);
    persist_after_manual_switch(app, &engine);
}

/// Fire-and-forget: persist the smart-switch table after a manual-switch
/// mutation made on the tap thread by [`record_manual_switch`]. Hops to the
/// main thread (`run_on_main_thread`), clones the table under the engine lock
/// there (cheap, in-memory), then writes it to disk after releasing the
/// lock -- so neither the tap-thread callback nor the shared engine lock is
/// ever held during I/O.
pub(crate) fn persist_after_manual_switch(app: &AppHandle, engine: &Arc<Mutex<EngineCore>>) {
    let app = app.clone();
    let engine = engine.clone();
    let _ = app.clone().run_on_main_thread(move || {
        let table = engine.lock().smart_switch.clone();
        let _ = crate::settings::save_smart_switch(&table);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use vnkey_engine::CodeTable::{Cp1258, Unicode};
    use vnkey_engine::{Engine, EngineOptions};

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

    /// Final review fix regression: before this fix, a [`record_manual_switch`]
    /// mutation lived only in memory and was lost on restart -- nothing ever
    /// called [`crate::settings::save_smart_switch`] afterward. This proves the
    /// mutation itself is the kind of change that must survive a save+reload:
    /// run the exact mutation `tap.rs`'s hotkey path performs, then round-trip
    /// the resulting table through the same `to_blob`/`from_blob` pair
    /// [`crate::settings::save_smart_switch`]/`load_smart_switch` use against
    /// `smart_switch.bin` (see `settings::mod.rs`'s
    /// `smart_switch_blob_round_trips_through_file`, which covers the disk
    /// half of that same pair). Together they demonstrate the persisted blob
    /// really does carry a manual-switch mutation -- the missing link is now
    /// closed by `tap.rs` calling [`persist_after_manual_switch`] right after
    /// [`record_manual_switch`] (verified by inspection: both hotkey branches
    /// in `tap.rs::tap_callback` call it immediately after `record_manual_switch`,
    /// and `on_active_app` above saves its own `registered` mutation directly).
    #[test]
    fn record_manual_switch_mutation_survives_a_save_reload_round_trip() {
        let mut core = EngineCore::new(Engine::new(EngineOptions::default()));
        let config = SharedConfig::new(&EngineOptions::default());
        config.set_frontmost_bundle_id(Some("com.apple.Safari".into()));
        config.use_smart_switch_key.store(true, Ordering::Relaxed);
        config.enabled.store(false, Ordering::Relaxed); // manually switched to English
        config.set_code_table(Cp1258);

        record_manual_switch(&mut core, &config);

        // Round-trip exactly like `save_smart_switch`/`load_smart_switch`
        // (settings::mod.rs) do against `smart_switch.bin`.
        let blob = core.smart_switch.to_blob();
        let mut reloaded = SmartSwitchTable::from_blob(&blob);
        assert_eq!(
            reloaded.get_status("com.apple.Safari", -1),
            pack_status(false, Cp1258),
            "the manual-switch mutation must survive a save+reload blob round trip"
        );
    }
}
