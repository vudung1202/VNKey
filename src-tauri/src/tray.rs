//! The 15-item status-bar menu (`AppDelegate.m: createStatusBarMenu`,
//! `201-248`) and the single [`update_tray`] entry point every state-change
//! source (menu click, IPC command, hotkey, smart-switch) funnels through to
//! keep it in sync.
//!
//! Icon approach: rather than shipping separate V/E icon assets and toggling
//! them (OpenKey's `Status`/`StatusHighlighted` images), the tray keeps one
//! static app icon and uses [`tauri::tray::TrayIcon::set_title`] to show the
//! language state as literal "V"/"E" text next to it -- one code path instead
//! of an icon-swap-plus-template-image one, and no extra asset files.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};
use vnkey_engine::{CodeTable, InputType};

use crate::settings::{input_type_from_u8, Settings};
use crate::state::SharedConfig;

/// Handles to every tray widget whose state [`apply_tray_state`] needs to
/// update, managed as Tauri state so it's reachable from any menu-event or
/// IPC callback without re-building the menu.
struct TrayHandles {
    tray: tauri::tray::TrayIcon,
    toggle_language: CheckMenuItem<tauri::Wry>,
    input_type_telex: CheckMenuItem<tauri::Wry>,
    input_type_vni: CheckMenuItem<tauri::Wry>,
    input_type_simple_telex_1: CheckMenuItem<tauri::Wry>,
    input_type_simple_telex_2: CheckMenuItem<tauri::Wry>,
    code_table_unicode: CheckMenuItem<tauri::Wry>,
    code_table_tcvn3: CheckMenuItem<tauri::Wry>,
    code_table_vni_windows: CheckMenuItem<tauri::Wry>,
    code_table_unicode_compound: CheckMenuItem<tauri::Wry>,
    code_table_cp1258: CheckMenuItem<tauri::Wry>,
    quick_convert: MenuItem<tauri::Wry>,
}

/// The subset of [`Settings`] the tray displays, computed once so both
/// [`apply_tray_state`] and its tests work off one pure function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayState {
    pub title: &'static str,
    pub language_checked: bool,
    pub input_type: InputType,
    pub code_table: CodeTable,
}

/// Pure: `Settings` -> what the tray should show. No AppKit, no locks --
/// table-tested directly.
pub fn compute_tray_state(settings: &Settings) -> TrayState {
    let language_checked = settings.language == 1;
    TrayState {
        title: if language_checked { "V" } else { "E" },
        language_checked,
        input_type: input_type_from_u8(settings.input_type),
        code_table: CodeTable::from_index(settings.code_table).unwrap_or(CodeTable::Unicode),
    }
}

/// Port of `AppDelegate.m: setQuickConvertString` (`250-278`). `hotkey` uses
/// the same encoding as `switch_key_status`
/// (`ConvertToolViewController.mm: onMyTextFieldKeyChange`): bits
/// 0x100/0x200/0x400/0x800 are Control/Option/Command/Shift, and the top byte
/// is the pressed key's *character* (not a virtual keycode) -- `0xFE` means
/// "no key recorded".
///
/// One deliberate fix, not a literal port: OpenKey compares the top byte
/// against `kVK_Space` (0x31, a Carbon virtual keycode), but the top byte
/// holds an ASCII character, and space's character is 0x20 -- the comparison
/// can never be true, so OpenKey's "show ␣ for the space bar" case is dead
/// code. This compares against the actual ASCII space (`b' '`) so it works as
/// evidently intended.
pub fn quick_convert_title(hotkey: u32) -> String {
    let mut label = String::new();
    let mut has_add = false;

    if hotkey & 0x100 != 0 {
        label.push('⌃');
        has_add = true;
    }
    if hotkey & 0x200 != 0 {
        if has_add {
            label.push_str(" + ");
        }
        label.push('⌥');
        has_add = true;
    }
    if hotkey & 0x400 != 0 {
        if has_add {
            label.push_str(" + ");
        }
        label.push('⌘');
        has_add = true;
    }
    if hotkey & 0x800 != 0 {
        if has_add {
            label.push_str(" + ");
        }
        label.push('⇧');
        has_add = true;
    }

    let key = ((hotkey >> 24) & 0xFF) as u8;
    if key != 0xFE {
        if has_add {
            label.push_str(" + ");
        }
        if key == b' ' {
            label.push_str("␣ ");
        } else {
            label.push(key as char);
        }
    }

    if has_add {
        format!("Chuyển mã nhanh - [{}]", label.to_uppercase())
    } else {
        "Chuyển mã nhanh".to_string()
    }
}

/// A [`Settings`] snapshot for the tray/event updates that don't go through
/// [`crate::commands::apply_and_broadcast`]: the hotkey (tap thread) and
/// smart-switch (main-thread observer) paths change [`SharedConfig`]'s
/// atomics directly, without persisting (`hotkey::switch_language`'s and
/// `smart_switch::on_active_app`'s doc comments). Overlays the two fields
/// those paths can change onto the last-saved settings.
pub(crate) fn live_settings(config: &SharedConfig) -> Settings {
    overlay_live_config(Settings::load(), config)
}

/// The pure half of [`live_settings`], split out so tests can supply a base
/// [`Settings`] instead of going through the real, hardcoded-path
/// `Settings::load`. `pub(crate)` so [`crate::commands::apply_and_broadcast_core`]
/// can also drive it off an injected base settings value (Task 12 review fix).
pub(crate) fn overlay_live_config(mut settings: Settings, config: &SharedConfig) -> Settings {
    settings.language = if config.enabled.load(Ordering::Relaxed) { 1 } else { 0 };
    settings.code_table = config.code_table() as u8;
    settings
}

/// Build the 15-item menu and tray icon and manage the handles
/// [`apply_tray_state`] needs. Called once from `setup`.
pub fn build(app: &AppHandle, settings: &Settings) -> tauri::Result<()> {
    let state = compute_tray_state(settings);

    let toggle_language =
        CheckMenuItem::with_id(app, "toggle_language", "Bật Tiếng Việt", true, state.language_checked, None::<&str>)?;

    let input_type_telex = CheckMenuItem::with_id(
        app,
        "input_type_telex",
        "Telex",
        true,
        state.input_type == InputType::Telex,
        None::<&str>,
    )?;
    let input_type_vni = CheckMenuItem::with_id(
        app,
        "input_type_vni",
        "VNI",
        true,
        state.input_type == InputType::Vni,
        None::<&str>,
    )?;
    let input_type_simple_telex_1 = CheckMenuItem::with_id(
        app,
        "input_type_simple_telex_1",
        "Simple Telex 1",
        true,
        state.input_type == InputType::SimpleTelex1,
        None::<&str>,
    )?;
    let input_type_simple_telex_2 = CheckMenuItem::with_id(
        app,
        "input_type_simple_telex_2",
        "Simple Telex 2",
        true,
        state.input_type == InputType::SimpleTelex2,
        None::<&str>,
    )?;
    let input_type_submenu = Submenu::with_id_and_items(
        app,
        "input_type_submenu",
        "Kiểu gõ",
        true,
        &[
            &input_type_telex,
            &input_type_vni,
            &input_type_simple_telex_1,
            &input_type_simple_telex_2,
        ],
    )?;

    let code_table_unicode = CheckMenuItem::with_id(
        app,
        "code_table_unicode",
        "Unicode dựng sẵn",
        true,
        state.code_table == CodeTable::Unicode,
        None::<&str>,
    )?;
    let code_table_tcvn3 = CheckMenuItem::with_id(
        app,
        "code_table_tcvn3",
        "TCVN3 (ABC)",
        true,
        state.code_table == CodeTable::Tcvn3,
        None::<&str>,
    )?;
    let code_table_vni_windows = CheckMenuItem::with_id(
        app,
        "code_table_vni_windows",
        "VNI Windows",
        true,
        state.code_table == CodeTable::VniWindows,
        None::<&str>,
    )?;

    let code_table_unicode_compound = CheckMenuItem::with_id(
        app,
        "code_table_unicode_compound",
        "Unicode tổ hợp",
        true,
        state.code_table == CodeTable::UnicodeCompound,
        None::<&str>,
    )?;
    let code_table_cp1258 = CheckMenuItem::with_id(
        app,
        "code_table_cp1258",
        "Vietnamese Locale CP 1258",
        true,
        state.code_table == CodeTable::Cp1258,
        None::<&str>,
    )?;
    let code_table_other_submenu = Submenu::with_id_and_items(
        app,
        "code_table_other_submenu",
        "Bảng mã khác",
        true,
        &[&code_table_unicode_compound, &code_table_cp1258],
    )?;

    let open_convert = MenuItem::with_id(app, "open_convert", "Công cụ chuyển mã...", true, None::<&str>)?;
    let quick_convert =
        MenuItem::with_id(app, "quick_convert", quick_convert_title(settings.convert_hotkey), true, None::<&str>)?;

    let open_settings = MenuItem::with_id(app, "open_settings", "Bảng điều khiển...", true, None::<&str>)?;
    let open_macro = MenuItem::with_id(app, "open_macro", "Gõ tắt...", true, None::<&str>)?;
    let open_about = MenuItem::with_id(app, "open_about", "Giới thiệu", true, None::<&str>)?;

    let quit = MenuItem::with_id(app, "quit", "Thoát", true, Some("Cmd+Q"))?;

    let menu = Menu::with_items(
        app,
        &[
            &toggle_language,
            &PredefinedMenuItem::separator(app)?,
            &input_type_submenu,
            &PredefinedMenuItem::separator(app)?,
            &code_table_unicode,
            &code_table_tcvn3,
            &code_table_vni_windows,
            &code_table_other_submenu,
            &PredefinedMenuItem::separator(app)?,
            &open_convert,
            &quick_convert,
            &PredefinedMenuItem::separator(app)?,
            &open_settings,
            &open_macro,
            &open_about,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().expect("bundle icon configured in tauri.conf.json").clone())
        .menu(&menu)
        .title(state.title)
        .on_menu_event(handle_menu_event)
        .build(app)?;

    app.manage(TrayHandles {
        tray,
        toggle_language,
        input_type_telex,
        input_type_vni,
        input_type_simple_telex_1,
        input_type_simple_telex_2,
        code_table_unicode,
        code_table_tcvn3,
        code_table_vni_windows,
        code_table_unicode_compound,
        code_table_cp1258,
        quick_convert,
    });

    Ok(())
}

/// The one function every state-change source calls, on the main thread
/// (`app.run_on_main_thread`) since it touches AppKit-backed menu/tray
/// objects. Non-blocking: schedules `apply_tray_state` and returns.
pub fn update_tray(app: &AppHandle, settings: Settings) {
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || apply_tray_state(&app, &settings));
}

/// The actual main-thread work `update_tray` schedules: recompute
/// [`TrayState`] and push it into every menu item + the tray title. Also
/// callable directly by callers already known to be on the main thread (the
/// smart-switch NSWorkspace observer).
pub(crate) fn apply_tray_state(app: &AppHandle, settings: &Settings) {
    let Some(handles) = app.try_state::<TrayHandles>() else { return };
    let state = compute_tray_state(settings);

    let _ = handles.tray.set_title(Some(state.title));
    let _ = handles.toggle_language.set_checked(state.language_checked);

    let _ = handles.input_type_telex.set_checked(state.input_type == InputType::Telex);
    let _ = handles.input_type_vni.set_checked(state.input_type == InputType::Vni);
    let _ = handles
        .input_type_simple_telex_1
        .set_checked(state.input_type == InputType::SimpleTelex1);
    let _ = handles
        .input_type_simple_telex_2
        .set_checked(state.input_type == InputType::SimpleTelex2);

    let _ = handles.code_table_unicode.set_checked(state.code_table == CodeTable::Unicode);
    let _ = handles.code_table_tcvn3.set_checked(state.code_table == CodeTable::Tcvn3);
    let _ = handles
        .code_table_vni_windows
        .set_checked(state.code_table == CodeTable::VniWindows);
    let _ = handles
        .code_table_unicode_compound
        .set_checked(state.code_table == CodeTable::UnicodeCompound);
    let _ = handles.code_table_cp1258.set_checked(state.code_table == CodeTable::Cp1258);

    let _ = handles.quick_convert.set_text(quick_convert_title(settings.convert_hotkey));
}

fn show_window(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// "Chuyển mã nhanh" / the convert hotkey: convert the clipboard using
/// whatever the user last configured in the convert-tool window (unlike
/// `commands::convert_clipboard`, which takes options from the IPC caller).
fn quick_convert_now() {
    let options = Settings::load().convert_options();
    let _ = crate::pasteboard::quick_convert(&crate::pasteboard::SystemClipboard, &options);
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id.as_ref() {
        "toggle_language" => {
            crate::commands::apply_and_broadcast(app, |s| {
                let next = if s.language == 1 { 0 } else { 1 };
                crate::commands::set_language_settings(s, next)
            });
        }
        "input_type_telex" => set_input_type(app, InputType::Telex as u8),
        "input_type_vni" => set_input_type(app, InputType::Vni as u8),
        "input_type_simple_telex_1" => set_input_type(app, InputType::SimpleTelex1 as u8),
        "input_type_simple_telex_2" => set_input_type(app, InputType::SimpleTelex2 as u8),
        "code_table_unicode" => set_code_table(app, CodeTable::Unicode as u8),
        "code_table_tcvn3" => set_code_table(app, CodeTable::Tcvn3 as u8),
        "code_table_vni_windows" => set_code_table(app, CodeTable::VniWindows as u8),
        "code_table_unicode_compound" => set_code_table(app, CodeTable::UnicodeCompound as u8),
        "code_table_cp1258" => set_code_table(app, CodeTable::Cp1258 as u8),
        "open_convert" => show_window(app, "convert"),
        "quick_convert" => quick_convert_now(),
        "open_settings" => show_window(app, "settings"),
        "open_macro" => show_window(app, "macro"),
        "open_about" => show_window(app, "about"),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn set_input_type(app: &AppHandle, input_type: u8) {
    crate::commands::apply_and_broadcast(app, |s| crate::commands::set_input_type_settings(s, input_type));
}

fn set_code_table(app: &AppHandle, code_table: u8) {
    crate::commands::apply_and_broadcast(app, |s| crate::commands::set_code_table_settings(s, code_table));
}

/// Called from the hotkey path (tap thread, fire-and-forget) and the
/// smart-switch observer (already on the main thread) to sync the tray +
/// emit `status-changed`/`settings-changed` after a change that bypasses
/// `apply_and_broadcast`.
///
/// Also persists the live state (Task 12 review fix): both callers are on
/// the main thread by the time this runs (the hotkey path hops here via
/// `notify_language_switched`'s `run_on_main_thread`; the smart-switch
/// observer is already on it), so a plain `Settings::save` here is safe --
/// no AppKit/disk I/O is added to the tap thread's callback. Without this, a
/// manual language switch or smart-switch code-table change would be lost on
/// restart unless a later menu/IPC action happened to save it first.
///
/// Also emits `settings-changed` (Task 13 review fix) so the Control Panel,
/// which mirrors `Settings` purely off that event, picks up hotkey/smart-
/// switch-driven changes too -- not just changes made through IPC commands.
pub(crate) fn sync_after_live_change(app: &AppHandle, config: &SharedConfig) {
    let settings = live_settings(config);
    apply_tray_state(app, &settings);
    crate::events::status_changed(app, &settings);
    crate::events::settings_changed(app, &settings);
    let _ = settings.save();
}

/// Fire-and-forget: post `sync_after_live_change` to the main thread and
/// return immediately, for callers on the tap thread that must not block
/// (`hotkey::switch_language`'s doc comment).
pub(crate) fn notify_language_switched(app: &AppHandle, config: &Arc<SharedConfig>) {
    let app = app.clone();
    let config = config.clone();
    let _ = app.clone().run_on_main_thread(move || sync_after_live_change(&app, &config));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_tray_state_table() {
        struct Case {
            settings: Settings,
            expected: TrayState,
        }

        let cases = [
            Case {
                settings: Settings::default(),
                expected: TrayState {
                    title: "V",
                    language_checked: true,
                    input_type: InputType::Telex,
                    code_table: CodeTable::Unicode,
                },
            },
            Case {
                settings: Settings { language: 0, ..Settings::default() },
                expected: TrayState {
                    title: "E",
                    language_checked: false,
                    input_type: InputType::Telex,
                    code_table: CodeTable::Unicode,
                },
            },
            Case {
                settings: Settings { input_type: InputType::Vni as u8, ..Settings::default() },
                expected: TrayState {
                    title: "V",
                    language_checked: true,
                    input_type: InputType::Vni,
                    code_table: CodeTable::Unicode,
                },
            },
            Case {
                settings: Settings { code_table: CodeTable::Tcvn3 as u8, ..Settings::default() },
                expected: TrayState {
                    title: "V",
                    language_checked: true,
                    input_type: InputType::Telex,
                    code_table: CodeTable::Tcvn3,
                },
            },
            Case {
                settings: Settings { code_table: CodeTable::Cp1258 as u8, ..Settings::default() },
                expected: TrayState {
                    title: "V",
                    language_checked: true,
                    input_type: InputType::Telex,
                    code_table: CodeTable::Cp1258,
                },
            },
            Case {
                settings: Settings { code_table: 99, ..Settings::default() },
                expected: TrayState {
                    title: "V",
                    language_checked: true,
                    input_type: InputType::Telex,
                    code_table: CodeTable::Unicode,
                },
            },
        ];

        for case in cases {
            assert_eq!(compute_tray_state(&case.settings), case.expected);
        }
    }

    #[test]
    fn quick_convert_title_no_hotkey_configured_is_bare_label() {
        assert_eq!(quick_convert_title(0xFE00_00FE), "Chuyển mã nhanh");
    }

    #[test]
    fn quick_convert_title_single_modifier_and_key() {
        // Option + Z (same encoding shape as `DEFAULT_SWITCH_KEY_STATUS`).
        assert_eq!(quick_convert_title(0x7A00_0200 | 0x06), "Chuyển mã nhanh - [⌥ + Z]");
    }

    #[test]
    fn quick_convert_title_multiple_modifiers_in_ctrl_opt_cmd_shift_order() {
        let hotkey = (b'a' as u32) << 24 | 0x100 | 0x400 | 0x800;
        assert_eq!(quick_convert_title(hotkey), "Chuyển mã nhanh - [⌃ + ⌘ + ⇧ + A]");
    }

    #[test]
    fn quick_convert_title_space_key_shows_open_box_symbol() {
        let hotkey = (b' ' as u32) << 24 | 0x400;
        assert_eq!(quick_convert_title(hotkey), "Chuyển mã nhanh - [⌘ + ␣ ]");
    }

    #[test]
    fn quick_convert_title_key_without_any_modifier_stays_unconfigured() {
        // Faithful to `AppDelegate.m:250-278`: `hasAdd` is only set by the
        // modifier-bit branches, so a key with no modifier held still
        // produces the bare label -- not a bug fix, just what the source
        // does.
        let hotkey = (b'a' as u32) << 24;
        assert_eq!(quick_convert_title(hotkey), "Chuyển mã nhanh");
    }

    #[test]
    fn overlay_live_config_overwrites_only_language_and_code_table() {
        let base = Settings { convert_hotkey: 0xABCD, ..Settings::default() };
        let config = SharedConfig::new(&base.to_engine_options());
        config.enabled.store(false, Ordering::Relaxed);
        config.set_code_table(CodeTable::VniWindows);

        let live = overlay_live_config(base, &config);
        assert_eq!(live.language, 0);
        assert_eq!(live.code_table, CodeTable::VniWindows as u8);
        // Everything else passes through from `base` untouched.
        assert_eq!(live, Settings { language: 0, code_table: CodeTable::VniWindows as u8, ..base });
    }
}
