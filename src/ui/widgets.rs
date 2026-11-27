//! The building blocks, after BlurAutoClicker: pills split into segments by
//! thin dividers, segmented controls, OFF/ON toggles, chips, bordered
//! buttons, sliders and swatches. Colors come from [`theme::p`].

use std::sync::Arc;

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Galley, Id, Layout, Rect,
    Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2,
};

use super::icons::{self, Icon};
use super::theme::{self, p};
use crate::color::Rgb;

/// Height of a pill.
pub const PILL_H: f32 = 36.0;
/// Space between a pill's border and its segments.
const PAD: f32 = 4.0;
/// Height of a segment inside a pill.
pub const SEG_H: f32 = PILL_H - 2.0 * PAD;
/// Space between pills.
pub const GAP: f32 = 10.0;
/// Icon size inside segments.
pub const ICON: f32 = 16.0;
/// Height of free-standing controls (segmented, buttons, fields).
pub const CTRL_H: f32 = 30.0;

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
    let p = p();
    painter.rect_filled(rect, theme::PILL_RADIUS, p.surface);
    painter.rect_stroke(
        rect,
        theme::PILL_RADIUS,
        Stroke::new(1.0, p.border_subtle),
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

    /// A thin vertical line between segments, nearly as tall as the pill.
    pub fn divider(&mut self) {
        let (rect, _) = self
            .ui
            .allocate_exact_size(vec2(9.0, SEG_H), Sense::hover());
        let c = rect.center();
        self.ui.painter().vline(
            c.x,
            (c.y - 11.0)..=(c.y + 11.0),
            Stroke::new(1.0, p().divider),
        );
    }

    /// A label such as `HEX`.
    pub fn label(&mut self, text: &str, width: f32) {
        let (rect, _) = self
            .ui
            .allocate_exact_size(vec2(width, SEG_H), Sense::hover());
        self.ui.painter().text(
            rect.left_center() + vec2(10.0, 0.0),
            Align2::LEFT_CENTER,
            text,
            theme::regular(14.0),
            p().label,
        );
    }

    /// Bold text, shrunk to fit if it is too wide.
    pub fn value(&mut self, text: &str, color: Color32, width: f32) -> Response {
        let (rect, response) = self
            .ui
            .allocate_exact_size(vec2(width, SEG_H), Sense::hover());
        let inner = rect.shrink2(vec2(10.0, 0.0));
        let galley = fitted(self.ui, text, theme::bold(15.5), color, inner.width());
        let pos = pos2(inner.left(), rect.center().y - galley.size().y / 2.0);
        self.ui.painter().galley(pos, galley, color);
        response
    }

    /// A value that copies itself when clicked, with a copy icon on the right.
    pub fn copy_value(&mut self, text: &str, width: f32) -> Response {
        let (rect, response) = segment_button(self.ui, width);
        let p = p();
        let icon_rect = Rect::from_center_size(
            pos2(rect.right() - 6.0 - ICON / 2.0, rect.center().y),
            Vec2::splat(ICON - 1.0),
        );
        let icon_color = if response.hovered() {
            p.text
        } else {
            p.text_dim
        };
        icons::paint(self.ui.painter(), icon_rect, Icon::Copy, icon_color);
        let inner = Rect::from_min_max(
            pos2(rect.left() + 10.0, rect.top()),
            pos2(icon_rect.left() - 6.0, rect.bottom()),
        );
        let galley = fitted(self.ui, text, theme::bold(15.5), p.text, inner.width());
        let pos = pos2(inner.left(), rect.center().y - galley.size().y / 2.0);
        self.ui.painter().galley(pos, galley, p.text);
        response
    }

    pub fn icon_button(&mut self, icon: Icon, tooltip: &str) -> Response {
        let (rect, response) = segment_button(self.ui, SEG_H);
        let p = p();
        let color = if !self.ui.is_enabled() {
            p.text_dim.gamma_multiply(0.6)
        } else if response.hovered() {
            p.text
        } else {
            p.text_muted
        };
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(ICON));
        icons::paint(self.ui.painter(), icon_rect, icon, color);
        response.on_hover_text(tooltip)
    }

    /// A button with an optional icon and text, centered in `width`.
    pub fn button(&mut self, icon: Option<Icon>, text: &str, width: f32) -> Response {
        self.button_colored(icon, text, p().text, width)
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
            p().text_dim.gamma_multiply(0.7)
        };
        let icon_w = if icon.is_some() { ICON + 8.0 } else { 0.0 };
        let galley = fitted(
            self.ui,
            text,
            theme::bold(14.0),
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

    /// A slider over a color gradient. `color_at` gives the color for a
    /// value; `segments` is how finely the gradient is drawn. Dragging or
    /// clicking sets the value, the mouse wheel nudges it by `step`.
    pub fn gradient_slider(
        &mut self,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
        step: f32,
        width: f32,
        segments: usize,
        color_at: impl Fn(f32) -> Color32,
    ) -> Response {
        let (rect, mut response) = self
            .ui
            .allocate_exact_size(vec2(width, SEG_H), Sense::click_and_drag());
        let (lo, hi) = (*range.start(), *range.end());
        let track = Rect::from_min_max(
            pos2(rect.left() + 10.0, rect.center().y - 4.0),
            pos2(rect.right() - 10.0, rect.center().y + 4.0),
        );

        let old = *value;
        if response.is_pointer_button_down_on()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let t = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
            *value = ((lo + t * (hi - lo)) / step).round() * step;
        }
        if response.hovered() {
            let scroll = wheel(self.ui);
            if scroll != 0.0 {
                *value = (*value + step * scroll.signum()).clamp(lo, hi);
            }
        }
        if *value != old {
            response.mark_changed();
        }

        let painter = self.ui.painter();
        let segments = segments.max(1);
        let mut mesh = egui::Mesh::default();
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let x = egui::lerp(track.left()..=track.right(), t);
            let color = color_at(lo + t * (hi - lo));
            mesh.colored_vertex(pos2(x, track.top()), color);
            mesh.colored_vertex(pos2(x, track.bottom()), color);
            if i > 0 {
                let k = 2 * i as u32;
                mesh.add_triangle(k - 2, k - 1, k);
                mesh.add_triangle(k - 1, k, k + 1);
            }
        }
        painter.circle_filled(track.left_center(), 4.0, color_at(lo));
        painter.circle_filled(track.right_center(), 4.0, color_at(hi));
        painter.add(egui::Shape::mesh(mesh));

        let t = if hi > lo {
            (*value - lo) / (hi - lo)
        } else {
            0.0
        };
        let knob = pos2(
            egui::lerp(track.left()..=track.right(), t.clamp(0.0, 1.0)),
            track.center().y,
        );
        let radius = if response.hovered() || response.dragged() {
            8.5
        } else {
            8.0
        };
        painter.circle_filled(knob, radius + 1.0, Color32::from_black_alpha(150));
        painter.circle_filled(knob, radius, Color32::WHITE);
        painter.circle_filled(knob, radius - 2.5, color_at(*value));
        response.on_hover_cursor(CursorIcon::PointingHand)
    }

    /// A frameless single-line text field.
    pub fn text_edit(&mut self, id: Id, text: &mut String, hint: &str, width: f32) -> Response {
        let (rect, _) = self
            .ui
            .allocate_exact_size(vec2(width, SEG_H), Sense::hover());
        text_field(self.ui, rect, id, text, hint, theme::bold(15.5))
    }
}

fn text_field(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    text: &mut String,
    hint: &str,
    font: FontId,
) -> Response {
    let p = p();
    ui.put(
        rect,
        egui::TextEdit::singleline(text)
            .id(id)
            .frame(egui::Frame::NONE)
            .font(font)
            .text_color(p.text)
            .hint_text(egui::RichText::new(hint).color(p.text_dim))
            .align(Align2::LEFT_CENTER)
            .margin(egui::Margin::symmetric(10, 0))
            .desired_width(rect.width())
            .min_size(rect.size()),
    )
}

/// Wheel movement over the pointer this frame (one value per wheel event,
/// not smoothed).
pub fn wheel(ui: &Ui) -> f32 {
    ui.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::MouseWheel { delta, .. } => Some(delta.x + delta.y),
                _ => None,
            })
            .sum()
    })
}

/// An interactive segment: highlights on hover and press, shows a hand.
/// The caller paints the content into the returned rect.
pub fn segment_button(ui: &mut Ui, width: f32) -> (Rect, Response) {
    let (rect, response) = ui.allocate_exact_size(vec2(width, SEG_H), Sense::click());
    hover_fill(ui, rect, &response, theme::SEGMENT_RADIUS.into());
    with_hand(ui, rect, response)
}

fn hover_fill(ui: &Ui, rect: Rect, response: &Response, radius: f32) {
    if !ui.is_rect_visible(rect) || !ui.is_enabled() {
        return;
    }
    let p = p();
    let hover = ui
        .ctx()
        .animate_bool_responsive(response.id.with("hover"), response.hovered());
    let fill = if response.is_pointer_button_down_on() {
        p.input
    } else {
        p.elevated.gamma_multiply(hover)
    };
    if hover > 0.0 || response.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, radius, fill);
    }
}

fn with_hand(ui: &Ui, rect: Rect, response: Response) -> (Rect, Response) {
    if ui.is_enabled() {
        (rect, response.on_hover_cursor(CursorIcon::PointingHand))
    } else {
        (rect, response)
    }
}

/// Width of a [`segmented`] control with these labels.
pub fn segmented_width(ui: &Ui, labels: &[&str]) -> f32 {
    labels
        .iter()
        .map(|l| text_width(ui, l, theme::regular(13.5)) + 24.0)
        .sum()
}

/// BlurAutoClicker's segmented control: options side by side on a dark
/// track, the chosen one filled with `accent`. Returns `true` on change.
pub fn segmented<T: Copy + PartialEq>(
    ui: &mut Ui,
    current: &mut T,
    options: &[(T, &str)],
    accent: Color32,
) -> bool {
    let p = p();
    let font = theme::regular(13.5);
    let widths: Vec<f32> = options
        .iter()
        .map(|(_, l)| text_width(ui, l, font.clone()) + 24.0)
        .collect();
    let total: f32 = widths.iter().sum();
    let (rect, _) = ui.allocate_exact_size(vec2(total, CTRL_H), Sense::hover());
    ui.painter().rect_filled(rect, 6.0, p.input_off);
    let selected = selected_colors(accent);
    let mut changed = false;
    let mut x = rect.left();
    let last = options.len().saturating_sub(1);
    for (i, ((value, label), w)) in options.iter().zip(&widths).enumerate() {
        let item = Rect::from_min_size(pos2(x, rect.top()), vec2(*w, CTRL_H));
        x += w;
        let response = ui
            .interact(item, ui.auto_id_with(("seg", i)), Sense::click())
            .on_hover_cursor(CursorIcon::PointingHand);
        let on = *value == *current;
        if on {
            let radius = CornerRadius {
                nw: if i == 0 { 6 } else { 0 },
                sw: if i == 0 { 6 } else { 0 },
                ne: if i == last { 6 } else { 0 },
                se: if i == last { 6 } else { 0 },
            };
            ui.painter().rect_filled(item, radius, selected.0);
        }
        let color = if on {
            selected.1
        } else if response.hovered() {
            p.text_muted
        } else {
            p.text_dim
        };
        ui.painter().text(
            item.center(),
            Align2::CENTER_CENTER,
            label,
            font.clone(),
            color,
        );
        if response.clicked() && !on {
            *current = *value;
            changed = true;
        }
    }
    changed
}

/// The fill of a chosen option and the color of its text. The dark theme
/// tones the accent down over the track, as BlurAutoClicker does; the light
/// theme uses it as is, so it doesn't wash out. The text is white unless the
/// fill is too light for it (a pale custom accent).
fn selected_colors(accent: Color32) -> (Color32, Color32) {
    selected_colors_in(p().light, accent)
}

fn selected_colors_in(light: bool, accent: Color32) -> (Color32, Color32) {
    const SHOWN: f32 = 0.62;
    let (fill, shown) = if light {
        (accent, accent)
    } else {
        // What the translucent fill looks like over the dark track.
        let track = theme::DARK.input_off;
        let mix = |t: u8, a: u8| (f32::from(t) * (1.0 - SHOWN) + f32::from(a) * SHOWN) as u8;
        let shown = Color32::from_rgb(
            mix(track.r(), accent.r()),
            mix(track.g(), accent.g()),
            mix(track.b(), accent.b()),
        );
        (accent.gamma_multiply(SHOWN), shown)
    };
    (fill, text_on(shown))
}

/// White, or near black when white would read poorly on `fill` (contrast
/// under 3:1, WCAG's floor for large and bold text).
fn text_on(fill: Color32) -> Color32 {
    let linear = |c: u8| {
        let c = f32::from(c) / 255.0;
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance =
        0.2126 * linear(fill.r()) + 0.7152 * linear(fill.g()) + 0.0722 * linear(fill.b());
    if 1.05 / (luminance + 0.05) >= 3.0 {
        Color32::WHITE
    } else {
        Color32::from_rgb(18, 18, 20)
    }
}

/// BlurAutoClicker's OFF/ON toggle: OFF in red on gray, ON in green.
/// Flips `on` when the other half is clicked; returns `true` on change.
pub fn onoff(ui: &mut Ui, on: &mut bool) -> bool {
    let p = p();
    let font = theme::bold(12.0);
    let (rect, _) = ui.allocate_exact_size(vec2(92.0, CTRL_H), Sense::hover());
    ui.painter().rect_filled(rect, 6.0, p.input_off);
    let mut changed = false;
    for (i, (label, value)) in [("OFF", false), ("ON", true)].into_iter().enumerate() {
        let item = Rect::from_min_size(
            pos2(rect.left() + i as f32 * 46.0, rect.top()),
            vec2(46.0, CTRL_H),
        );
        let response = ui
            .interact(item, ui.auto_id_with(("onoff", i)), Sense::click())
            .on_hover_cursor(CursorIcon::PointingHand);
        let selected = *on == value;
        let radius = if i == 0 {
            CornerRadius {
                nw: 6,
                sw: 6,
                ne: 0,
                se: 0,
            }
        } else {
            CornerRadius {
                nw: 0,
                sw: 0,
                ne: 6,
                se: 6,
            }
        };
        let color = match (selected, value) {
            (true, false) => {
                ui.painter().rect_filled(item, radius, p.input);
                p.danger
            }
            (true, true) => {
                let (fill, text) = selected_colors(p.ok);
                ui.painter().rect_filled(item, radius, fill);
                text
            }
            _ if response.hovered() => p.text_muted,
            _ => p.text_dim,
        };
        ui.painter().text(
            item.center(),
            Align2::CENTER_CENTER,
            label,
            font.clone(),
            color,
        );
        if response.clicked() && !selected {
            *on = value;
            changed = true;
        }
    }
    changed
}

/// An upper-case chip (like BlurAutoClicker's LEFT / MIDDLE / RIGHT).
pub fn chip(ui: &mut Ui, text: &str, on: bool) -> Response {
    let p = p();
    let font = theme::bold(12.0);
    let width = text_width(ui, text, font.clone()) + 20.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, SEG_H), Sense::click());
    if on {
        ui.painter()
            .rect_filled(rect, theme::SEGMENT_RADIUS, p.input);
    } else {
        hover_fill(ui, rect, &response, theme::SEGMENT_RADIUS.into());
    }
    let color = if on {
        p.text
    } else if response.hovered() {
        p.text_muted
    } else {
        p.text_dim
    };
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, text, font, color);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// A bordered button with an optional icon (Settings rows).
pub fn pbtn(ui: &mut Ui, icon: Option<Icon>, text: &str, color: Option<Color32>) -> Response {
    let p = p();
    let font = theme::bold(13.5);
    let icon_w = if icon.is_some() { 15.0 + 7.0 } else { 0.0 };
    let text_w = if text.is_empty() {
        0.0
    } else {
        text_width(ui, text, font.clone())
    };
    let width = if text.is_empty() {
        CTRL_H
    } else {
        text_w + icon_w + 24.0
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, CTRL_H), Sense::click());
    let enabled = ui.is_enabled();
    let fill = if response.is_pointer_button_down_on() || (response.hovered() && enabled) {
        p.input
    } else {
        p.elevated
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    ui.painter().rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, p.border_subtle),
        StrokeKind::Inside,
    );
    let color = if enabled {
        color.unwrap_or(p.text)
    } else {
        p.text_dim
    };
    let mut x = rect.center().x - (icon_w + text_w) / 2.0;
    if let Some(icon) = icon {
        let r = Rect::from_min_size(pos2(x, rect.center().y - 7.5), Vec2::splat(15.0));
        icons::paint(ui.painter(), r, icon, color);
        x += icon_w;
    }
    if !text.is_empty() {
        ui.painter().text(
            pos2(x, rect.center().y),
            Align2::LEFT_CENTER,
            text,
            font,
            color,
        );
    }
    with_hand(ui, rect, response).1
}

/// A read-only box showing `text`, like a field (the hotkey).
pub fn field(ui: &mut Ui, text: &str, color: Color32, width: f32) {
    let p = p();
    let (rect, _) = ui.allocate_exact_size(vec2(width, CTRL_H), Sense::hover());
    ui.painter().rect_filled(rect, 6.0, p.base);
    ui.painter().rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, p.border_subtle),
        StrokeKind::Inside,
    );
    let galley = fitted(ui, text, theme::bold(14.5), color, width - 20.0);
    let pos = pos2(rect.left() + 10.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, color);
}

/// An editable boxed field (the accent HEX).
pub fn boxed_text_edit(ui: &mut Ui, id: Id, text: &mut String, width: f32) -> Response {
    let p = p();
    let (rect, _) = ui.allocate_exact_size(vec2(width, CTRL_H), Sense::hover());
    ui.painter().rect_filled(rect, 6.0, p.base);
    ui.painter().rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, p.border_subtle),
        StrokeKind::Inside,
    );
    text_field(ui, rect, id, text, "#RRGGBB", theme::bold(14.0))
}

/// A slider for a percentage, with the value printed on the right.
pub fn percent_slider(
    ui: &mut Ui,
    value: &mut u8,
    range: std::ops::RangeInclusive<u8>,
    accent: Color32,
) -> bool {
    let p = p();
    let (lo, hi) = (f32::from(*range.start()), f32::from(*range.end()));
    // One allocation for both parts, so they keep their order in
    // right-to-left rows.
    let (rect, whole) = ui.allocate_exact_size(vec2(216.0, CTRL_H), Sense::hover());
    let slider = Rect::from_min_size(rect.min, vec2(170.0, CTRL_H));
    let response = ui.interact(slider, whole.id.with("slider"), Sense::click_and_drag());
    let track = Rect::from_center_size(slider.center(), vec2(slider.width() - 16.0, 6.0));
    let old = *value;
    if response.is_pointer_button_down_on()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let t = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
        *value = (lo + t * (hi - lo)).round() as u8;
    }
    if response.hovered() {
        let scroll = wheel(ui);
        if scroll != 0.0 {
            let v = (f32::from(*value) + scroll.signum()).clamp(lo, hi);
            *value = v as u8;
        }
    }
    let t = ((f32::from(*value) - lo) / (hi - lo).max(1.0)).clamp(0.0, 1.0);
    let painter = ui.painter();
    painter.rect_filled(track, 3.0, p.input);
    let filled = Rect::from_min_max(track.min, pos2(egui::lerp(track.x_range(), t), track.max.y));
    painter.rect_filled(filled, 3.0, accent);
    let knob = pos2(egui::lerp(track.x_range(), t), track.center().y);
    painter.circle_filled(knob, 9.0, Color32::from_black_alpha(70));
    painter.circle_filled(knob, 8.0, Color32::WHITE);
    ui.painter().text(
        rect.right_center(),
        Align2::RIGHT_CENTER,
        format!("{}%", *value),
        theme::bold(14.0),
        p.text,
    );
    response.on_hover_cursor(CursorIcon::PointingHand);
    *value != old
}

/// `− 100% +`: steps through `steps`. Returns `true` on change.
pub fn stepper(ui: &mut Ui, value: &mut u16, steps: &[u16]) -> bool {
    let p = p();
    let (rect, _) = ui.allocate_exact_size(vec2(118.0, CTRL_H), Sense::hover());
    ui.painter().rect_filled(rect, 6.0, p.input_off);
    let index = steps.iter().position(|&s| s == *value).unwrap_or(0);
    let mut changed = false;
    for (i, (icon, step, enabled)) in [
        (Icon::Minus, -1i32, index > 0),
        (Icon::Plus, 1, index + 1 < steps.len()),
    ]
    .into_iter()
    .enumerate()
    {
        let x = if i == 0 {
            rect.left()
        } else {
            rect.right() - CTRL_H
        };
        let item = Rect::from_min_size(pos2(x, rect.top()), Vec2::splat(CTRL_H));
        let response = ui.interact(item, ui.auto_id_with(("step", i)), Sense::click());
        let color = if !enabled {
            p.text_dim.gamma_multiply(0.5)
        } else if response.hovered() {
            p.text
        } else {
            p.text_dim
        };
        icons::paint(
            ui.painter(),
            Rect::from_center_size(item.center(), Vec2::splat(15.0)),
            icon,
            color,
        );
        if enabled && response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
            let next = (index as i32 + step).clamp(0, steps.len() as i32 - 1) as usize;
            *value = steps[next];
            changed = true;
        }
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        format!("{}%", *value),
        theme::bold(14.0),
        p.text,
    );
    changed
}

/// A row in a menu, with an optional icon and a check mark when `selected`.
pub fn menu_row(ui: &mut Ui, icon: Option<Icon>, text: &str, selected: bool) -> Response {
    let p = p();
    let width = ui.available_width().max(120.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
    let hovered = response.hovered();
    let painter = ui.painter();
    if hovered {
        painter.rect_filled(rect, 5.0, p.elevated);
    }
    let color = if hovered || selected {
        p.text
    } else {
        p.text_muted
    };
    let mut x = rect.left() + 9.0;
    if let Some(icon) = icon {
        let icon_rect = Rect::from_min_size(pos2(x, rect.center().y - 7.0), Vec2::splat(14.0));
        icons::paint(painter, icon_rect, icon, color);
        x += 23.0;
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
        icons::paint(painter, check, Icon::Check, p.text);
    }
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// A color swatch with a faint border, so dark colors still show.
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
        Stroke::new(1.0, p().border_subtle),
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

/// Width of `text` in `font`, for sizing to content.
pub fn text_width(ui: &Ui, text: &str, font: FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font, Color32::PLACEHOLDER)
        .size()
        .x
}

/// Width of a [`Pill::button`] that fits its content.
pub fn button_width(ui: &Ui, icon: bool, text: &str) -> f32 {
    let icon_w = if icon { ICON + 8.0 } else { 0.0 };
    text_width(ui, text, theme::bold(14.0)) + icon_w + 28.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_stays_white_on_the_tab_colors() {
        for palette in [theme::DARK, theme::LIGHT] {
            for color in [
                palette.green,
                palette.blue,
                palette.amber,
                palette.teal,
                palette.violet,
            ] {
                let (_, text) = selected_colors_in(palette.light, color);
                assert_eq!(text, Color32::WHITE, "{color:?}");
            }
        }
    }

    #[test]
    fn text_turns_dark_on_pale_fills() {
        assert_ne!(text_on(Color32::from_rgb(255, 240, 120)), Color32::WHITE);
        assert_ne!(text_on(Color32::WHITE), Color32::WHITE);
        assert_eq!(text_on(Color32::BLACK), Color32::WHITE);
        let (_, text) = selected_colors_in(false, Color32::from_rgb(255, 255, 0));
        assert_ne!(text, Color32::WHITE, "pale accent in the dark theme");
    }
}
