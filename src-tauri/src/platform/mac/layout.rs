//! Keyboard-layout compatibility: translate a
//! physical key event into the QWERTY-position keycode the engine expects,
//! for users on a non-QWERTY system keyboard layout.

use objc2_app_kit::NSEvent;
use objc2_core_graphics::CGEvent;

/// The character a QWERTY key
/// *position* produces (unshifted) -> that position's virtual keycode. Used
/// to map the character a non-QWERTY layout actually produced back to the
/// QWERTY-position keycode the engine assumes everything is expressed in.
fn char_to_qwerty_keycode(c: char) -> Option<u16> {
    Some(match c {
        '`' | '~' => 50,
        '1' | '!' => 18,
        '2' | '@' => 19,
        '3' | '#' => 20,
        '4' | '$' => 21,
        '5' | '%' => 23,
        '6' | '^' => 22,
        '7' | '&' => 26,
        '8' | '*' => 28,
        '9' | '(' => 25,
        '0' | ')' => 29,
        '-' | '_' => 27,
        '=' | '+' => 24,
        'q' => 12,
        'w' => 13,
        'e' => 14,
        'r' => 15,
        't' => 17,
        'y' => 16,
        'u' => 32,
        'i' => 34,
        'o' => 31,
        'p' => 35,
        '[' | '{' => 33,
        ']' | '}' => 30,
        '\\' | '|' => 42,
        'a' => 0,
        's' => 1,
        'd' => 2,
        'f' => 3,
        'g' => 5,
        'h' => 4,
        'j' => 38,
        'k' => 40,
        'l' => 37,
        ';' | ':' => 41,
        '\'' | '"' => 39,
        'z' => 6,
        'x' => 7,
        'c' => 8,
        'v' => 9,
        'b' => 11,
        'n' => 45,
        'm' => 46,
        ',' | '<' => 43,
        '.' | '>' => 47,
        '/' | '?' => 44,
        _ => return None,
    })
}

/// Lowercase the produced string and look up its first character; `fallback`
/// on any miss (empty string or an unmapped character). The map's entries are
/// all case-paired (e.g. `a`/`A` share a position), so lowercasing just the one
/// character we look up is equivalent to `NSString.lowercaseString` for this
/// closed set.
pub fn convert_key_string_to_key_code(s: &str, fallback: u16) -> u16 {
    match s.chars().next() {
        Some(c) => char_to_qwerty_keycode(c.to_ascii_lowercase()).unwrap_or(fallback),
        None => fallback,
    }
}

/// Re-read the physical key's unmodified character via
/// `NSEvent.charactersIgnoringModifiers` and map it back to the
/// QWERTY-position keycode. Returns `fallback` (the event's own keycode) if
/// either AppKit call yields nothing.
pub fn convert_event_to_layout_compat_keycode(event: &CGEvent, fallback: u16) -> u16 {
    let Some(ns_event) = NSEvent::eventWithCGEvent(event) else {
        return fallback;
    };
    let Some(chars) = ns_event.charactersIgnoringModifiers() else {
        return fallback;
    };
    convert_key_string_to_key_code(&chars.to_string(), fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qwerty_letters_map_to_expected_keycodes() {
        assert_eq!(convert_key_string_to_key_code("a", 99), 0);
        assert_eq!(convert_key_string_to_key_code("z", 99), 6);
        assert_eq!(convert_key_string_to_key_code("q", 99), 12);
        assert_eq!(convert_key_string_to_key_code("m", 99), 46);
    }

    #[test]
    fn uppercase_and_shifted_symbols_resolve_to_the_unshifted_position() {
        assert_eq!(convert_key_string_to_key_code("A", 99), 0);
        assert_eq!(convert_key_string_to_key_code("Z", 99), 6);
        assert_eq!(convert_key_string_to_key_code("!", 99), 18);
        assert_eq!(convert_key_string_to_key_code("1", 99), 18);
    }

    #[test]
    fn number_row_and_punctuation_map_correctly() {
        assert_eq!(convert_key_string_to_key_code("0", 99), 29);
        assert_eq!(convert_key_string_to_key_code(")", 99), 29);
        assert_eq!(convert_key_string_to_key_code(";", 99), 41);
        assert_eq!(convert_key_string_to_key_code("/", 99), 44);
    }

    #[test]
    fn unmapped_or_empty_string_returns_fallback() {
        assert_eq!(convert_key_string_to_key_code("", 42), 42);
        assert_eq!(convert_key_string_to_key_code("e", 42), 14); // sanity: mapped
        assert_eq!(convert_key_string_to_key_code("\u{00e9}", 42), 42); // 'é' has no entry
    }
}
