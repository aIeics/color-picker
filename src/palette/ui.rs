//! The Palette tab: the current color's values, then saved colors.

use std::time::{Duration, Instant};

use eframe::egui::Ui;

use super::Palette;
use crate::action::Action;
use crate::color::Rgb;
use crate::settings::Settings;
use crate::ui::grid::{self, GridEvent};
use crate::ui::icons::Icon;
use crate::ui::theme::p;
use crate::ui::{details, widgets};

/// How long "Clear" waits for the confirming second click.
const CONFIRM_TIME: Duration = Duration::from_secs(3);

/// UI state of the Palette tab.
#[derive(Debug, Default)]
pub struct PaletteTab {
    /// Clearing needs a second click before this moment.
    confirm_clear_until: Option<Instant>,
}

pub fn show(
    ui: &mut Ui,
    tab: &mut PaletteTab,
    palette: &Palette,
    color: Rgb,
    settings: &Settings,
    actions: &mut Vec<Action>,
) {
    let count = palette.colors().len();
    let now = Instant::now();
    let confirming = tab.confirm_clear_until.is_some_and(|until| now < until);
    if let Some(until) = tab.confirm_clear_until.filter(|_| confirming) {
        ui.ctx().request_repaint_after(until - now);
    }

    let mut top_row = |ui: &mut Ui, width: f32, actions: &mut Vec<Action>| {
        widgets::pill(ui, width, |pill| {
            pill.label("Palette", 76.0);
            pill.divider();
            let add_w = widgets::button_width(pill.ui, true, "Save current");
            let clear_text = if confirming {
                "Click to confirm"
            } else {
                "Clear"
            };
            let clear_w = widgets::button_width(pill.ui, true, clear_text);
            let text = format!("{count} of {} colors", Palette::MAX);
            let width = pill.remaining() - add_w - if count > 0 { clear_w } else { 0.0 };
            pill.value(&text, p().text_dim, width);
            let in_palette = palette.contains(color);
            pill.ui.add_enabled_ui(!in_palette, |ui| {
                let mut pill = widgets::Pill { ui };
                if pill
                    .button(Some(Icon::Plus), "Save current", add_w)
                    .on_hover_text("Save the current color to the palette")
                    .clicked()
                {
                    actions.push(Action::AddToPalette(color));
                }
            });
            if count > 0 {
                let color = if confirming { p().danger } else { p().text };
                if pill
                    .button_colored(Some(Icon::Trash), clear_text, color, clear_w)
                    .clicked()
                {
                    if confirming {
                        tab.confirm_clear_until = None;
                        actions.push(Action::ClearPalette);
                    } else {
                        tab.confirm_clear_until = Some(Instant::now() + CONFIRM_TIME);
                    }
                }
            }
        });
    };
    details::show(ui, color, settings, actions, Some((1, &mut top_row)));

    let event = grid::show(
        ui,
        palette.colors(),
        color,
        settings.hex,
        "Save colors here with \"Save current\" or \"Save to palette\".",
        false,
    );
    let colors = palette.colors();
    match event {
        Some(GridEvent::Select(i)) => actions.push(Action::SetColor(colors[i])),
        Some(GridEvent::Copy(i)) => actions.push(Action::Copy(settings.hex.format(colors[i]))),
        Some(GridEvent::Remove(i)) => actions.push(Action::RemoveFromPalette(i)),
        Some(GridEvent::AddToPalette(_)) | None => {}
    }
}
