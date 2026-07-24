//! Key-classification predicates.
//!
//! These predicates classify keys for the Vietnamese transform. The original
//! C++ macros read the input-type global; here the input type is passed
//! explicitly.
//!
//! `ProcessingChar` column order: S F R X J A O E W D Z.

use crate::data::InputType;
use crate::keys::*;
use crate::vietnamese::PROCESSING_CHAR;

#[inline]
fn pc(input_type: InputType, col: usize) -> u16 {
    PROCESSING_CHAR[input_type as usize][col]
}

pub(crate) fn is_key_s(input_type: InputType, key: u16) -> bool {
    pc(input_type, 0) == key
}
pub(crate) fn is_key_f(input_type: InputType, key: u16) -> bool {
    pc(input_type, 1) == key
}
pub(crate) fn is_key_r(input_type: InputType, key: u16) -> bool {
    pc(input_type, 2) == key
}
pub(crate) fn is_key_x(input_type: InputType, key: u16) -> bool {
    pc(input_type, 3) == key
}
pub(crate) fn is_key_j(input_type: InputType, key: u16) -> bool {
    pc(input_type, 4) == key
}
pub(crate) fn is_key_d(input_type: InputType, key: u16) -> bool {
    pc(input_type, 9) == key
}
pub(crate) fn is_key_z(input_type: InputType, key: u16) -> bool {
    pc(input_type, 10) == key
}

/// Non-VNI: column 8. VNI: column 8 or 7 (KEY_8 or KEY_7).
pub(crate) fn is_key_w(input_type: InputType, key: u16) -> bool {
    if input_type != InputType::Vni {
        pc(input_type, 8) == key
    } else {
        pc(input_type, 8) == key || pc(input_type, 7) == key
    }
}

/// Non-VNI: column 5, 6 or 7 (A/O/E for Telex). VNI: column 6 (KEY_6).
pub(crate) fn is_key_double(input_type: InputType, key: u16) -> bool {
    if input_type != InputType::Vni {
        pc(input_type, 5) == key || pc(input_type, 6) == key || pc(input_type, 7) == key
    } else {
        pc(input_type, 6) == key
    }
}

/// Non-VNI uses the literal S/F/R/J/X keys; VNI uses digits 1/2/3/5/4
/// (verbatim, incl. the source's 1,2,3,5,4 order).
pub(crate) fn is_mark_key(input_type: InputType, key_code: u16) -> bool {
    if input_type != InputType::Vni {
        key_code == KEY_S || key_code == KEY_F || key_code == KEY_R || key_code == KEY_J
            || key_code == KEY_X
    } else {
        key_code == KEY_1 || key_code == KEY_2 || key_code == KEY_3 || key_code == KEY_5
            || key_code == KEY_4
    }
}

pub(crate) fn is_bracket_key(key: u16) -> bool {
    key == KEY_LEFT_BRACKET || key == KEY_RIGHT_BRACKET
}

/// Everything that is not a base vowel.
pub(crate) fn is_consonant(key_code: u16) -> bool {
    !(key_code == KEY_A
        || key_code == KEY_E
        || key_code == KEY_U
        || key_code == KEY_Y
        || key_code == KEY_I
        || key_code == KEY_O)
}

/// Whether a key participates in the Vietnamese transform for the given
/// input type.
pub(crate) fn is_special_key(input_type: InputType, key_code: u16) -> bool {
    match input_type {
        InputType::Telex => {
            key_code == KEY_W
                || key_code == KEY_E
                || key_code == KEY_R
                || key_code == KEY_O
                || key_code == KEY_LEFT_BRACKET
                || key_code == KEY_RIGHT_BRACKET
                || key_code == KEY_A
                || key_code == KEY_S
                || key_code == KEY_D
                || key_code == KEY_F
                || key_code == KEY_J
                || key_code == KEY_Z
                || key_code == KEY_X
        }
        InputType::Vni => {
            key_code == KEY_1
                || key_code == KEY_2
                || key_code == KEY_3
                || key_code == KEY_4
                || key_code == KEY_5
                || key_code == KEY_6
                || key_code == KEY_7
                || key_code == KEY_8
                || key_code == KEY_9
                || key_code == KEY_0
        }
        // SimpleTelex1 and SimpleTelex2 share the same set (no brackets).
        InputType::SimpleTelex1 | InputType::SimpleTelex2 => {
            key_code == KEY_W
                || key_code == KEY_E
                || key_code == KEY_R
                || key_code == KEY_O
                || key_code == KEY_A
                || key_code == KEY_S
                || key_code == KEY_D
                || key_code == KEY_F
                || key_code == KEY_J
                || key_code == KEY_Z
                || key_code == KEY_X
        }
    }
}

pub(crate) fn is_number_key(code: u16) -> bool {
    code == KEY_1
        || code == KEY_2
        || code == KEY_3
        || code == KEY_4
        || code == KEY_5
        || code == KEY_6
        || code == KEY_7
        || code == KEY_8
        || code == KEY_9
        || code == KEY_0
}

/// `prev_char` is `CHR(index-1)` (only read when `index > 0`, exactly as the
/// C++ macro's `_index > 0` short-circuit guarantees).
pub(crate) fn is_quick_telex_key(index: i32, prev_char: u16, code: u16) -> bool {
    index > 0
        && (code == KEY_C
            || code == KEY_G
            || code == KEY_K
            || code == KEY_N
            || code == KEY_Q
            || code == KEY_P
            || code == KEY_T)
        && prev_char == code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telex_special_keys_include_brackets_simple_telex_does_not() {
        assert!(is_special_key(InputType::Telex, KEY_LEFT_BRACKET));
        assert!(is_special_key(InputType::Telex, KEY_RIGHT_BRACKET));
        assert!(!is_special_key(InputType::SimpleTelex1, KEY_LEFT_BRACKET));
        assert!(!is_special_key(InputType::SimpleTelex2, KEY_RIGHT_BRACKET));
    }

    #[test]
    fn vni_key_mappings_via_processing_char() {
        // VNI: S=1, F=2, R=3, X=4, J=5, D=9, Z=0; DOUBLE=6; W=7 or 8.
        assert!(is_key_s(InputType::Vni, KEY_1));
        assert!(is_key_j(InputType::Vni, KEY_5));
        assert!(is_key_d(InputType::Vni, KEY_9));
        assert!(is_key_z(InputType::Vni, KEY_0));
        assert!(is_key_double(InputType::Vni, KEY_6));
        assert!(is_key_w(InputType::Vni, KEY_7));
        assert!(is_key_w(InputType::Vni, KEY_8));
        assert!(!is_key_w(InputType::Vni, KEY_6));
    }

    #[test]
    fn telex_double_is_a_o_e() {
        assert!(is_key_double(InputType::Telex, KEY_A));
        assert!(is_key_double(InputType::Telex, KEY_O));
        assert!(is_key_double(InputType::Telex, KEY_E));
        assert!(!is_key_double(InputType::Telex, KEY_W));
        assert!(is_key_w(InputType::Telex, KEY_W));
    }

    #[test]
    fn consonant_and_number_predicates() {
        assert!(is_consonant(KEY_B));
        assert!(!is_consonant(KEY_A));
        assert!(!is_consonant(KEY_U));
        assert!(is_number_key(KEY_0));
        assert!(!is_number_key(KEY_A));
    }

    #[test]
    fn quick_telex_key_needs_matching_prev_and_nonzero_index() {
        assert!(is_quick_telex_key(1, KEY_C, KEY_C));
        assert!(!is_quick_telex_key(0, KEY_C, KEY_C));
        assert!(!is_quick_telex_key(1, KEY_C, KEY_G));
    }
}
