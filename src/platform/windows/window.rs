//! The main window's native handle: rounded corners, position, and keeping
//! it on screen when it grows.

use std::ffi::c_void;
use std::mem::size_of;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTONULL, MONITORINFO,
    MonitorFromPoint, MonitorFromWindow,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
};

/// The app's main window. Holds the raw handle as a number, so it can be
/// copied and sent to other threads (it is only ever passed back to Win32).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MainWindow(isize);

impl MainWindow {
    pub fn from_handle(handle: &impl HasWindowHandle) -> Option<Self> {
        match handle.window_handle().ok()?.as_raw() {
            RawWindowHandle::Win32(handle) => Some(Self(handle.hwnd.get())),
            _ => None,
        }
    }

    pub(crate) fn hwnd(self) -> HWND {
        self.0 as HWND
    }

    /// The handle as a number, for passing to other threads.
    pub(crate) fn raw(self) -> isize {
        self.0
    }

    /// Asks Windows 11 for rounded corners and a thin gray border. Returns
    /// `false` where that isn't supported (Windows 10), so the app can draw
    /// its own border instead.
    pub fn round_corners(self) -> bool {
        let corner = DWMWCP_ROUND;
        let border: u32 = 0x0028_2828; // COLORREF 0x00BBGGRR: rgb(40, 40, 40)
        // SAFETY: both values outlive the calls and their sizes are passed.
        unsafe {
            let rounded = DwmSetWindowAttribute(
                self.hwnd(),
                DWMWA_WINDOW_CORNER_PREFERENCE as _,
                (&raw const corner).cast::<c_void>(),
                size_of::<i32>() as u32,
            ) >= 0;
            DwmSetWindowAttribute(
                self.hwnd(),
                DWMWA_BORDER_COLOR as _,
                (&raw const border).cast::<c_void>(),
                size_of::<u32>() as u32,
            );
            rounded
        }
    }

    /// Top-left corner in physical pixels.
    pub fn position(self) -> Option<[i32; 2]> {
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: `rect` is a valid out pointer.
        (unsafe { GetWindowRect(self.hwnd(), &mut rect) } != 0).then_some([rect.left, rect.top])
    }

    /// Moves the window to a saved position, unless that spot is no longer
    /// on any monitor (a monitor was unplugged or rearranged).
    pub fn move_to(self, [x, y]: [i32; 2]) -> bool {
        // A point a little inside the title bar must be on a monitor.
        let probe = POINT {
            x: x + 60,
            y: y + 16,
        };
        // SAFETY: plain value arguments.
        unsafe {
            if MonitorFromPoint(probe, MONITOR_DEFAULTTONULL).is_null() {
                return false;
            }
            SetWindowPos(
                self.hwnd(),
                std::ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            ) != 0
        }
    }

    /// Parks the window far off every monitor, so the frame eframe shows
    /// before the app can hide it is never seen.
    pub fn move_offscreen(self) {
        // SAFETY: plain value arguments.
        unsafe {
            SetWindowPos(
                self.hwnd(),
                std::ptr::null_mut(),
                -32000,
                -32000,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    /// Moves the window back inside its monitor's work area (not under the
    /// taskbar) if it sticks out, e.g. after growing for the Image tab.
    pub fn keep_on_screen(self) {
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            rcMonitor: rect,
            rcWork: rect,
            dwFlags: 0,
        };
        // SAFETY: valid out pointers; `cbSize` is set as the API requires.
        unsafe {
            if GetWindowRect(self.hwnd(), &mut rect) == 0 {
                return;
            }
            let monitor = MonitorFromWindow(self.hwnd(), MONITOR_DEFAULTTONEAREST);
            if GetMonitorInfoW(monitor, &mut info) == 0 {
                return;
            }
            let work = info.rcWork;
            let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
            let x = rect.left.min(work.right - w).max(work.left);
            let y = rect.top.min(work.bottom - h).max(work.top);
            if (x, y) != (rect.left, rect.top) {
                SetWindowPos(
                    self.hwnd(),
                    std::ptr::null_mut(),
                    x,
                    y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }
}
