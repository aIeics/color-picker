//! Picking colors from the screen: the Screen tab. The pick itself (hotkey,
//! mouse hook and loupe) runs on the platform thread, see
//! `platform::windows::service`; this module shows its state.

use eframe::egui::Ui;

use crate::action::Action;
use crate::color::{HexStyle, Rgb};
use crate::settings::Settings;
use crate::ui::icons::Icon;
use crate::ui::{details, theme, widgets};

/// How a pick looks and samples: loupe zoom (screen pixels per picked pixel
/// at 100 % scaling), the side of the averaged square, and the HEX style of
/// the loupe's label.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickStyle {
    pub zoom: u8,
    pub sample: i32,
    pub hex: HexStyle,
}

impl PickStyle {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            zoom: settings.zoom,
            sample: settings.sample.side() as i32,
            hex: settings.hex,
        }
    }
}

/// State of screen picking, as reported by the platform thread.
#[derive(Debug, Default)]
pub struct ScreenPick {
    /// A pick is running: the loupe follows the cursor.
    pub picking: bool,
    /// Why the hotkey couldn't be registered, if it couldn't.
    pub hotkey_error: Option<String>,
}

pub fn show(
    ui: &mut Ui,
    state: &ScreenPick,
    color: Rgb,
    settings: &Settings,
    actions: &mut Vec<Action>,
) {
    let mut top_row = |ui: &mut Ui, width: f32, actions: &mut Vec<Action>| {
        let narrow = widgets::column_width(width, 3);
        let wide = width - narrow - widgets::GAP;
        ui.horizontal(|ui| {
            widgets::pill(ui, wide, |p| {
                let hotkey = match (&state.hotkey_error, settings.hotkey) {
                    (Some(_), Some(_)) => "Hotkey in use".to_owned(),
                    (_, Some(hotkey)) => hotkey.label(),
                    (_, None) => "No hotkey".to_owned(),
                };
                let hotkey_w = widgets::text_width(p.ui, &hotkey, theme::regular(13.5)) + 22.0;
                let button_w = p.remaining() - hotkey_w - 9.0;
                if state.picking {
                    if p.button_colored(
                        Some(Icon::Crosshair),
                        "Click to pick · Esc cancels",
                        theme::TEXT,
                        button_w,
                    )
                    .clicked()
                    {
                        actions.push(Action::CancelScreenPick);
                    }
                } else if p
                    .button(Some(Icon::Crosshair), "Pick from screen", button_w)
                    .on_hover_text("Then click anywhere. Esc or right-click cancels.")
                    .clicked()
                {
                    actions.push(Action::StartScreenPick);
                }
                p.divider();
                let width = p.remaining();
                p.label(&hotkey, width);
            });
            widgets::pill(ui, narrow, |p| {
                let width = p.remaining();
                if p.button(Some(Icon::Plus), "Save to palette", width)
                    .clicked()
                {
                    actions.push(Action::AddToPalette(color));
                }
            });
        });
    };
    details::show(ui, color, settings, actions, Some((1, &mut top_row)));
}
