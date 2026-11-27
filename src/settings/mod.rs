//! User settings: everything the Settings tab edits. Saved by [`crate::store`].

pub mod ui;

use crate::color::{Format, HexStyle, Rgb};
use crate::hotkey::Hotkey;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// `None` when the user removed the hotkey.
    pub hotkey: Option<Hotkey>,
    pub sample: SampleSize,
    /// Loupe zoom, one of [`ZOOM_LEVELS`].
    pub zoom: u8,
    /// Copy each picked color to the clipboard, as `copy_format`.
    pub copy_on_pick: bool,
    pub copy_format: Format,
    pub hex: HexStyle,
    /// Formats shown in the color details, never empty.
    pub formats: FormatSet,
    pub always_on_top: bool,
    pub remember_position: bool,
    pub tray: TrayMode,
    pub theme: ThemeMode,
    pub accent: AccentMode,
    /// The accent color when `accent` is [`AccentMode::Single`].
    pub accent_color: Rgb,
    /// Interface scale in percent, one of [`SCALES`].
    pub scale: u16,
    pub status_bar: bool,
    /// Image shown behind every tab, if any.
    pub background: Option<String>,
    /// Background image opacity, 0-100 %.
    pub background_opacity: u8,
    /// Opacity of pills, cards and bars, 30-100 %.
    pub panel_opacity: u8,
    /// A script font for the title.
    pub fancy_title: bool,
    pub tray_icon: IconStyle,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: Some(Hotkey::default()),
            sample: SampleSize::One,
            zoom: 8,
            copy_on_pick: true,
            copy_format: Format::Hex,
            hex: HexStyle::default(),
            formats: FormatSet::ALL,
            always_on_top: false,
            remember_position: true,
            tray: TrayMode::Close,
            theme: ThemeMode::Dark,
            accent: AccentMode::PerTab,
            accent_color: Rgb::new(0x19, 0xC2, 0x33),
            scale: 100,
            status_bar: true,
            background: None,
            background_opacity: 35,
            panel_opacity: 100,
            fancy_title: false,
            tray_icon: IconStyle::Wheel,
        }
    }
}

/// Interface scales in percent.
pub const SCALES: [u16; 8] = [75, 90, 100, 110, 125, 150, 175, 200];

/// The allowed scale closest to `scale`.
pub fn nearest_scale(scale: u16) -> u16 {
    SCALES
        .into_iter()
        .min_by_key(|&s| s.abs_diff(scale))
        .unwrap_or(100)
}

/// Lowest panel opacity, so controls never disappear.
pub const MIN_PANEL_OPACITY: u8 = 30;

/// A choice with a stable name in the settings file and a label.
macro_rules! keyed_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = ($key:literal, $label:literal)),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            pub fn key(self) -> &'static str {
                match self { $(Self::$variant => $key),+ }
            }

            pub fn label(self) -> &'static str {
                match self { $(Self::$variant => $label),+ }
            }

            pub fn from_key(key: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|v| v.key().eq_ignore_ascii_case(key.trim()))
            }
        }
    };
}

keyed_enum!(
    /// Dark, light, or following the Windows app theme.
    ThemeMode { Dark = ("dark", "Dark"), Light = ("light", "Light"), System = ("system", "System") }
);

keyed_enum!(
    /// A color per tab, or one color everywhere.
    AccentMode { PerTab = ("per-tab", "Per tab"), Single = ("single", "One color") }
);

keyed_enum!(
    /// The tray and taskbar icon.
    IconStyle { Wheel = ("wheel", "Wheel"), Pipette = ("pipette", "Pipette"), Mono = ("mono", "Mono") }
);

/// Loupe zoom levels: how many screen pixels one picked pixel covers.
pub const ZOOM_LEVELS: [u8; 5] = [4, 6, 8, 12, 16];

/// The allowed zoom level closest to `zoom`.
pub fn nearest_zoom(zoom: u8) -> u8 {
    ZOOM_LEVELS
        .into_iter()
        .min_by_key(|&z| z.abs_diff(zoom))
        .unwrap_or(8)
}

/// How many pixels a pick averages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleSize {
    One,
    Three,
    Five,
}

impl SampleSize {
    pub const ALL: [Self; 3] = [Self::One, Self::Three, Self::Five];

    /// Side of the averaged square, in pixels.
    pub fn side(self) -> u32 {
        match self {
            Self::One => 1,
            Self::Three => 3,
            Self::Five => 5,
        }
    }

    pub fn from_side(side: u32) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.side() == side)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::One => "1 px",
            Self::Three => "3×3",
            Self::Five => "5×5",
        }
    }
}

/// What the tray icon does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayMode {
    /// No tray icon; close quits.
    Off,
    /// Close (X) hides the window to the tray.
    Close,
    /// Minimize hides the window to the tray; close quits.
    Minimize,
}

impl TrayMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::Close, Self::Minimize];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Close => "Close",
            Self::Minimize => "Minimize",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Close => "close",
            Self::Minimize => "minimize",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|m| m.key().eq_ignore_ascii_case(key.trim()))
    }
}

/// A set of [`Format`]s, iterated in [`Format::ALL`] order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatSet(u8);

impl FormatSet {
    pub const ALL: Self = Self(0b11_1111);
    pub const EMPTY: Self = Self(0);

    fn bit(format: Format) -> u8 {
        let index = Format::ALL.iter().position(|&f| f == format).unwrap_or(0);
        1 << index
    }

    pub fn contains(self, format: Format) -> bool {
        self.0 & Self::bit(format) != 0
    }

    pub fn set(&mut self, format: Format, on: bool) {
        if on {
            self.0 |= Self::bit(format);
        } else {
            self.0 &= !Self::bit(format);
        }
    }

    pub fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn iter(self) -> impl Iterator<Item = Format> {
        Format::ALL.into_iter().filter(move |&f| self.contains(f))
    }
}

impl FromIterator<Format> for FormatSet {
    fn from_iter<I: IntoIterator<Item = Format>>(iter: I) -> Self {
        let mut set = Self::EMPTY;
        for format in iter {
            set.set(format, true);
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_set_keeps_order() {
        let set: FormatSet = [Format::Oklch, Format::Hex].into_iter().collect();
        assert_eq!(set.iter().collect::<Vec<_>>(), [Format::Hex, Format::Oklch]);
        assert_eq!(set.len(), 2);
        assert_eq!(FormatSet::ALL.len(), Format::ALL.len());
    }

    #[test]
    fn format_set_toggles() {
        let mut set = FormatSet::EMPTY;
        assert!(set.is_empty());
        set.set(Format::Cmyk, true);
        assert!(set.contains(Format::Cmyk) && !set.contains(Format::Hex));
        set.set(Format::Cmyk, false);
        assert!(set.is_empty());
    }

    #[test]
    fn zoom_snaps_to_levels() {
        assert_eq!(nearest_zoom(8), 8);
        assert_eq!(nearest_zoom(0), 4);
        assert_eq!(nearest_zoom(200), 16);
        assert_eq!(nearest_zoom(11), 12);
    }

    #[test]
    fn scale_snaps_to_steps() {
        assert_eq!(nearest_scale(100), 100);
        assert_eq!(nearest_scale(0), 75);
        assert_eq!(nearest_scale(1000), 200);
        assert_eq!(nearest_scale(130), 125);
    }

    #[test]
    fn appearance_keys_round_trip() {
        for &mode in ThemeMode::ALL {
            assert_eq!(ThemeMode::from_key(mode.key()), Some(mode));
        }
        for &mode in AccentMode::ALL {
            assert_eq!(AccentMode::from_key(mode.key()), Some(mode));
        }
        for &style in IconStyle::ALL {
            assert_eq!(IconStyle::from_key(style.key()), Some(style));
        }
        assert_eq!(ThemeMode::from_key(" LIGHT "), Some(ThemeMode::Light));
        assert_eq!(IconStyle::from_key("nope"), None);
    }

    #[test]
    fn keys_round_trip() {
        for mode in TrayMode::ALL {
            assert_eq!(TrayMode::from_key(mode.key()), Some(mode));
        }
        for size in SampleSize::ALL {
            assert_eq!(SampleSize::from_side(size.side()), Some(size));
        }
    }
}
