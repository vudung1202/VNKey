//! Engine configuration options.
//!
//! Configuration options for the input engine. `Default` reproduces its
//! first-run defaults.

use crate::data::CodeTable;
use crate::data::InputType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineOptions {
    /// 0: English, 1: Vietnamese.
    pub language: u8,
    pub input_type: InputType,
    pub code_table: CodeTable,
    pub free_mark: bool,
    pub check_spelling: bool,
    /// 0: òa, úy; 1: oà, uý.
    pub use_modern_orthography: bool,
    /// cc=ch, gg=gi, kk=kh, nn=ng, qq=qu, pp=ph, tt=th, uu=ươ.
    pub quick_telex: bool,
    pub restore_if_wrong_spelling: bool,
    pub fix_recommend_browser: bool,
    pub use_macro: bool,
    pub use_macro_in_english_mode: bool,
    pub auto_caps_macro: bool,
    pub use_smart_switch_key: bool,
    pub upper_case_first_char: bool,
    pub temp_off_spelling: bool,
    pub allow_consonant_zfwj: bool,
    /// f -> ph, j -> gi, w -> qu.
    pub quick_start_consonant: bool,
    /// g -> ng, h -> nh, k -> ch.
    pub quick_end_consonant: bool,
    pub remember_code: bool,
    pub other_language: bool,
    /// Temporarily disables the whole engine for the in-progress word.
    pub temp_off_openkey: bool,
}

impl Default for EngineOptions {
    /// First-run defaults.
    fn default() -> Self {
        Self {
            language: 1,
            input_type: InputType::Telex,
            code_table: CodeTable::Unicode,
            free_mark: false,
            check_spelling: true,
            use_modern_orthography: false,
            quick_telex: false,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_openkey_first_run_defaults() {
        let opts = EngineOptions::default();
        assert_eq!(opts.language, 1);
        assert_eq!(opts.input_type, InputType::Telex);
        assert_eq!(opts.code_table, CodeTable::Unicode);
        assert!(opts.check_spelling);
        assert!(opts.fix_recommend_browser);
        assert!(opts.use_macro);
        assert!(opts.use_smart_switch_key);
        assert!(opts.remember_code);
        assert!(opts.other_language);

        assert!(!opts.free_mark);
        assert!(!opts.use_modern_orthography);
        assert!(!opts.quick_telex);
        assert!(!opts.restore_if_wrong_spelling);
        assert!(!opts.use_macro_in_english_mode);
        assert!(!opts.auto_caps_macro);
        assert!(!opts.upper_case_first_char);
        assert!(!opts.temp_off_spelling);
        assert!(!opts.allow_consonant_zfwj);
        assert!(!opts.quick_start_consonant);
        assert!(!opts.quick_end_consonant);
        assert!(!opts.temp_off_openkey);
    }
}
