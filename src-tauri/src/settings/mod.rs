//! Settings persistence: JSON config + OpenKey-compatible macro/smart-switch
//! blobs, loaded and applied before the platform layer starts.
//!
//! `settings.json`, `macros.bin`, `smart_switch.bin` all live in
//! `~/Library/Application Support/VNKey/`. The macro/smart-switch tables are
//! deliberately NOT part of the JSON (they're large, append-heavy, and
//! already have an OpenKey-compatible blob format via
//! [`MacroTable`]/[`SmartSwitchTable`]).
//!
//! `save`/`save_macros`/`save_smart_switch` and the `openkey_import` module
//! are this task's deliverables, but nothing in the crate calls them yet --
//! that's the settings IPC commands/UI and the first-run "import from
//! OpenKey?" prompt, both later tasks. Allow the resulting dead-code/
//! unused-import warnings for this module tree rather than wiring premature
//! callers just to silence the compiler.
#![allow(dead_code, unused_imports)]

#[cfg(target_os = "macos")]
pub mod openkey_import;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vnkey_engine::{CodeTable, EngineOptions, InputType, MacroTable, SmartSwitchTable};

/// Reserved for future settings-file migrations; no migration logic exists
/// yet (this is the first schema).
const SCHEMA_VERSION: u32 = 1;

/// `AppDelegate.m: DEFAULT_SWITCH_STATUS` (Option+Z, no beep). Duplicated
/// from `platform::mac::hotkey::DEFAULT_SWITCH_KEY_STATUS` (same value) so
/// this module stays buildable without the macOS-only platform layer.
const DEFAULT_SWITCH_KEY_STATUS: u32 = 0x7A00_0206;

/// `OpenKey.mm: EMPTY_HOTKEY` -- the "no hotkey configured" sentinel used for
/// the convert-tool quick-hotkey.
const EMPTY_HOTKEY: u32 = 0xFE00_00FE;

/// VNKey's persisted configuration. Field names map 1:1 to OpenKey's
/// `NSUserDefaults` keys (see `openkey_import`'s doc comments for the exact
/// key names); grouped as documented in the task brief.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub schema_version: u32,

    // -- Engine (mirrors `vnkey_engine::EngineOptions` field-for-field) --
    pub language: u8,
    /// Raw `InputType` discriminant (0 Telex, 1 VNI, 2/3 Simple Telex).
    pub input_type: u8,
    /// Raw `CodeTable` discriminant (0 Unicode .. 4 CP1258).
    pub code_table: u8,
    pub free_mark: bool,
    pub check_spelling: bool,
    pub modern_orthography: bool,
    pub restore_if_wrong_spelling: bool,
    pub fix_recommend_browser: bool,
    pub use_macro: bool,
    pub use_macro_in_english_mode: bool,
    pub auto_caps_macro: bool,
    pub use_smart_switch_key: bool,
    pub upper_case_first_char: bool,
    pub temp_off_spelling: bool,
    pub allow_consonant_zfwj: bool,
    pub quick_start_consonant: bool,
    pub quick_end_consonant: bool,
    pub remember_code: bool,
    pub other_language: bool,
    pub temp_off_openkey: bool,
    pub quick_telex: bool,

    // -- Platform --
    pub switch_key_status: u32,
    pub send_key_step_by_step: bool,
    pub fix_chromium_browser: bool,
    pub perform_layout_compat: bool,
    pub show_icon_on_dock: bool,
    pub gray_icon: bool,
    pub run_on_startup: bool,
    pub show_ui_on_startup: bool,
    pub dont_check_update: bool,
    /// Task 15 review fix: whether the first-run OpenKey-import offer has
    /// already been evaluated (with permission granted) and answered --
    /// Yes or No doesn't matter, just "asked" -- so it's never asked again.
    /// `#[serde(default)]` so a `settings.json` written before this field
    /// existed still deserializes (missing key -> `false`), i.e. those
    /// installs still get the offer once.
    ///
    /// Deliberately NOT derived from `settings.json`'s mere existence (the
    /// bug this field fixes): `platform::mac::start`'s `Settings::load()`
    /// auto-persists the file on the very first launch regardless of
    /// Accessibility permission, well before the offer can ever be
    /// evaluated with permission granted -- so file-existence alone was
    /// already `true` by the time an untrusted-first-launch user granted
    /// permission and relaunched, silently skipping the offer forever.
    #[serde(default)]
    pub first_run_offer_shown: bool,
    /// UI appearance preference: 0 = follow the OS (default), 1 = light,
    /// 2 = dark. Frontend-only -- the engine, tray, and platform layer never
    /// read it; it exists here so the choice persists and broadcasts to every
    /// window like any other setting. `#[serde(default)]` (0) keeps
    /// pre-theme `settings.json` files loading.
    #[serde(default)]
    pub theme: u8,

    // -- Convert tool --
    /// Raw `CodeTable` discriminant of the "convert from" table.
    pub convert_from_code: u8,
    /// Raw `CodeTable` discriminant of the "convert to" table.
    pub convert_to_code: u8,
    pub convert_remove_mark: bool,
    pub convert_to_all_caps: bool,
    pub convert_to_all_non_caps: bool,
    pub convert_to_caps_first_letter: bool,
    pub convert_to_caps_each_word: bool,
    pub convert_hotkey: u32,
    pub convert_dont_alert: bool,
}

impl Default for Settings {
    /// OpenKey's first-run defaults (`AppDelegate.m: loadDefaultConfig`,
    /// `OpenKey.mm: OpenKeyInit`). Fields not called out in the body default
    /// to 0/false, matching OpenKey's first-run writes.
    ///
    /// Two `loadDefaultConfig` copy-paste bugs are deliberately NOT
    /// reproduced:
    /// - it writes `vCodeTable` (0) into the `SwitchKeyStatus` NSUserDefaults
    ///   key instead of `vSwitchKeyStatus` -- `switch_key_status` below is
    ///   the correct `DEFAULT_SWITCH_KEY_STATUS`, not 0.
    /// - it writes `vUseMacroInEnglishMode` into the `SendKeyStepByStep` key
    ///   instead of `vSendKeyStepByStep`. Both source values happen to be 0
    ///   at first run, so the bug is numerically invisible here, but
    ///   `send_key_step_by_step` is still assigned from the conceptually
    ///   correct source (see `openkey_import`'s regression test, where the
    ///   two keys are given different values).
    fn default() -> Self {
        Settings {
            schema_version: SCHEMA_VERSION,

            // -- Engine --
            language: 1,
            input_type: InputType::Telex as u8,
            code_table: CodeTable::Unicode as u8,
            free_mark: false,
            check_spelling: true,
            modern_orthography: false,
            restore_if_wrong_spelling: false,
            fix_recommend_browser: true,
            use_macro: true,
            use_macro_in_english_mode: false,
            auto_caps_macro: false,
            use_smart_switch_key: true,
            upper_case_first_char: false,
            temp_off_spelling: false,
            allow_consonant_zfwj: false,
            quick_start_consonant: false,
            quick_end_consonant: false,
            remember_code: true,
            other_language: true,
            temp_off_openkey: false,
            quick_telex: false,

            // -- Platform --
            switch_key_status: DEFAULT_SWITCH_KEY_STATUS,
            send_key_step_by_step: false,
            fix_chromium_browser: false,
            perform_layout_compat: false,
            show_icon_on_dock: false,
            gray_icon: true,
            run_on_startup: true,
            show_ui_on_startup: false,
            dont_check_update: false,
            first_run_offer_shown: false,
            theme: 0,

            // -- Convert tool --
            convert_from_code: 0,
            convert_to_code: 0,
            convert_remove_mark: false,
            convert_to_all_caps: false,
            convert_to_all_non_caps: false,
            convert_to_caps_first_letter: false,
            convert_to_caps_each_word: false,
            // `OpenKeyInit`: an unset (0) `convertToolHotKey` is replaced
            // with `EMPTY_HOTKEY` at every launch -- 0 is never a valid
            // "first run" value.
            convert_hotkey: EMPTY_HOTKEY,
            // `OpenKeyInit`: `convertToolDontAlertWhenCompleted =
            // ![prefs boolForKey:...]` -- the stored flag means "alert when
            // completed", so an unset (false) key inverts to `true`.
            convert_dont_alert: true,
        }
    }
}

/// `InputType::from(u8)` (no such helper exists on the engine's `InputType`,
/// unlike `CodeTable::from_index`); out-of-range falls back to `Telex`,
/// mirroring `CodeTable::from_index`'s None-on-corrupt-data convention.
pub(crate) fn input_type_from_u8(value: u8) -> InputType {
    match value {
        0 => InputType::Telex,
        1 => InputType::Vni,
        2 => InputType::SimpleTelex1,
        3 => InputType::SimpleTelex2,
        _ => InputType::Telex,
    }
}

impl Settings {
    /// Convert the Engine-group fields to the engine crate's own options
    /// type.
    pub fn to_engine_options(self) -> EngineOptions {
        EngineOptions {
            language: self.language,
            input_type: input_type_from_u8(self.input_type),
            code_table: CodeTable::from_index(self.code_table).unwrap_or(CodeTable::Unicode),
            free_mark: self.free_mark,
            check_spelling: self.check_spelling,
            use_modern_orthography: self.modern_orthography,
            quick_telex: self.quick_telex,
            restore_if_wrong_spelling: self.restore_if_wrong_spelling,
            fix_recommend_browser: self.fix_recommend_browser,
            use_macro: self.use_macro,
            use_macro_in_english_mode: self.use_macro_in_english_mode,
            auto_caps_macro: self.auto_caps_macro,
            use_smart_switch_key: self.use_smart_switch_key,
            upper_case_first_char: self.upper_case_first_char,
            temp_off_spelling: self.temp_off_spelling,
            allow_consonant_zfwj: self.allow_consonant_zfwj,
            quick_start_consonant: self.quick_start_consonant,
            quick_end_consonant: self.quick_end_consonant,
            remember_code: self.remember_code,
            other_language: self.other_language,
            temp_off_openkey: self.temp_off_openkey,
        }
    }

    /// Apply the Platform-group fields to the tap thread's atomics.
    /// `SharedConfig::new` seeds a few of these (`send_key_step_by_step`,
    /// `fix_chromium_browser`, `switch_key_status`, `perform_layout_compat`)
    /// with hardcoded defaults regardless of `EngineOptions` -- this is the
    /// one place that overwrites them with the actual persisted values.
    #[cfg(target_os = "macos")]
    pub fn apply_to_shared_config(&self, config: &crate::state::SharedConfig) {
        use std::sync::atomic::Ordering;

        config.enabled.store(self.language == 1, Ordering::Relaxed);
        config
            .send_key_step_by_step
            .store(self.send_key_step_by_step, Ordering::Relaxed);
        config
            .fix_recommend_browser
            .store(self.fix_recommend_browser, Ordering::Relaxed);
        config
            .fix_chromium_browser
            .store(self.fix_chromium_browser, Ordering::Relaxed);
        config.switch_key_status.store(self.switch_key_status, Ordering::Relaxed);
        config.temp_off_spelling.store(self.temp_off_spelling, Ordering::Relaxed);
        config.temp_off_openkey.store(self.temp_off_openkey, Ordering::Relaxed);
        config.other_language.store(self.other_language, Ordering::Relaxed);
        config
            .perform_layout_compat
            .store(self.perform_layout_compat, Ordering::Relaxed);
        config
            .use_smart_switch_key
            .store(self.use_smart_switch_key, Ordering::Relaxed);
        config.remember_code.store(self.remember_code, Ordering::Relaxed);
        config.set_code_table(CodeTable::from_index(self.code_table).unwrap_or(CodeTable::Unicode));
    }

    /// Convert the Convert-tool-group fields to the engine crate's
    /// conversion options, for the "Chuyển mã nhanh" tray item / hotkey,
    /// which converts the clipboard using whatever the user last configured
    /// in the convert-tool window rather than caller-supplied options (that's
    /// `commands::ConvertOptions`, the IPC-facing type).
    pub fn convert_options(&self) -> vnkey_engine::ConvertOptions {
        vnkey_engine::ConvertOptions {
            from_code: CodeTable::from_index(self.convert_from_code).unwrap_or(CodeTable::Unicode),
            to_code: CodeTable::from_index(self.convert_to_code).unwrap_or(CodeTable::Unicode),
            remove_mark: self.convert_remove_mark,
            to_all_caps: self.convert_to_all_caps,
            to_all_non_caps: self.convert_to_all_non_caps,
            to_caps_first_letter: self.convert_to_caps_first_letter,
            to_caps_each_word: self.convert_to_caps_each_word,
        }
    }
}

/// `~/Library/Application Support/VNKey` (macOS `dirs::config_dir()`).
fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("VNKey")
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

fn macros_path() -> PathBuf {
    config_dir().join("macros.bin")
}

fn smart_switch_path() -> PathBuf {
    config_dir().join("smart_switch.bin")
}

/// Write `bytes` to `path` atomically: write to a sibling `.tmp` file, then
/// rename over the destination.
fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp_path = PathBuf::from(tmp);
    fs::write(&tmp_path, bytes)?;
    fs::rename(&tmp_path, path)
}

impl Settings {
    /// Load from `~/Library/Application Support/VNKey/settings.json`.
    /// Missing file or unparseable content falls back to
    /// [`Settings::default`] and re-persists it.
    pub fn load() -> Settings {
        Self::load_from(&settings_path())
    }

    fn load_from(path: &Path) -> Settings {
        let loaded = fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok());
        match loaded {
            Some(settings) => settings,
            None => {
                let settings = Settings::default();
                let _ = settings.save_to(path);
                settings
            }
        }
    }

    /// Save to `~/Library/Application Support/VNKey/settings.json` (pretty
    /// JSON, atomic write).
    pub fn save(&self) -> std::io::Result<()> {
        self.save_to(&settings_path())
    }

    fn save_to(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).expect("Settings has no non-serializable fields");
        atomic_write(path, json.as_bytes())
    }
}

/// Load the macro table from `macros.bin`. Missing/unreadable file yields an
/// empty table (first run, or the file was never created).
pub fn load_macros() -> MacroTable {
    load_macros_from(&macros_path())
}

fn load_macros_from(path: &Path) -> MacroTable {
    fs::read(path).map(|bytes| MacroTable::from_blob(&bytes)).unwrap_or_default()
}

/// Save the macro table to `macros.bin` (OpenKey-compatible blob, atomic
/// write).
pub fn save_macros(table: &MacroTable) -> std::io::Result<()> {
    atomic_write(&macros_path(), &table.to_blob())
}

/// Load the smart-switch table from `smart_switch.bin`. Missing/unreadable
/// file yields an empty table.
pub fn load_smart_switch() -> SmartSwitchTable {
    load_smart_switch_from(&smart_switch_path())
}

fn load_smart_switch_from(path: &Path) -> SmartSwitchTable {
    fs::read(path)
        .map(|bytes| SmartSwitchTable::from_blob(&bytes))
        .unwrap_or_default()
}

/// Save the smart-switch table to `smart_switch.bin` (OpenKey-compatible
/// blob, atomic write).
pub fn save_smart_switch(table: &SmartSwitchTable) -> std::io::Result<()> {
    atomic_write(&smart_switch_path(), &table.to_blob())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    /// A fresh, unique scratch directory for a single test (parallel-safe:
    /// each call gets its own subdirectory).
    fn tempdir() -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
        let dir = std::env::temp_dir().join(format!("vnkey-settings-test-{}-{}", std::process::id(), n));
        fs::create_dir_all(&dir).expect("create test tempdir");
        dir
    }

    #[test]
    fn default_matches_openkey_first_run_table() {
        let s = Settings::default();
        assert_eq!(s.schema_version, SCHEMA_VERSION);

        assert_eq!(s.language, 1);
        assert_eq!(s.input_type, InputType::Telex as u8);
        assert_eq!(s.code_table, CodeTable::Unicode as u8);
        assert!(s.check_spelling);
        assert!(s.fix_recommend_browser);
        assert!(s.use_macro);
        assert!(s.use_smart_switch_key);
        assert!(s.remember_code);
        assert!(s.other_language);
        assert!(s.gray_icon);
        assert!(s.run_on_startup);

        assert!(!s.free_mark);
        assert!(!s.modern_orthography);
        assert!(!s.quick_telex);
        assert!(!s.restore_if_wrong_spelling);
        assert!(!s.use_macro_in_english_mode);
        assert!(!s.auto_caps_macro);
        assert!(!s.upper_case_first_char);
        assert!(!s.temp_off_spelling);
        assert!(!s.allow_consonant_zfwj);
        assert!(!s.quick_start_consonant);
        assert!(!s.quick_end_consonant);
        assert!(!s.temp_off_openkey);
        assert!(!s.send_key_step_by_step);
        assert!(!s.fix_chromium_browser);
        assert!(!s.perform_layout_compat);
        assert!(!s.show_icon_on_dock);
        assert!(!s.show_ui_on_startup);
        assert!(!s.dont_check_update);
        assert!(!s.first_run_offer_shown);
    }

    #[test]
    fn default_switch_key_status_avoids_openkey_bug() {
        // `loadDefaultConfig` writes `vCodeTable` (0) into the
        // `SwitchKeyStatus` key. The correct default is Option+Z, not 0.
        let s = Settings::default();
        assert_eq!(s.switch_key_status, 0x7A00_0206);
        assert_ne!(s.switch_key_status, 0);
    }

    #[test]
    fn default_convert_hotkey_and_dont_alert_match_openkey_init() {
        let s = Settings::default();
        assert_eq!(s.convert_hotkey, EMPTY_HOTKEY);
        assert!(s.convert_dont_alert);
    }

    #[test]
    fn json_round_trip_preserves_every_field() {
        let s = Settings {
            input_type: InputType::Vni as u8,
            switch_key_status: 0x1234_5678,
            convert_hotkey: 0xABCD,
            first_run_offer_shown: true,
            ..Settings::default()
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let restored: Settings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s, restored);
    }

    /// Task 15 review fix regression: a `settings.json` written before
    /// `first_run_offer_shown` existed (every other real field present and
    /// non-default, but no `"first_run_offer_shown"` key at all) must still
    /// deserialize with those other fields intact -- NOT fall back to
    /// `Settings::default()` wholesale, which would silently reset the
    /// user's actual configuration. Only the new field should default to
    /// `false` (so pre-existing installs still get the first-run offer
    /// once). Fails without `#[serde(default)]` on the field: a plain
    /// `#[derive(Deserialize)]` treats a missing field as a hard parse
    /// error, and `load_from`'s `.ok()` fallback would then return
    /// `Settings::default()` entirely, losing `language`/`input_type` too
    /// (confirmed by temporarily removing the attribute and re-running this
    /// test, which then fails on the `language`/`input_type` assertions).
    #[test]
    fn old_settings_json_without_first_run_offer_shown_key_still_loads_other_fields() {
        let path = tempdir().join("settings.json");
        let old = Settings { language: 0, input_type: InputType::Vni as u8, ..Settings::default() };
        let mut json: serde_json::Value = serde_json::to_value(old).expect("serialize to Value");
        json.as_object_mut()
            .expect("Settings serializes to a JSON object")
            .remove("first_run_offer_shown");
        fs::write(&path, serde_json::to_vec(&json).expect("serialize without the new key")).expect("write fixture");

        let loaded = Settings::load_from(&path);
        assert!(!loaded.first_run_offer_shown, "a missing key must default to false");
        assert_eq!(loaded.language, 0, "other persisted fields must survive, not reset to Settings::default()");
        assert_eq!(loaded.input_type, InputType::Vni as u8);
    }

    #[test]
    fn to_engine_options_maps_engine_fields() {
        let s = Settings {
            input_type: InputType::Vni as u8,
            code_table: CodeTable::Cp1258 as u8,
            check_spelling: false,
            quick_telex: true,
            ..Settings::default()
        };

        let opts = s.to_engine_options();
        assert_eq!(opts.language, s.language);
        assert_eq!(opts.input_type, InputType::Vni);
        assert_eq!(opts.code_table, CodeTable::Cp1258);
        assert!(!opts.check_spelling);
        assert!(opts.quick_telex);
        assert_eq!(opts.use_modern_orthography, s.modern_orthography);
        assert_eq!(opts.remember_code, s.remember_code);
    }

    #[test]
    fn to_engine_options_out_of_range_code_table_falls_back_to_unicode() {
        let s = Settings { code_table: 99, ..Settings::default() };
        assert_eq!(s.to_engine_options().code_table, CodeTable::Unicode);
    }

    #[test]
    fn convert_options_maps_convert_group_fields() {
        let s = Settings {
            convert_from_code: CodeTable::Tcvn3 as u8,
            convert_to_code: CodeTable::VniWindows as u8,
            convert_remove_mark: true,
            convert_to_all_caps: true,
            ..Settings::default()
        };
        let opts = s.convert_options();
        assert_eq!(opts.from_code, CodeTable::Tcvn3);
        assert_eq!(opts.to_code, CodeTable::VniWindows);
        assert!(opts.remove_mark);
        assert!(opts.to_all_caps);
        assert!(!opts.to_all_non_caps);
    }

    #[test]
    fn convert_options_out_of_range_code_table_falls_back_to_unicode() {
        let s = Settings { convert_from_code: 99, convert_to_code: 99, ..Settings::default() };
        let opts = s.convert_options();
        assert_eq!(opts.from_code, CodeTable::Unicode);
        assert_eq!(opts.to_code, CodeTable::Unicode);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn apply_to_shared_config_overwrites_shared_config_new_hardcoded_fields() {
        use std::sync::atomic::Ordering;

        let s = Settings {
            send_key_step_by_step: true,
            fix_chromium_browser: true,
            switch_key_status: 0x1122_3344,
            perform_layout_compat: true,
            code_table: CodeTable::VniWindows as u8,
            ..Settings::default()
        };

        // `SharedConfig::new` hardcodes these regardless of the options it's
        // given -- confirm the pre-apply state really is the hardcoded one.
        let config = crate::state::SharedConfig::new(&s.to_engine_options());
        assert!(!config.send_key_step_by_step.load(Ordering::Relaxed));
        assert!(!config.fix_chromium_browser.load(Ordering::Relaxed));
        assert_eq!(config.switch_key_status.load(Ordering::Relaxed), DEFAULT_SWITCH_KEY_STATUS);
        assert!(!config.perform_layout_compat.load(Ordering::Relaxed));

        s.apply_to_shared_config(&config);

        assert!(config.send_key_step_by_step.load(Ordering::Relaxed));
        assert!(config.fix_chromium_browser.load(Ordering::Relaxed));
        assert_eq!(config.switch_key_status.load(Ordering::Relaxed), 0x1122_3344);
        assert!(config.perform_layout_compat.load(Ordering::Relaxed));
        assert_eq!(config.code_table(), CodeTable::VniWindows);
    }

    #[test]
    fn load_from_missing_file_returns_default_and_persists_it() {
        let path = tempdir().join("settings.json");
        let loaded = Settings::load_from(&path);
        assert_eq!(loaded, Settings::default());
        assert!(path.exists(), "a missing settings file should be re-persisted");
    }

    #[test]
    fn load_from_corrupt_file_returns_default_without_panicking() {
        let path = tempdir().join("settings.json");
        fs::write(&path, b"{ not valid json").expect("write corrupt fixture");
        let loaded = Settings::load_from(&path);
        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn load_from_json_missing_new_fields_returns_default_without_panicking() {
        // Models an old/incompatible settings.json (e.g. a future schema
        // change) rather than crashing.
        let path = tempdir().join("settings.json");
        fs::write(&path, b"{\"schema_version\": 1}").expect("write partial fixture");
        let loaded = Settings::load_from(&path);
        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn save_then_load_round_trips_through_disk() {
        let path = tempdir().join("settings.json");
        let s = Settings { language: 0, input_type: InputType::Vni as u8, ..Settings::default() };
        s.save_to(&path).expect("save");
        let loaded = Settings::load_from(&path);
        assert_eq!(loaded, s);
    }

    #[test]
    fn macro_blob_round_trips_through_file() {
        let path = tempdir().join("macros.bin");
        let mut table = MacroTable::new();
        table.add("vk", "Việt Nam", CodeTable::Unicode);
        atomic_write(&path, &table.to_blob()).expect("write blob");
        let loaded = load_macros_from(&path);
        assert_eq!(loaded.to_blob(), table.to_blob());
    }

    #[test]
    fn load_macros_from_missing_file_returns_empty_table() {
        let path = tempdir().join("nope.bin");
        let loaded = load_macros_from(&path);
        assert_eq!(loaded.to_blob(), MacroTable::new().to_blob());
    }

    #[test]
    fn smart_switch_blob_round_trips_through_file() {
        let path = tempdir().join("smart_switch.bin");
        let mut table = SmartSwitchTable::new();
        table.set_status("com.apple.Safari", 1);
        atomic_write(&path, &table.to_blob()).expect("write blob");
        let loaded = load_smart_switch_from(&path);
        assert_eq!(loaded.to_blob(), table.to_blob());
    }

    #[test]
    fn load_smart_switch_from_missing_file_returns_empty_table() {
        let path = tempdir().join("nope.bin");
        let loaded = load_smart_switch_from(&path);
        assert_eq!(loaded.to_blob(), SmartSwitchTable::new().to_blob());
    }
}
