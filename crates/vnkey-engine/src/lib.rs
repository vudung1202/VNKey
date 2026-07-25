//! Vietnamese input engine.
//!
//! This crate implements the C++ engine types and Vietnamese data tables
//! (constants, enums, code tables) in Rust. State-machine/typing logic is
//! ported in later tasks.

mod buffer;
pub mod charset;
mod convert;
pub mod data;
mod engine;
mod english;
mod input_method;
pub mod keys;
mod macros;
mod marks;
pub mod options;
mod smart_switch;
mod spelling;
mod vietnamese;

pub use charset::{get_character_code, key_code_to_character};
pub use convert::{ConvertOptions, convert_util};
pub use macros::MacroTable;
pub use smart_switch::SmartSwitchTable;
pub use data::{
    CAPS_MASK, CHAR_CODE_MASK, CHAR_MASK, CONSONANT_ALLOW_MASK, CodeTable, END_CONSONANT_MASK,
    HookCode, HookState, KeyEvent, KeyEventState, InputType, MARK1_MASK, MARK2_MASK, MARK3_MASK,
    MARK4_MASK, MARK5_MASK, MARK_MASK, MAX_BUFF, PURE_CHARACTER_MASK, STANDALONE_MASK, TONEW_MASK,
    TONE_MASK, is_double_code,
};
pub use engine::Engine;
pub use options::EngineOptions;
