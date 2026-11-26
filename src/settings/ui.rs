//! The Settings tab: the pick hotkey and loupe, copying, which formats show,
//! and how the window behaves.

use eframe::egui::{self, Key, Ui};

use super::{SampleSize, Settings, TrayMode, ZOOM_LEVELS};
use crate::action::Action;
use crate::color::{Format, HexStyle};
use crate::hotkey::Hotkey;
use crate::platform;
use crate::ui::icons::Icon;
use crate::ui::{theme, widgets};

/// UI state of the Settings tab.
#[derive(Debug, Default)]
pub struct SettingsTab {
    /// Waiting for the user to press a new hotkey.
    pub recording: bool,
}

/// What the tab shows but doesn't own.
pub struct Status<'a> {
    pub hotkey_error: Option<&'a str>,
    pub autostart: bool,
}

pub fn show(
    ui: &mut Ui,
    tab: &mut SettingsTab,
    settings: &mut Settings,
    status: &Status<'_>,
    actions: &mut Vec<Action>,
) {
    if tab.recording {
        record(ui, tab, settings, actions);
    }
    let width = ui.available_width();
    let column = widgets::column_width(width, 3);
    let wide = column * 2.0 + widgets::GAP;
    let mut changed = false;

    ui.horizontal(|ui| {
        widgets::pill(ui, wide, |p| {
            p.label("Pick hotkey", 100.0);
            p.divider();
            let label = settings
                .hotkey
                .map_or_else(|| "None".to_owned(), Hotkey::label);
            let (text, color) = if tab.recording {
                ("Press a shortcut…  Esc cancels".to_owned(), theme::TEXT)
            } else if let (Some(error), Some(_)) = (status.hotkey_error, settings.hotkey) {
                (format!("{label}  ·  {error}"), theme::DANGER)
            } else {
                (label, theme::TEXT)
            };
            let width = p.remaining() - 2.0 * widgets::SEG_H;
            p.value(&text, color, width);
            if p.icon_button(Icon::Clear, "No hotkey").clicked() && settings.hotkey.is_some() {
                settings.hotkey = None;
                if tab.recording {
                    tab.recording = false;
                    actions.push(Action::HotkeyRecording(false));
                }
                changed = true;
            }
            let tip = if tab.recording {
                "Stop recording"
            } else {
                "Record a new hotkey"
            };
            if p.icon_button(Icon::Keyboard, tip).clicked() {
                tab.recording = !tab.recording;
                actions.push(Action::HotkeyRecording(tab.recording));
            }
        });
        widgets::pill(ui, column, |p| {
            p.label("Tray", 52.0);
            p.divider();
            let width = p.remaining();
            changed |= p.stepper(
                &mut settings.tray,
                &TrayMode::ALL,
                |m| m.label().into(),
                width,
            );
        });
    });

    ui.horizontal(|ui| {
        widgets::pill(ui, column, |p| {
            p.label("Sample", 70.0);
            p.divider();
            let chip = p.remaining() / SampleSize::ALL.len() as f32;
            for size in SampleSize::ALL {
                let tip = match size {
                    SampleSize::One => "The pixel under the cursor".to_owned(),
                    _ => format!("The average of {} pixels", size.label()),
                };
                if p.chip(size.label(), settings.sample == size, chip)
                    .on_hover_text(tip)
                    .clicked()
                    && settings.sample != size
                {
                    settings.sample = size;
                    changed = true;
                }
            }
        });
        widgets::pill(ui, column, |p| {
            p.label("Zoom", 58.0);
            p.divider();
            let chip = p.remaining() / ZOOM_LEVELS.len() as f32;
            for zoom in ZOOM_LEVELS {
                if p.chip(&format!("{zoom}×"), settings.zoom == zoom, chip)
                    .on_hover_text("Loupe zoom (the mouse wheel changes it while picking)")
                    .clicked()
                    && settings.zoom != zoom
                {
                    settings.zoom = zoom;
                    changed = true;
                }
            }
        });
        widgets::pill(ui, column, |p| {
            p.label("Copy on pick", 100.0);
            p.divider();
            let options: Vec<Option<Format>> =
                std::iter::once(None).chain(Format::ALL.map(Some)).collect();
            let width = p.remaining();
            changed |= p.stepper(
                &mut settings.copy_on_pick,
                &options,
                |f| f.map_or("Off", Format::label).into(),
                width,
            );
        });
    });

    ui.horizontal(|ui| {
        widgets::pill(ui, column, |p| {
            p.label("HEX style", 82.0);
            p.divider();
            let width = p.remaining();
            changed |= p.stepper(
                &mut settings.hex,
                &HexStyle::ALL,
                |s| s.label().into(),
                width,
            );
        });
        widgets::pill(ui, wide, |p| {
            p.label("Show", 58.0);
            p.divider();
            let chip = p.remaining() / Format::ALL.len() as f32;
            for format in Format::ALL {
                let on = settings.formats.contains(format);
                if p.chip(format.label(), on, chip).clicked() {
                    if on && settings.formats.len() == 1 {
                        actions.push(Action::Toast("Keep at least one format".into()));
                    } else {
                        settings.formats.set(format, !on);
                        changed = true;
                    }
                }
            }
        });
    });

    ui.horizontal(|ui| {
        changed |= switch_pill(ui, column, "Always on top", &mut settings.always_on_top);
        changed |= switch_pill(
            ui,
            column,
            "Remember position",
            &mut settings.remember_position,
        );
        let mut autostart = status.autostart;
        if switch_pill(ui, column, "Start with Windows", &mut autostart) {
            actions.push(Action::SetAutostart(autostart));
        }
    });

    if changed {
        actions.push(Action::SettingsChanged);
    }
}

fn switch_pill(ui: &mut Ui, width: f32, label: &str, on: &mut bool) -> bool {
    widgets::pill(ui, width, |p| {
        let label_w = p.remaining() - 48.0;
        p.label(label, label_w);
        p.switch(on).changed()
    })
}

/// Takes the next key press as the new hotkey. Esc cancels.
fn record(ui: &Ui, tab: &mut SettingsTab, settings: &mut Settings, actions: &mut Vec<Action>) {
    let pressed = ui.input(|i| {
        i.events.iter().find_map(|event| match event {
            egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } if !is_modifier(*key) => Some((*key, *modifiers)),
            _ => None,
        })
    });
    let Some((key, modifiers)) = pressed else {
        return;
    };
    if key == Key::Escape {
        tab.recording = false;
        actions.push(Action::HotkeyRecording(false));
        return;
    }
    let hotkey = Hotkey {
        ctrl: modifiers.ctrl,
        alt: modifiers.alt,
        shift: modifiers.shift,
        win: platform::win_key_down(),
        key,
    };
    if hotkey.is_valid() {
        settings.hotkey = Some(hotkey);
        tab.recording = false;
        actions.push(Action::HotkeyRecording(false));
        actions.push(Action::SettingsChanged);
    } else {
        actions.push(Action::Toast(
            "Hold Ctrl, Alt, Shift or Win with a key (F-keys work alone)".into(),
        ));
    }
}

/// Ctrl, Shift, Alt and Win arrive as key presses of their own; a hotkey
/// is recorded at the key pressed with them.
fn is_modifier(key: Key) -> bool {
    matches!(
        key,
        Key::ShiftLeft
            | Key::ShiftRight
            | Key::ControlLeft
            | Key::ControlRight
            | Key::AltLeft
            | Key::AltRight
            | Key::SuperLeft
            | Key::SuperRight
    )
}
