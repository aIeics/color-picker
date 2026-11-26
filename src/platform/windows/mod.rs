//! Windows implementation.

mod capture;
mod instance;
mod loupe;
mod service;
mod window;

pub use instance::{is_first_instance, wake_first_instance};
pub use service::Service;
pub use window::MainWindow;

/// `text` as a NUL-terminated UTF-16 string for Win32 calls.
pub(crate) fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
