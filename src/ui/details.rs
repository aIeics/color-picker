//! The current color: a tall swatch next to one pill per shown format.
//! Clicking a value copies it.

use eframe::egui::{Sense, Ui, vec2};

use super::widgets::{self, GAP, PILL_H};
use crate::action::Action;
use crate::color::{Format, Rgb};
use crate::settings::Settings;

pub const SWATCH_W: f32 = 72.0;
const PER_ROW: usize = 3;
const LABEL_W: f32 = 58.0;

/// Draws extra rows of controls above the formats, next to the swatch.
/// Gets the rows' width.
pub type TopRows<'a> = &'a mut dyn FnMut(&mut Ui, f32, &mut Vec<Action>);

/// Height of [`show`] with `formats` shown and `top_rows` extra rows.
pub fn height(formats: usize, top_rows: usize) -> f32 {
    let rows = formats.div_ceil(PER_ROW) + top_rows;
    rows as f32 * PILL_H + rows.saturating_sub(1) as f32 * GAP
}

pub fn show(
    ui: &mut Ui,
    color: Rgb,
    settings: &Settings,
    actions: &mut Vec<Action>,
    top: Option<(usize, TopRows<'_>)>,
) {
    let formats: Vec<Format> = settings.formats.iter().collect();
    let height = height(formats.len(), top.as_ref().map_or(0, |(rows, _)| *rows));
    let right = ui.available_width() - SWATCH_W - GAP;

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing = vec2(GAP, GAP);
        widgets::swatch(ui, vec2(SWATCH_W, height), color, Sense::hover());
        ui.vertical(|ui| {
            ui.set_width(right);
            if let Some((_, top)) = top {
                top(ui, right, actions);
            }
            let width = widgets::column_width(right, PER_ROW);
            for row in formats.chunks(PER_ROW) {
                ui.horizontal(|ui| {
                    for &format in row {
                        format_pill(ui, width, format, color, settings, actions);
                    }
                });
            }
        });
    });
}

fn format_pill(
    ui: &mut Ui,
    width: f32,
    format: Format,
    color: Rgb,
    settings: &Settings,
    actions: &mut Vec<Action>,
) {
    widgets::pill(ui, width, |p| {
        p.label(format.label(), LABEL_W);
        p.divider();
        let copy = format.copy_text(color, settings.hex);
        let width = p.remaining();
        let response = p
            .copy_value(&format.display(color, settings.hex), width)
            .on_hover_text(format!("Copy {copy}"));
        if response.clicked() {
            actions.push(Action::Copy(copy));
        }
    });
}
