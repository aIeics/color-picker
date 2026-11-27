//! Colors, fonts and widget style. The palettes, radii and per-tab accent
//! colors follow BlurAutoClicker by Blur009; the font is Roboto Flex.
//!
//! One palette is active at a time. Widgets read it with [`p`]; the app
//! changes it with [`set`] when the theme, accent or panel opacity changes.

use std::sync::RwLock;

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, Theme, Vec2};

/// Every color the interface uses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub light: bool,
    /// Behind everything.
    pub base: Color32,
    /// Pills, cards, the title and status bars.
    pub surface: Color32,
    /// Hovered segments and buttons, the active nav item.
    pub elevated: Color32,
    /// Selected chips, pressed buttons.
    pub input: Color32,
    /// The track of segmented controls.
    pub input_off: Color32,
    pub border: Color32,
    pub border_subtle: Color32,
    pub card_border: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub text_dim: Color32,
    pub label: Color32,
    pub divider: Color32,
    pub danger: Color32,
    pub danger_bg: Color32,
    /// "ON" in on/off toggles.
    pub ok: Color32,
    /// The window border Windows 11 draws, as RGB.
    pub frame: [u8; 3],
    pub green: Color32,
    pub blue: Color32,
    pub amber: Color32,
    pub teal: Color32,
    pub violet: Color32,
}

/// A color with straight alpha, premultiplied as `Color32` stores it.
const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    const fn m(c: u8, a: u8) -> u8 {
        ((c as u16 * a as u16 + 127) / 255) as u8
    }
    Color32::from_rgba_premultiplied(m(r, a), m(g, a), m(b, a), a)
}

pub const DARK: Palette = Palette {
    light: false,
    base: Color32::from_rgb(12, 12, 14),
    surface: Color32::from_rgb(26, 26, 26),
    elevated: Color32::from_rgb(38, 38, 38),
    input: Color32::from_rgb(59, 59, 59),
    input_off: Color32::from_rgb(51, 51, 51),
    border: rgba(251, 254, 254, 51),
    border_subtle: rgba(251, 254, 254, 26),
    card_border: rgba(251, 254, 254, 31),
    text: rgba(251, 254, 254, 230),
    text_muted: rgba(251, 254, 254, 191),
    text_dim: rgba(251, 254, 254, 128),
    label: rgba(251, 254, 254, 214),
    divider: rgba(255, 255, 255, 64),
    danger: Color32::from_rgb(255, 117, 107),
    danger_bg: rgba(248, 113, 113, 38),
    ok: Color32::from_rgb(25, 194, 51),
    frame: [51, 51, 53],
    green: Color32::from_rgb(25, 194, 51),
    blue: Color32::from_rgb(57, 154, 239),
    amber: Color32::from_rgb(254, 188, 47),
    teal: Color32::from_rgb(27, 187, 187),
    violet: Color32::from_rgb(204, 112, 235),
};

pub const LIGHT: Palette = Palette {
    light: true,
    base: Color32::from_rgb(229, 223, 231),
    surface: Color32::from_rgb(255, 255, 255),
    elevated: Color32::from_rgb(242, 242, 242),
    input: Color32::from_rgb(217, 217, 217),
    input_off: Color32::from_rgb(227, 227, 227),
    border: Color32::from_rgb(102, 102, 102),
    border_subtle: Color32::from_rgb(196, 196, 196),
    card_border: Color32::from_rgb(207, 207, 207),
    text: Color32::from_rgb(0, 0, 0),
    text_muted: rgba(0, 0, 0, 209),
    text_dim: rgba(0, 0, 0, 140),
    label: rgba(0, 0, 0, 217),
    divider: rgba(0, 0, 0, 46),
    danger: Color32::from_rgb(209, 27, 15),
    danger_bg: rgba(230, 15, 0, 31),
    ok: Color32::from_rgb(18, 135, 35),
    frame: [165, 161, 166],
    green: Color32::from_rgb(18, 135, 35),
    blue: Color32::from_rgb(27, 112, 187),
    amber: Color32::from_rgb(168, 111, 0),
    teal: Color32::from_rgb(19, 134, 134),
    violet: Color32::from_rgb(150, 53, 182),
};

/// What decides the active palette.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub light: bool,
    /// One accent for every tab, if the user chose one.
    pub accent: Option<Color32>,
    /// Opacity of pills, cards and bars (0.3-1), so a background image can
    /// show through.
    pub panel_opacity: f32,
}

struct State {
    palette: Palette,
    accent: Option<Color32>,
}

static STATE: RwLock<State> = RwLock::new(State {
    palette: DARK,
    accent: None,
});

/// The active palette.
pub fn p() -> Palette {
    STATE.read().map_or(DARK, |s| s.palette)
}

/// The accent every tab uses, if the user chose a single one.
pub fn single_accent() -> Option<Color32> {
    STATE.read().ok().and_then(|s| s.accent)
}

/// Switches the palette and egui's style to `look`.
pub fn set(ctx: &egui::Context, look: Look) {
    let mut palette = if look.light { LIGHT } else { DARK };
    let alpha = look.panel_opacity.clamp(0.3, 1.0);
    if alpha < 1.0 {
        for color in [
            &mut palette.surface,
            &mut palette.elevated,
            &mut palette.input,
            &mut palette.input_off,
        ] {
            *color = color.gamma_multiply(alpha);
        }
    }
    if let Ok(mut state) = STATE.write() {
        state.palette = palette;
        state.accent = look.accent;
    }
    apply_style(ctx, look.light);
}

/// Corner radius of pills (BlurAutoClicker's radius-md).
pub const PILL_RADIUS: u8 = 8;
/// Corner radius of segments, title-bar buttons (radius-sm).
pub const SEGMENT_RADIUS: u8 = 4;

fn apply_style(ctx: &egui::Context, light: bool) {
    // Menus and tooltips stay opaque, whatever the panel opacity.
    let solid = if light { LIGHT } else { DARK };
    let theme = if light { Theme::Light } else { Theme::Dark };
    ctx.set_theme(theme);
    ctx.style_mut_of(theme, |style| {
        style.animation_time = 0.15;
        let spacing = &mut style.spacing;
        spacing.item_spacing = Vec2::new(8.0, 8.0);
        spacing.button_padding = Vec2::new(10.0, 6.0);
        spacing.interact_size.y = 28.0;
        spacing.menu_margin = egui::Margin::same(5);
        spacing.scroll.bar_width = 6.0;
        spacing.scroll.floating = true;

        let text = &mut style.text_styles;
        text.insert(egui::TextStyle::Heading, bold(16.0));
        text.insert(egui::TextStyle::Body, FontId::proportional(13.5));
        text.insert(egui::TextStyle::Button, FontId::proportional(13.5));
        text.insert(egui::TextStyle::Small, FontId::proportional(11.5));

        let v = &mut style.visuals;
        v.dark_mode = !light;
        v.override_text_color = Some(solid.text);
        v.panel_fill = solid.base;
        v.window_fill = solid.surface;
        v.window_stroke = Stroke::new(1.0, solid.border);
        v.window_corner_radius = CornerRadius::same(PILL_RADIUS);
        v.menu_corner_radius = CornerRadius::same(PILL_RADIUS);
        v.window_shadow = egui::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(if light { 60 } else { 115 }),
        };
        v.popup_shadow = v.window_shadow;
        v.extreme_bg_color = solid.base;
        v.text_edit_bg_color = Some(solid.base);
        v.faint_bg_color = solid.surface;
        v.selection.bg_fill = if light {
            Color32::from_rgba_premultiplied(0, 0, 0, 40)
        } else {
            Color32::from_white_alpha(60)
        };
        v.selection.stroke = Stroke::new(1.0, solid.text);
        v.text_cursor.stroke = Stroke::new(1.5, solid.text);

        let widgets = &mut v.widgets;
        for (state, fill, stroke) in [
            (
                &mut widgets.noninteractive,
                solid.surface,
                solid.border_subtle,
            ),
            (&mut widgets.inactive, solid.elevated, Color32::TRANSPARENT),
            (&mut widgets.hovered, solid.elevated, solid.border),
            (&mut widgets.active, solid.input, solid.text),
            (&mut widgets.open, solid.elevated, solid.border),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = Stroke::new(1.0, stroke);
            state.corner_radius = CornerRadius::same(SEGMENT_RADIUS);
            state.fg_stroke = Stroke::new(1.0, solid.text);
            state.expansion = 0.0;
        }
        widgets.noninteractive.fg_stroke = Stroke::new(1.0, solid.text_muted);
    });
}

/// Roboto Flex, a variable font (SIL Open Font License, see
/// `assets/fonts/RobotoFlex-OFL.txt`). One file gives every weight we use.
const ROBOTO_FLEX: &[u8] = include_bytes!("../../assets/fonts/RobotoFlex-Variable.ttf");

/// 500: descriptions and the status bar.
pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}

/// 600: labels and most text (the default).
pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}

/// 780: values, titles and buttons.
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}

/// Segoe Script, for the fancy title; bold Roboto Flex where it's missing.
pub fn fancy(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("fancy".into()))
}

/// Installs Roboto Flex at three weights, and Segoe Script from Windows for
/// the fancy title (read at runtime, not bundled). egui's built-in fonts stay
/// behind them as fallbacks for missing glyphs.
pub fn install_fonts(ctx: &egui::Context) {
    let weight = |wght: f32| {
        egui::FontData::from_static(ROBOTO_FLEX).tweak(egui::epaint::text::FontTweak {
            coords: egui::epaint::text::VariationCoords::new([(*b"wght", wght)]),
            ..Default::default()
        })
    };
    let mut fonts = egui::FontDefinitions::default();
    let fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let family = |first: &str| {
        let mut list = vec![first.to_owned()];
        list.extend(fallbacks.iter().cloned());
        list
    };
    for (name, wght) in [("rf-500", 500.0), ("rf-600", 600.0), ("rf-780", 780.0)] {
        fonts.font_data.insert(name.into(), weight(wght).into());
    }
    fonts
        .families
        .insert(FontFamily::Proportional, family("rf-600"));
    fonts
        .families
        .insert(FontFamily::Name("medium".into()), family("rf-500"));
    fonts
        .families
        .insert(FontFamily::Name("bold".into()), family("rf-780"));

    let mut fancy = family("rf-780");
    if let Some(bytes) = windows_font("segoesc.ttf") {
        fonts
            .font_data
            .insert("fancy".into(), egui::FontData::from_owned(bytes).into());
        fancy.insert(0, "fancy".into());
    }
    fonts
        .families
        .insert(FontFamily::Name("fancy".into()), fancy);
    ctx.set_fonts(fonts);
}

/// A font file from the Windows fonts folder, if there is one.
fn windows_font(file: &str) -> Option<Vec<u8>> {
    let dir = std::env::var_os("WINDIR")?;
    std::fs::read(std::path::Path::new(&dir).join("Fonts").join(file)).ok()
}
