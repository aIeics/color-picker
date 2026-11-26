//! Stubs for systems other than Windows: the app builds and runs, but screen
//! picking, the tray and the native dialogs are unavailable.

use raw_window_handle::HasWindowHandle;

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
