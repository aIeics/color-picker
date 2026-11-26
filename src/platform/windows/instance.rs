//! One running copy at a time. A second launch (or the Start menu while the
//! app sits in the tray) shows the running window instead of starting a
//! second tray icon whose hotkey would fail.

use std::ptr::null;

use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, FindWindowW, GetWindowThreadProcessId, PostMessageW,
};

use super::service::{CLASS, WM_APP_SHOW};
use super::wide;

/// Claims the single-instance lock. `false` if another copy holds it.
pub fn is_first_instance() -> bool {
    let name = wide("Local\\color-picker-single-instance");
    // SAFETY: a named mutex; the handle stays open until the process exits,
    // which is what keeps the lock.
    unsafe {
        let handle = CreateMutexW(null(), 0, name.as_ptr());
        !handle.is_null() && GetLastError() != ERROR_ALREADY_EXISTS
    }
}

/// Asks the running copy to show its window, and lets it come to the front.
pub fn wake_first_instance() {
    let class = wide(CLASS);
    // SAFETY: looks up a window by class and posts to it.
    unsafe {
        let hwnd = FindWindowW(class.as_ptr(), null());
        if hwnd.is_null() {
            return;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        AllowSetForegroundWindow(pid);
        PostMessageW(hwnd, WM_APP_SHOW, 0, 0);
    }
}
