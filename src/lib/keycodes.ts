// DOM `KeyboardEvent.code` -> macOS virtual keycode (Carbon HIToolbox
// `kVK_*`, from `Events.h`), for `HotkeyRecorder`. Covers every key
// realistically usable as a hotkey chord's non-modifier key: letters,
// digits, punctuation, Return/Tab/Escape, arrows, Home/End/Page Up/Down, and
// F1-F12. Space and Backspace/Delete are handled specially by
// `HotkeyRecorder` itself (see `MyTextField.m`'s `textDidChange`), not
// through this table.
export const DOM_CODE_TO_VKEYCODE: Record<string, number> = {
  KeyA: 0x00, KeyS: 0x01, KeyD: 0x02, KeyF: 0x03, KeyH: 0x04, KeyG: 0x05, KeyZ: 0x06, KeyX: 0x07,
  KeyC: 0x08, KeyV: 0x09, KeyB: 0x0b, KeyQ: 0x0c, KeyW: 0x0d, KeyE: 0x0e, KeyR: 0x0f, KeyY: 0x10,
  KeyT: 0x11, KeyO: 0x1f, KeyU: 0x20, KeyI: 0x22, KeyP: 0x23, KeyL: 0x25, KeyJ: 0x26, KeyK: 0x28,
  KeyN: 0x2d, KeyM: 0x2e,

  Digit1: 0x12, Digit2: 0x13, Digit3: 0x14, Digit4: 0x15, Digit6: 0x16, Digit5: 0x17,
  Digit9: 0x19, Digit7: 0x1a, Digit8: 0x1c, Digit0: 0x1d,

  Equal: 0x18, Minus: 0x1b, BracketRight: 0x1e, BracketLeft: 0x21, Quote: 0x27, Semicolon: 0x29,
  Backslash: 0x2a, Comma: 0x2b, Slash: 0x2c, Period: 0x2f, Backquote: 0x32,

  Enter: 0x24, Tab: 0x30, Escape: 0x35,

  ArrowLeft: 0x7b, ArrowRight: 0x7c, ArrowDown: 0x7d, ArrowUp: 0x7e,
  Home: 0x73, End: 0x77, PageUp: 0x74, PageDown: 0x79,

  F1: 0x7a, F2: 0x78, F3: 0x63, F4: 0x76, F5: 0x60, F6: 0x61, F7: 0x62, F8: 0x64,
  F9: 0x65, F10: 0x6d, F11: 0x67, F12: 0x6f,
};

// vkeycode -> display name, for the non-printable keys above (`HotkeyRecorder`
// packs the vkeycode itself as the "char" byte for these -- see its
// `e.key.length !== 1` fallback -- so `keyDisplayText` (`lib/hotkey.ts`) needs
// this to show e.g. "Enter"/"F1" instead of `String.fromCharCode` on the
// vkeycode's numeric value, which coincidentally spells out an unrelated
// ASCII character (0x24 -> "$", 0x7a -> "z").
export const VKEYCODE_TO_DISPLAY_NAME: Record<number, string> = {
  [DOM_CODE_TO_VKEYCODE.Enter]: "Enter",
  [DOM_CODE_TO_VKEYCODE.Tab]: "Tab",
  [DOM_CODE_TO_VKEYCODE.Escape]: "Escape",
  [DOM_CODE_TO_VKEYCODE.ArrowLeft]: "←",
  [DOM_CODE_TO_VKEYCODE.ArrowRight]: "→",
  [DOM_CODE_TO_VKEYCODE.ArrowUp]: "↑",
  [DOM_CODE_TO_VKEYCODE.ArrowDown]: "↓",
  [DOM_CODE_TO_VKEYCODE.Home]: "Home",
  [DOM_CODE_TO_VKEYCODE.End]: "End",
  [DOM_CODE_TO_VKEYCODE.PageUp]: "Page Up",
  [DOM_CODE_TO_VKEYCODE.PageDown]: "Page Down",
  [DOM_CODE_TO_VKEYCODE.F1]: "F1",
  [DOM_CODE_TO_VKEYCODE.F2]: "F2",
  [DOM_CODE_TO_VKEYCODE.F3]: "F3",
  [DOM_CODE_TO_VKEYCODE.F4]: "F4",
  [DOM_CODE_TO_VKEYCODE.F5]: "F5",
  [DOM_CODE_TO_VKEYCODE.F6]: "F6",
  [DOM_CODE_TO_VKEYCODE.F7]: "F7",
  [DOM_CODE_TO_VKEYCODE.F8]: "F8",
  [DOM_CODE_TO_VKEYCODE.F9]: "F9",
  [DOM_CODE_TO_VKEYCODE.F10]: "F10",
  [DOM_CODE_TO_VKEYCODE.F11]: "F11",
  [DOM_CODE_TO_VKEYCODE.F12]: "F12",
};
