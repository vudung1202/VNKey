//! The status-bar menu and the single [`apply_tray_state`] entry point every
//! state-change source (menu click, UI action, hotkey, smart-switch) funnels
//! through to keep it in sync.
//!
//! GPUI has no tray API, so this drives `tray-icon`/`muda` directly (the same
//! crates Tauri wrapped). Menu clicks arrive on the main thread through
//! `MenuEvent`'s handler, which only forwards them as [`AppEvent::Menu`] so
//! the actual handling gets a `&mut App`.
//!
//! The menu is deliberately short -- six entries: the language toggle, the
//! "Kiểu gõ"/"Bảng mã" submenus, "Bảng điều khiển...", "Gõ tắt...", "Thoát".
//! Everything else (the convert tool, the About text) lives in the Control
//! Panel's tabs instead, so the status-bar menu stays a quick-switch surface
//! rather than a second settings UI.
//!
//! Icon approach: the menu bar shows the language state as literal "V"/"E"
//! text only -- no image at all. This is a deliberate product choice: a
//! compact text-only status item, one code path instead of an
//! icon-swap-plus-template-image one, and no extra asset files. The app logo
//! lives on the Dock icon and in the About tab, not here.

use std::sync::atomic::Ordering;

use gpui_kit::{App, Global};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{TrayIcon, TrayIconBuilder};
use vnkey_engine::{CodeTable, InputType};

use crate::app::{AppEvent, AppSender, Core};
use crate::settings::{Settings, input_type_from_u8};
use crate::state::SharedConfig;

/// Every tray widget whose state [`apply_tray_state`] updates, kept as a
/// global (main-thread only, like the AppKit objects behind it).
struct TrayHandles {
    tray: TrayIcon,
    toggle_language: CheckMenuItem,
    input_types: [(InputType, CheckMenuItem); 4],
    code_tables: [(CodeTable, CheckMenuItem); 5],
}

impl Global for TrayHandles {}

const INPUT_TYPES: [(InputType, &str, &str); 4] = [
    (InputType::Telex, "input_type_telex", "Telex"),
    (InputType::Vni, "input_type_vni", "VNI"),
    (InputType::SimpleTelex1, "input_type_simple_telex_1", "Simple Telex 1"),
    (InputType::SimpleTelex2, "input_type_simple_telex_2", "Simple Telex 2"),
];

const CODE_TABLES: [(CodeTable, &str, &str); 5] = [
    (CodeTable::Unicode, "code_table_unicode", "Unicode dựng sẵn"),
    (CodeTable::Tcvn3, "code_table_tcvn3", "TCVN3 (ABC)"),
    (CodeTable::VniWindows, "code_table_vni_windows", "VNI Windows"),
    (CodeTable::UnicodeCompound, "code_table_unicode_compound", "Unicode tổ hợp"),
    (CodeTable::Cp1258, "code_table_cp1258", "Vietnamese Locale CP 1258"),
];

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

/// Render the menu-bar letter as a bold, larger, vertically-centered title.
///
/// `tray-icon` only sets a plain, regular-weight title, and a plain title in a
/// bigger font sits too high: a capital has no descender, so centering it in
/// the line box leaves the glyph near the top of the bar. This sets an
/// attributed title -- a bold system font plus a negative baseline offset that
/// nudges the glyph back down to the optical center.
///
/// `set_title` would overwrite the attributed title, so [`apply_tray_state`]
/// calls this on every change instead. Main thread only (all callers are).
fn set_tray_title(tray: &TrayIcon, letter: &'static str) {
    use objc2::MainThreadMarker;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{NSBaselineOffsetAttributeName, NSFont, NSFontAttributeName};
    use objc2_foundation::{NSAttributedString, NSDictionary, NSNumber, NSString};

    let Some(mtm) = MainThreadMarker::new() else { return };
    let Some(item) = tray.ns_status_item() else { return };
    // Fixed width so the item (and everything left of it) doesn't shift
    // when the letter toggles between the narrower "V" and wider "E".
    item.setLength(26.0);
    let Some(button) = item.button(mtm) else { return };

    let font = NSFont::boldSystemFontOfSize(18.0);
    let offset = NSNumber::numberWithDouble(-1.5);
    let font_obj: &AnyObject = &font;
    let offset_obj: &AnyObject = &offset;
    // The attribute-name constants are extern statics (unsafe to read).
    let (font_key, baseline_key) = unsafe { (NSFontAttributeName, NSBaselineOffsetAttributeName) };
    let attrs: objc2::rc::Retained<NSDictionary<NSString, AnyObject>> =
        NSDictionary::from_slices(&[font_key, baseline_key], &[font_obj, offset_obj]);

    let title = NSString::from_str(letter);
    let attributed = unsafe { NSAttributedString::initWithString_attributes(mtm.alloc(), &title, Some(&attrs)) };
    button.setAttributedTitle(&attributed);
}

/// Build the menu and the status item. Called once at startup, on the main
/// thread with the event loop already running (tray-icon requires both).
pub fn build(settings: &Settings, sender: AppSender, cx: &mut App) {
    let state = compute_tray_state(settings);
    let check = |id: &str, text: &str, checked: bool| CheckMenuItem::with_id(id, text, true, checked, None);

    let toggle_language = check("toggle_language", "Bật Tiếng Việt", state.language_checked);
    let input_types = INPUT_TYPES.map(|(ty, id, text)| (ty, check(id, text, state.input_type == ty)));
    let code_tables = CODE_TABLES.map(|(ct, id, text)| (ct, check(id, text, state.code_table == ct)));

    let input_type_items: Vec<&dyn tray_icon::menu::IsMenuItem> =
        input_types.iter().map(|(_, item)| item as &dyn tray_icon::menu::IsMenuItem).collect();
    let input_type_submenu = Submenu::with_id_and_items("input_type_submenu", "Kiểu gõ", true, &input_type_items)
        .expect("VNKey: failed to build the input-type menu");
    // All five code tables sit in one submenu rather than three at the top
    // level plus a "Bảng mã khác" overflow: same shape as "Kiểu gõ", and it
    // keeps the top level down to six entries.
    let code_table_items: Vec<&dyn tray_icon::menu::IsMenuItem> =
        code_tables.iter().map(|(_, item)| item as &dyn tray_icon::menu::IsMenuItem).collect();
    let code_table_submenu = Submenu::with_id_and_items("code_table_submenu", "Bảng mã", true, &code_table_items)
        .expect("VNKey: failed to build the code-table menu");

    let open_settings = MenuItem::with_id("open_settings", "Bảng điều khiển...", true, None);
    let open_macro = MenuItem::with_id("open_macro", "Gõ tắt...", true, None);
    let quit = MenuItem::with_id("quit", "Thoát", true, "Cmd+Q".parse().ok());

    let menu = Menu::with_items(&[
        &toggle_language,
        &PredefinedMenuItem::separator(),
        &input_type_submenu,
        &code_table_submenu,
        &PredefinedMenuItem::separator(),
        &open_settings,
        &open_macro,
        &PredefinedMenuItem::separator(),
        &quit,
    ])
    .expect("VNKey: failed to build the status-bar menu");

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| sender.send(AppEvent::Menu(event.id))));

    // No icon: the status item is text-only ("V"/"E"), per the module doc.
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_title(state.title)
        .build()
        .expect("VNKey: failed to create the status-bar item");
    set_tray_title(&tray, state.title);

    cx.set_global(TrayHandles { tray, toggle_language, input_types, code_tables });
}

/// Recompute [`TrayState`] and push it into every menu item + the title.
pub(crate) fn apply_tray_state(settings: &Settings, cx: &App) {
    let Some(handles) = cx.try_global::<TrayHandles>() else { return };
    let state = compute_tray_state(settings);

    set_tray_title(&handles.tray, state.title);
    handles.toggle_language.set_checked(state.language_checked);
    for (ty, item) in &handles.input_types {
        item.set_checked(state.input_type == *ty);
    }
    for (ct, item) in &handles.code_tables {
        item.set_checked(state.code_table == *ct);
    }
}

pub(crate) fn handle_menu_event(id: &MenuId, cx: &mut App) {
    let id = id.as_ref();
    if let Some((ty, _, _)) = INPUT_TYPES.iter().find(|(_, item_id, _)| *item_id == id) {
        let ty = *ty as u8;
        crate::commands::apply_and_broadcast(cx, |s| crate::commands::set_input_type_settings(s, ty));
        return;
    }
    if let Some((ct, _, _)) = CODE_TABLES.iter().find(|(_, item_id, _)| *item_id == id) {
        let ct = *ct as u8;
        crate::commands::apply_and_broadcast(cx, |s| crate::commands::set_code_table_settings(s, ct));
        return;
    }
    match id {
        "toggle_language" => {
            // The live atomic, not settings.json: the hotkey may have switched
            // without saving yet.
            let next = if cx.global::<Core>().config.enabled.load(Ordering::Relaxed) { 0 } else { 1 };
            crate::commands::set_language(cx, next);
        }
        "open_settings" => crate::ui::open_settings(cx),
        "open_macro" => crate::ui::open_macro(cx),
        "quit" => cx.quit(),
        _ => {}
    }
}

/// Run on the main thread after a change that bypassed
/// [`crate::commands::apply_and_broadcast`] (hotkey, smart-switch -- both
/// arrive as [`AppEvent::LiveChange`]): sync the tray, redraw the windows and
/// persist the live state, so a manual language switch or smart-switch
/// code-table change survives a restart even if nothing else saves first.
pub(crate) fn sync_after_live_change(cx: &mut App) {
    let settings = live_settings(&cx.global::<Core>().config);
    crate::app::broadcast(settings, cx);
    let _ = settings.save();
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
