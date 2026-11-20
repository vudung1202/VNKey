//! VNKey: a menu-bar Vietnamese input method for macOS.
//!
//! GPUI (via gpui-kit) owns the NSApplication event loop and draws the
//! windows; the keystroke path runs on its own CGEventTap thread
//! (`platform::mac`) and reaches the main thread through [`app::AppEvent`].

mod platform;
mod settings;

// Everything below is built on `state.rs`'s mac-gated engine handles and on
// AppKit, so only macOS gets it.
#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod commands;
#[cfg(target_os = "macos")]
mod pasteboard;
#[cfg(target_os = "macos")]
mod state;
#[cfg(target_os = "macos")]
mod tray;
#[cfg(target_os = "macos")]
mod ui;

#[cfg(target_os = "macos")]
fn main() {
    app::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("VNKey chỉ chạy trên macOS.");
}
