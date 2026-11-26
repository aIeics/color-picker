//! Color model and conversions between color spaces.
//!
//! Everything here is plain math on 8-bit sRGB colors, so it is unit tested
//! and has no UI or platform code.

mod format;

pub use format::{Format, HexStyle};

/// An 8-bit sRGB color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Hue in degrees (0-360), saturation and lightness as fractions (0-1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsl {
    pub h: f32,
    pub s: f32,
    pub l: f32,
}

/// Hue in degrees (0-360), saturation and value as fractions (0-1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

/// Cyan, magenta, yellow and black as fractions (0-1). A plain conversion
/// without a color profile, like most color pickers show.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cmyk {
    pub c: f32,
    pub m: f32,
    pub y: f32,
    pub k: f32,
}

/// OKLCH: lightness (0-1), chroma (0 to about 0.4) and hue in degrees (0-360).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklch {
    pub l: f32,
    pub c: f32,
    pub h: f32,
}

impl Rgb {
    pub const BLACK: Self = Self::new(0, 0, 0);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses `#RRGGBB`, `RRGGBB`, `#RGB` or `RGB`. Surrounding whitespace and
    /// upper/lower case are ignored.
    pub fn from_hex(text: &str) -> Option<Self> {
        let hex = text.trim();
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if !hex.is_ascii() {
            return None;
        }

        match hex.len() {
            3 => {
                let mut digits = hex.chars().map(|c| c.to_digit(16));
                let r = digits.next().flatten()? as u8 * 17;
                let g = digits.next().flatten()? as u8 * 17;
                let b = digits.next().flatten()? as u8 * 17;
                Some(Self::new(r, g, b))
            }
            6 => Some(Self::new(
                u8::from_str_radix(&hex[0..2], 16).ok()?,
                u8::from_str_radix(&hex[2..4], 16).ok()?,
                u8::from_str_radix(&hex[4..6], 16).ok()?,
            )),
            _ => None,
        }
    }

    /// The channels as fractions (0-1).
    fn unit(self) -> (f32, f32, f32) {
        (
            f32::from(self.r) / 255.0,
            f32::from(self.g) / 255.0,
            f32::from(self.b) / 255.0,
        )
    }

    pub fn to_hsl(self) -> Hsl {
        let (r, g, b) = self.unit();
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let l = (max + min) / 2.0;
        let s = if delta == 0.0 {
            0.0
        } else {
            delta / (1.0 - (2.0 * l - 1.0).abs())
        };
        Hsl {
            h: hue(r, g, b, max, delta),
            s: s.clamp(0.0, 1.0),
            l,
        }
    }

    pub fn from_hsl(hsl: Hsl) -> Self {
        let s = finite_unit(hsl.s);
        let l = finite_unit(hsl.l);
        let chroma = (1.0 - (2.0 * l - 1.0).abs()) * s;
        from_hue_chroma(hsl.h, chroma, l - chroma / 2.0)
    }

    pub fn to_hsv(self) -> Hsv {
        let (r, g, b) = self.unit();
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        Hsv {
            h: hue(r, g, b, max, delta),
            s: if max == 0.0 { 0.0 } else { delta / max },
            v: max,
        }
    }

    pub fn from_hsv(hsv: Hsv) -> Self {
        let s = finite_unit(hsv.s);
        let v = finite_unit(hsv.v);
        let chroma = v * s;
        from_hue_chroma(hsv.h, chroma, v - chroma)
    }

    pub fn to_cmyk(self) -> Cmyk {
        let (r, g, b) = self.unit();
        let k = 1.0 - r.max(g).max(b);
        if k >= 1.0 {
            return Cmyk {
                c: 0.0,
                m: 0.0,
                y: 0.0,
                k: 1.0,
            };
        }
        Cmyk {
            c: (1.0 - r - k) / (1.0 - k),
            m: (1.0 - g - k) / (1.0 - k),
            y: (1.0 - b - k) / (1.0 - k),
            k,
        }
    }

    /// Converts through linear sRGB and Oklab (Björn Ottosson's matrices).
    pub fn to_oklch(self) -> Oklch {
        let (r, g, b) = self.unit();
        let (r, g, b) = (to_linear(r), to_linear(g), to_linear(b));

        let l = 0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b;
        let m = 0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b;
        let s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;
        let (l, m, s) = (l.cbrt(), m.cbrt(), s.cbrt());

        let lightness = 0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s;
        let a = 1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s;
        let b = 0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s;

        let chroma = (a * a + b * b).sqrt();
        // Grays have no meaningful hue; report 0 instead of rounding noise.
        let h = if chroma < 1e-4 {
            0.0
        } else {
            b.atan2(a).to_degrees().rem_euclid(360.0)
        };
        Oklch {
            l: lightness.clamp(0.0, 1.0),
            c: if chroma < 1e-4 { 0.0 } else { chroma },
            h,
        }
    }

    /// Perceived brightness, 0.0 (black) to 1.0 (white). Used to pick a
    /// readable text color on top of a swatch.
    pub fn luminance(self) -> f32 {
        (0.2126 * f32::from(self.r) + 0.7152 * f32::from(self.g) + 0.0722 * f32::from(self.b))
            / 255.0
    }

    /// The rounded mean of `colors`, or `None` when there are none.
    pub fn average(colors: impl IntoIterator<Item = Self>) -> Option<Self> {
        let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
        for c in colors {
            r += u64::from(c.r);
            g += u64::from(c.g);
            b += u64::from(c.b);
            n += 1;
        }
        if n == 0 {
            return None;
        }
        let mean = |sum: u64| ((sum + n / 2) / n) as u8;
        Some(Self::new(mean(r), mean(g), mean(b)))
    }
}

impl From<Rgb> for eframe::egui::Color32 {
    fn from(color: Rgb) -> Self {
        Self::from_rgb(color.r, color.g, color.b)
    }
}

/// Hue in degrees for unit RGB, given its maximum channel and range.
fn hue(r: f32, g: f32, b: f32, max: f32, delta: f32) -> f32 {
    if delta == 0.0 {
        return 0.0;
    }
    let h = if max == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    (h * 60.0).rem_euclid(360.0)
}

/// Shared tail of the HSL and HSV conversions: a hue, a chroma and the amount
/// `m` added to every channel.
fn from_hue_chroma(h: f32, chroma: f32, m: f32) -> Rgb {
    let h = if h.is_finite() {
        h.rem_euclid(360.0)
    } else {
        0.0
    } / 60.0;
    let x = chroma * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    Rgb::new(to_u8(r + m), to_u8(g + m), to_u8(b + m))
}

fn finite_unit(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn to_u8(v: f32) -> u8 {
    (finite_unit(v) * 255.0).round() as u8
}

/// sRGB transfer function, gamma-encoded to linear light.
fn to_linear(c: f32) -> f32 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32, tolerance: f32) -> bool {
        (a - b).abs() <= tolerance
    }

    #[test]
    fn parses_hex_with_and_without_hash() {
        assert_eq!(Rgb::from_hex("#3B82F6"), Some(Rgb::new(0x3B, 0x82, 0xF6)));
        assert_eq!(Rgb::from_hex("3b82f6"), Some(Rgb::new(0x3B, 0x82, 0xF6)));
        assert_eq!(
            Rgb::from_hex("  #3B82F6  "),
            Some(Rgb::new(0x3B, 0x82, 0xF6))
        );
    }

    #[test]
    fn parses_short_hex() {
        assert_eq!(Rgb::from_hex("#f0a"), Some(Rgb::new(255, 0, 170)));
        assert_eq!(Rgb::from_hex("fff"), Some(Rgb::new(255, 255, 255)));
    }

    #[test]
    fn rejects_invalid_hex() {
        for text in [
            "", "#", "#12", "12345", "#1234567", "gggggg", "#12345z", "#ä12",
        ] {
            assert_eq!(Rgb::from_hex(text), None, "accepted {text:?}");
        }
    }

    #[test]
    fn hsl_known_colors() {
        let hsl = Rgb::new(255, 0, 128).to_hsl();
        assert!(close(hsl.h, 329.9, 0.1), "{hsl:?}");
        assert!(
            close(hsl.s, 1.0, 1e-6) && close(hsl.l, 0.5, 1e-6),
            "{hsl:?}"
        );
        let gray = Rgb::new(128, 128, 128).to_hsl();
        assert_eq!((gray.h, gray.s), (0.0, 0.0));
    }

    #[test]
    fn hsv_known_colors() {
        let hsv = Rgb::new(0x3B, 0x82, 0xF6).to_hsv();
        assert!(close(hsv.h, 217.2, 0.1), "{hsv:?}");
        assert!(
            close(hsv.s, 0.760, 0.001) && close(hsv.v, 0.965, 0.001),
            "{hsv:?}"
        );
        assert_eq!(
            Rgb::BLACK.to_hsv(),
            Hsv {
                h: 0.0,
                s: 0.0,
                v: 0.0
            }
        );
    }

    #[test]
    fn hsl_and_hsv_round_trip() {
        for r in (0..=255).step_by(15) {
            for g in (0..=255).step_by(15) {
                for b in (0..=255).step_by(15) {
                    let color = Rgb::new(r, g, b);
                    assert_eq!(Rgb::from_hsl(color.to_hsl()), color);
                    assert_eq!(Rgb::from_hsv(color.to_hsv()), color);
                }
            }
        }
    }

    #[test]
    fn conversions_survive_bad_input() {
        let nan = f32::NAN;
        assert_eq!(
            Rgb::from_hsl(Hsl {
                h: nan,
                s: nan,
                l: nan
            }),
            Rgb::BLACK
        );
        assert_eq!(
            Rgb::from_hsv(Hsv {
                h: -720.0,
                s: 2.0,
                v: 1.0
            }),
            Rgb::new(255, 0, 0)
        );
        assert_eq!(
            Rgb::from_hsv(Hsv {
                h: 360.0,
                s: 1.0,
                v: 1.0
            }),
            Rgb::new(255, 0, 0)
        );
    }

    #[test]
    fn cmyk_known_colors() {
        let cmyk = Rgb::new(0x3B, 0x82, 0xF6).to_cmyk();
        assert!(close(cmyk.c, 0.760, 0.001), "{cmyk:?}");
        assert!(close(cmyk.m, 0.472, 0.001), "{cmyk:?}");
        assert!(
            close(cmyk.y, 0.0, 1e-6) && close(cmyk.k, 0.035, 0.001),
            "{cmyk:?}"
        );
        assert_eq!(
            Rgb::BLACK.to_cmyk(),
            Cmyk {
                c: 0.0,
                m: 0.0,
                y: 0.0,
                k: 1.0
            }
        );
    }

    #[test]
    fn oklch_known_colors() {
        // Reference values from the CSS Color 4 spec's sample code.
        let red = Rgb::new(255, 0, 0).to_oklch();
        assert!(close(red.l, 0.6280, 0.0005), "{red:?}");
        assert!(close(red.c, 0.2577, 0.0005), "{red:?}");
        assert!(close(red.h, 29.23, 0.05), "{red:?}");

        let blue = Rgb::new(0, 0, 255).to_oklch();
        assert!(close(blue.l, 0.4520, 0.0005), "{blue:?}");
        assert!(close(blue.c, 0.3132, 0.0005), "{blue:?}");
        assert!(close(blue.h, 264.05, 0.05), "{blue:?}");

        let white = Rgb::new(255, 255, 255).to_oklch();
        assert!(close(white.l, 1.0, 0.0005), "{white:?}");
        assert_eq!((white.c, white.h), (0.0, 0.0));
        assert_eq!(
            Rgb::BLACK.to_oklch(),
            Oklch {
                l: 0.0,
                c: 0.0,
                h: 0.0
            }
        );
    }

    #[test]
    fn average_rounds_and_handles_empty() {
        let avg = Rgb::average([Rgb::new(0, 0, 0), Rgb::new(255, 1, 2)]);
        assert_eq!(avg, Some(Rgb::new(128, 1, 1)));
        assert_eq!(Rgb::average([]), None);
    }

    #[test]
    fn luminance_distinguishes_light_and_dark() {
        assert!(Rgb::new(255, 255, 255).luminance() > 0.9);
        assert!(Rgb::BLACK.luminance() < 0.1);
    }
}
