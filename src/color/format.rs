//! The text formats the app shows and copies to the clipboard.

use super::Rgb;

/// A way to write a color as text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Hex,
    Rgb,
    Hsl,
    Hsv,
    Cmyk,
    Oklch,
}

impl Format {
    pub const ALL: [Self; 6] = [
        Self::Hex,
        Self::Rgb,
        Self::Hsl,
        Self::Hsv,
        Self::Cmyk,
        Self::Oklch,
    ];

    /// Upper-case name for labels, e.g. `HSL`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Hex => "HEX",
            Self::Rgb => "RGB",
            Self::Hsl => "HSL",
            Self::Hsv => "HSV",
            Self::Cmyk => "CMYK",
            Self::Oklch => "OKLCH",
        }
    }

    /// Stable lower-case name for the settings file.
    pub fn key(self) -> &'static str {
        match self {
            Self::Hex => "hex",
            Self::Rgb => "rgb",
            Self::Hsl => "hsl",
            Self::Hsv => "hsv",
            Self::Cmyk => "cmyk",
            Self::Oklch => "oklch",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|f| f.key().eq_ignore_ascii_case(key.trim()))
    }

    /// Short value for display, e.g. `59, 130, 246`.
    pub fn display(self, color: Rgb, hex: HexStyle) -> String {
        match self {
            Self::Hex => hex.format(color),
            Self::Rgb => format!("{}, {}, {}", color.r, color.g, color.b),
            Self::Hsl => {
                let hsl = color.to_hsl();
                format!("{}°, {}%, {}%", degrees(hsl.h), pct(hsl.s), pct(hsl.l))
            }
            Self::Hsv => {
                let hsv = color.to_hsv();
                format!("{}°, {}%, {}%", degrees(hsv.h), pct(hsv.s), pct(hsv.v))
            }
            Self::Cmyk => {
                let c = color.to_cmyk();
                format!("{}%, {}%, {}%, {}%", pct(c.c), pct(c.m), pct(c.y), pct(c.k))
            }
            Self::Oklch => {
                let o = color.to_oklch();
                format!("{:.1}%, {:.3}, {:.1}°", o.l * 100.0, o.c, hue_tenths(o.h))
            }
        }
    }

    /// Full value for the clipboard, e.g. `rgb(59, 130, 246)`.
    pub fn copy_text(self, color: Rgb, hex: HexStyle) -> String {
        match self {
            Self::Hex => hex.format(color),
            Self::Rgb => format!("rgb({}, {}, {})", color.r, color.g, color.b),
            Self::Hsl => {
                let hsl = color.to_hsl();
                format!("hsl({}, {}%, {}%)", degrees(hsl.h), pct(hsl.s), pct(hsl.l))
            }
            Self::Hsv => {
                let hsv = color.to_hsv();
                format!("hsv({}, {}%, {}%)", degrees(hsv.h), pct(hsv.s), pct(hsv.v))
            }
            Self::Cmyk => {
                let c = color.to_cmyk();
                format!(
                    "cmyk({}%, {}%, {}%, {}%)",
                    pct(c.c),
                    pct(c.m),
                    pct(c.y),
                    pct(c.k)
                )
            }
            Self::Oklch => {
                let o = color.to_oklch();
                format!(
                    "oklch({:.1}% {:.3} {:.1})",
                    o.l * 100.0,
                    o.c,
                    hue_tenths(o.h)
                )
            }
        }
    }
}

/// How HEX values are written: letter case and the leading `#`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HexStyle {
    pub uppercase: bool,
    pub hash: bool,
}

impl Default for HexStyle {
    fn default() -> Self {
        Self {
            uppercase: true,
            hash: true,
        }
    }
}

impl HexStyle {
    pub const ALL: [Self; 4] = [
        Self {
            uppercase: true,
            hash: true,
        },
        Self {
            uppercase: false,
            hash: true,
        },
        Self {
            uppercase: true,
            hash: false,
        },
        Self {
            uppercase: false,
            hash: false,
        },
    ];

    pub fn format(self, color: Rgb) -> String {
        let hash = if self.hash { "#" } else { "" };
        if self.uppercase {
            format!("{hash}{:02X}{:02X}{:02X}", color.r, color.g, color.b)
        } else {
            format!("{hash}{:02x}{:02x}{:02x}", color.r, color.g, color.b)
        }
    }

    /// An example value, used as the option's label.
    pub fn label(self) -> &'static str {
        match (self.uppercase, self.hash) {
            (true, true) => "#ABCDEF",
            (false, true) => "#abcdef",
            (true, false) => "ABCDEF",
            (false, false) => "abcdef",
        }
    }

    /// Stable name for the settings file.
    pub fn key(self) -> &'static str {
        match (self.uppercase, self.hash) {
            (true, true) => "upper",
            (false, true) => "lower",
            (true, false) => "upper-no-hash",
            (false, false) => "lower-no-hash",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|s| s.key().eq_ignore_ascii_case(key.trim()))
    }
}

/// A fraction (0-1) as a whole percentage.
fn pct(v: f32) -> u32 {
    (v.clamp(0.0, 1.0) * 100.0).round() as u32
}

/// Whole degrees, where 359.6 rounds to 0 rather than 360.
fn degrees(h: f32) -> u32 {
    (h.round() as u32) % 360
}

/// Degrees to one decimal, where 359.96 rounds to 0.0 rather than 360.0.
fn hue_tenths(h: f32) -> f32 {
    let h = (h * 10.0).round() / 10.0;
    if h >= 360.0 { 0.0 } else { h }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLUE: Rgb = Rgb::new(0x3B, 0x82, 0xF6);

    #[test]
    fn display_and_copy_every_format() {
        let hex = HexStyle::default();
        let cases = [
            (Format::Hex, "#3B82F6", "#3B82F6"),
            (Format::Rgb, "59, 130, 246", "rgb(59, 130, 246)"),
            (Format::Hsl, "217°, 91%, 60%", "hsl(217, 91%, 60%)"),
            (Format::Hsv, "217°, 76%, 96%", "hsv(217, 76%, 96%)"),
            (Format::Cmyk, "76%, 47%, 0%, 4%", "cmyk(76%, 47%, 0%, 4%)"),
        ];
        for (format, display, copy) in cases {
            assert_eq!(format.display(BLUE, hex), display, "{format:?}");
            assert_eq!(format.copy_text(BLUE, hex), copy, "{format:?}");
        }
    }

    #[test]
    fn oklch_text() {
        let red = Rgb::new(255, 0, 0);
        let hex = HexStyle::default();
        assert_eq!(Format::Oklch.copy_text(red, hex), "oklch(62.8% 0.258 29.2)");
        assert_eq!(Format::Oklch.display(red, hex), "62.8%, 0.258, 29.2°");
        let white = Rgb::new(255, 255, 255);
        assert_eq!(
            Format::Oklch.copy_text(white, hex),
            "oklch(100.0% 0.000 0.0)"
        );
    }

    #[test]
    fn grays_and_black_have_clean_values() {
        let hex = HexStyle::default();
        assert_eq!(Format::Hsl.copy_text(Rgb::BLACK, hex), "hsl(0, 0%, 0%)");
        assert_eq!(Format::Hsv.copy_text(Rgb::BLACK, hex), "hsv(0, 0%, 0%)");
        assert_eq!(
            Format::Cmyk.copy_text(Rgb::BLACK, hex),
            "cmyk(0%, 0%, 0%, 100%)"
        );
        let gray = Rgb::new(128, 128, 128);
        assert_eq!(Format::Hsl.copy_text(gray, hex), "hsl(0, 0%, 50%)");
    }

    #[test]
    fn hex_styles() {
        let values: Vec<String> = HexStyle::ALL.iter().map(|s| s.format(BLUE)).collect();
        assert_eq!(values, ["#3B82F6", "#3b82f6", "3B82F6", "3b82f6"]);
        for style in HexStyle::ALL {
            assert_eq!(HexStyle::from_key(style.key()), Some(style));
        }
        assert_eq!(HexStyle::from_key("nope"), None);
    }

    #[test]
    fn format_keys_round_trip() {
        for format in Format::ALL {
            assert_eq!(Format::from_key(format.key()), Some(format));
        }
        assert_eq!(Format::from_key(" OKLCH "), Some(Format::Oklch));
        assert_eq!(Format::from_key("lab"), None);
    }

    #[test]
    fn hue_never_reads_360() {
        // Hue 359.8 must print as 0, not 360.
        let hex = HexStyle::default();
        let almost_red = Rgb::new(255, 0, 1);
        assert_eq!(Format::Hsl.copy_text(almost_red, hex), "hsl(0, 100%, 50%)");
    }
}
