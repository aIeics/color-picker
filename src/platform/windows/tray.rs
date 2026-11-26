//! The tray icon and its menu. Owned by the platform thread's window, which
//! receives the icon's clicks.

use std::mem::{size_of, zeroed};
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETVERSION,
    NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos,
    GetSystemMetrics, HICON, ICONINFO, MF_SEPARATOR, MF_STRING, PostMessageW, SM_CXSMICON,
    SetForegroundWindow, SetMenuDefaultItem, TPM_BOTTOMALIGN, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, TrackPopupMenuEx, WM_NULL,
};

use super::capture;
use super::wide;
use crate::ui::app_icon;

const ICON_ID: u32 = 1;

/// Tray menu commands.
pub const MENU_SHOW: u32 = 1;
pub const MENU_PICK: u32 = 2;
pub const MENU_EXIT: u32 = 3;

pub struct Tray {
    hwnd: HWND,
    icon: HICON,
}

impl Tray {
    /// Adds the icon. Its clicks arrive at `hwnd` as `callback` messages.
    pub fn add(hwnd: HWND, callback: u32) -> Option<Self> {
        // SAFETY: plain query.
        let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.clamp(16, 64);
        let icon = make_icon(size)?;
        let tray = Self { hwnd, icon };
        if !tray.notify(NIM_ADD, callback) {
            return None;
        }
        Some(tray)
    }

    /// Adds the icon again after Explorer restarted.
    pub fn readd(&self, callback: u32) {
        self.notify(NIM_ADD, callback);
    }

    fn notify(&self, message: u32, callback: u32) -> bool {
        // SAFETY: a zeroed NOTIFYICONDATAW is valid; we fill what we use.
        unsafe {
            let mut data: NOTIFYICONDATAW = zeroed();
            data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
            data.hWnd = self.hwnd;
            data.uID = ICON_ID;
            data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
            data.uCallbackMessage = callback;
            data.hIcon = self.icon;
            let tip = wide("Color Picker");
            data.szTip[..tip.len()].copy_from_slice(&tip);
            if Shell_NotifyIconW(message, &data) == 0 {
                return false;
            }
            if message == NIM_ADD {
                data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
                Shell_NotifyIconW(NIM_SETVERSION, &data);
            }
            true
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        // SAFETY: removes our icon and frees its HICON.
        unsafe {
            let mut data: NOTIFYICONDATAW = zeroed();
            data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
            data.hWnd = self.hwnd;
            data.uID = ICON_ID;
            Shell_NotifyIconW(NIM_DELETE, &data);
            DestroyIcon(self.icon);
        }
    }
}

/// Shows the tray menu at the cursor and returns the chosen command (0 if
/// none). Runs a modal loop: callers must not hold borrowed state.
pub fn show_menu(hwnd: HWND) -> u32 {
    // SAFETY: a temporary menu, destroyed before returning.
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return 0;
        }
        AppendMenuW(
            menu,
            MF_STRING,
            MENU_SHOW as usize,
            wide("Show Color Picker").as_ptr(),
        );
        AppendMenuW(
            menu,
            MF_STRING,
            MENU_PICK as usize,
            wide("Pick a color").as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, null_mut());
        AppendMenuW(menu, MF_STRING, MENU_EXIT as usize, wide("Exit").as_ptr());
        SetMenuDefaultItem(menu, MENU_SHOW, 0);

        let mut cursor = POINT { x: 0, y: 0 };
        GetCursorPos(&mut cursor);
        // Without this the menu doesn't close when clicking elsewhere.
        SetForegroundWindow(hwnd);
        let command = TrackPopupMenuEx(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
            cursor.x,
            cursor.y,
            hwnd,
            null_mut(),
        );
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(menu);
        command as u32
    }
}

/// An HICON from the app icon's pixels.
fn make_icon(size: i32) -> Option<HICON> {
    let rgba = app_icon::pixels(size as u32);
    let (dc, color, bits) = capture::dib(size, size)?;
    // SAFETY: `bits` holds size × size 32-bit pixels; the GDI objects are
    // freed once the icon has copied them.
    unsafe {
        let pixels = std::slice::from_raw_parts_mut(bits, (size * size) as usize);
        for (pixel, c) in pixels.iter_mut().zip(rgba.as_chunks::<4>().0) {
            *pixel = u32::from_be_bytes([c[3], c[0], c[1], c[2]]);
        }
        let mask = CreateBitmap(size, size, 1, 1, null_mut());
        let info = ICONINFO {
            fIcon: 1,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: color,
        };
        let icon = CreateIconIndirect(&info);
        DeleteObject(mask);
        DeleteObject(color);
        windows_sys::Win32::Graphics::Gdi::DeleteDC(dc);
        (!icon.is_null()).then_some(icon)
    }
}
