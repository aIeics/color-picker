//! Shared look: black background, white text, Manrope and rounded widgets.
//!
//! This is the part of clock-app's theme this app uses. Keeping it in one
//! module means no widget falls back to egui's default colors or fonts.

use eframe::egui::{
    self, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Response, Sense, Stroke, StrokeKind,
    Theme, Ui, Vec2,
};

pub const BG: Color32 = Color32::BLACK;
pub const SURFACE: Color32 = Color32::from_rgb(14, 14, 14);
pub const SURFACE_HI: Color32 = Color32::from_rgb(26, 26, 26);
pub const BORDER: Color32 = Color32::from_rgb(46, 46, 46);
pub const TEXT: Color32 = Color32::WHITE;
pub const TEXT_DIM: Color32 = Color32::from_rgb(140, 140, 140);

const RADIUS: u8 = 10;

/// Content never gets wider than this, so wide windows keep a centred column.
pub const MAX_WIDTH: f32 = 560.0;

/// Runs `add` in a column centred in the remaining space, at most `max_width`
/// wide, with its widgets centred horizontally.
pub fn column<R>(ui: &mut Ui, max_width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let avail = ui.available_rect_before_wrap();
    let width = avail.width().min(max_width);
    let rect = Rect::from_min_size(
        Pos2::new(avail.center().x - width / 2.0, avail.top()),
        Vec2::new(width, avail.height()),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Center)),
        add,
    )
    .inner
}

/// Installs the style. The theme is forced to dark so the look doesn't
/// depend on the Windows light/dark setting.
pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.set_theme(Theme::Dark);
    ctx.style_mut_of(Theme::Dark, |style| {
        style.animation_time = 0.18;

        let spacing = &mut style.spacing;
        spacing.item_spacing = Vec2::new(8.0, 8.0);
        spacing.button_padding = Vec2::new(12.0, 6.0);
        spacing.interact_size.y = 30.0;
        spacing.combo_width = 90.0;

        let text = &mut style.text_styles;
        text.insert(egui::TextStyle::Heading, bold(20.0));
        text.insert(egui::TextStyle::Body, FontId::proportional(14.0));
        text.insert(egui::TextStyle::Button, FontId::proportional(14.0));
        text.insert(egui::TextStyle::Small, FontId::proportional(12.0));

        let visuals = &mut style.visuals;
        visuals.dark_mode = true;
        visuals.override_text_color = None;
        visuals.panel_fill = BG;
        visuals.window_fill = SURFACE_HI;
        visuals.window_stroke = Stroke::new(1.0, BORDER);
        visuals.window_corner_radius = CornerRadius::same(14);
        visuals.menu_corner_radius = CornerRadius::same(12);
        visuals.window_shadow = egui::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(160),
        };
        visuals.popup_shadow = visuals.window_shadow;
        visuals.extreme_bg_color = SURFACE;
        visuals.text_edit_bg_color = Some(SURFACE);
        visuals.faint_bg_color = SURFACE;
        visuals.selection.bg_fill = Color32::from_white_alpha(60);
        visuals.selection.stroke = Stroke::new(1.0, TEXT);

        let widgets = &mut visuals.widgets;
        for (state, fill, stroke) in [
            (&mut widgets.noninteractive, SURFACE, BORDER),
            (&mut widgets.inactive, SURFACE_HI, BORDER),
            (
                &mut widgets.hovered,
                Color32::from_rgb(36, 36, 36),
                TEXT_DIM,
            ),
            (&mut widgets.active, Color32::from_rgb(44, 44, 44), TEXT),
            (&mut widgets.open, SURFACE_HI, TEXT_DIM),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = Stroke::new(1.0, stroke);
            state.corner_radius = CornerRadius::same(RADIUS);
            state.fg_stroke = Stroke::new(1.0, TEXT);
            state.expansion = 0.0;
        }
    });
}

/// Manrope, a variable font (SIL Open Font License, see
/// `assets/fonts/Manrope-OFL.txt`). One file gives every weight we use.
const MANROPE: &[u8] = include_bytes!("../assets/fonts/Manrope-Variable.ttf");

/// The font family for headings and the large swatch text.
pub fn bold_family() -> egui::FontFamily {
    egui::FontFamily::Name("bold".into())
}

/// A bold font at `size`, for headings and large values.
pub fn bold(size: f32) -> FontId {
    FontId::new(size, bold_family())
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
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let with = |first: &str| {
        let mut list = vec![first.to_string()];
        list.extend(fallbacks.iter().cloned());
        list
    };
    fonts
        .families
        .insert(egui::FontFamily::Proportional, with("manrope-semibold"));
    fonts.families.insert(bold_family(), with("manrope-bold"));
    ctx.set_fonts(fonts);
}

/// A card: a slightly raised, rounded panel with a thin border.
pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(16)
        .inner_margin(Margin::same(16))
}

/// Small, dimmed, upper-case label above a group of controls.
pub fn section_label(ui: &mut Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .size(11.0)
            .color(TEXT_DIM)
            .extra_letter_spacing(1.2),
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Solid white with black text: the main action on a screen.
    Primary,
    /// Dark fill with a border that lights up on hover.
    Ghost,
}

/// A rounded button with hover and press feedback, at least `min_width` wide.
pub fn button_sized(ui: &mut Ui, text: &str, kind: Kind, min_width: f32) -> Response {
    let font = FontId::proportional(14.0);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font, Color32::PLACEHOLDER);
    let size = Vec2::new((galley.size().x + 28.0).max(min_width), 34.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    if ui.is_rect_visible(rect) {
        let enabled = ui.is_enabled();
        let hover = ui
            .ctx()
            .animate_bool_responsive(response.id.with("hover"), enabled && response.hovered());
        let rect = if response.is_pointer_button_down_on() {
            rect.shrink(1.0)
        } else {
            rect
        };
        let painter = ui.painter();
        let radius = f32::from(RADIUS);

        let text_color = match kind {
            Kind::Primary => {
                painter.rect_filled(rect, radius, TEXT.lerp_to_gamma(TEXT_DIM, 0.25 * hover));
                BG
            }
            Kind::Ghost => {
                painter.rect_filled(rect, radius, SURFACE_HI.lerp_to_gamma(TEXT, 0.08 * hover));
                painter.rect_stroke(
                    rect,
                    radius,
                    Stroke::new(1.0, BORDER.lerp_to_gamma(TEXT, hover)),
                    StrokeKind::Inside,
                );
                TEXT
            }
        };
        let text_color = if enabled {
            text_color
        } else {
            text_color.gamma_multiply(0.4)
        };
        painter.galley(rect.center() - galley.size() / 2.0, galley, text_color);
    }

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A text field with the app's styling.
pub fn text_field(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    ui.add_sized(
        [width, 30.0],
        egui::TextEdit::singleline(text)
            .hint_text(egui::RichText::new(hint).color(TEXT_DIM.gamma_multiply(0.7)))
            .desired_width(width)
            .margin(Margin::symmetric(10, 7)),
    )
}
