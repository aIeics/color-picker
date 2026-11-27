//! Operating-system integration: the native window, screen picking (hotkey,
//! mouse hook, loupe), the tray icon, the Open dialog and starting with
//! Windows. Windows is the real implementation; other systems get stubs so
//! the app still builds there.

use crate::color::Rgb;
use crate::hotkey::Hotkey;
use crate::screen::PickStyle;
use crate::settings::IconStyle;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

#[cfg(not(windows))]
mod other;
#[cfg(not(windows))]
pub use self::other::*;

/// What the app asks the platform thread to do.
#[derive(Clone, Debug)]
pub enum Command {
    SetHotkey(Option<Hotkey>),
    SetStyle(PickStyle),
    StartPick,
    CancelPick,
    /// Show or remove the tray icon.
    SetTray(bool),
    /// The tray icon's look (applies now, or when the icon is shown).
    SetTrayIcon(IconStyle),
}

/// What happened on the platform thread.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The hotkey was (re)registered; `Some` explains a failure.
    HotkeyStatus(Option<String>),
    PickStarted,
    Picked(Rgb),
    PickCancelled,
    /// The mouse wheel changed the loupe zoom during a pick.
    ZoomChanged(u8),
    /// Show the window: another launch of the app, or the tray icon.
    ShowRequested,
    /// "Exit" in the tray menu.
    ExitRequested,
}
