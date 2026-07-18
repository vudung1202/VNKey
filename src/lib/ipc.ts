// Typed `invoke()` wrappers for every IPC command Task 12 exposed to the UI,
// plus `set_hotkey_recording` (added this task, see HotkeyRecorder.tsx).
//
// Tauri's `#[tauri::command]` macro exposes Rust snake_case PARAMETERS to JS
// as camelCase keys (`ArgumentCase::Camel`, the v2 default) -- e.g. Rust's
// `input_type` becomes the JS payload key `inputType`. This does NOT apply to
// nested struct fields (`Settings`, `ConvertOptions`): those keep their exact
// Rust snake_case field names, since neither struct uses `rename_all`.
import { invoke } from "@tauri-apps/api/core";

import type { ConvertOptions, MacroEntry, Settings, UpdateInfo } from "./types";

export function getSettings(): Promise<Settings> {
  return invoke("get_settings");
}

export function setSettings(settings: Settings): Promise<void> {
  return invoke("set_settings", { settings });
}

// Most checkboxes have no dedicated command (only language/input-type/
// code-table/hotkeys do) -- their "corresponding command" per the task
// brief is `set_settings` with the full struct, one field patched.
export function patchSettings(current: Settings, patch: Partial<Settings>): Promise<void> {
  return setSettings({ ...current, ...patch });
}

export function setLanguage(language: number): Promise<void> {
  return invoke("set_language", { language });
}

export function setInputType(inputType: number): Promise<void> {
  return invoke("set_input_type", { inputType });
}

export function setCodeTable(codeTable: number): Promise<void> {
  return invoke("set_code_table", { codeTable });
}

export function setSwitchHotkey(hotkey: number): Promise<void> {
  return invoke("set_switch_hotkey", { hotkey });
}

export function setConvertHotkey(hotkey: number): Promise<void> {
  return invoke("set_convert_hotkey", { hotkey });
}

export function restoreDefaultSettings(): Promise<Settings> {
  return invoke("restore_default_settings");
}

export function macroList(): Promise<MacroEntry[]> {
  return invoke("macro_list");
}

export function macroAdd(text: string, content: string): Promise<void> {
  return invoke("macro_add", { text, content });
}

export function macroDelete(text: string): Promise<void> {
  return invoke("macro_delete", { text });
}

export function macroImport(path: string, append: boolean): Promise<number> {
  return invoke("macro_import", { path, append });
}

export function macroExport(path: string): Promise<void> {
  return invoke("macro_export", { path });
}

export function convertClipboard(options: ConvertOptions): Promise<void> {
  return invoke("convert_clipboard", { options });
}

export function getPermissionStatus(): Promise<boolean> {
  return invoke("get_permission_status");
}

export function openAccessibilitySettings(): Promise<void> {
  return invoke("open_accessibility_settings");
}

export function restartApp(): Promise<void> {
  return invoke("restart_app");
}

export function hasOpenKeyData(): Promise<boolean> {
  return invoke("has_openkey_data");
}

export function importOpenKeySettings(): Promise<Settings> {
  return invoke("import_openkey_settings");
}

export function checkUpdate(): Promise<UpdateInfo | null> {
  return invoke("check_update");
}

export function quitApp(): Promise<void> {
  return invoke("quit_app");
}

// Task 13 addition: the `HotkeyRecorder` calls this with `true` on focus and
// `false` on blur so the CGEventTap passes keys through untouched while it
// captures a new hotkey chord.
export function setHotkeyRecording(recording: boolean): Promise<void> {
  return invoke("set_hotkey_recording", { recording });
}
