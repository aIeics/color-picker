//! Operating-system integration: the native window, screen picking (hotkey,
//! mouse hook, loupe), the tray icon, the Open dialog and starting with
//! Windows. Windows is the real implementation; other systems get stubs so
//! the app still builds there.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

#[cfg(not(windows))]
mod other;
#[cfg(not(windows))]
pub use self::other::*;
