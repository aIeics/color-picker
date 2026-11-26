//! The color editor's state. While sliders move, HSL and HSV are kept as
//! floats and are the source of truth, so hue and saturation don't snap
//! when the color passes through grays or black.

pub mod ui;

use crate::color::{HexStyle, Hsl, Hsv, Rgb};

/// Which sliders the editor shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Rgb,
    Hsl,
    Hsv,
}

impl Mode {
    pub const ALL: [Self; 3] = [Self::Rgb, Self::Hsl, Self::Hsv];

    pub fn label(self) -> &'static str {
        match self {
            Self::Rgb => "RGB",
            Self::Hsl => "HSL",
            Self::Hsv => "HSV",
        }
    }
}

#[derive(Debug)]
pub struct Editor {
    pub mode: Mode,
    hsl: Hsl,
    hsv: Hsv,
    /// The color `hsl` and `hsv` describe; anything else means the color
    /// was changed elsewhere and the editor follows it.
    synced: Option<Rgb>,
    /// What the HEX field shows.
    pub hex_text: String,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            hsl: Rgb::BLACK.to_hsl(),
            hsv: Rgb::BLACK.to_hsv(),
            synced: None,
            hex_text: String::new(),
        }
    }
}

impl Editor {
    pub fn hsl(&self) -> Hsl {
        self.hsl
    }

    pub fn hsv(&self) -> Hsv {
        self.hsv
    }

    /// Follows `color` if it was changed outside the editor. The HEX field
    /// is left alone while the user types in it.
    pub fn sync(&mut self, color: Rgb, hex: HexStyle, typing: bool) {
        if self.synced != Some(color) {
            self.hsl = color.to_hsl();
            self.hsv = color.to_hsv();
            self.synced = Some(color);
            if !typing {
                self.hex_text = hex.format(color);
            }
        } else if !typing && Rgb::from_hex(&self.hex_text) != Some(color) {
            self.hex_text = hex.format(color);
        }
    }

    /// A new color from the RGB sliders or the HEX field.
    pub fn set_rgb(&mut self, color: Rgb) -> Rgb {
        let hsl = color.to_hsl();
        let hsv = color.to_hsv();
        // Grays have no hue: keep the one the user was on.
        self.hsl = keep_hue(hsl, hsl.s, self.hsl.h);
        self.hsv = Hsv {
            h: if hsv.s == 0.0 { self.hsv.h } else { hsv.h },
            ..hsv
        };
        self.synced = Some(color);
        color
    }

    pub fn set_hsl(&mut self, hsl: Hsl) -> Rgb {
        let color = Rgb::from_hsl(hsl);
        self.hsl = hsl;
        let hsv = color.to_hsv();
        self.hsv = Hsv {
            h: if hsv.s == 0.0 { hsl.h } else { hsv.h },
            ..hsv
        };
        self.synced = Some(color);
        color
    }

    pub fn set_hsv(&mut self, hsv: Hsv) -> Rgb {
        let color = Rgb::from_hsv(hsv);
        self.hsv = hsv;
        let hsl = color.to_hsl();
        self.hsl = keep_hue(hsl, hsl.s, hsv.h);
        self.synced = Some(color);
        color
    }
}

fn keep_hue(hsl: Hsl, saturation: f32, fallback: f32) -> Hsl {
    Hsl {
        h: if saturation == 0.0 { fallback } else { hsl.h },
        ..hsl
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hue_survives_grays() {
        let mut editor = Editor::default();
        editor.sync(Rgb::new(128, 128, 128), HexStyle::default(), false);
        // Pick a hue while gray, then add saturation: the hue sticks.
        editor.set_hsl(Hsl {
            h: 200.0,
            s: 0.0,
            l: 0.5,
        });
        let color = editor.set_hsl(Hsl {
            s: 1.0,
            ..editor.hsl()
        });
        assert!((color.to_hsl().h - 200.0).abs() < 1.0, "{color:?}");
    }

    #[test]
    fn hsv_value_zero_keeps_hue_and_saturation() {
        let mut editor = Editor::default();
        editor.set_hsv(Hsv {
            h: 120.0,
            s: 0.8,
            v: 0.0,
        });
        assert_eq!(editor.hsv().h, 120.0);
        assert_eq!(editor.hsv().s, 0.8);
        let color = editor.set_hsv(Hsv {
            v: 1.0,
            ..editor.hsv()
        });
        assert_eq!(
            color,
            Rgb::from_hsv(Hsv {
                h: 120.0,
                s: 0.8,
                v: 1.0
            })
        );
    }

    #[test]
    fn follows_outside_changes_but_not_its_own() {
        let mut editor = Editor::default();
        let hex = HexStyle::default();
        let own = editor.set_hsv(Hsv {
            h: 300.0,
            s: 0.0,
            v: 0.4,
        });
        editor.sync(own, hex, false);
        assert_eq!(
            editor.hsv().h,
            300.0,
            "its own change must not reset the hue"
        );

        editor.sync(Rgb::new(255, 0, 0), hex, false);
        assert_eq!(editor.hsv().h, 0.0);
        assert_eq!(editor.hex_text, "#FF0000");
    }

    #[test]
    fn typing_is_not_overwritten() {
        let mut editor = Editor {
            hex_text: "#12".to_owned(),
            ..Editor::default()
        };
        editor.sync(Rgb::new(1, 2, 3), HexStyle::default(), true);
        assert_eq!(editor.hex_text, "#12");
        editor.sync(Rgb::new(1, 2, 3), HexStyle::default(), false);
        assert_eq!(editor.hex_text, "#010203");
    }
}
