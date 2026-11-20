//! Application startup, the globals every window and the status-bar menu read,
//! and the one channel off-main-thread code uses to reach the main thread.
//!
//! The channel replaces Tauri's `run_on_main_thread`: the tap thread (hotkey
//! switch) and the NSWorkspace observers send an [`AppEvent`], and a GPUI
//! foreground task drains it with a live `&mut App`. Nothing on the tap thread
//! ever blocks on it.

use std::sync::Arc;

use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui_kit::{ActivationPolicy, App, BorrowAppContext as _, Global, WindowingRequest};
use parking_lot::Mutex;

use crate::platform::mac::{ffi, smart_switch};
use crate::settings::Settings;
use crate::state::{EngineCore, SharedConfig};

/// Work that must run on the main thread, posted from anywhere.
pub enum AppEvent {
    /// The hotkey (tap thread) or smart-switch (observer) changed
    /// `SharedConfig`'s language/code-table atomics without saving: persist,
    /// sync the tray and redraw the windows.
    LiveChange,
    /// The tap thread recorded a manual switch into the smart-switch table;
    /// disk I/O must not run there.
    PersistSmartSwitch,
    /// A status-bar menu item was clicked.
    Menu(tray_icon::menu::MenuId),
}

/// Cheap, cloneable, `Send` handle for posting an [`AppEvent`].
#[derive(Clone)]
pub struct AppSender(mpsc::UnboundedSender<AppEvent>);

impl AppSender {
    /// Fire-and-forget. Only fails after the app has quit, when there is
    /// nothing left to update.
    pub fn send(&self, event: AppEvent) {
        let _ = self.0.unbounded_send(event);
    }
}

/// The shared engine/config handles (Tauri managed state before).
pub struct Core {
    pub engine: Arc<Mutex<EngineCore>>,
    pub config: Arc<SharedConfig>,
}

impl Global for Core {}

/// What the windows render. Replaced through [`broadcast`] on every change, so
/// views that `observe_global::<UiState>` redraw for changes from any source:
/// UI, tray, hotkey or smart-switch.
pub struct UiState {
    pub settings: Settings,
    /// Bumped whenever the macro table changes.
    pub macros_rev: u64,
}

impl Global for UiState {}

/// Whether the Control Panel should open by itself at launch: the
/// Accessibility gate must block, or the user opted into `show_ui_on_startup`.
fn should_auto_show_settings(permission_granted: bool, show_ui_on_startup: bool) -> bool {
    !permission_granted || show_ui_on_startup
}

/// macOS gives `Regular` apps a Dock tile and `Accessory` ones none.
fn dock_activation_policy(show_icon_on_dock: bool) -> ActivationPolicy {
    if show_icon_on_dock { ActivationPolicy::Regular } else { ActivationPolicy::Accessory }
}

pub fn run() {
    let settings = Settings::load();
    let (tx, mut rx) = mpsc::unbounded();
    let sender = AppSender(tx);

    let mut app = gpui_kit::application().with_assets(crate::ui::Assets);
    // Applied before the run loop starts, so an Accessory app never flashes a
    // Dock tile at launch.
    if !settings.show_icon_on_dock {
        app = app.with_windowing(WindowingRequest::Headless);
    }
    // Dock click / relaunching the bundle while running. With a window open
    // AppKit brings it forward itself; with none, open the Control Panel.
    app.on_reopen(|cx| {
        if cx.windows().is_empty() {
            crate::ui::open_settings(cx);
        }
    });

    app.run(move |cx| {
        gpui_kit::init(cx);

        let handles = crate::platform::mac::start(sender.clone());
        cx.set_global(Core { engine: handles.engine, config: handles.config });
        cx.set_global(UiState { settings, macros_rev: 0 });
        // Re-point the login item at this copy, in case the app was moved.
        sync_autostart(settings.run_on_startup, cx);
        crate::ui::apply_theme(settings.theme, cx);
        crate::tray::build(&settings, sender, cx);

        cx.spawn(async move |cx| {
            while let Some(event) = rx.next().await {
                cx.update(|cx| handle_event(event, cx));
            }
        })
        .detach();

        if should_auto_show_settings(ffi::is_trusted(), settings.show_ui_on_startup) {
            crate::ui::open_settings(cx);
        }
    });
}

fn handle_event(event: AppEvent, cx: &mut App) {
    match event {
        AppEvent::LiveChange => crate::tray::sync_after_live_change(cx),
        AppEvent::PersistSmartSwitch => smart_switch::persist_table(&cx.global::<Core>().engine),
        AppEvent::Menu(id) => crate::tray::handle_menu_event(&id, cx),
    }
}

/// Push `settings` to the tray and every window.
pub(crate) fn broadcast(settings: Settings, cx: &mut App) {
    crate::tray::apply_tray_state(&settings, cx);
    let theme_changed = cx.global::<UiState>().settings.theme != settings.theme;
    cx.update_global::<UiState, _>(|ui, _| ui.settings = settings);
    if theme_changed {
        crate::ui::apply_theme(settings.theme, cx);
    }
}

/// Tell windows showing the macro table to reload it.
pub(crate) fn macros_changed(cx: &mut App) {
    cx.update_global::<UiState, _>(|ui, _| ui.macros_rev += 1);
}

/// Make "Khởi động cùng macOS" true: add or remove VNKey's login item.
pub(crate) fn sync_autostart(enabled: bool, cx: &App) {
    let exe = std::env::current_exe().unwrap_or_default();
    let app_path = cx.app_path().unwrap_or_else(|_| exe.clone());
    let command = crate::platform::mac::autostart::launch_command(&app_path, &exe);
    if let Err(err) = crate::platform::mac::autostart::sync(enabled, &command) {
        eprintln!("VNKey: could not update the login item: {err}");
    }
}

pub(crate) fn set_dock_icon(show: bool, cx: &mut App) {
    cx.set_activation_policy(dock_activation_policy(show));
    if show {
        // A newly `Regular` app's menu bar may not appear until reactivated.
        cx.activate(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_when_permission_missing_regardless_of_startup_flag() {
        assert!(should_auto_show_settings(false, false));
        assert!(should_auto_show_settings(false, true));
    }

    #[test]
    fn shows_for_show_ui_on_startup() {
        assert!(should_auto_show_settings(true, true));
    }

    #[test]
    fn stays_hidden_when_permission_granted_and_no_startup_flag() {
        assert!(!should_auto_show_settings(true, false));
    }

    #[test]
    fn dock_icon_setting_maps_onto_the_activation_policy() {
        assert_eq!(dock_activation_policy(true), ActivationPolicy::Regular);
        assert_eq!(dock_activation_policy(false), ActivationPolicy::Accessory);
    }
}
