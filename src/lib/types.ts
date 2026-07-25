// Mirrors `src-tauri/src/settings/mod.rs`'s `Settings` struct field-for-field.
// Field names are exact Rust snake_case: the struct has no `rename_all`, so
// serde serializes it with these names verbatim over the wire.
export interface Settings {
  schema_version: number;

  // Engine
  language: number;
  input_type: number;
  code_table: number;
  free_mark: boolean;
  check_spelling: boolean;
  modern_orthography: boolean;
  restore_if_wrong_spelling: boolean;
  fix_recommend_browser: boolean;
  use_macro: boolean;
  use_macro_in_english_mode: boolean;
  auto_caps_macro: boolean;
  use_smart_switch_key: boolean;
  upper_case_first_char: boolean;
  temp_off_spelling: boolean;
  allow_consonant_zfwj: boolean;
  quick_start_consonant: boolean;
  quick_end_consonant: boolean;
  remember_code: boolean;
  other_language: boolean;
  temp_off_engine: boolean;
  auto_restore_english: boolean;
  esc_restore_english: boolean;
  quick_telex: boolean;

  // Platform
  switch_key_status: number;
  send_key_step_by_step: boolean;
  fix_chromium_browser: boolean;
  perform_layout_compat: boolean;
  show_icon_on_dock: boolean;
  gray_icon: boolean;
  run_on_startup: boolean;
  show_ui_on_startup: boolean;
  dont_check_update: boolean;
  // UI appearance: 0 = follow OS, 1 = light, 2 = dark. See lib/theme.ts.
  theme: number;

  // Convert tool
  convert_from_code: number;
  convert_to_code: number;
  convert_remove_mark: boolean;
  convert_to_all_caps: boolean;
  convert_to_all_non_caps: boolean;
  convert_to_caps_first_letter: boolean;
  convert_to_caps_each_word: boolean;
  convert_hotkey: number;
  convert_dont_alert: boolean;
}

// `crates/vnkey-engine/src/data.rs` discriminants.
export enum InputType {
  Telex = 0,
  Vni = 1,
  SimpleTelex1 = 2,
  SimpleTelex2 = 3,
}

export enum CodeTable {
  Unicode = 0,
  Tcvn3 = 1,
  VniWindows = 2,
  UnicodeCompound = 3,
  Cp1258 = 4,
}

// `src-tauri/src/events.rs`'s `StatusChanged` payload.
export interface StatusChanged {
  language: number;
  input_type: number;
  code_table: number;
}

// `src-tauri/src/commands.rs`'s `MacroEntry`.
export interface MacroEntry {
  text: string;
  content: string;
}

// `src-tauri/src/commands.rs`'s `UpdateInfo`.
export interface UpdateInfo {
  version: string;
  url: string;
}

// `src-tauri/src/commands.rs`'s `ConvertOptions` (the `convert_clipboard`
// argument shape) -- field names match the Rust struct verbatim.
export interface ConvertOptions {
  from_code: number;
  to_code: number;
  remove_mark: boolean;
  to_all_caps: boolean;
  to_all_non_caps: boolean;
  to_caps_first_letter: boolean;
  to_caps_each_word: boolean;
}
