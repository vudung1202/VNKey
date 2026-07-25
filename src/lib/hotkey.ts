import { VKEYCODE_TO_DISPLAY_NAME } from "./keycodes";

// Packed hotkey (u32) encode/decode. The bit layout is shared with the Rust
// side (`switch_key_status`), so both ends must agree exactly:
//   bit 0x100  = Control
//   bit 0x200  = Option
//   bit 0x400  = Command
//   bit 0x800  = Shift
//   bit 0x8000 = beep on switch
//   low byte (0xFF)          = macOS virtual keycode
//   top byte (>>24 & 0xFF)   = display character
//
// `MODIFIER_ONLY_KEYCODE` (0xFE) is stored in BOTH the keycode and char
// bytes when the key portion is cleared (Delete/ForwardDelete in
// `MyTextField.m`). `SPACE_KEYCODE` (`kVK_Space`, 0x31) is likewise stored
// in both bytes for the Space case -- `textDidChange` passes `kVK_Space` as
// both the `keyCode:` and `character:` delegate arguments.
export const MODIFIER_ONLY_KEYCODE = 0xfe;
export const SPACE_KEYCODE = 0x31;

const CTRL_BIT = 0x100;
const OPT_BIT = 0x200;
const CMD_BIT = 0x400;
const SHIFT_BIT = 0x800;
const BEEP_BIT = 0x8000;

export interface DecodedHotkey {
  ctrl: boolean;
  opt: boolean;
  cmd: boolean;
  shift: boolean;
  beep: boolean;
  keycode: number;
  char: number;
}

export function decodeHotkey(packed: number): DecodedHotkey {
  return {
    ctrl: (packed & CTRL_BIT) !== 0,
    opt: (packed & OPT_BIT) !== 0,
    cmd: (packed & CMD_BIT) !== 0,
    shift: (packed & SHIFT_BIT) !== 0,
    beep: (packed & BEEP_BIT) !== 0,
    keycode: packed & 0xff,
    char: (packed >>> 24) & 0xff,
  };
}

export function encodeHotkey(hotkey: DecodedHotkey): number {
  let packed = hotkey.keycode & 0xff;
  packed |= (hotkey.char & 0xff) << 24;
  if (hotkey.ctrl) packed |= CTRL_BIT;
  if (hotkey.opt) packed |= OPT_BIT;
  if (hotkey.cmd) packed |= CMD_BIT;
  if (hotkey.shift) packed |= SHIFT_BIT;
  if (hotkey.beep) packed |= BEEP_BIT;
  return packed >>> 0;
}

// Mirrors `MyTextField.m`'s `setTextByChar:`, extended to show a proper name
// for the non-printable keys in `keycodes.ts`'s `VKEYCODE_TO_DISPLAY_NAME`
// (Enter/Tab/Escape/arrows/Home/End/PageUp/PageDown/F1-F12) instead of
// `String.fromCharCode` on their vkeycode, which is what gets packed as
// `char` for those keys (see `HotkeyRecorder`) and does not represent an
// actual character.
export function keyDisplayText(char: number): string {
  if (char === SPACE_KEYCODE) return "Space";
  if (char === MODIFIER_ONLY_KEYCODE) return "";
  return VKEYCODE_TO_DISPLAY_NAME[char] ?? String.fromCharCode(char);
}
