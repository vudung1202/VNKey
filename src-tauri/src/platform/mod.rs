//! OS platform layer. Only macOS is implemented; other targets get a no-op so
//! the crate still builds.

#[cfg(target_os = "macos")]
pub mod mac;

/// Start the platform keystroke interception + injection layer.
#[cfg(target_os = "macos")]
pub fn start() {
    mac::start();
}

#[cfg(not(target_os = "macos"))]
pub fn start() {}
