//! A card of small swatches, used by History and Palette. Click selects,
//! right-click opens a menu.

use eframe::egui::{Align2, CursorIcon, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, vec2};

use super::icons::Icon;
use super::theme;
use super::widgets::{self, paint_pill};
use crate::color::{HexStyle, Rgb};

const CELL: f32 = 30.0;
const CELL_GAP: f32 = 6.0;
const PAD: f32 = 10.0;

/// What the user did with a swatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridEvent {
    Select(usize),
    Copy(usize),
    AddToPalette(usize),
    Remove(usize),
}

/// Draws `colors` in a card as wide as the available space. `can_save`
/// adds "Save to palette" to the right-click menu.
pub fn show(
    ui: &mut Ui,
    colors: &[Rgb],
    current: Rgb,
    hex: HexStyle,
    empty_text: &str,
    can_save: bool,
) -> Option<GridEvent> {
    let width = ui.available_width();
    let per_row = (((width - 2.0 * PAD + CELL_GAP) / (CELL + CELL_GAP)).floor() as usize).max(1);
    let rows = colors.len().div_ceil(per_row).max(1);
    let height = 2.0 * PAD + rows as f32 * CELL + (rows - 1) as f32 * CELL_GAP;
    let (card, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    paint_pill(ui.painter(), card);

    if colors.is_empty() {
        ui.painter().text(
            card.center(),
            Align2::CENTER_CENTER,
            empty_text,
            theme::regular(13.0),
            theme::TEXT_DIM,
        );
        return None;
    }

    let mut event = None;
    for (index, &color) in colors.iter().enumerate() {
        let (row, col) = (index / per_row, index % per_row);
        let min = card.min + vec2(PAD, PAD) + vec2(col as f32, row as f32) * (CELL + CELL_GAP);
        let rect = Rect::from_min_size(min, Vec2::splat(CELL));
        let response = ui
            .interact(
                rect,
                ui.make_persistent_id(("swatch", index)),
                Sense::click(),
            )
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(hex.format(color));

        widgets::paint_swatch(ui.painter(), rect, color, 6.0);
        if color == current {
            ui.painter().rect_stroke(
                rect.expand(2.5),
                8.0,
                Stroke::new(1.5, theme::TEXT),
                StrokeKind::Middle,
            );
        } else if response.hovered() {
            ui.painter().rect_stroke(
                rect.expand(2.5),
                8.0,
                Stroke::new(1.5, theme::TEXT_DIM),
                StrokeKind::Middle,
            );
        }

        if response.clicked() {
            event = Some(GridEvent::Select(index));
        }
        response.context_menu(|ui| {
            if widgets::menu_row(
                ui,
                Some(Icon::Copy),
                &format!("Copy {}", hex.format(color)),
                false,
            )
            .clicked()
            {
                event = Some(GridEvent::Copy(index));
            }
            if can_save
                && widgets::menu_row(ui, Some(Icon::Plus), "Save to palette", false).clicked()
            {
                event = Some(GridEvent::AddToPalette(index));
            }
            if widgets::menu_row(ui, Some(Icon::Trash), "Remove", false).clicked() {
                event = Some(GridEvent::Remove(index));
            }
        });
    }
    event
}
