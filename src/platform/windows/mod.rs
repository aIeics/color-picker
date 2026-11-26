//! Windows implementation.

mod autostart;
mod capture;
mod dialog;
mod instance;
mod loupe;
mod service;
mod tray;
mod window;

pub use autostart::{STARTUP_ARG, is_enabled as autostart_enabled, set as set_autostart};
pub use dialog::open_image_dialog;
pub use instance::{is_first_instance, wake_first_instance};
pub use service::Service;
pub use window::MainWindow;

/// Whether a Windows key is held (egui doesn't report it as a modifier).
pub fn win_key_down() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LWIN, VK_RWIN};
    // SAFETY: reads the key state.
    unsafe { GetAsyncKeyState(i32::from(VK_LWIN)) < 0 || GetAsyncKeyState(i32::from(VK_RWIN)) < 0 }
}

/// `text` as a NUL-terminated UTF-16 string for Win32 calls.
pub(crate) fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
