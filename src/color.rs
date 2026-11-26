//! Color model and the text formats the app copies to the clipboard.

/// An 8-bit sRGB color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// A color in HSL, with hue in degrees (0-360) and saturation/lightness as
/// fractions (0.0-1.0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsl {
    pub h: f32,
    pub s: f32,
    pub l: f32,
}

impl Rgb {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };

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
                let mut digits = hex.bytes().map(|b| (b as char).to_digit(16));
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

    /// `#RRGGBB`, upper case.
    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// `rgb(r, g, b)`.
    pub fn to_rgb_string(self) -> String {
        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }

    /// `hsl(h, s%, l%)`.
    pub fn to_hsl_string(self) -> String {
        let hsl = self.to_hsl();
        format!(
            "hsl({}, {}%, {}%)",
            (hsl.h.round() as u32) % 360,
            (hsl.s * 100.0).round() as u32,
            (hsl.l * 100.0).round() as u32
        )
    }

    pub fn to_hsl(self) -> Hsl {
        let r = f32::from(self.r) / 255.0;
        let g = f32::from(self.g) / 255.0;
        let b = f32::from(self.b) / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let l = (max + min) / 2.0;

        if delta == 0.0 {
            return Hsl { h: 0.0, s: 0.0, l };
        }

        let s = if l > 0.5 {
            delta / (2.0 - max - min)
        } else {
            delta / (max + min)
        };
        let h = if max == r {
            ((g - b) / delta).rem_euclid(6.0)
        } else if max == g {
            (b - r) / delta + 2.0
        } else {
            (r - g) / delta + 4.0
        };

        Hsl { h: h * 60.0, s, l }
    }

    /// Perceived brightness, 0.0 (black) to 1.0 (white). Useful for picking a
    /// readable text color on top of the swatch.
    pub fn luminance(self) -> f32 {
        (0.2126 * f32::from(self.r) + 0.7152 * f32::from(self.g) + 0.0722 * f32::from(self.b))
            / 255.0
    }
}

impl From<Rgb> for eframe::egui::Color32 {
    fn from(color: Rgb) -> Self {
        Self::from_rgb(color.r, color.g, color.b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        assert_eq!(Rgb::new(0x12, 0x34, 0x56).to_hex(), "#123456");
        assert_eq!(Rgb::new(255, 0, 0).to_hex(), "#FF0000");
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
        for text in ["", "#", "#12", "12345", "#1234567", "gggggg", "#12345z"] {
            assert_eq!(Rgb::from_hex(text), None, "accepted {text:?}");
        }
    }

    #[test]
    fn rgb_format() {
        assert_eq!(Rgb::new(255, 0, 128).to_rgb_string(), "rgb(255, 0, 128)");
    }

    #[test]
    fn hsl_known_colors() {
        assert_eq!(Rgb::new(255, 0, 0).to_hsl_string(), "hsl(0, 100%, 50%)");
        assert_eq!(Rgb::new(0, 255, 0).to_hsl_string(), "hsl(120, 100%, 50%)");
        assert_eq!(Rgb::new(0, 0, 255).to_hsl_string(), "hsl(240, 100%, 50%)");
        assert_eq!(Rgb::new(255, 255, 255).to_hsl_string(), "hsl(0, 0%, 100%)");
        assert_eq!(Rgb::new(0, 0, 0).to_hsl_string(), "hsl(0, 0%, 0%)");
        assert_eq!(Rgb::new(128, 128, 128).to_hsl_string(), "hsl(0, 0%, 50%)");
        assert_eq!(Rgb::new(255, 0, 128).to_hsl_string(), "hsl(330, 100%, 50%)");
    }

    #[test]
    fn luminance_distinguishes_light_and_dark() {
        assert!(Rgb::new(255, 255, 255).luminance() > 0.9);
        assert!(Rgb::new(0, 0, 0).luminance() < 0.1);
    }
}
