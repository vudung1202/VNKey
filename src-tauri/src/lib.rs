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
        ]);
    }

    // Task 13: the `settings` window is created once and reused (the tray's
    // `show_window` just shows/focuses it) -- closing it via the titlebar
    // button must hide it, not destroy it, or the tray's "Bảng điều khiển"
    // item would have nothing left to reopen.
    builder = builder.on_window_event(|window, event| {
        if window.label() == "settings" && let tauri::WindowEvent::CloseRequested { api, .. } = event {
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
