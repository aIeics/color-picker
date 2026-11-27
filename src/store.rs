//! Saving settings, the current color, history and palette between runs.
//!
//! Everything goes into one small `key = value` text file:
//! `%APPDATA%\color-picker\settings.txt` on Windows. Reading never fails:
//! unknown keys are ignored and bad values fall back to their defaults, so a
//! hand-edited or damaged file can't stop the app from starting.

use std::io;
use std::path::PathBuf;

use crate::color::{Format, HexStyle, Rgb};
use crate::history::History;
use crate::hotkey::Hotkey;
use crate::palette::Palette;
use crate::settings::{
    self, AccentMode, FormatSet, IconStyle, SampleSize, Settings, ThemeMode, TrayMode,
};

/// The color shown on first start.
pub const DEFAULT_COLOR: Rgb = Rgb::new(0x3B, 0x82, 0xF6);

/// Everything saved between runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Saved {
    pub settings: Settings,
    pub color: Rgb,
    pub history: Vec<Rgb>,
    pub palette: Vec<Rgb>,
    /// Top-left corner of the window, in physical pixels.
    pub window_pos: Option<[i32; 2]>,
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            color: DEFAULT_COLOR,
            history: Vec::new(),
            palette: Vec::new(),
            window_pos: None,
        }
    }
}

/// Where the file lives, or `None` if there is no config folder.
pub fn path() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    base.map(|dir| dir.join("color-picker").join("settings.txt"))
}

/// Loads the saved state, or the defaults if there is none.
pub fn load() -> Saved {
    path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|text| decode(&text))
        .unwrap_or_default()
}

/// Writes the state to a temporary file and renames it over the old one, so
/// a crash or shutdown mid-write never leaves a half-written file.
pub fn save(saved: &Saved) -> io::Result<()> {
    let path = path().ok_or_else(|| io::Error::other("no config folder"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension("txt.tmp");
    std::fs::write(&temp, encode(saved))?;
    std::fs::rename(&temp, &path)
}

pub fn encode(saved: &Saved) -> String {
    let s = &saved.settings;
    let colors = |colors: &[Rgb]| {
        colors
            .iter()
            .map(|&c| HexStyle::default().format(c))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let formats = s.formats.iter().map(Format::key).collect::<Vec<_>>();

    let mut lines = vec![
        "# Color Picker settings. Delete this file to reset everything.".to_owned(),
        format!(
            "hotkey = {}",
            s.hotkey.map_or_else(|| "none".to_owned(), Hotkey::to_text)
        ),
        format!("sample = {}", s.sample.side()),
        format!("zoom = {}", s.zoom),
        format!("copy_on_pick = {}", on_off(s.copy_on_pick)),
        format!("copy_format = {}", s.copy_format.key()),
        format!("hex = {}", s.hex.key()),
        format!("formats = {}", formats.join(", ")),
        format!("always_on_top = {}", s.always_on_top),
        format!("remember_position = {}", s.remember_position),
        format!("tray = {}", s.tray.key()),
        format!("theme = {}", s.theme.key()),
        format!("accent = {}", s.accent.key()),
        format!(
            "accent_color = {}",
            HexStyle::default().format(s.accent_color)
        ),
        format!("scale = {}", s.scale),
        format!("status_bar = {}", on_off(s.status_bar)),
        format!("background = {}", s.background.as_deref().unwrap_or("")),
        format!("background_opacity = {}", s.background_opacity),
        format!("panel_opacity = {}", s.panel_opacity),
        format!("fancy_title = {}", on_off(s.fancy_title)),
        format!("tray_icon = {}", s.tray_icon.key()),
        format!("color = {}", HexStyle::default().format(saved.color)),
        format!("history = {}", colors(&saved.history)),
        format!("palette = {}", colors(&saved.palette)),
    ];
    if let Some([x, y]) = saved.window_pos {
        lines.push(format!("window = {x}, {y}"));
    }
    lines.push(String::new());
    lines.join("\n")
}

pub fn decode(text: &str) -> Saved {
    let mut saved = Saved::default();
    let s = &mut saved.settings;

    for line in text.lines() {
        let line = line.trim_start_matches('\u{feff}').trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "hotkey" => {
                s.hotkey = match value.to_ascii_lowercase().as_str() {
                    "" | "none" | "off" => None,
                    _ => Hotkey::parse(value).or(s.hotkey),
                }
            }
            "sample" => {
                if let Some(size) = value.parse().ok().and_then(SampleSize::from_side) {
                    s.sample = size;
                }
            }
            "zoom" => {
                if let Ok(zoom) = value.parse::<u32>() {
                    s.zoom = settings::nearest_zoom(zoom.min(255) as u8);
                }
            }
            "copy_on_pick" => {
                if let Some(on) = parse_bool(value) {
                    s.copy_on_pick = on;
                } else if let Some(format) = Format::from_key(value) {
                    // Older files: the format itself meant "on".
                    s.copy_on_pick = true;
                    s.copy_format = format;
                }
            }
            "copy_format" => s.copy_format = Format::from_key(value).unwrap_or(s.copy_format),
            "hex" => s.hex = HexStyle::from_key(value).unwrap_or(s.hex),
            "formats" => {
                let set: FormatSet = value.split(',').filter_map(Format::from_key).collect();
                if !set.is_empty() {
                    s.formats = set;
                }
            }
            "always_on_top" => s.always_on_top = parse_bool(value).unwrap_or(s.always_on_top),
            "remember_position" => {
                s.remember_position = parse_bool(value).unwrap_or(s.remember_position);
            }
            "tray" => s.tray = TrayMode::from_key(value).unwrap_or(s.tray),
            "theme" => s.theme = ThemeMode::from_key(value).unwrap_or(s.theme),
            "accent" => s.accent = AccentMode::from_key(value).unwrap_or(s.accent),
            "accent_color" => s.accent_color = Rgb::from_hex(value).unwrap_or(s.accent_color),
            "scale" => {
                if let Ok(scale) = value.parse::<u32>() {
                    s.scale = settings::nearest_scale(scale.min(1000) as u16);
                }
            }
            "status_bar" => s.status_bar = parse_bool(value).unwrap_or(s.status_bar),
            "background" => s.background = (!value.is_empty()).then(|| value.to_owned()),
            "background_opacity" => {
                if let Ok(v) = value.parse::<u32>() {
                    s.background_opacity = v.min(100) as u8;
                }
            }
            "panel_opacity" => {
                if let Ok(v) = value.parse::<u32>() {
                    s.panel_opacity = v.clamp(u32::from(settings::MIN_PANEL_OPACITY), 100) as u8;
                }
            }
            "fancy_title" => s.fancy_title = parse_bool(value).unwrap_or(s.fancy_title),
            "tray_icon" => s.tray_icon = IconStyle::from_key(value).unwrap_or(s.tray_icon),
            "color" => saved.color = Rgb::from_hex(value).unwrap_or(saved.color),
            "history" => {
                saved.history = History::new(parse_colors(value)).colors().to_vec();
            }
            "palette" => {
                saved.palette = Palette::new(parse_colors(value)).colors().to_vec();
            }
            "window" => saved.window_pos = parse_pos(value),
            _ => {}
        }
    }
    saved
}

fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "on" | "yes" | "1" => Some(true),
        "false" | "off" | "no" | "0" => Some(false),
        _ => None,
    }
}

fn parse_colors(value: &str) -> impl Iterator<Item = Rgb> + '_ {
    value.split(',').filter_map(Rgb::from_hex)
}

fn parse_pos(value: &str) -> Option<[i32; 2]> {
    let (x, y) = value.split_once(',')?;
    let plausible = |v: &str| v.trim().parse::<i32>().ok().filter(|v| v.abs() < 100_000);
    Some([plausible(x)?, plausible(y)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Saved {
        Saved {
            settings: Settings {
                hotkey: Hotkey::parse("Win+Shift+C"),
                sample: SampleSize::Five,
                zoom: 12,
                copy_on_pick: false,
                copy_format: Format::Cmyk,
                hex: HexStyle {
                    uppercase: false,
                    hash: false,
                },
                formats: [Format::Hex, Format::Oklch].into_iter().collect(),
                always_on_top: true,
                remember_position: false,
                tray: TrayMode::Minimize,
                theme: ThemeMode::System,
                accent: AccentMode::Single,
                accent_color: Rgb::new(0x12, 0x34, 0x56),
                scale: 125,
                status_bar: false,
                background: Some(r"C:\Pictures\a = b.png".to_owned()),
                background_opacity: 0,
                panel_opacity: 60,
                fancy_title: true,
                tray_icon: IconStyle::Mono,
            },
            color: Rgb::new(1, 2, 3),
            history: vec![Rgb::new(255, 0, 0), Rgb::new(0, 255, 0)],
            palette: vec![Rgb::new(0, 0, 255)],
            window_pos: Some([-1200, 40]),
        }
    }

    #[test]
    fn round_trip() {
        let saved = sample();
        assert_eq!(decode(&encode(&saved)), saved);
        assert_eq!(decode(&encode(&Saved::default())), Saved::default());
    }

    #[test]
    fn no_hotkey_round_trips() {
        let mut saved = Saved::default();
        saved.settings.hotkey = None;
        assert_eq!(decode(&encode(&saved)).settings.hotkey, None);
    }

    #[test]
    fn garbage_falls_back_to_defaults() {
        let text = "\u{feff}zoom = 999999\nsample = 4\nhex = purple\nformats = lab, xyz\n\
                    tray = maybe\nhotkey = Ctrl+Nope\ncolor = #12\nwindow = 1, nope\n\
                    always_on_top = sure\n= =\nno equals sign\nhistory = #FFF, junk, #000000\n\
                    \u{0}\u{1}=\u{2}";
        let saved = decode(text);
        let defaults = Saved::default();
        assert_eq!(saved.settings.zoom, 16);
        assert_eq!(saved.settings.sample, defaults.settings.sample);
        assert_eq!(saved.settings.hex, defaults.settings.hex);
        assert_eq!(saved.settings.formats, FormatSet::ALL);
        assert_eq!(saved.settings.tray, defaults.settings.tray);
        assert_eq!(saved.settings.hotkey, defaults.settings.hotkey);
        assert_eq!(saved.color, DEFAULT_COLOR);
        assert_eq!(saved.window_pos, None);
        assert!(!saved.settings.always_on_top);
        assert_eq!(saved.history, [Rgb::new(255, 255, 255), Rgb::BLACK]);
    }

    #[test]
    fn empty_and_binary_input_never_panic() {
        assert_eq!(decode(""), Saved::default());
        let noise: String = (0..2000u32).filter_map(char::from_u32).collect();
        let _ = decode(&noise);
    }

    #[test]
    fn long_lists_are_capped() {
        let many = (0..=255)
            .map(|v| format!("#{v:02X}0000"))
            .collect::<Vec<_>>()
            .join(",");
        let saved = decode(&format!("history = {many}\npalette = {many}"));
        assert_eq!(saved.history.len(), History::MAX);
        assert_eq!(saved.palette.len(), Palette::MAX);
    }

    #[test]
    fn older_copy_on_pick_values() {
        let saved = decode("copy_on_pick = rgb");
        assert!(saved.settings.copy_on_pick);
        assert_eq!(saved.settings.copy_format, Format::Rgb);
        let saved = decode("copy_on_pick = off");
        assert!(!saved.settings.copy_on_pick);
        assert_eq!(saved.settings.copy_format, Format::Hex);
    }

    #[test]
    fn appearance_values_are_clamped() {
        let saved = decode(
            "scale = 999\npanel_opacity = 5\nbackground_opacity = 300\nbackground =   \n\
             theme = purple\ntray_icon = ???\naccent_color = nope",
        );
        let s = saved.settings;
        let d = Settings::default();
        assert_eq!(s.scale, 200);
        assert_eq!(s.panel_opacity, settings::MIN_PANEL_OPACITY);
        assert_eq!(s.background_opacity, 100);
        assert_eq!(s.background, None);
        assert_eq!(
            (s.theme, s.tray_icon, s.accent_color),
            (d.theme, d.tray_icon, d.accent_color)
        );
    }
}
