//! Reading the color under the mouse cursor.

use crate::color::Rgb;

/// Whether picking a color from the screen is supported on this platform.
pub const SCREEN_PICKING_SUPPORTED: bool = cfg!(windows);

/// Reads the screen pixel under the mouse cursor.
///
/// Returns `None` when the cursor position or the screen pixel cannot be read.
#[cfg(windows)]
pub fn color_at_cursor() -> Option<Rgb> {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{CLR_INVALID, GetDC, GetPixel, ReleaseDC};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    // SAFETY: The device context is checked before use and always released.
    // GetCursorPos only writes to `point`, and GetPixel takes plain integers.
    unsafe {
        let mut point = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut point) == 0 {
            return None;
        }

        let dc = GetDC(null_mut());
        if dc.is_null() {
            return None;
        }
        let pixel = GetPixel(dc, point.x, point.y);
        ReleaseDC(null_mut(), dc);

        if pixel == CLR_INVALID {
            return None;
        }

        // COLORREF is 0x00BBGGRR.
        Some(Rgb::new(
            (pixel & 0xFF) as u8,
            ((pixel >> 8) & 0xFF) as u8,
            ((pixel >> 16) & 0xFF) as u8,
        ))
    }
}

/// Reading screen pixels is Windows-only for now.
#[cfg(not(windows))]
pub fn color_at_cursor() -> Option<Rgb> {
    None
}
