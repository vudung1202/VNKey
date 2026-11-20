//! The actions the windows and the status-bar menu run (the Tauri IPC
//! commands before the UI moved to GPUI).
//!
//! Every action that changes configuration funnels through
//! [`apply_and_broadcast`] (`App`-facing) / [`apply_settings_core`] (pure,
//! unit-testable): lock the engine, apply the new [`EngineOptions`], apply the
//! new [`Settings`] to [`SharedConfig`], persist, then update the tray and
//! redraw the windows. No action hand-rolls that sequence itself.
//!
//! [`EngineOptions`]: vnkey_engine::EngineOptions

use std::path::Path;
use std::sync::atomic::Ordering;

use gpui_kit::App;
use parking_lot::Mutex;

use crate::app::Core;
use crate::platform::mac::smart_switch;
use crate::settings::Settings;
use crate::state::{EngineCore, SharedConfig};

/// Apply `settings` to the engine + shared config and persist it. Pure aside
/// from the injected `save` (so tests can swap in a mock), which is the
/// entire reason this is split out from [`apply_and_broadcast`]: the tray
/// update and window redraw need a live `App`, but the state-mutation itself
/// doesn't.
///
/// The engine-options update and the `SharedConfig` update run under the
/// *same* engine-lock critical section (final review fix): previously they
/// were two separate statements, so the engine's `code_table` could be
/// updated while `SharedConfig::code_table` (the atomic `tap.rs`'s injector
/// reads) still held the old value. A keystroke handled on the tap thread in
/// that window would encode with the engine's new table but decode/split
/// with `SharedConfig`'s old one, desyncing the `_syncKey` stack. Locking
/// across both writes means any tap-thread dispatch -- which now also reads
/// `SharedConfig::code_table` inside its own engine-lock hold (`tap.rs`) --
/// can never observe a torn state between the two.
pub(crate) fn apply_settings_core(
    engine: &Mutex<EngineCore>,
    config: &SharedConfig,
    settings: Settings,
    save: impl FnOnce(&Settings) -> std::io::Result<()>,
) -> Settings {
    {
        let mut core = engine.lock();
        core.engine.set_options(settings.to_engine_options());
        settings.apply_to_shared_config(config);
    }
    let _ = save(&settings);
    settings
}

/// The state-mutation portion of [`apply_and_broadcast`], pulled out so it's
/// testable without an `App` -- same injected-`save` shape as
/// [`apply_settings_core`], plus an injected `on_disk` base instead of a
/// hardcoded `Settings::load()`.
///
/// `on_disk` (the last-saved settings) is overlaid with the live
/// hotkey/smart-switch state in `config`
/// ([`crate::tray::overlay_live_config`]) *before* `mutate` runs. This is
/// the Task 12 review fix: the hotkey (`hotkey::switch_language` /
/// `tray::sync_after_live_change`) and smart-switch
/// (`smart_switch::on_active_app`) paths flip only `SharedConfig`'s
/// `enabled`/`code_table` atomics without saving, so `on_disk` can be stale.
/// Seeding `mutate` from the overlay instead of `on_disk` directly means an
/// unrelated menu/UI change can never clobber a live change that hasn't
/// been persisted yet -- and persisting the result (via `save`) also catches
/// `settings.json` up to the live state as a side effect.
pub(crate) fn apply_and_broadcast_core(
    engine: &Mutex<EngineCore>,
    config: &SharedConfig,
    on_disk: Settings,
    mutate: impl FnOnce(Settings) -> Settings,
    save: impl FnOnce(&Settings) -> std::io::Result<()>,
) -> Settings {
    let seed = crate::tray::overlay_live_config(on_disk, config);
    let settings = mutate(seed);
    apply_settings_core(engine, config, settings, save)
}

/// The one path every config-changing action goes through: load the current
/// settings, let `mutate` produce the new ones, apply + persist them, then
/// update the tray and redraw the windows.
pub(crate) fn apply_and_broadcast(cx: &mut App, mutate: impl FnOnce(Settings) -> Settings) -> Settings {
    let core = cx.global::<Core>();
    let before = Settings::load();
    let settings = apply_and_broadcast_core(&core.engine, &core.config, before, mutate, |s| s.save());

    // Only on an actual change: switching the activation policy is a heavier,
    // focus-affecting AppKit transition than the tray/window updates below,
    // and this funnel runs for every setting, not just this one.
    if settings.show_icon_on_dock != before.show_icon_on_dock {
        crate::app::set_dock_icon(settings.show_icon_on_dock, cx);
    }

    crate::app::broadcast(settings, cx);
    settings
}

pub(crate) fn set_language_settings(mut s: Settings, language: u8) -> Settings {
    s.language = language;
    s
}

pub(crate) fn set_input_type_settings(mut s: Settings, input_type: u8) -> Settings {
    s.input_type = input_type;
    s
}

pub(crate) fn set_code_table_settings(mut s: Settings, code_table: u8) -> Settings {
    s.code_table = code_table;
    s
}

pub(crate) fn set_switch_hotkey_settings(mut s: Settings, hotkey: u32) -> Settings {
    s.switch_key_status = hotkey;
    s
}

pub(crate) fn set_convert_hotkey_settings(mut s: Settings, hotkey: u32) -> Settings {
    s.convert_hotkey = hotkey;
    s
}

/// Switch Vietnamese/English and remember the choice for the frontmost app,
/// exactly as the switch hotkey does.
pub(crate) fn set_language(cx: &mut App, language: u8) {
    apply_and_broadcast(cx, |s| set_language_settings(s, language));
    let core = cx.global::<Core>();
    smart_switch::remember_language_choice(&core.engine, &core.config);
}

/// Sorted by key, so a table of a few hundred entries can be scanned.
pub(crate) fn macro_list(cx: &App) -> Vec<(String, String)> {
    let mut list = cx.global::<Core>().engine.lock().engine.macro_list();
    list.sort();
    list
}

pub(crate) fn macro_add(cx: &mut App, text: &str, content: &str) {
    edit_macros(cx, |core| {
        core.engine.macro_add(text, content);
    });
}

pub(crate) fn macro_delete(cx: &mut App, text: &str) {
    edit_macros(cx, |core| {
        core.engine.macro_delete(text);
    });
}

/// Import a UniKey-format macro file, appending to or replacing the table.
/// Returns how many entries were imported.
pub(crate) fn macro_import(cx: &mut App, path: &Path, append: bool) -> Result<usize, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut imported = 0;
    edit_macros(cx, |core| {
        let code_table = core.engine.macro_table().code_table();
        imported = core.engine.macro_table_mut().import_unikey_txt(&content, append, code_table);
    });
    Ok(imported)
}

pub(crate) fn macro_export(cx: &App, path: &Path) -> Result<(), String> {
    let text = cx.global::<Core>().engine.lock().engine.macro_table().export_unikey_txt();
    std::fs::write(path, text).map_err(|e| e.to_string())
}

/// Mutate the macro table under the engine lock, persist it, then tell the
/// windows to reload it.
fn edit_macros(cx: &mut App, edit: impl FnOnce(&mut EngineCore)) {
    {
        let mut core = cx.global::<Core>().engine.lock();
        edit(&mut core);
        let _ = crate::settings::save_macros(core.engine.macro_table());
    }
    crate::app::macros_changed(cx);
}

/// Convert the clipboard with the Convert tab's options. Returns whether
/// there was anything to convert, so the UI can tell "done" from "clipboard
/// empty". Runs on the main thread, as NSPasteboard requires.
pub(crate) fn convert_clipboard(settings: &Settings) -> bool {
    crate::pasteboard::quick_convert(&crate::pasteboard::SystemClipboard, &settings.convert_options())
}

pub(crate) fn open_accessibility_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

/// While a hotkey field is focused the CGEventTap passes every key through
/// untouched (`tap.rs` step 1.5), so it sees the raw chord being recorded.
/// Transient focus state, not a persisted setting, so it skips
/// [`apply_and_broadcast`].
pub(crate) fn set_hotkey_recording(cx: &App, recording: bool) {
    cx.global::<Core>().config.hotkey_recording.store(recording, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use vnkey_engine::{CodeTable, Engine, EngineOptions, InputType};

    use super::*;

    /// Task 12 review regression: the hotkey path (`hotkey::switch_language`)
    /// flips `SharedConfig::enabled` without saving, so `settings.json` stays
    /// stale. Before the fix, `apply_and_broadcast_core` seeded `mutate`
    /// straight from `on_disk` -- an unrelated change (here, setting the
    /// input type) would resurrect the stale `language` and
    /// `apply_settings_core`'s `apply_to_shared_config` would clobber the
    /// live `enabled = false` back to `true`, silently reverting the user
    /// back to Vietnamese. Fails on the pre-fix code; passes after seeding
    /// from `tray::overlay_live_config(on_disk, config)` instead.
    #[test]
    fn apply_and_broadcast_core_preserves_live_hotkey_language_over_stale_on_disk_settings() {
        let stale_on_disk = Settings::default();
        assert_eq!(stale_on_disk.language, 1, "precondition: on-disk settings still say Vietnamese");

        let engine = Mutex::new(EngineCore::new(Engine::new(stale_on_disk.to_engine_options())));
        let config = SharedConfig::new(&stale_on_disk.to_engine_options());

        // Hotkey path: flips only the atomic, never persists.
        config.enabled.store(false, std::sync::atomic::Ordering::Relaxed); // now English

        // An UNRELATED menu/IPC change.
        let saved = RefCell::new(None);
        let result = apply_and_broadcast_core(
            &engine,
            &config,
            stale_on_disk,
            |s| set_input_type_settings(s, InputType::SimpleTelex2 as u8),
            |s| {
                *saved.borrow_mut() = Some(*s);
                Ok(())
            },
        );

        assert_eq!(result.language, 0, "the unrelated change must not revert the live English state");
        assert!(
            !config.enabled.load(std::sync::atomic::Ordering::Relaxed),
            "SharedConfig.enabled must stay false (English), not clobbered back to Vietnamese"
        );
        assert_eq!(
            saved.into_inner().unwrap().language,
            0,
            "the persisted settings.json must also catch up to the live English state"
        );
    }

    /// Same clobber, smart-switch flavor: `smart_switch::on_active_app`'s
    /// remember-code branch flips only `SharedConfig::code_table` without
    /// saving. Fails on the pre-fix code for the same reason as the hotkey
    /// test above.
    #[test]
    fn apply_and_broadcast_core_preserves_live_smart_switch_code_table_over_stale_on_disk_settings() {
        let stale_on_disk = Settings::default();
        assert_eq!(stale_on_disk.code_table, CodeTable::Unicode as u8, "precondition: on-disk settings still say Unicode");

        let engine = Mutex::new(EngineCore::new(Engine::new(stale_on_disk.to_engine_options())));
        let config = SharedConfig::new(&stale_on_disk.to_engine_options());

        // Smart-switch path: flips only the atomic, never persists.
        config.set_code_table(CodeTable::VniWindows);

        // An UNRELATED menu/IPC change.
        let result = apply_and_broadcast_core(&engine, &config, stale_on_disk, |s| set_language_settings(s, 0), |_| Ok(()));

        assert_eq!(
            result.code_table,
            CodeTable::VniWindows as u8,
            "the unrelated change must not revert the live code table"
        );
        assert_eq!(
            config.code_table(),
            CodeTable::VniWindows,
            "SharedConfig.code_table must stay VNI Windows, not clobbered back to Unicode"
        );
    }

    /// Final review fix regression: `apply_settings_core` used to update the
    /// engine's code table and `SharedConfig::code_table` as two separate
    /// statements (the engine lock from the first was dropped before the
    /// second ran), so a tap-thread keystroke landing in between could read a
    /// torn state. There's no public accessor for `Engine.opts.code_table`
    /// directly (it's `pub(crate)` inside `vnkey-engine`), but
    /// `MacroTable::code_table()` tracks it exactly -- `Engine::set_options`
    /// re-syncs the macro table's code whenever `opts.code_table` changes
    /// (`engine.rs::set_options`) -- so it's usable here as an observable
    /// proxy for "what the engine's encode step will use". This confirms
    /// both ends land on the new table after one `apply_settings_core` call.
    #[test]
    fn apply_settings_core_syncs_engine_and_shared_config_code_table() {
        let engine = Mutex::new(EngineCore::new(Engine::new(EngineOptions::default())));
        let config = SharedConfig::new(&EngineOptions::default());
        assert_eq!(config.code_table(), CodeTable::Unicode, "precondition: default table");

        let settings = Settings { code_table: CodeTable::VniWindows as u8, ..Settings::default() };
        apply_settings_core(&engine, &config, settings, |_| Ok(()));

        assert_eq!(config.code_table(), CodeTable::VniWindows);
        assert_eq!(
            engine.lock().engine.macro_table().code_table(),
            CodeTable::VniWindows,
            "the engine's own code table (proxied via the macro table it keeps in sync) \
             must agree with SharedConfig's after a single apply_settings_core call"
        );
    }

    #[test]
    fn apply_settings_core_updates_engine_shared_config_and_saves() {
        let engine = Mutex::new(EngineCore::new(Engine::new(EngineOptions::default())));
        let config = SharedConfig::new(&EngineOptions::default());
        assert!(config.enabled.load(std::sync::atomic::Ordering::Relaxed));

        let saved = RefCell::new(None);
        let settings = Settings { language: 0, ..Settings::default() };
        let result = apply_settings_core(&engine, &config, settings, |s| {
            *saved.borrow_mut() = Some(*s);
            Ok(())
        });

        assert_eq!(result, settings);
        assert!(!config.enabled.load(std::sync::atomic::Ordering::Relaxed));
        assert_eq!(saved.into_inner(), Some(settings));
    }

    #[test]
    fn set_language_settings_only_changes_language() {
        let s = Settings::default();
        let updated = set_language_settings(s, 0);
        assert_eq!(updated.language, 0);
        assert_eq!(updated, Settings { language: 0, ..s });
    }

    #[test]
    fn set_input_type_settings_only_changes_input_type() {
        let s = Settings::default();
        let updated = set_input_type_settings(s, 1);
        assert_eq!(updated, Settings { input_type: 1, ..s });
    }

    #[test]
    fn set_code_table_settings_only_changes_code_table() {
        let s = Settings::default();
        let updated = set_code_table_settings(s, 2);
        assert_eq!(updated, Settings { code_table: 2, ..s });
    }

    #[test]
    fn set_switch_hotkey_settings_only_changes_switch_key_status() {
        let s = Settings::default();
        let updated = set_switch_hotkey_settings(s, 0x1234);
        assert_eq!(updated, Settings { switch_key_status: 0x1234, ..s });
    }

    #[test]
    fn set_convert_hotkey_settings_only_changes_convert_hotkey() {
        let s = Settings::default();
        let updated = set_convert_hotkey_settings(s, 0x5678);
        assert_eq!(updated, Settings { convert_hotkey: 0x5678, ..s });
    }

    #[test]
    fn macro_add_delete_and_list_round_trip_through_a_real_engine() {
        let mut engine = Engine::new(EngineOptions::default());
        assert!(engine.macro_list().is_empty());

        engine.macro_add("vk", "Việt Nam");
        let listed = engine.macro_list();
        assert_eq!(listed, vec![("vk".to_string(), "Việt Nam".to_string())]);

        engine.macro_delete("vk");
        assert!(engine.macro_list().is_empty());
    }
}
