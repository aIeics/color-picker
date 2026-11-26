//! Stubs for systems other than Windows: the app builds and runs, but screen
//! picking, the tray and the native dialogs are unavailable.

use eframe::egui;
use raw_window_handle::HasWindowHandle;

use super::{Command, Event};
use crate::hotkey::Hotkey;
use crate::screen::PickStyle;

pub fn is_first_instance() -> bool {
    true
}

pub fn wake_first_instance() {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MainWindow;

impl MainWindow {
    pub fn from_handle(_handle: &impl HasWindowHandle) -> Option<Self> {
        None
    }

    pub fn round_corners(self) -> bool {
        false
    }

    pub fn position(self) -> Option<[i32; 2]> {
        None
    }

    pub fn move_to(self, _pos: [i32; 2]) -> bool {
        false
    }

    pub fn keep_on_screen(self) {}
}

pub struct Service;

impl Service {
    pub fn start(_ctx: egui::Context, _style: PickStyle, _hotkey: Option<Hotkey>) -> Option<Self> {
        None
    }

    pub fn send(&self, _command: Command) {}

    pub fn poll(&self) -> Option<Event> {
        None
    }
}
