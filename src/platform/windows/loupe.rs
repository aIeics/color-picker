//! The loupe: a small window next to the cursor with the zoomed pixels under
//! it and the color's HEX value. It is a layered window drawn into a 32-bit
//! bitmap with per-pixel alpha (for anti-aliased rounded corners). It is
//! click-through, never activated, topmost and left out of screen captures,
//! so it can't end up in its own picture.

use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use std::sync::Once;

use windows_sys::Win32::Foundation::{HWND, POINT, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW,
    DEFAULT_CHARSET, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject,
    DrawTextW, FW_SEMIBOLD, GdiFlush, GetDC, GetMonitorInfoW, HBITMAP, HDC, HFONT, HGDIOBJ,
    MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, OUT_DEFAULT_PRECIS, ReleaseDC,
    SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, HWND_TOPMOST, RegisterClassW, SW_HIDE,
    SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowDisplayAffinity,
    SetWindowPos, ShowWindow, ULW_ALPHA, UpdateLayeredWindow, WDA_EXCLUDEFROMCAPTURE, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use super::capture::{self, Capture};
use super::wide;
use crate::color::Rgb;
use crate::screen::PickStyle as Style;

const CLASS: &str = "ColorPickerLoupe";

// Colors as 0x00RRGGBB, matching the app's theme.
const SURFACE: u32 = 0x11_11_11;
const BORDER: u32 = 0x3A_3A_3A;
const SWATCH_BORDER: u32 = 0x50_50_50;

pub struct Loupe {
    hwnd: HWND,
    canvas: Option<Canvas>,
    capture: Capture,
    font: Option<(HFONT, i32)>,
    visible: bool,
}

impl Loupe {
    pub fn new() -> Option<Self> {
        static REGISTER: Once = Once::new();
        let class = wide(CLASS);
        // SAFETY: plain Win32 calls with valid, NUL-terminated strings.
        unsafe {
            let instance = GetModuleHandleW(null_mut());
            REGISTER.call_once(|| {
                let wc = WNDCLASSW {
                    lpfnWndProc: Some(DefWindowProcW),
                    hInstance: instance,
                    lpszClassName: class.as_ptr(),
                    ..zeroed()
                };
                RegisterClassW(&wc);
            });
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE,
                class.as_ptr(),
                wide("Color Picker loupe").as_ptr(),
                WS_POPUP,
                0,
                0,
                1,
                1,
                null_mut(),
                null_mut(),
                instance,
                null_mut(),
            );
            if hwnd.is_null() {
                return None;
            }
            // Windows 10 2004+; older versions just capture it, which is
            // fine because it never overlaps the pixels being read.
            SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
            Some(Self {
                hwnd,
                canvas: None,
                capture: Capture::new()?,
                font: None,
                visible: false,
            })
        }
    }

    /// Reads the pixels around `cursor`, redraws, and moves next to it.
    /// Returns the color that a click would pick there.
    pub fn update(&mut self, cursor: POINT, style: Style) -> Rgb {
        let scale = monitor_scale(cursor);
        let px = |v: f32| (v * scale).round() as i32;

        let cell = px(f32::from(style.zoom)).max(2);
        let mut n = (px(126.0) / cell).clamp(5, Capture::MAX - 1);
        if n % 2 == 0 {
            n -= 1;
        }
        let zoom_px = n * cell;
        let pad = px(6.0);
        let label_h = px(28.0);
        let width = zoom_px + 2 * pad;
        let height = pad + zoom_px + label_h;

        self.capture.grab(cursor, n);
        let color = self.capture.center_average(style.sample);

        let font = self.font_of(px(13.0));
        if !self.ensure_canvas(width, height) {
            return color;
        }
        let capture = &self.capture;
        let Some(canvas) = self.canvas.as_mut() else {
            return color;
        };
        let pixels = canvas.pixels_mut();
        pixels.fill(0xFF00_0000 | SURFACE);

        // Zoomed pixels, with faint grid lines when cells are big enough.
        let grid = cell >= 6;
        for y in 0..zoom_px {
            for x in 0..zoom_px {
                let c = capture.pixel(x / cell, y / cell);
                let mut v = (u32::from(c.r) << 16) | (u32::from(c.g) << 8) | u32::from(c.b);
                if grid && (x % cell == 0 || y % cell == 0) {
                    v = scale_rgb(v, 0.82);
                }
                pixels[((pad + y) * width + pad + x) as usize] = 0xFF00_0000 | v;
            }
        }

        // The picked square: white inside a black ring, readable on any color.
        let first = n / 2 - style.sample / 2;
        let left = pad + first * cell;
        let side = style.sample * cell;
        outline(pixels, width, left, left, side, side, 0xFFFF_FFFF);
        outline(
            pixels,
            width,
            left - 1,
            left - 1,
            side + 2,
            side + 2,
            0xFF00_0000,
        );

        // Label: a small swatch and the HEX value.
        let sw = px(14.0);
        let sx = pad + px(3.0);
        let sy = pad + zoom_px + (label_h - sw) / 2;
        let swatch = (u32::from(color.r) << 16) | (u32::from(color.g) << 8) | u32::from(color.b);
        fill(pixels, width, sx, sy, sw, sw, 0xFF00_0000 | swatch);
        outline(pixels, width, sx, sy, sw, sw, 0xFF00_0000 | SWATCH_BORDER);

        let mut text_rect = RECT {
            left: sx + sw + px(8.0),
            top: pad + zoom_px,
            right: width - pad,
            bottom: height,
        };
        let text = wide(&style.hex.format(color));
        // SAFETY: the canvas DC and font are valid; the text is NUL-terminated.
        unsafe {
            let old_font = SelectObject(canvas.dc, font);
            SetBkMode(canvas.dc, TRANSPARENT as _);
            SetTextColor(canvas.dc, 0x00FF_FFFF);
            DrawTextW(
                canvas.dc,
                text.as_ptr(),
                -1,
                &mut text_rect,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
            );
            SelectObject(canvas.dc, old_font);
            GdiFlush();
        }

        round_corners(canvas.pixels_mut(), width, height, px(10.0) as f32);
        self.show(cursor, width, height, scale);
        color
    }

    pub fn hide(&mut self) {
        if self.visible {
            // SAFETY: our own window handle.
            unsafe { ShowWindow(self.hwnd, SW_HIDE) };
            self.visible = false;
        }
    }

    /// Puts the drawn bitmap on screen next to the cursor, flipped to the
    /// other side near monitor edges.
    fn show(&mut self, cursor: POINT, width: i32, height: i32, scale: f32) {
        let Some(canvas) = &self.canvas else {
            return;
        };
        let offset = (20.0 * scale).round() as i32;
        let bounds = monitor_rect(cursor);
        let mut x = cursor.x + offset;
        if x + width > bounds.right {
            x = cursor.x - offset - width;
        }
        let mut y = cursor.y + offset;
        if y + height > bounds.bottom {
            y = cursor.y - offset - height;
        }

        let position = POINT { x, y };
        let size = SIZE {
            cx: width,
            cy: height,
        };
        let source = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        // SAFETY: valid handles and pointers to locals.
        unsafe {
            let screen = GetDC(null_mut());
            UpdateLayeredWindow(
                self.hwnd, screen, &position, &size, canvas.dc, &source, 0, &blend, ULW_ALPHA,
            );
            ReleaseDC(null_mut(), screen);
            if !self.visible {
                // Above other topmost windows, such as the pinned app.
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
                ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                self.visible = true;
            }
        }
    }

    /// Makes sure the bitmap is `width`×`height`; `false` if it can't be made.
    fn ensure_canvas(&mut self, width: i32, height: i32) -> bool {
        if self
            .canvas
            .as_ref()
            .is_none_or(|c| c.width != width || c.height != height)
        {
            self.canvas = Canvas::new(width, height);
        }
        self.canvas.is_some()
    }

    fn font_of(&mut self, px: i32) -> HFONT {
        if let Some((font, size)) = self.font
            && size == px
        {
            return font;
        }
        if let Some((old, _)) = self.font.take() {
            // SAFETY: the font isn't selected into any DC between draws.
            unsafe { DeleteObject(old) };
        }
        let face = wide("Segoe UI");
        // SAFETY: plain font creation with a NUL-terminated face name.
        let font = unsafe {
            CreateFontW(
                -px,
                0,
                0,
                0,
                FW_SEMIBOLD as _,
                0,
                0,
                0,
                DEFAULT_CHARSET as _,
                OUT_DEFAULT_PRECIS as _,
                CLIP_DEFAULT_PRECIS as _,
                CLEARTYPE_QUALITY as _,
                0,
                face.as_ptr(),
            )
        };
        self.font = Some((font, px));
        font
    }
}

impl Drop for Loupe {
    fn drop(&mut self) {
        self.canvas = None;
        // SAFETY: our own window and font.
        unsafe {
            if let Some((font, _)) = self.font.take() {
                DeleteObject(font);
            }
            DestroyWindow(self.hwnd);
        }
    }
}

/// The loupe's bitmap and the memory DC it is selected into.
struct Canvas {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u32,
    width: i32,
    height: i32,
}

impl Canvas {
    fn new(width: i32, height: i32) -> Option<Self> {
        let (dc, bitmap, bits) = capture::dib(width, height)?;
        // SAFETY: both were just created.
        let old = unsafe { SelectObject(dc, bitmap) };
        Some(Self {
            dc,
            bitmap,
            old,
            bits,
            width,
            height,
        })
    }

    fn pixels_mut(&mut self) -> &mut [u32] {
        // SAFETY: the DIB holds exactly width × height 32-bit pixels and
        // lives as long as `self`.
        unsafe { std::slice::from_raw_parts_mut(self.bits, (self.width * self.height) as usize) }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        // SAFETY: restores the original bitmap before freeing ours.
        unsafe {
            SelectObject(self.dc, self.old);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

/// Effective DPI scale of the monitor under `point` (1.0 at 100 %).
fn monitor_scale(point: POINT) -> f32 {
    let (mut x, mut y) = (96u32, 96u32);
    // SAFETY: valid out pointers.
    unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) < 0 {
            return 1.0;
        }
    }
    (x as f32 / 96.0).clamp(1.0, 4.0)
}

fn monitor_rect(point: POINT) -> RECT {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        rcMonitor: RECT {
            left: i32::MIN / 2,
            top: i32::MIN / 2,
            right: i32::MAX / 2,
            bottom: i32::MAX / 2,
        },
        rcWork: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        dwFlags: 0,
    };
    // SAFETY: `cbSize` is set; valid out pointer.
    unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        GetMonitorInfoW(monitor, &mut info);
    }
    info.rcMonitor
}

fn scale_rgb(v: u32, factor: f32) -> u32 {
    let channel = |shift: u32| (((v >> shift) & 0xFF) as f32 * factor) as u32;
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

fn fill(pixels: &mut [u32], stride: i32, x: i32, y: i32, w: i32, h: i32, value: u32) {
    for row in y.max(0)..y + h {
        for col in x.max(0)..(x + w).min(stride) {
            if let Some(p) = pixels.get_mut((row * stride + col) as usize) {
                *p = value;
            }
        }
    }
}

/// A 1-pixel rectangle outline.
fn outline(pixels: &mut [u32], stride: i32, x: i32, y: i32, w: i32, h: i32, value: u32) {
    fill(pixels, stride, x, y, w, 1, value);
    fill(pixels, stride, x, y + h - 1, w, 1, value);
    fill(pixels, stride, x, y, 1, h, value);
    fill(pixels, stride, x + w - 1, y, 1, h, value);
}

/// Cuts anti-aliased rounded corners, draws a 1-pixel border along them and
/// premultiplies alpha, as `UpdateLayeredWindow` expects.
fn round_corners(pixels: &mut [u32], width: i32, height: i32, radius: f32) {
    let (w, h) = (width as f32, height as f32);
    for y in 0..height {
        for x in 0..width {
            // Signed distance from the pixel center to the rounded outline.
            let px = (x as f32 + 0.5 - w / 2.0).abs() - (w / 2.0 - radius);
            let py = (y as f32 + 0.5 - h / 2.0).abs() - (h / 2.0 - radius);
            let outside = px.max(0.0).hypot(py.max(0.0)) + px.max(py).min(0.0) - radius;
            let coverage = (0.5 - outside).clamp(0.0, 1.0);
            let index = (y * width + x) as usize;
            let mut v = pixels[index] & 0x00FF_FFFF;
            // Within about a pixel of the edge: the border color.
            let border = (outside + 1.5).clamp(0.0, 1.0);
            if border > 0.0 {
                v = mix(v, BORDER, border);
            }
            let a = (coverage * 255.0).round() as u32;
            pixels[index] = (a << 24) | scale_rgb(v, coverage);
        }
    }
}

fn mix(a: u32, b: u32, t: f32) -> u32 {
    let channel = |shift: u32| {
        let a = ((a >> shift) & 0xFF) as f32;
        let b = ((b >> shift) & 0xFF) as f32;
        (a + (b - a) * t).round() as u32
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}
