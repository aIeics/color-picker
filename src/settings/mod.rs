//! User settings: everything the Settings tab edits. Saved by [`crate::store`].

pub mod ui;

use crate::color::{Format, HexStyle};
use crate::hotkey::Hotkey;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// `None` when the user removed the hotkey.
    pub hotkey: Option<Hotkey>,
    pub sample: SampleSize,
    /// Loupe zoom, one of [`ZOOM_LEVELS`].
    pub zoom: u8,
    /// Format copied to the clipboard after each pick, `None` for off.
    pub copy_on_pick: Option<Format>,
    pub hex: HexStyle,
    /// Formats shown in the color details, never empty.
    pub formats: FormatSet,
    pub always_on_top: bool,
    pub remember_position: bool,
    pub tray: TrayMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: Some(Hotkey::default()),
            sample: SampleSize::One,
            zoom: 8,
            copy_on_pick: Some(Format::Hex),
            hex: HexStyle::default(),
            formats: FormatSet::ALL,
            always_on_top: false,
            remember_position: true,
            tray: TrayMode::Close,
        }
    }
}

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
    pub const ALL: [Self; 3] = [Self::Close, Self::Minimize, Self::Off];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Close => "Close to tray",
            Self::Minimize => "Minimize to tray",
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
    fn keys_round_trip() {
        for mode in TrayMode::ALL {
            assert_eq!(TrayMode::from_key(mode.key()), Some(mode));
        }
        for size in SampleSize::ALL {
            assert_eq!(SampleSize::from_side(size.side()), Some(size));
        }
    }
}
