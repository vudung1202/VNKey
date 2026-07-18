//! `#[tauri::command]` IPC surface for the React UI.
//!
//! Every command that changes configuration funnels through
//! [`apply_and_broadcast`] (Tauri-facing) / [`apply_settings_core`] (pure,
//! unit-testable): lock the engine, apply the new [`EngineOptions`], apply the
//! new [`Settings`] to [`SharedConfig`], persist, then update the tray and
//! emit events. No command hand-rolls that sequence itself.

use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use vnkey_engine::CodeTable;

use crate::settings::Settings;
use crate::state::{EngineCore, SharedConfig};

/// The `convert_clipboard` IPC parameter -- mirrors
/// `vnkey_engine::ConvertOptions` field-for-field, but with `u8` code-table
/// discriminants (the wire-friendly form `Settings` also uses) instead of the
/// engine's `CodeTable` enum.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ConvertOptions {
    pub from_code: u8,
    pub to_code: u8,
    pub remove_mark: bool,
    pub to_all_caps: bool,
    pub to_all_non_caps: bool,
    pub to_caps_first_letter: bool,
    pub to_caps_each_word: bool,
}

impl ConvertOptions {
    fn to_engine(self) -> vnkey_engine::ConvertOptions {
        vnkey_engine::ConvertOptions {
            from_code: CodeTable::from_index(self.from_code).unwrap_or(CodeTable::Unicode),
            to_code: CodeTable::from_index(self.to_code).unwrap_or(CodeTable::Unicode),
            remove_mark: self.remove_mark,
            to_all_caps: self.to_all_caps,
            to_all_non_caps: self.to_all_non_caps,
            to_caps_first_letter: self.to_caps_first_letter,
            to_caps_each_word: self.to_caps_each_word,
        }
    }
}

/// One entry of the macro table, as sent to/from the UI.
#[derive(Debug, Clone, Serialize)]
pub struct MacroEntry {
    pub text: String,
    pub content: String,
}

/// `check_update`'s result. No release feed exists yet (no GitHub URL is
/// known for this project) -- the command is a documented stub, not wired to
/// anything, until that's decided.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
}

/// Apply `settings` to the engine + shared config and persist it. Pure aside
/// from the injected `save` (so tests can swap in a mock), which is the
/// entire reason this is split out from [`apply_and_broadcast`]: the tray
/// update and event emission need a live `AppHandle`, but the state-mutation
/// itself doesn't.
pub(crate) fn apply_settings_core(
    engine: &Mutex<EngineCore>,
    config: &SharedConfig,
    settings: Settings,
    save: impl FnOnce(&Settings) -> std::io::Result<()>,
) -> Settings {
    engine.lock().engine.set_options(settings.to_engine_options());
    settings.apply_to_shared_config(config);
    let _ = save(&settings);
    settings
}

/// The one path every config-changing command goes through: load the current
/// settings, let `mutate` produce the new ones, apply + persist them, then
/// update the tray and emit `status-changed`/`settings-changed` for the UI.
pub(crate) fn apply_and_broadcast(app: &AppHandle, mutate: impl FnOnce(Settings) -> Settings) -> Settings {
    let engine = app.state::<Arc<Mutex<EngineCore>>>();
    let config = app.state::<Arc<SharedConfig>>();

    let settings = mutate(Settings::load());
    let settings = apply_settings_core(&engine, &config, settings, |s| s.save());

    crate::tray::update_tray(app, settings);
    crate::events::status_changed(app, &settings);
    crate::events::settings_changed(app, &settings);

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

#[tauri::command]
pub fn get_settings() -> Settings {
    Settings::load()
}

#[tauri::command]
pub fn set_settings(app: AppHandle, settings: Settings) {
    apply_and_broadcast(&app, |_| settings);
}

#[tauri::command]
pub fn set_language(app: AppHandle, language: u8) {
    apply_and_broadcast(&app, |s| set_language_settings(s, language));
}

#[tauri::command]
pub fn set_input_type(app: AppHandle, input_type: u8) {
    apply_and_broadcast(&app, |s| set_input_type_settings(s, input_type));
}

#[tauri::command]
pub fn set_code_table(app: AppHandle, code_table: u8) {
    apply_and_broadcast(&app, |s| set_code_table_settings(s, code_table));
}

#[tauri::command]
pub fn set_switch_hotkey(app: AppHandle, hotkey: u32) {
    apply_and_broadcast(&app, |s| set_switch_hotkey_settings(s, hotkey));
}

#[tauri::command]
pub fn set_convert_hotkey(app: AppHandle, hotkey: u32) {
    apply_and_broadcast(&app, |s| set_convert_hotkey_settings(s, hotkey));
}

#[tauri::command]
pub fn restore_default_settings(app: AppHandle) -> Settings {
    apply_and_broadcast(&app, |_| Settings::default())
}

#[tauri::command]
pub fn macro_list(app: AppHandle) -> Vec<MacroEntry> {
    let engine = app.state::<Arc<Mutex<EngineCore>>>();
    engine
        .lock()
        .engine
        .macro_list()
        .into_iter()
        .map(|(text, content)| MacroEntry { text, content })
        .collect()
}

#[tauri::command]
pub fn macro_add(app: AppHandle, text: String, content: String) {
    let engine = app.state::<Arc<Mutex<EngineCore>>>();
    let mut core = engine.lock();
    core.engine.macro_add(&text, &content);
    let _ = crate::settings::save_macros(core.engine.macro_table());
    drop(core);
    crate::events::macros_changed(&app);
}

#[tauri::command]
pub fn macro_delete(app: AppHandle, text: String) {
    let engine = app.state::<Arc<Mutex<EngineCore>>>();
    let mut core = engine.lock();
    core.engine.macro_delete(&text);
    let _ = crate::settings::save_macros(core.engine.macro_table());
    drop(core);
    crate::events::macros_changed(&app);
}

#[tauri::command]
pub fn macro_import(app: AppHandle, path: String, append: bool) -> Result<usize, String> {
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;

    let engine = app.state::<Arc<Mutex<EngineCore>>>();
    let mut core = engine.lock();
    let code_table = core.engine.macro_table().code_table();
    let imported = core.engine.macro_table_mut().import_unikey_txt(&content, append, code_table);
    let _ = crate::settings::save_macros(core.engine.macro_table());
    drop(core);
    crate::events::macros_changed(&app);

    Ok(imported)
}

#[tauri::command]
pub fn macro_export(app: AppHandle, path: String) -> Result<(), String> {
    let engine = app.state::<Arc<Mutex<EngineCore>>>();
    let text = engine.lock().engine.macro_table().export_unikey_txt();
    std::fs::write(&path, text).map_err(|e| e.to_string())
}

/// `OpenKeyManager.m: quickConvert` (`109-132`), triggered from the UI's
/// convert-tool window rather than the tray/hotkey (see `tray::quick_convert_now`
/// for that path, which uses the persisted `Settings::convert_options` instead
/// of caller-supplied options). Runs on the main thread because AppKit's
/// `NSPasteboard` requires it; the command handler blocks on a rendezvous
/// channel so the IPC caller gets a real result instead of firing blind.
#[tauri::command]
pub fn convert_clipboard(app: AppHandle, options: ConvertOptions) -> Result<(), String> {
    let engine_opts = options.to_engine();
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let converted = crate::pasteboard::quick_convert(&crate::pasteboard::SystemClipboard, &engine_opts);
        let _ = tx.send(converted);
    })
    .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_permission_status() -> bool {
    crate::platform::mac::ffi::is_trusted()
}

#[tauri::command]
pub fn open_accessibility_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}

#[tauri::command]
pub fn has_openkey_data() -> bool {
    crate::settings::openkey_import::has_openkey_data()
}

#[tauri::command]
pub fn import_openkey_settings(app: AppHandle) -> Result<Settings, String> {
    let imported = crate::settings::openkey_import::import_openkey_settings()?;

    let _ = crate::settings::save_macros(&imported.macros);
    let _ = crate::settings::save_smart_switch(&imported.smart_switch);

    let engine = app.state::<Arc<Mutex<EngineCore>>>();
    let mut core = engine.lock();
    core.engine.set_macro_table(imported.macros);
    core.smart_switch = imported.smart_switch;
    drop(core);

    let settings = apply_and_broadcast(&app, |_| imported.settings);
    crate::events::macros_changed(&app);

    Ok(settings)
}

/// No release feed is wired up yet -- there's no known GitHub (or other)
/// releases URL for this project. Stubbed rather than guessing one.
#[tauri::command]
pub fn check_update() -> Option<UpdateInfo> {
    None
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use vnkey_engine::{Engine, EngineOptions};

    use super::*;

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
    fn convert_options_to_engine_maps_every_field() {
        let opts = ConvertOptions {
            from_code: CodeTable::Tcvn3 as u8,
            to_code: CodeTable::VniWindows as u8,
            remove_mark: true,
            to_all_caps: true,
            to_all_non_caps: false,
            to_caps_first_letter: true,
            to_caps_each_word: false,
        };
        let engine_opts = opts.to_engine();
        assert_eq!(engine_opts.from_code, CodeTable::Tcvn3);
        assert_eq!(engine_opts.to_code, CodeTable::VniWindows);
        assert!(engine_opts.remove_mark);
        assert!(engine_opts.to_all_caps);
        assert!(!engine_opts.to_all_non_caps);
        assert!(engine_opts.to_caps_first_letter);
        assert!(!engine_opts.to_caps_each_word);
    }

    #[test]
    fn convert_options_to_engine_out_of_range_code_falls_back_to_unicode() {
        let opts = ConvertOptions {
            from_code: 99,
            to_code: 99,
            remove_mark: false,
            to_all_caps: false,
            to_all_non_caps: false,
            to_caps_first_letter: false,
            to_caps_each_word: false,
        };
        let engine_opts = opts.to_engine();
        assert_eq!(engine_opts.from_code, CodeTable::Unicode);
        assert_eq!(engine_opts.to_code, CodeTable::Unicode);
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
