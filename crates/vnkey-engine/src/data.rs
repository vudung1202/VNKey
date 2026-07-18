//! Core engine data types.
//!
//! Ported verbatim from OpenKey `engine/DataType.h`. Values and bit layout
//! must match the C++ source exactly: later tasks differential-test the
//! engine against the original C++ build.

/// Max length of a typing word buffer (`DataType.h: MAX_BUFF`).
pub const MAX_BUFF: usize = 32;

// -- internal engine data masks (DataType.h) --

pub const CAPS_MASK: u32 = 0x10000;
pub const TONE_MASK: u32 = 0x20000;
pub const TONEW_MASK: u32 = 0x40000;

/// Dấu Sắc - á
pub const MARK1_MASK: u32 = 0x80000;
/// Dấu Huyền - à
pub const MARK2_MASK: u32 = 0x100000;
/// Dấu Hỏi - ả
pub const MARK3_MASK: u32 = 0x200000;
/// Dấu Ngã - ã
pub const MARK4_MASK: u32 = 0x400000;
/// Dấu Nặng - ạ
pub const MARK5_MASK: u32 = 0x800000;

/// For checking has mark or not.
pub const MARK_MASK: u32 = 0xF80000;

/// Mask and get first 16 bits of character.
pub const CHAR_MASK: u32 = 0xFFFF;

/// Whether the data is created by a standalone key or not (W).
pub const STANDALONE_MASK: u32 = 0x1000000;

/// Whether the data is keyboard code or character code.
pub const CHAR_CODE_MASK: u32 = 0x2000000;

pub const PURE_CHARACTER_MASK: u32 = 0x8000_0000;

// -- for special features --

pub const END_CONSONANT_MASK: u32 = 0x4000;
pub const CONSONANT_ALLOW_MASK: u32 = 0x8000;

/// `DataType.h: vKeyEvent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Keyboard,
    Mouse,
}

/// `DataType.h: vKeyEventState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventState {
    KeyDown,
    KeyUp,
    MouseDown,
    MouseUp,
}

/// `DataType.h: vKeyInputType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    Telex = 0,
    Vni = 1,
    SimpleTelex1 = 2,
    SimpleTelex2 = 3,
}

/// `Vietnamese.cpp` comment above `_codeTable[]`:
/// 0 Unicode, 1 TCVN3, 2 VNI, 3 Unicode Compound, 4 Vietnamese Locale CP1258.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeTable {
    Unicode = 0,
    Tcvn3 = 1,
    VniWindows = 2,
    UnicodeCompound = 3,
    Cp1258 = 4,
}

impl CodeTable {
    /// Map a 0-based `vCodeTable` index back to the enum (inverse of `as u8`).
    /// Returns `None` for an out-of-range index, e.g. a corrupt smart-switch
    /// packed value whose `codeTable` bits exceed the known tables.
    pub fn from_index(index: u8) -> Option<CodeTable> {
        match index {
            0 => Some(CodeTable::Unicode),
            1 => Some(CodeTable::Tcvn3),
            2 => Some(CodeTable::VniWindows),
            3 => Some(CodeTable::UnicodeCompound),
            4 => Some(CodeTable::Cp1258),
            _ => None,
        }
    }
}

/// `DataType.h: HoolCodeState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookCode {
    /// Do not do anything.
    DoNothing = 0,
    /// Will reverse.
    WillProcess = 1,
    /// Start new.
    BreakWord = 2,
    /// Restore character to old char.
    Restore = 3,
    /// Replace by macro.
    ReplaceMacro = 4,
    /// Special flag: use for restore key if invalid word with break
    /// character (`,` `.` `"`).
    RestoreAndStartNewSession = 5,
}

/// Bytes data sent back to the host application (`DataType.h: vKeyHookState`).
#[derive(Debug, Clone)]
pub struct HookState {
    pub code: HookCode,
    pub backspace_count: u8,
    pub new_char_count: u8,
    /// 1: Word Break, 2: Delete key, 3: Normal key, 4: Should not send empty character.
    pub ext_code: u8,
    /// New characters to put in the queue.
    pub char_data: [u32; MAX_BUFF],
    /// Used for macro function; it is a key.
    pub macro_key: Vec<u32>,
    /// Used for macro function; it is keycode data.
    pub macro_data: Vec<u32>,
}

impl Default for HookState {
    fn default() -> Self {
        Self {
            code: HookCode::DoNothing,
            backspace_count: 0,
            new_char_count: 0,
            ext_code: 0,
            char_data: [0; MAX_BUFF],
            macro_key: Vec::new(),
            macro_data: Vec::new(),
        }
    }
}

/// `DataType.h: IS_DOUBLE_CODE(code)` -- is VNI or Unicode Compound.
pub fn is_double_code(code_table: CodeTable) -> bool {
    matches!(code_table, CodeTable::VniWindows | CodeTable::UnicodeCompound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_match_datatype_h() {
        assert_eq!(CAPS_MASK, 0x10000);
        assert_eq!(TONE_MASK, 0x20000);
        assert_eq!(TONEW_MASK, 0x40000);
        assert_eq!(MARK1_MASK, 0x80000);
        assert_eq!(MARK2_MASK, 0x100000);
        assert_eq!(MARK3_MASK, 0x200000);
        assert_eq!(MARK4_MASK, 0x400000);
        assert_eq!(MARK5_MASK, 0x800000);
        assert_eq!(MARK_MASK, 0xF80000);
        assert_eq!(CHAR_MASK, 0xFFFF);
        assert_eq!(STANDALONE_MASK, 0x1000000);
        assert_eq!(CHAR_CODE_MASK, 0x2000000);
        assert_eq!(PURE_CHARACTER_MASK, 0x8000_0000);
        assert_eq!(END_CONSONANT_MASK, 0x4000);
        assert_eq!(CONSONANT_ALLOW_MASK, 0x8000);
        assert_eq!(MAX_BUFF, 32);
    }

    #[test]
    fn mark_mask_is_union_of_mark1_5() {
        assert_eq!(
            MARK_MASK,
            MARK1_MASK | MARK2_MASK | MARK3_MASK | MARK4_MASK | MARK5_MASK
        );
    }

    #[test]
    fn is_double_code_matches_datatype_h_macro() {
        assert!(!is_double_code(CodeTable::Unicode));
        assert!(!is_double_code(CodeTable::Tcvn3));
        assert!(is_double_code(CodeTable::VniWindows));
        assert!(is_double_code(CodeTable::UnicodeCompound));
        assert!(!is_double_code(CodeTable::Cp1258));
    }

    #[test]
    fn hook_state_default_is_all_zeroed() {
        let hs = HookState::default();
        assert_eq!(hs.code, HookCode::DoNothing);
        assert_eq!(hs.backspace_count, 0);
        assert_eq!(hs.new_char_count, 0);
        assert_eq!(hs.ext_code, 0);
        assert_eq!(hs.char_data, [0u32; MAX_BUFF]);
        assert!(hs.macro_key.is_empty());
        assert!(hs.macro_data.is_empty());
    }
}
