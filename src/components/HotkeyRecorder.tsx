import type { KeyboardEvent } from "react";

import { setHotkeyRecording } from "../lib/ipc";
import { keyDisplayText, MODIFIER_ONLY_KEYCODE, SPACE_KEYCODE } from "../lib/hotkey";
import { DOM_CODE_TO_VKEYCODE } from "../lib/keycodes";

/**
 * Ported from OpenKey's `MyTextField` (`MyTextField.m`'s `textDidChange`):
 * captures only the non-modifier key of a hotkey chord (macOS virtual
 * keycode + a display character), NOT the modifiers -- those are separate
 * checkboxes owned by `TopBar`, which combines everything into the packed
 * `switch_key_status` u32.
 *
 * While focused, the CGEventTap must not swallow/rewrite the chord being
 * recorded, so focus/blur toggle `set_hotkey_recording` (Task 13 addition,
 * `commands.rs` + `SharedConfig.hotkey_recording`, checked in `tap.rs` right
 * after the self-event filter).
 */
export function HotkeyRecorder({
  char,
  onChange,
}: {
  char: number;
  onChange: (keycode: number, char: number) => void;
}) {
  function handleKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    e.preventDefault();

    if (e.code === "Space") {
      onChange(SPACE_KEYCODE, SPACE_KEYCODE);
      return;
    }
    if (e.code === "Backspace" || e.code === "Delete") {
      onChange(MODIFIER_ONLY_KEYCODE, MODIFIER_ONLY_KEYCODE);
      return;
    }

    const vkeycode = DOM_CODE_TO_VKEYCODE[e.code];
    if (vkeycode === undefined) return;

    const displayChar = e.key.length === 1 ? e.key.toUpperCase().charCodeAt(0) : vkeycode;
    onChange(vkeycode, displayChar);
  }

  return (
    <input
      className="ui-kbd"
      type="text"
      readOnly
      value={keyDisplayText(char)}
      placeholder="…"
      onFocus={() => void setHotkeyRecording(true)}
      onBlur={() => void setHotkeyRecording(false)}
      onKeyDown={handleKeyDown}
    />
  );
}
