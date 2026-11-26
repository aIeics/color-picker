//! The building blocks of the BlurAutoClicker-style layout: rounded pills
//! split into segments by thin dividers, swatches and menu rows.

use std::sync::Arc;

use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Galley, Layout, Rect, Response, Sense,
    Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2,
};

use super::icons::{self, Icon};
use super::theme;
use crate::color::Rgb;

/// Height of a pill.
pub const PILL_H: f32 = 36.0;
/// Space between a pill's border and its segments.
const PAD: f32 = 4.0;
/// Height of a segment inside a pill.
pub const SEG_H: f32 = PILL_H - 2.0 * PAD;
/// Space between pills.
pub const GAP: f32 = 10.0;
/// Icon size inside segments and the title bar.
pub const ICON: f32 = 16.0;

/// Width of each of `n` equal columns in `width`, with [`GAP`] between them.
pub fn column_width(width: f32, n: usize) -> f32 {
    let n = n.max(1) as f32;
    (width - GAP * (n - 1.0)) / n
}

/// Draws a pill `width` wide and lays out its segments left to right.
pub fn pill<R>(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Pill<'_>) -> R) -> R {
    let (rect, _) = ui.allocate_exact_size(vec2(width, PILL_H), Sense::hover());
    paint_pill(ui.painter(), rect);
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.shrink(PAD))
            .layout(Layout::left_to_right(Align::Center)),
    );
    child.spacing_mut().item_spacing = Vec2::ZERO;
    add(&mut Pill { ui: &mut child })
}

pub fn paint_pill(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, theme::PILL_RADIUS, theme::SURFACE);
    painter.rect_stroke(
        rect,
        theme::PILL_RADIUS,
        Stroke::new(1.0, theme::BORDER),
        StrokeKind::Inside,
    );
}

/// The inside of a pill. Segments are added left to right.
pub struct Pill<'a> {
    pub ui: &'a mut Ui,
}

impl Pill<'_> {
    /// Width left for more segments.
    pub fn remaining(&self) -> f32 {
        self.ui.available_width()
    }

    pub fn space(&mut self, width: f32) {
        self.ui.add_space(width);
    }

    /// A short vertical line between segments.
    pub fn divider(&mut self) {
        let (rect, _) = self
            .ui
            .allocate_exact_size(vec2(9.0, SEG_H), Sense::hover());
        let c = rect.center();
        self.ui.painter().vline(
            c.x,
            (c.y - 12.0)..=(c.y + 12.0),
            Stroke::new(1.0, theme::DIVIDER),
        );
    }

    /// A dimmed label such as `HEX`.
    pub fn label(&mut self, text: &str, width: f32) {
        let (rect, _) = self
            .ui
            .allocate_exact_size(vec2(width, SEG_H), Sense::hover());
        self.ui.painter().text(
            rect.left_center() + vec2(10.0, 0.0),
            Align2::LEFT_CENTER,
            text,
            theme::regular(13.5),
            theme::LABEL,
        );
    }

    /// Bold text, shrunk to fit if it is too wide.
    pub fn value(&mut self, text: &str, color: Color32, width: f32) -> Response {
        let (rect, response) = self
            .ui
            .allocate_exact_size(vec2(width, SEG_H), Sense::hover());
        let inner = rect.shrink2(vec2(10.0, 0.0));
        let galley = fitted(self.ui, text, theme::bold(15.0), color, inner.width());
        let pos = pos2(inner.left(), rect.center().y - galley.size().y / 2.0);
        self.ui.painter().galley(pos, galley, color);
        response
    }

    /// A value that copies itself when clicked, with a copy icon on the right.
    pub fn copy_value(&mut self, text: &str, width: f32) -> Response {
        let (rect, response) = segment_button(self.ui, width);
        let hovered = response.hovered();
        let icon_rect = Rect::from_center_size(
            pos2(rect.right() - 6.0 - ICON / 2.0, rect.center().y),
            Vec2::splat(ICON - 2.0),
        );
        let icon_color = if hovered {
            theme::TEXT
        } else {
            theme::TEXT_FAINT
        };
        icons::paint(self.ui.painter(), icon_rect, Icon::Copy, icon_color);

        let inner = Rect::from_min_max(
            pos2(rect.left() + 10.0, rect.top()),
            pos2(icon_rect.left() - 6.0, rect.bottom()),
        );
        let galley = fitted(self.ui, text, theme::bold(15.0), theme::TEXT, inner.width());
        let pos = pos2(inner.left(), rect.center().y - galley.size().y / 2.0);
        self.ui.painter().galley(pos, galley, theme::TEXT);
        response
    }

    pub fn icon_button(&mut self, icon: Icon, tooltip: &str) -> Response {
        let (rect, response) = segment_button(self.ui, SEG_H);
        let color = if !self.ui.is_enabled() {
            theme::TEXT_FAINT
        } else if response.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(ICON));
        icons::paint(self.ui.painter(), icon_rect, icon, color);
        response.on_hover_text(tooltip)
    }

    /// A button with an optional icon and bold text, centered in `width`.
    pub fn button(&mut self, icon: Option<Icon>, text: &str, width: f32) -> Response {
        self.button_colored(icon, text, theme::TEXT, width)
    }

    pub fn button_colored(
        &mut self,
        icon: Option<Icon>,
        text: &str,
        color: Color32,
        width: f32,
    ) -> Response {
        let (rect, response) = segment_button(self.ui, width);
        let color = if self.ui.is_enabled() {
            color
        } else {
            theme::TEXT_FAINT
        };
        let icon_w = if icon.is_some() { ICON + 8.0 } else { 0.0 };
        let galley = fitted(
            self.ui,
            text,
            theme::bold(13.5),
            color,
            rect.width() - icon_w - 16.0,
        );
        let mut x = rect.center().x - (icon_w + galley.size().x) / 2.0;
        if let Some(icon) = icon {
            let icon_rect =
                Rect::from_min_size(pos2(x, rect.center().y - ICON / 2.0), Vec2::splat(ICON));
            icons::paint(self.ui.painter(), icon_rect, icon, color);
            x += icon_w;
        }
        let pos = pos2(x, rect.center().y - galley.size().y / 2.0);
        self.ui.painter().galley(pos, galley, color);
        response
    }

    /// The current option with a chevron; clicking opens a menu of options.
    /// Returns `true` when the user picked a different option.
    pub fn dropdown<T: Copy + PartialEq>(
        &mut self,
        current: &mut T,
        options: &[T],
        label: impl Fn(T) -> String,
        width: f32,
    ) -> bool {
        let (rect, response) = segment_button(self.ui, width);
        let chevron = Rect::from_center_size(
            pos2(rect.right() - 6.0 - 7.0, rect.center().y),
            Vec2::splat(14.0),
        );
        let color = if response.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        icons::paint(self.ui.painter(), chevron, Icon::ChevronDown, color);
        let galley = fitted(
            self.ui,
            &label(*current),
            theme::bold(13.5),
            theme::TEXT,
            chevron.left() - rect.left() - 16.0,
        );
        let pos = pos2(rect.left() + 10.0, rect.center().y - galley.size().y / 2.0);
        self.ui.painter().galley(pos, galley, theme::TEXT);

        let mut changed = false;
        egui::Popup::menu(&response)
            .width(width.max(150.0))
            .show(|ui| {
                for &option in options {
                    if menu_row(ui, None, &label(option), option == *current).clicked()
                        && option != *current
                    {
                        *current = option;
                        changed = true;
                    }
                }
            });
        changed
    }

    /// An on/off switch. Flips `on` when clicked.
    pub fn switch(&mut self, on: &mut bool) -> Response {
        let (rect, mut response) = segment_button(self.ui, 48.0);
        if response.clicked() {
            *on = !*on;
            response.mark_changed();
        }
        let t = self
            .ui
            .ctx()
            .animate_bool_responsive(response.id.with("switch"), *on);
        let track = Rect::from_center_size(rect.center(), vec2(30.0, 17.0));
        let painter = self.ui.painter();
        painter.rect_filled(track, 8.5, theme::BORDER.lerp_to_gamma(theme::TEXT, t));
        let x = egui::lerp((track.left() + 8.5)..=(track.right() - 8.5), t);
        painter.circle_filled(
            pos2(x, track.center().y),
            6.0,
            theme::TEXT_DIM.lerp_to_gamma(theme::BG, t),
        );
        response
    }

    /// A toggle chip: white when on, faint when off.
    pub fn chip(&mut self, text: &str, on: bool, width: f32) -> Response {
        let (rect, response) = segment_button(self.ui, width);
        if on {
            self.ui
                .painter()
                .rect_filled(rect, theme::SEGMENT_RADIUS, theme::SURFACE_HI);
        }
        let color = if on { theme::TEXT } else { theme::TEXT_FAINT };
        self.ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            theme::bold(12.5),
            color,
        );
        response
    }

    /// A frameless single-line text field.
    pub fn text_edit(&mut self, text: &mut String, hint: &str, width: f32) -> Response {
        let (rect, _) = self
            .ui
            .allocate_exact_size(vec2(width, SEG_H), Sense::hover());
        self.ui.put(
            rect,
            egui::TextEdit::singleline(text)
                .frame(egui::Frame::NONE)
                .font(theme::bold(14.0))
                .text_color(theme::TEXT)
                .hint_text(egui::RichText::new(hint).color(theme::TEXT_FAINT))
                .align(Align2::LEFT_CENTER)
                .margin(egui::Margin::symmetric(10, 0))
                .desired_width(width)
                .min_size(vec2(width, SEG_H)),
        )
    }
}

/// An interactive segment: highlights on hover and press, shows a hand.
/// The caller paints the content into the returned rect.
pub fn segment_button(ui: &mut Ui, width: f32) -> (Rect, Response) {
    let (rect, response) = ui.allocate_exact_size(vec2(width, SEG_H), Sense::click());
    if ui.is_rect_visible(rect) && ui.is_enabled() {
        let hover = ui
            .ctx()
            .animate_bool_responsive(response.id.with("hover"), response.hovered());
        let fill = if response.is_pointer_button_down_on() {
            theme::SURFACE_PRESS
        } else {
            theme::SURFACE.lerp_to_gamma(theme::SURFACE_HI, hover)
        };
        if fill != theme::SURFACE {
            ui.painter().rect_filled(rect, theme::SEGMENT_RADIUS, fill);
        }
    }
    if ui.is_enabled() {
        (rect, response.on_hover_cursor(CursorIcon::PointingHand))
    } else {
        (rect, response)
    }
}

/// A row in a dropdown or context menu, with an optional icon and a check
/// mark when `selected`.
pub fn menu_row(ui: &mut Ui, icon: Option<Icon>, text: &str, selected: bool) -> Response {
    let width = ui.available_width().max(120.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
    let hovered = response.hovered();
    let painter = ui.painter();
    if hovered {
        painter.rect_filled(rect, 6.0, theme::SURFACE_HI);
    }
    let color = if hovered || selected {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    let mut x = rect.left() + 10.0;
    if let Some(icon) = icon {
        let icon_rect = Rect::from_min_size(pos2(x, rect.center().y - 7.0), Vec2::splat(14.0));
        icons::paint(painter, icon_rect, icon, color);
        x += 22.0;
    }
    painter.text(
        pos2(x, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        theme::regular(13.0),
        color,
    );
    if selected {
        let check = Rect::from_center_size(
            pos2(rect.right() - 16.0, rect.center().y),
            Vec2::splat(14.0),
        );
        icons::paint(painter, check, Icon::Check, theme::TEXT);
    }
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// A color swatch with a faint border, so black still shows on black.
pub fn swatch(ui: &mut Ui, size: Vec2, color: Rgb, sense: Sense) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, sense);
    paint_swatch(ui.painter(), rect, color, theme::PILL_RADIUS.into());
    response
}

pub fn paint_swatch(painter: &egui::Painter, rect: Rect, color: Rgb, radius: f32) {
    painter.rect_filled(rect, radius, color);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, Color32::from_white_alpha(30)),
        StrokeKind::Inside,
    );
}

/// Lays out `text` in `font`, shrinking the font when the text is wider
/// than `max_width` (never below 9 points).
pub fn fitted(ui: &Ui, text: &str, font: FontId, color: Color32, max_width: f32) -> Arc<Galley> {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), color);
    let width = galley.size().x;
    if width <= max_width || max_width <= 0.0 {
        return galley;
    }
    let size = (font.size * max_width / width).floor().max(9.0);
    ui.painter()
        .layout_no_wrap(text.to_owned(), FontId::new(size, font.family), color)
}

/// Width of `text` in `font`, for sizing pills to their content.
pub fn text_width(ui: &Ui, text: &str, font: FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font, Color32::PLACEHOLDER)
        .size()
        .x
}

/// Width of a [`Pill::button`] that fits its content.
pub fn button_width(ui: &Ui, icon: bool, text: &str) -> f32 {
    let icon_w = if icon { ICON + 8.0 } else { 0.0 };
    text_width(ui, text, theme::bold(13.5)) + icon_w + 28.0
}
