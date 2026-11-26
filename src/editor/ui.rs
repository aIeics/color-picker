//! The Color editor tab: type a HEX value or move RGB, HSL or HSV sliders;
//! the color's values in every format update below.

use eframe::egui::{Color32, Ui};

use super::{Editor, Mode};
use crate::action::Action;
use crate::color::{Hsl, Hsv, Rgb};
use crate::settings::Settings;
use crate::ui::icons::Icon;
use crate::ui::{details, theme, widgets};

/// Width of a slider's letter and of its value.
const LETTER_W: f32 = 30.0;
const VALUE_W: f32 = 50.0;

pub fn show(
    ui: &mut Ui,
    editor: &mut Editor,
    color: Rgb,
    settings: &Settings,
    actions: &mut Vec<Action>,
) {
    let hex_id = ui.make_persistent_id("editor-hex");
    let typing = ui.ctx().memory(|m| m.has_focus(hex_id));
    editor.sync(color, settings.hex, typing);

    let mut rows = |ui: &mut Ui, width: f32, actions: &mut Vec<Action>| {
        let column = widgets::column_width(width, 3);
        ui.horizontal(|ui| {
            widgets::pill(ui, column, |p| {
                p.label("HEX", 52.0);
                p.divider();
                let width = p.remaining();
                let response = p.text_edit(hex_id, &mut editor.hex_text, "#RRGGBB", width);
                if response.changed()
                    && let Some(typed) = Rgb::from_hex(&editor.hex_text)
                {
                    actions.push(Action::SetColor(editor.set_rgb(typed)));
                }
            });
            widgets::pill(ui, column, |p| {
                let width = p.remaining() / Mode::ALL.len() as f32;
                for mode in Mode::ALL {
                    if p.chip(mode.label(), editor.mode == mode, width).clicked() {
                        editor.mode = mode;
                    }
                }
            });
            widgets::pill(ui, column, |p| {
                let width = p.remaining();
                if p.button(Some(Icon::Plus), "Save to palette", width)
                    .clicked()
                {
                    actions.push(Action::AddToPalette(color));
                }
            });
        });
        ui.horizontal(|ui| {
            if let Some(new) = sliders(ui, editor, color, column) {
                actions.push(Action::SetColor(new));
            }
        });
    };
    details::show(ui, color, settings, actions, Some((2, &mut rows)));
}

/// Three slider pills for the editor's mode. Returns the new color when a
/// slider moved.
fn sliders(ui: &mut Ui, editor: &mut Editor, color: Rgb, width: f32) -> Option<Rgb> {
    let mut changed = None;
    match editor.mode {
        Mode::Rgb => {
            let channels = [color.r, color.g, color.b];
            for (index, letter) in ["R", "G", "B"].into_iter().enumerate() {
                let mut value = f32::from(channels[index]);
                let with = |v: f32| {
                    let mut c = channels;
                    c[index] = v.round().clamp(0.0, 255.0) as u8;
                    Color32::from_rgb(c[0], c[1], c[2])
                };
                if slider(ui, letter, &mut value, 0.0..=255.0, width, 1, with, |v| {
                    format!("{v:.0}")
                }) {
                    let mut c = channels;
                    c[index] = value.round().clamp(0.0, 255.0) as u8;
                    changed = Some(editor.set_rgb(Rgb::new(c[0], c[1], c[2])));
                }
            }
        }
        Mode::Hsl => {
            let hsl = editor.hsl();
            let mut values = [hsl.h, hsl.s * 100.0, hsl.l * 100.0];
            let make = |v: [f32; 3]| Hsl {
                h: v[0],
                s: v[1] / 100.0,
                l: v[2] / 100.0,
            };
            for index in 0..3 {
                if hue_slider(ui, index, "HSL", &mut values, width, |v| {
                    Rgb::from_hsl(make(v)).into()
                }) {
                    changed = Some(editor.set_hsl(make(values)));
                }
            }
        }
        Mode::Hsv => {
            let hsv = editor.hsv();
            let mut values = [hsv.h, hsv.s * 100.0, hsv.v * 100.0];
            let make = |v: [f32; 3]| Hsv {
                h: v[0],
                s: v[1] / 100.0,
                v: v[2] / 100.0,
            };
            for index in 0..3 {
                if hue_slider(ui, index, "HSV", &mut values, width, |v| {
                    Rgb::from_hsv(make(v)).into()
                }) {
                    changed = Some(editor.set_hsv(make(values)));
                }
            }
        }
    }
    changed
}

/// Slider `index` of an HSL or HSV editor (`letters` names the three). The
/// hue track always shows the full rainbow; the others show the range of
/// the current color.
fn hue_slider(
    ui: &mut Ui,
    index: usize,
    letters: &str,
    values: &mut [f32; 3],
    width: f32,
    color_of: impl Fn([f32; 3]) -> Color32,
) -> bool {
    let current = *values;
    let (max, segments) = if index == 0 { (360.0, 36) } else { (100.0, 12) };
    let letter = &letters[index..=index];
    let color_at = |v: f32| {
        if index == 0 {
            Rgb::from_hsl(Hsl {
                h: v,
                s: 1.0,
                l: 0.5,
            })
            .into()
        } else {
            let mut at = current;
            at[index] = v;
            color_of(at)
        }
    };
    let format = |v: f32| {
        if index == 0 {
            format!("{:.0}°", v.round() % 360.0)
        } else {
            format!("{v:.0}%")
        }
    };
    let mut value = values[index];
    let moved = slider(
        ui,
        letter,
        &mut value,
        0.0..=max,
        width,
        segments,
        color_at,
        format,
    );
    values[index] = value;
    moved
}

/// A pill with a letter, a gradient slider and the value.
#[allow(clippy::too_many_arguments)]
fn slider(
    ui: &mut Ui,
    letter: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    width: f32,
    segments: usize,
    color_at: impl Fn(f32) -> Color32,
    format: impl Fn(f32) -> String,
) -> bool {
    widgets::pill(ui, width, |p| {
        p.label(letter, LETTER_W);
        let track = p.remaining() - VALUE_W;
        let moved = p
            .gradient_slider(value, range, 1.0, track, segments, color_at)
            .changed();
        p.value(&format(*value), theme::TEXT, VALUE_W);
        moved
    })
}
