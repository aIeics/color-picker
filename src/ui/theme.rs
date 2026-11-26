//! Shared look: black background, white text, Manrope and rounded pills.
//!
//! The colors and font come from clock-app; the layout follows
//! BlurAutoClicker. Keeping the style in one module means no widget falls
//! back to egui's default colors or fonts.

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, Theme, Vec2};

pub const BG: Color32 = Color32::BLACK;
/// Pill fill.
pub const SURFACE: Color32 = Color32::from_rgb(17, 17, 17);
/// Hovered segment inside a pill, hovered title-bar button.
pub const SURFACE_HI: Color32 = Color32::from_rgb(36, 36, 36);
/// Pressed or selected segment.
pub const SURFACE_PRESS: Color32 = Color32::from_rgb(54, 54, 54);
pub const BORDER: Color32 = Color32::from_rgb(40, 40, 40);
pub const DIVIDER: Color32 = Color32::from_rgb(62, 62, 62);
pub const TEXT: Color32 = Color32::WHITE;
/// Labels next to values ("Duty Cycle" in BlurAutoClicker).
pub const LABEL: Color32 = Color32::from_rgb(214, 214, 214);
pub const TEXT_DIM: Color32 = Color32::from_rgb(140, 140, 140);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(90, 90, 90);
pub const DANGER: Color32 = Color32::from_rgb(255, 117, 107);

/// Corner radius of pills.
pub const PILL_RADIUS: u8 = 8;
/// Corner radius of segments inside pills and title-bar buttons.
pub const SEGMENT_RADIUS: u8 = 5;

/// Manrope, a variable font (SIL Open Font License, see
/// `assets/fonts/Manrope-OFL.txt`). One file gives every weight we use.
const MANROPE: &[u8] = include_bytes!("../../assets/fonts/Manrope-Variable.ttf");

/// Installs the fonts and style. The theme is forced to dark so the look
/// doesn't depend on the Windows light/dark setting.
pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.set_theme(Theme::Dark);
    ctx.style_mut_of(Theme::Dark, |style| {
        style.animation_time = 0.12;

        let spacing = &mut style.spacing;
        spacing.item_spacing = Vec2::new(8.0, 8.0);
        spacing.button_padding = Vec2::new(10.0, 6.0);
        spacing.interact_size.y = 28.0;
        spacing.menu_margin = egui::Margin::same(5);

        let text = &mut style.text_styles;
        text.insert(egui::TextStyle::Heading, bold(15.0));
        text.insert(egui::TextStyle::Body, FontId::proportional(13.0));
        text.insert(egui::TextStyle::Button, FontId::proportional(13.0));
        text.insert(egui::TextStyle::Small, FontId::proportional(11.0));

        let visuals = &mut style.visuals;
        visuals.dark_mode = true;
        visuals.override_text_color = None;
        visuals.panel_fill = BG;
        visuals.window_fill = SURFACE;
        visuals.window_stroke = Stroke::new(1.0, BORDER);
        visuals.window_corner_radius = CornerRadius::same(12);
        visuals.menu_corner_radius = CornerRadius::same(PILL_RADIUS);
        visuals.window_shadow = egui::Shadow {
            offset: [0, 6],
            blur: 18,
            spread: 0,
            color: Color32::from_black_alpha(200),
        };
        visuals.popup_shadow = visuals.window_shadow;
        visuals.extreme_bg_color = SURFACE;
        visuals.text_edit_bg_color = Some(SURFACE);
        visuals.faint_bg_color = SURFACE;
        visuals.selection.bg_fill = Color32::from_white_alpha(70);
        visuals.selection.stroke = Stroke::new(1.0, TEXT);
        visuals.text_cursor.stroke = Stroke::new(1.5, TEXT);

        let widgets = &mut visuals.widgets;
        for (state, fill, stroke) in [
            (&mut widgets.noninteractive, SURFACE, BORDER),
            (&mut widgets.inactive, SURFACE, BORDER),
            (&mut widgets.hovered, SURFACE_HI, TEXT_DIM),
            (&mut widgets.active, SURFACE_PRESS, TEXT),
            (&mut widgets.open, SURFACE_HI, TEXT_DIM),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = Stroke::new(1.0, stroke);
            state.corner_radius = CornerRadius::same(SEGMENT_RADIUS);
            state.fg_stroke = Stroke::new(1.0, TEXT);
            state.expansion = 0.0;
        }
        // Text fields keep a quiet border until focused.
        widgets.inactive.bg_stroke = Stroke::NONE;
        widgets.hovered.bg_stroke = Stroke::NONE;
    });
}

/// The font family for headings and values.
pub fn bold_family() -> FontFamily {
    FontFamily::Name("bold".into())
}

/// A bold font at `size`, for the title and values.
pub fn bold(size: f32) -> FontId {
    FontId::new(size, bold_family())
}

/// The regular (semi-bold) font at `size`, for labels.
pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}

/// Makes Manrope semi-bold the default text font and adds a bold family.
/// egui's built-in fonts stay behind them as fallbacks for missing glyphs.
fn install_fonts(ctx: &egui::Context) {
    let weight = |wght: f32| {
        egui::FontData::from_static(MANROPE).tweak(egui::epaint::text::FontTweak {
            coords: egui::epaint::text::VariationCoords::new([(*b"wght", wght)]),
            ..Default::default()
        })
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("manrope-semibold".into(), weight(600.0).into());
    fonts
        .font_data
        .insert("manrope-bold".into(), weight(780.0).into());

    let fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let with = |first: &str| {
        let mut list = vec![first.to_string()];
        list.extend(fallbacks.iter().cloned());
        list
    };
    fonts
        .families
        .insert(FontFamily::Proportional, with("manrope-semibold"));
    fonts.families.insert(bold_family(), with("manrope-bold"));
    ctx.set_fonts(fonts);
}
