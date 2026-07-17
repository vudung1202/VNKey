//! Vietnamese input engine (pure Rust port of OpenKey's core).
//!
//! This crate ports OpenKey's C++ engine types and Vietnamese data tables
//! (constants, enums, code tables) to Rust. State-machine/typing logic is
//! ported in later tasks.

pub mod data;
pub mod keys;
pub mod options;

pub use data::{
    CAPS_MASK, CHAR_CODE_MASK, CHAR_MASK, CONSONANT_ALLOW_MASK, CodeTable, END_CONSONANT_MASK,
    HookCode, HookState, KeyEvent, KeyEventState, InputType, MARK1_MASK, MARK2_MASK, MARK3_MASK,
    MARK4_MASK, MARK5_MASK, MARK_MASK, MAX_BUFF, PURE_CHARACTER_MASK, STANDALONE_MASK, TONEW_MASK,
    TONE_MASK, is_double_code,
};
pub use options::EngineOptions;
