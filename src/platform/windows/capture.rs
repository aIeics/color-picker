//! Reading screen pixels: one `BitBlt` of a small square into a memory
//! bitmap, then plain reads from its pixels.

use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GdiFlush, GetDC, HBITMAP, HDC, HGDIOBJ, ReleaseDC,
    SRCCOPY, SelectObject,
};

use crate::color::Rgb;

/// A square of screen pixels, at most [`Capture::MAX`] wide.
pub struct Capture {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u32,
    /// Side of the last grabbed square.
    side: i32,
}

impl Capture {
    /// The largest square the loupe needs (zoom 4 on a big monitor).
    pub const MAX: i32 = 64;

    pub fn new() -> Option<Self> {
        let (dc, bitmap, bits) = dib(Self::MAX, Self::MAX)?;
        // SAFETY: `dc` and `bitmap` were just created and are valid.
        let old = unsafe { SelectObject(dc, bitmap) };
        Some(Self {
            dc,
            bitmap,
            old,
            bits,
            side: 0,
        })
    }

    /// Copies the `side`×`side` square of screen pixels centered on
    /// `center` (physical pixels). Parts off every monitor read as black.
    pub fn grab(&mut self, center: POINT, side: i32) -> bool {
        let side = side.clamp(1, Self::MAX);
        // SAFETY: the screen DC is released before returning; our DC and
        // bitmap stay valid for `self`'s lifetime.
        let ok = unsafe {
            let screen = GetDC(null_mut());
            if screen.is_null() {
                return false;
            }
            let ok = BitBlt(
                self.dc,
                0,
                0,
                side,
                side,
                screen,
                center.x - side / 2,
                center.y - side / 2,
                SRCCOPY | CAPTUREBLT,
            ) != 0;
            ReleaseDC(null_mut(), screen);
            GdiFlush();
            ok
        };
        self.side = if ok { side } else { 0 };
        ok
    }

    /// The pixel at (`x`, `y`) of the last grabbed square.
    pub fn pixel(&self, x: i32, y: i32) -> Rgb {
        if !(0..self.side).contains(&x) || !(0..self.side).contains(&y) {
            return Rgb::BLACK;
        }
        // SAFETY: the bitmap is MAX×MAX 32-bit pixels and x, y < side <= MAX.
        let v = unsafe { *self.bits.add((y * Self::MAX + x) as usize) };
        Rgb::new((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }

    /// The average of the `sample`×`sample` square in the middle of the last
    /// grabbed square.
    pub fn center_average(&self, sample: i32) -> Rgb {
        let mid = self.side / 2;
        let half = sample.max(1) / 2;
        let pixels = (mid - half..=mid + half)
            .flat_map(|y| (mid - half..=mid + half).map(move |x| (x, y)))
            .map(|(x, y)| self.pixel(x, y));
        Rgb::average(pixels).unwrap_or(Rgb::BLACK)
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        // SAFETY: restores the DC's original bitmap, then frees ours.
        unsafe {
            SelectObject(self.dc, self.old);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

/// The color under `point`, averaged over a `sample`×`sample` square.
pub fn sample_at(point: POINT, sample: i32) -> Option<Rgb> {
    let mut capture = Capture::new()?;
    capture
        .grab(point, sample)
        .then(|| capture.center_average(sample))
}

/// A memory DC and a top-down 32-bit DIB of `width`×`height`, with a pointer
/// to its pixels (BGRA, row after row).
pub fn dib(width: i32, height: i32) -> Option<(HDC, HBITMAP, *mut u32)> {
    // SAFETY: plain GDI object creation; everything is checked and freed on
    // failure.
    unsafe {
        let dc = CreateCompatibleDC(null_mut());
        if dc.is_null() {
            return None;
        }
        let mut info: BITMAPINFO = zeroed();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // negative: rows run top to bottom
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB as _,
            ..zeroed()
        };
        let mut bits: *mut c_void = null_mut();
        let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        if bitmap.is_null() || bits.is_null() {
            DeleteDC(dc);
            return None;
        }
        Some((dc, bitmap, bits.cast()))
    }
}
