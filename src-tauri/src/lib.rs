mod platform;
mod settings;

use tauri::Manager;

// The engine/tray/IPC/pasteboard layer is macOS-only: it's built directly on
// `state.rs`'s `Arc<Mutex<EngineCore>>`/`SharedConfig` (mac-gated) and
// AppKit (NSPasteboard, NSWorkspace, the status-bar menu). Other targets only
// get the `platform::start()` no-op stub, same as before this task.
#[cfg(target_os = "macos")]
mod state;
#[cfg(target_os = "macos")]
mod events;
#[cfg(target_os = "macos")]
mod pasteboard;
#[cfg(target_os = "macos")]
mod commands;
#[cfg(target_os = "macos")]
mod tray;

/// Task 15 review fix: whether the first-run OpenKey-import offer should
/// fire. Pure -- mirrors the exact condition `ControlPanel.tsx`'s effect
/// checks once `getSettings()`/`hasOpenKeyData()` resolve. Permission must
/// actually be granted (the gate takes priority over everything, including
/// this), the offer must not have already been shown -- via the persisted
/// `Settings::first_run_offer_shown` flag, NOT `settings.json`'s incidental
/// existence, which is what let this bug through originally (see this
/// function's table tests below for the exact repro sequence) -- and there
/// must be OpenKey data worth offering to import.
#[cfg(target_os = "macos")]
fn should_offer_first_run_import(permission_granted: bool, first_run_offer_shown: bool, has_openkey_data: bool) -> bool {
    permission_granted && !first_run_offer_shown && has_openkey_data
}

/// Whether the `settings` window should auto-show itself at launch, before
/// anything else happens: the Accessibility gate must block
/// (`permission_granted` false, regardless of the other flags -- task 15
/// brief: the permission screen takes priority), a first-run OpenKey import
/// can be offered ([`should_offer_first_run_import`]), or the user opted
/// into `show_ui_on_startup`. Pure and table-tested below, mirroring
/// `tray.rs`'s `compute_tray_state` split (state decision separate from the
/// `AppHandle` side effect that acts on it).
#[cfg(target_os = "macos")]
fn should_auto_show_settings(permission_granted: bool, first_run_offer_shown: bool, has_openkey_data: bool, show_ui_on_startup: bool) -> bool {
    !permission_granted || should_offer_first_run_import(permission_granted, first_run_offer_shown, has_openkey_data) || show_ui_on_startup
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("settings") {
                let _ = window.set_focus();
            }
        }));
    }

    builder = builder
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init());

    #[cfg(target_os = "macos")]
    {
        builder = builder.invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_settings,
            commands::set_language,
            commands::set_input_type,
            commands::set_code_table,
            commands::set_switch_hotkey,
            commands::set_convert_hotkey,
            commands::restore_default_settings,
            commands::macro_list,
            commands::macro_add,
            commands::macro_delete,
            commands::macro_import,
            commands::macro_export,
            commands::convert_clipboard,
            pasteboard::read_clipboard,
            pasteboard::write_clipboard,
            commands::get_permission_status,
            commands::open_accessibility_settings,
            commands::restart_app,
            commands::has_openkey_data,
            commands::import_openkey_settings,
            commands::check_update,
            commands::quit_app,
            commands::set_hotkey_recording,
            commands::mark_first_run_offer_shown,
        ]);
    }

    // Task 13/14: all 4 windows are created once and reused (the tray's
    // `show_window` just shows/focuses them) -- closing any of them via the
    // titlebar button must hide it, not destroy it, or the corresponding
    // tray menu item ("Bảng điều khiển"/"Gõ tắt"/"Công cụ chuyển mã"/"Giới
    // thiệu") would have nothing left to reopen.
    builder = builder.on_window_event(|window, event| {
        let is_managed_window = matches!(window.label(), "settings" | "macro" | "convert" | "about");
        if is_managed_window && let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window.hide();
        }
    });

    builder
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                let handles = platform::start(app.handle());
                app.manage(handles.engine);
                app.manage(handles.config);

                let settings = settings::Settings::load();
                tray::build(app.handle(), &settings)?;

                // Task 15: auto-show the Control Panel at launch when the
                // Accessibility gate needs to block, a first-run OpenKey
                // import can be offered, or `show_ui_on_startup` is set --
                // otherwise the window stays hidden (`tauri.conf.json`'s
                // `visible: false`) until the tray menu opens it, same as
                // before this task.
                //
                // Task 15 review fix: `settings.first_run_offer_shown` (not
                // `settings.json`'s mere existence) drives the offer branch,
                // so this still evaluates true on a trusted relaunch after
                // an untrusted first launch already auto-persisted the file.
                let permission_granted = platform::mac::ffi::is_trusted();
                let has_openkey_data = settings::openkey_import::has_openkey_data();
                if should_auto_show_settings(permission_granted, settings.first_run_offer_shown, has_openkey_data, settings.show_ui_on_startup)
                    && let Some(window) = app.get_webview_window("settings")
                {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                platform::start();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn shows_when_permission_missing_regardless_of_other_flags() {
        assert!(should_auto_show_settings(false, false, false, false));
        assert!(should_auto_show_settings(false, true, true, true));
    }

    #[test]
    fn shows_for_first_run_import_offer_when_permission_granted() {
        assert!(should_auto_show_settings(true, false, true, false));
    }

    #[test]
    fn does_not_offer_import_without_openkey_data() {
        assert!(!should_auto_show_settings(true, false, false, false));
    }

    #[test]
    fn does_not_offer_import_once_offer_already_shown() {
        assert!(!should_auto_show_settings(true, true, true, false));
    }

    #[test]
    fn shows_for_show_ui_on_startup() {
        assert!(should_auto_show_settings(true, true, false, true));
    }

    #[test]
    fn stays_hidden_when_nothing_applies() {
        assert!(!should_auto_show_settings(true, true, false, false));
    }

    #[test]
    fn offer_only_true_when_granted_not_shown_and_has_data() {
        assert!(should_offer_first_run_import(true, false, true));
        assert!(!should_offer_first_run_import(false, false, true), "permission gate must block the offer");
        assert!(!should_offer_first_run_import(true, true, true), "must not re-ask once already shown");
        assert!(!should_offer_first_run_import(true, false, false), "no OpenKey data to import");
    }

    /// Task 15 review regression: the exact sequence that exposed the bug.
    /// Launch 1 is untrusted -- the offer can't evaluate because permission
    /// isn't granted, and (this is the key point) nothing marks the flag
    /// shown just because `settings.json` got auto-persisted. The user then
    /// grants Accessibility and restarts; launch 2 must now offer the
    /// import because the flag is still false. The old file-existence-based
    /// check (`!settings_file_exists()`, captured before `Settings::load()`
    /// would auto-persist the file) would have read `false` for `first_run`
    /// on launch 2 -- since the file already existed from launch 1 -- and
    /// silently skipped the offer forever. This function never even sees
    /// file existence, only the persisted flag, so it can't reproduce that
    /// failure mode.
    #[test]
    fn untrusted_launch_then_trusted_relaunch_still_offers_import() {
        let first_run_offer_shown = false; // nothing on launch 1 ever sets this true

        // Launch 1: untrusted -- the offer must not fire.
        assert!(!should_offer_first_run_import(false, first_run_offer_shown, true));

        // Launch 2: permission now granted, flag still false -> offers.
        assert!(should_offer_first_run_import(true, first_run_offer_shown, true));
    }
}
