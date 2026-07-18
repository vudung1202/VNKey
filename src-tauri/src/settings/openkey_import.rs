//! One-time importer for OpenKey's `com.tuyenmai.openkey` NSUserDefaults
//! domain: maps every persisted OpenKey key onto a [`Settings`], plus the
//! `macroData`/`smartSwitchKey` blobs.
//!
//! [`settings_from_dict`] is the pure key -> field mapping, independent of
//! how the map was obtained, so it's unit-testable with a plain `HashMap`
//! fixture. The `mac` submodule is thin NSUserDefaults-reading glue around
//! it; it has no real OpenKey install to test against in CI, so it's
//! exercised only by a smoke test here plus manual testing (see the task
//! report).
//!
//! OpenKey `NSUserDefaults` key names, by `Settings` field (from
//! `AppDelegate.m: loadDefaultConfig`/`fillData` and `OpenKey.mm:
//! OpenKeyInit`'s `LOAD_DATA` calls):
//!
//! - Engine: language="InputMethod", input_type="InputType",
//!   code_table="CodeTable" (clamped to >= 0), check_spelling="Spelling",
//!   modern_orthography="ModernOrthography",
//!   restore_if_wrong_spelling="RestoreIfInvalidWord",
//!   fix_recommend_browser="FixRecommendBrowser", use_macro="UseMacro",
//!   use_macro_in_english_mode="UseMacroInEnglishMode",
//!   auto_caps_macro="vAutoCapsMacro",
//!   use_smart_switch_key="UseSmartSwitchKey",
//!   upper_case_first_char="UpperCaseFirstChar",
//!   temp_off_spelling="vTempOffSpelling",
//!   allow_consonant_zfwj="vAllowConsonantZFWJ",
//!   quick_start_consonant="vQuickStartConsonant",
//!   quick_end_consonant="vQuickEndConsonant", remember_code="vRememberCode",
//!   other_language="vOtherLanguage", temp_off_openkey="vTempOffOpenKey",
//!   quick_telex="QuickTelex". `free_mark` has no live key: `OpenKeyInit`
//!   hardcodes `vFreeMark = 0` and never actually reads `"FreeMark"` back
//!   (the read is dead code), so it's deliberately left at
//!   `Settings::default()`'s value here too.
//! - Platform: switch_key_status="SwitchKeyStatus" (0 falls back to the
//!   default hotkey, matching `fillData`'s runtime correction of the
//!   `loadDefaultConfig` bug), send_key_step_by_step="SendKeyStepByStep",
//!   fix_chromium_browser="vFixChromiumBrowser",
//!   perform_layout_compat="vPerformLayoutCompat",
//!   show_icon_on_dock="vShowIconOnDock", gray_icon="GrayIcon",
//!   run_on_startup="RunOnStartup", show_ui_on_startup="ShowUIOnStartup",
//!   dont_check_update="DontCheckUpdate".
//! - Convert tool: convert_from_code="convertToolFromCode",
//!   convert_to_code="convertToolToCode",
//!   convert_remove_mark="convertToolRemoveMark",
//!   convert_to_all_caps="convertToolToAllCaps",
//!   convert_to_all_non_caps="convertToolToAllNonCaps",
//!   convert_to_caps_first_letter="convertToolToCapsFirstLetter",
//!   convert_to_caps_each_word="convertToolToCapsEachWord",
//!   convert_hotkey="convertToolHotKey" (0 falls back to `EMPTY_HOTKEY`),
//!   convert_dont_alert="convertToolDontAlertWhenCompleted" (inverted:
//!   the stored flag means "alert when completed").

use std::collections::HashMap;

use super::Settings;

fn get_bool(dict: &HashMap<String, i64>, key: &str) -> Option<bool> {
    dict.get(key).map(|v| *v != 0)
}

/// Map an OpenKey NSUserDefaults domain (already flattened to a plain
/// `HashMap`) onto a [`Settings`]. Keys absent from `dict` keep VNKey's own
/// default for that field rather than 0/false -- an OpenKey install that
/// never touched a setting should import as VNKey's sane default for it, not
/// as "off".
pub fn settings_from_dict(dict: &HashMap<String, i64>) -> Settings {
    let mut s = Settings::default();

    if let Some(v) = dict.get("InputMethod") {
        s.language = *v as u8;
    }
    if let Some(v) = dict.get("InputType") {
        s.input_type = *v as u8;
    }
    if let Some(v) = dict.get("CodeTable") {
        // `OpenKeyInit`: `if (vCodeTable < 0) vCodeTable = 0;`
        s.code_table = if *v < 0 { 0 } else { *v as u8 };
    }
    if let Some(v) = get_bool(dict, "Spelling") {
        s.check_spelling = v;
    }
    if let Some(v) = get_bool(dict, "ModernOrthography") {
        s.modern_orthography = v;
    }
    if let Some(v) = get_bool(dict, "RestoreIfInvalidWord") {
        s.restore_if_wrong_spelling = v;
    }
    if let Some(v) = get_bool(dict, "FixRecommendBrowser") {
        s.fix_recommend_browser = v;
    }
    if let Some(v) = get_bool(dict, "UseMacro") {
        s.use_macro = v;
    }
    if let Some(v) = get_bool(dict, "UseMacroInEnglishMode") {
        s.use_macro_in_english_mode = v;
    }
    if let Some(v) = get_bool(dict, "vAutoCapsMacro") {
        s.auto_caps_macro = v;
    }
    if let Some(v) = get_bool(dict, "UseSmartSwitchKey") {
        s.use_smart_switch_key = v;
    }
    if let Some(v) = get_bool(dict, "UpperCaseFirstChar") {
        s.upper_case_first_char = v;
    }
    if let Some(v) = get_bool(dict, "vTempOffSpelling") {
        s.temp_off_spelling = v;
    }
    if let Some(v) = get_bool(dict, "vAllowConsonantZFWJ") {
        s.allow_consonant_zfwj = v;
    }
    if let Some(v) = get_bool(dict, "vQuickStartConsonant") {
        s.quick_start_consonant = v;
    }
    if let Some(v) = get_bool(dict, "vQuickEndConsonant") {
        s.quick_end_consonant = v;
    }
    if let Some(v) = get_bool(dict, "vRememberCode") {
        s.remember_code = v;
    }
    if let Some(v) = get_bool(dict, "vOtherLanguage") {
        s.other_language = v;
    }
    if let Some(v) = get_bool(dict, "vTempOffOpenKey") {
        s.temp_off_openkey = v;
    }
    if let Some(v) = get_bool(dict, "QuickTelex") {
        s.quick_telex = v;
    }

    // `fillData` re-derives this on every read and falls back to the default
    // hotkey when the stored value is 0 -- including the 0 that
    // `loadDefaultConfig`'s copy-paste bug persists on first run.
    let switch_key_status = dict.get("SwitchKeyStatus").copied().unwrap_or(0) as u32;
    s.switch_key_status = if switch_key_status == 0 {
        super::DEFAULT_SWITCH_KEY_STATUS
    } else {
        switch_key_status
    };
    if let Some(v) = get_bool(dict, "SendKeyStepByStep") {
        s.send_key_step_by_step = v;
    }
    if let Some(v) = get_bool(dict, "vFixChromiumBrowser") {
        s.fix_chromium_browser = v;
    }
    if let Some(v) = get_bool(dict, "vPerformLayoutCompat") {
        s.perform_layout_compat = v;
    }
    if let Some(v) = get_bool(dict, "vShowIconOnDock") {
        s.show_icon_on_dock = v;
    }
    if let Some(v) = get_bool(dict, "GrayIcon") {
        s.gray_icon = v;
    }
    if let Some(v) = get_bool(dict, "RunOnStartup") {
        s.run_on_startup = v;
    }
    if let Some(v) = get_bool(dict, "ShowUIOnStartup") {
        s.show_ui_on_startup = v;
    }
    if let Some(v) = get_bool(dict, "DontCheckUpdate") {
        s.dont_check_update = v;
    }

    if let Some(v) = dict.get("convertToolFromCode") {
        s.convert_from_code = *v as u8;
    }
    if let Some(v) = dict.get("convertToolToCode") {
        s.convert_to_code = *v as u8;
    }
    if let Some(v) = get_bool(dict, "convertToolRemoveMark") {
        s.convert_remove_mark = v;
    }
    if let Some(v) = get_bool(dict, "convertToolToAllCaps") {
        s.convert_to_all_caps = v;
    }
    if let Some(v) = get_bool(dict, "convertToolToAllNonCaps") {
        s.convert_to_all_non_caps = v;
    }
    if let Some(v) = get_bool(dict, "convertToolToCapsFirstLetter") {
        s.convert_to_caps_first_letter = v;
    }
    if let Some(v) = get_bool(dict, "convertToolToCapsEachWord") {
        s.convert_to_caps_each_word = v;
    }
    // `OpenKeyInit`: an unset (0) `convertToolHotKey` is replaced with
    // `EMPTY_HOTKEY` at every launch -- 0 is never a valid configured hotkey.
    let convert_hotkey = dict.get("convertToolHotKey").copied().unwrap_or(0) as u32;
    s.convert_hotkey = if convert_hotkey == 0 { super::EMPTY_HOTKEY } else { convert_hotkey };
    // `OpenKeyInit`: `convertToolDontAlertWhenCompleted = ![prefs
    // boolForKey:...]` -- the stored flag means "alert when completed", so
    // it's inverted here too.
    s.convert_dont_alert = !get_bool(dict, "convertToolDontAlertWhenCompleted").unwrap_or(false);

    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use vnkey_engine::{CodeTable, InputType};

    /// Every key OpenKey persists, each set to a value distinguishable from
    /// both 0 and `Settings::default()`.
    fn sample_dict() -> HashMap<String, i64> {
        HashMap::from([
            ("InputMethod".to_string(), 0),
            ("InputType".to_string(), InputType::Vni as i64),
            ("CodeTable".to_string(), CodeTable::VniWindows as i64),
            ("Spelling".to_string(), 0),
            ("ModernOrthography".to_string(), 1),
            ("RestoreIfInvalidWord".to_string(), 1),
            ("FixRecommendBrowser".to_string(), 0),
            ("UseMacro".to_string(), 0),
            ("UseMacroInEnglishMode".to_string(), 1),
            ("vAutoCapsMacro".to_string(), 1),
            ("UseSmartSwitchKey".to_string(), 0),
            ("UpperCaseFirstChar".to_string(), 1),
            ("vTempOffSpelling".to_string(), 1),
            ("vAllowConsonantZFWJ".to_string(), 1),
            ("vQuickStartConsonant".to_string(), 1),
            ("vQuickEndConsonant".to_string(), 1),
            ("vRememberCode".to_string(), 0),
            ("vOtherLanguage".to_string(), 0),
            ("vTempOffOpenKey".to_string(), 1),
            ("QuickTelex".to_string(), 1),
            ("SwitchKeyStatus".to_string(), 0x1234_5678),
            ("SendKeyStepByStep".to_string(), 1),
            ("vFixChromiumBrowser".to_string(), 1),
            ("vPerformLayoutCompat".to_string(), 1),
            ("vShowIconOnDock".to_string(), 1),
            ("GrayIcon".to_string(), 0),
            ("RunOnStartup".to_string(), 0),
            ("ShowUIOnStartup".to_string(), 1),
            ("DontCheckUpdate".to_string(), 1),
            ("convertToolFromCode".to_string(), 2),
            ("convertToolToCode".to_string(), 3),
            ("convertToolRemoveMark".to_string(), 1),
            ("convertToolToAllCaps".to_string(), 1),
            ("convertToolToAllNonCaps".to_string(), 1),
            ("convertToolToCapsFirstLetter".to_string(), 1),
            ("convertToolToCapsEachWord".to_string(), 1),
            ("convertToolHotKey".to_string(), 0xABCD),
            ("convertToolDontAlertWhenCompleted".to_string(), 1),
        ])
    }

    #[test]
    fn maps_every_key_from_a_full_openkey_dict() {
        let s = settings_from_dict(&sample_dict());

        assert_eq!(s.language, 0);
        assert_eq!(s.input_type, InputType::Vni as u8);
        assert_eq!(s.code_table, CodeTable::VniWindows as u8);
        assert!(!s.check_spelling);
        assert!(s.modern_orthography);
        assert!(s.restore_if_wrong_spelling);
        assert!(!s.fix_recommend_browser);
        assert!(!s.use_macro);
        assert!(s.use_macro_in_english_mode);
        assert!(s.auto_caps_macro);
        assert!(!s.use_smart_switch_key);
        assert!(s.upper_case_first_char);
        assert!(s.temp_off_spelling);
        assert!(s.allow_consonant_zfwj);
        assert!(s.quick_start_consonant);
        assert!(s.quick_end_consonant);
        assert!(!s.remember_code);
        assert!(!s.other_language);
        assert!(s.temp_off_openkey);
        assert!(s.quick_telex);
        // `free_mark` is never read from OpenKey -- stays at the default.
        assert!(!s.free_mark);

        assert_eq!(s.switch_key_status, 0x1234_5678);
        assert!(s.send_key_step_by_step);
        assert!(s.fix_chromium_browser);
        assert!(s.perform_layout_compat);
        assert!(s.show_icon_on_dock);
        assert!(!s.gray_icon);
        assert!(!s.run_on_startup);
        assert!(s.show_ui_on_startup);
        assert!(s.dont_check_update);

        assert_eq!(s.convert_from_code, 2);
        assert_eq!(s.convert_to_code, 3);
        assert!(s.convert_remove_mark);
        assert!(s.convert_to_all_caps);
        assert!(s.convert_to_all_non_caps);
        assert!(s.convert_to_caps_first_letter);
        assert!(s.convert_to_caps_each_word);
        assert_eq!(s.convert_hotkey, 0xABCD);
        // Stored "alert when completed" = true -> `convert_dont_alert` = false.
        assert!(!s.convert_dont_alert);
    }

    #[test]
    fn empty_dict_falls_back_to_documented_defaults() {
        assert_eq!(settings_from_dict(&HashMap::new()), Settings::default());
    }

    #[test]
    fn switch_key_status_of_zero_falls_back_to_default() {
        let dict = HashMap::from([("SwitchKeyStatus".to_string(), 0)]);
        let s = settings_from_dict(&dict);
        assert_eq!(s.switch_key_status, super::super::DEFAULT_SWITCH_KEY_STATUS);
    }

    #[test]
    fn convert_hotkey_of_zero_falls_back_to_empty_hotkey() {
        let dict = HashMap::from([("convertToolHotKey".to_string(), 0)]);
        let s = settings_from_dict(&dict);
        assert_eq!(s.convert_hotkey, super::super::EMPTY_HOTKEY);
    }

    #[test]
    fn send_key_step_by_step_reads_its_own_key_not_use_macro_in_english_mode() {
        // Regression test for `loadDefaultConfig`'s bug 2 (it writes
        // `vUseMacroInEnglishMode` into the `SendKeyStepByStep` key). The
        // importer must read `SendKeyStepByStep` independently.
        let dict = HashMap::from([
            ("SendKeyStepByStep".to_string(), 1),
            ("UseMacroInEnglishMode".to_string(), 0),
        ]);
        let s = settings_from_dict(&dict);
        assert!(s.send_key_step_by_step);
        assert!(!s.use_macro_in_english_mode);
    }

    #[test]
    fn negative_code_table_clamps_to_zero() {
        let dict = HashMap::from([("CodeTable".to_string(), -5)]);
        let s = settings_from_dict(&dict);
        assert_eq!(s.code_table, 0);
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::ptr::NonNull;

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, Bool};
    use objc2_foundation::{NSData, NSDictionary, NSNumber, NSString, NSUserDefaults};
    use vnkey_engine::{MacroTable, SmartSwitchTable};

    use super::settings_from_dict;
    use crate::settings::Settings;

    /// `OpenKey.mm`: the app's own bundle id, and the NSUserDefaults domain
    /// its settings are persisted under.
    const OPENKEY_BUNDLE: &str = "com.tuyenmai.openkey";

    fn persistent_domain() -> Option<Retained<NSDictionary<NSString, AnyObject>>> {
        let name = NSString::from_str(OPENKEY_BUNDLE);
        NSUserDefaults::standardUserDefaults().persistentDomainForName(&name)
    }

    /// Whether OpenKey has ever persisted settings on this machine.
    pub fn has_openkey_data() -> bool {
        persistent_domain().is_some()
    }

    /// Flatten every `NSNumber`-valued entry of `dict` into a plain map so
    /// [`settings_from_dict`] stays pure and testable. Non-`NSNumber` values
    /// (the `macroData`/`smartSwitchKey` blobs, read separately by
    /// [`data_for_key`]) are skipped.
    fn dict_to_int_map(dict: &NSDictionary<NSString, AnyObject>) -> HashMap<String, i64> {
        let map = RefCell::new(HashMap::new());
        let block = RcBlock::new(
            |key: NonNull<NSString>, value: NonNull<AnyObject>, _stop: NonNull<Bool>| {
                // SAFETY: `enumerateKeysAndObjectsUsingBlock` guarantees `key`
                // and `value` are valid, live objects for the duration of
                // this synchronous callback.
                let (key, value) = unsafe { (key.as_ref(), value.as_ref()) };
                if let Some(number) = value.downcast_ref::<NSNumber>() {
                    map.borrow_mut().insert(key.to_string(), number.as_i64());
                }
            },
        );
        dict.enumerateKeysAndObjectsUsingBlock(&block);
        drop(block);
        map.into_inner()
    }

    /// Read an `NSData`-valued entry (the macro/smart-switch blobs) by key.
    fn data_for_key(dict: &NSDictionary<NSString, AnyObject>, key: &str) -> Option<Vec<u8>> {
        let key = NSString::from_str(key);
        let value = dict.objectForKeyedSubscript(&key)?;
        Some(value.downcast_ref::<NSData>()?.to_vec())
    }

    /// The result of a successful OpenKey import: settings plus the two blob
    /// tables, ready for the caller to persist (`Settings::save`,
    /// `settings::save_macros`, `settings::save_smart_switch`) and apply.
    pub struct OpenKeyImport {
        pub settings: Settings,
        pub macros: MacroTable,
        pub smart_switch: SmartSwitchTable,
    }

    /// Import OpenKey's persisted settings + macro/smart-switch blobs. Fails
    /// softly (no panic) if OpenKey has never run on this machine.
    pub fn import_openkey_settings() -> Result<OpenKeyImport, String> {
        let dict = persistent_domain().ok_or_else(|| {
            format!("no NSUserDefaults domain for {OPENKEY_BUNDLE} (OpenKey not installed or never run)")
        })?;

        let int_map = dict_to_int_map(&dict);
        let settings = settings_from_dict(&int_map);
        let macros = data_for_key(&dict, "macroData")
            .map(|bytes| MacroTable::from_blob(&bytes))
            .unwrap_or_default();
        let smart_switch = data_for_key(&dict, "smartSwitchKey")
            .map(|bytes| SmartSwitchTable::from_blob(&bytes))
            .unwrap_or_default();

        Ok(OpenKeyImport { settings, macros, smart_switch })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Smoke test only: exercises the real NSUserDefaults glue path
        /// end-to-end without asserting a specific outcome (whether OpenKey
        /// is installed on the test machine is environment-dependent). The
        /// mapping logic itself is covered by the pure `settings_from_dict`
        /// tests above.
        #[test]
        fn import_openkey_settings_does_not_panic() {
            let _ = import_openkey_settings();
        }

        #[test]
        fn has_openkey_data_does_not_panic() {
            let _ = has_openkey_data();
        }
    }
}

#[cfg(target_os = "macos")]
pub use mac::{has_openkey_data, import_openkey_settings, OpenKeyImport};
