//! The History tab: the current color's values, then recent picks.

use eframe::egui::Ui;

use super::History;
use crate::action::Action;
use crate::color::Rgb;
use crate::settings::Settings;
use crate::ui::grid::{self, GridEvent};
use crate::ui::icons::Icon;
use crate::ui::{details, theme, widgets};

pub fn show(
    ui: &mut Ui,
    history: &History,
    color: Rgb,
    settings: &Settings,
    actions: &mut Vec<Action>,
) {
    let count = history.colors().len();
    let mut top_row = |ui: &mut Ui, width: f32, actions: &mut Vec<Action>| {
        widgets::pill(ui, width, |p| {
            p.label("History", 76.0);
            p.divider();
            let clear_w = widgets::button_width(p.ui, true, "Clear");
            let text = format!("{count} of {} recent picks", History::MAX);
            let width = p.remaining() - clear_w;
            p.value(&text, theme::TEXT_DIM, width);
            if count > 0
                && p.button(Some(Icon::Trash), "Clear", clear_w)
                    .on_hover_text("Remove all recent picks")
                    .clicked()
            {
                actions.push(Action::ClearHistory);
            }
        });
    };
    details::show(ui, color, settings, actions, Some((1, &mut top_row)));

    let event = grid::show(
        ui,
        history.colors(),
        color,
        settings.hex,
        "Colors you pick from the screen or an image show up here.",
        true,
    );
    let colors = history.colors();
    match event {
        Some(GridEvent::Select(i)) => actions.push(Action::SetColor(colors[i])),
        Some(GridEvent::Copy(i)) => actions.push(Action::Copy(settings.hex.format(colors[i]))),
        Some(GridEvent::AddToPalette(i)) => actions.push(Action::AddToPalette(colors[i])),
        Some(GridEvent::Remove(i)) => actions.push(Action::RemoveFromHistory(i)),
        None => {}
    }
}
