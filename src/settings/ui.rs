//! The Settings tab, after BlurAutoClicker's Settings: sections on the left,
//! cards with labeled rows on the right, the content framed by the accent
//! line. Rows change the settings directly and push actions for what the
//! app must apply.

use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, Galley, Key, Layout, Rect, Sense, Shape, Stroke,
    StrokeKind, TextureId, Ui, UiBuilder, Vec2, pos2, text::LayoutJob, vec2,
};

use super::{
    AccentMode, IconStyle, SCALES, SampleSize, Settings, ThemeMode, TrayMode, ZOOM_LEVELS,
};
use crate::action::Action;
use crate::app::Tab;
use crate::color::{Format, HexStyle, Rgb};
use crate::hotkey::Hotkey;
use crate::platform;
use crate::ui::icons::{self, Icon};
use crate::ui::theme::{self, p};
use crate::ui::widgets::{self, CTRL_H};

const SIDEBAR_W: f32 = 178.0;
const ROW_H: f32 = 52.0;
/// How long "Reset" waits for the confirming second click.
const CONFIRM_TIME: Duration = Duration::from_secs(3);

pub const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Section {
    #[default]
    General,
    Picking,
    Copying,
    Appearance,
    Window,
}

impl Section {
    const ALL: [Self; 5] = [
        Self::General,
        Self::Picking,
        Self::Copying,
        Self::Appearance,
        Self::Window,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Picking => "Picking",
            Self::Copying => "Copying",
            Self::Appearance => "Appearance",
            Self::Window => "Window",
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::General => Icon::Info,
            Self::Picking => Icon::Crosshair,
            Self::Copying => Icon::Copy,
            Self::Appearance => Icon::Sun,
            Self::Window => Icon::AppWindow,
        }
    }
}

/// UI state of the Settings tab.
#[derive(Debug, Default)]
pub struct SettingsTab {
    /// Waiting for the user to press a new hotkey.
    pub recording: bool,
    section: Section,
    /// "Reset everything" needs a second click before this moment.
    confirm_reset_until: Option<Instant>,
    /// What the accent HEX field shows, and the color it was synced from.
    accent_text: String,
    accent_synced: Option<Rgb>,
}

/// What the tab shows but doesn't own.
pub struct Status<'a> {
    pub hotkey_error: Option<&'a str>,
    pub autostart: bool,
    /// The current color, for "Use current".
    pub color: Rgb,
    pub settings_path: Option<String>,
    /// The loaded background image, for its thumbnail.
    pub background: Option<(TextureId, Vec2)>,
}

/// Draws the tab into `rect` (between the title bar and the status bar).
pub fn show(
    ui: &mut Ui,
    rect: Rect,
    tab: &mut SettingsTab,
    settings: &mut Settings,
    status: &Status<'_>,
    actions: &mut Vec<Action>,
) {
    if tab.recording {
        record(ui, tab, settings, actions);
    }
    let accent = Tab::Settings.color();
    sidebar(
        ui,
        Rect::from_min_size(rect.min, vec2(SIDEBAR_W, rect.height())),
        tab,
        accent,
    );

    // The content, framed on the top and left by the accent line with a
    // rounded corner, like BlurAutoClicker's Settings.
    let content = Rect::from_min_max(pos2(rect.left() + SIDEBAR_W, rect.top()), rect.max);
    frame_line(ui, content, accent);
    let inner = Rect::from_min_max(
        content.min + vec2(14.0, 12.0),
        content.max - vec2(12.0, 0.0),
    );
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::top_down(Align::Min)),
    );
    egui::ScrollArea::vertical()
        .id_salt(("settings", tab.section as u8))
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 12.0);
            ui.set_width(inner.width() - 8.0);
            let mut cx = Cx {
                settings,
                status,
                actions,
                accent,
            };
            match tab.section {
                Section::General => general(ui, tab, &mut cx),
                Section::Picking => picking(ui, tab, &mut cx),
                Section::Copying => copying(ui, &mut cx),
                Section::Appearance => appearance(ui, tab, &mut cx),
                Section::Window => window(ui, &mut cx),
            }
            ui.add_space(2.0);
        });
}

/// What the section functions work with.
struct Cx<'a, 'b> {
    settings: &'a mut Settings,
    status: &'a Status<'b>,
    actions: &'a mut Vec<Action>,
    accent: Color32,
}

impl Cx<'_, '_> {
    fn changed(&mut self) {
        self.actions.push(Action::SettingsChanged);
    }
}

fn sidebar(ui: &mut Ui, rect: Rect, tab: &mut SettingsTab, accent: Color32) {
    let p = p();
    let mut y = rect.top() + 12.0;
    for section in Section::ALL {
        let item =
            Rect::from_min_size(pos2(rect.left() + 10.0, y), vec2(rect.width() - 20.0, 36.0));
        y += 40.0;
        let response = ui
            .interact(
                item,
                ui.make_persistent_id(("nav", section as u8)),
                Sense::click(),
            )
            .on_hover_cursor(CursorIcon::PointingHand);
        let on = tab.section == section;
        if on || response.hovered() {
            ui.painter().rect_filled(item, 6.0, p.elevated);
        }
        let icon_color = if on { accent } else { p.text_dim };
        icons::paint(
            ui.painter(),
            Rect::from_center_size(pos2(item.left() + 19.0, item.center().y), Vec2::splat(18.0)),
            section.icon(),
            icon_color,
        );
        let color = if on || response.hovered() {
            p.text
        } else {
            p.text_muted
        };
        ui.painter().text(
            pos2(item.left() + 39.0, item.center().y),
            Align2::LEFT_CENTER,
            section.label(),
            theme::bold(15.0),
            color,
        );
        if response.clicked() {
            tab.section = section;
        }
    }
}

/// The accent line along the content's top and left, rounded at the corner.
fn frame_line(ui: &Ui, rect: Rect, color: Color32) {
    const R: f32 = 16.0;
    let mut points = vec![pos2(rect.left() + 1.0, rect.bottom())];
    let corner = pos2(rect.left() + 1.0 + R, rect.top() + 1.0 + R);
    for i in 0..=12 {
        let a = std::f32::consts::PI + std::f32::consts::FRAC_PI_2 * i as f32 / 12.0;
        points.push(corner + vec2(a.cos(), a.sin()) * R);
    }
    points.push(pos2(rect.right(), rect.top() + 1.0));
    ui.painter()
        .add(Shape::line(points, Stroke::new(2.0, color)));
}

/// A card: a title and a short description, then rows.
fn card(ui: &mut Ui, title: &str, desc: &str, rows: impl FnOnce(&mut Ui)) {
    let p = p();
    egui::Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(2.0, p.card_border))
        .corner_radius(14)
        .inner_margin(egui::Margin {
            left: 16,
            right: 16,
            top: 12,
            bottom: 4,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                ui.label(
                    egui::RichText::new(title)
                        .font(theme::bold(17.0))
                        .color(p.text),
                );
                ui.label(
                    egui::RichText::new(desc)
                        .font(theme::medium(12.5))
                        .color(p.text_dim),
                );
            });
            ui.add_space(2.0);
            rows(ui);
        });
}

/// A row: a label and a hint on the left, controls on the right. Controls
/// are added right to left. A line separates it from the row above.
fn row(
    ui: &mut Ui,
    label: &str,
    hint: &str,
    hint_color: Option<Color32>,
    controls: impl FnOnce(&mut Ui),
) {
    let p = p();
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::hover());
    let first = ui.min_rect().top() + 40.0 > rect.top();
    if !first {
        ui.painter().hline(
            rect.x_range(),
            rect.top(),
            Stroke::new(1.0, p.border_subtle),
        );
    }
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::right_to_left(Align::Center)),
    );
    child.spacing_mut().item_spacing = vec2(8.0, 0.0);
    controls(&mut child);
    let controls_left = child.min_rect().left().min(rect.right());
    let max = (controls_left - rect.left() - 14.0).max(40.0);
    let title = truncated(ui, label, theme::bold(14.5), p.text, max);
    let sub = truncated(
        ui,
        hint,
        theme::medium(12.5),
        hint_color.unwrap_or(p.text_dim),
        max,
    );
    let total = title.size().y + 2.0 + sub.size().y;
    let top = rect.center().y - total / 2.0;
    ui.painter().galley(pos2(rect.left(), top), title, p.text);
    ui.painter().galley(
        pos2(rect.left(), top + total - sub.size().y),
        sub,
        hint_color.unwrap_or(p.text_dim),
    );
}

/// One line of text, cut with "…" when wider than `max_width`.
fn truncated(
    ui: &Ui,
    text: &str,
    font: egui::FontId,
    color: Color32,
    max_width: f32,
) -> Arc<Galley> {
    let mut job = LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap.max_width = max_width;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    ui.painter().layout_job(job)
}

fn general(ui: &mut Ui, tab: &mut SettingsTab, cx: &mut Cx<'_, '_>) {
    let now = Instant::now();
    let confirming = tab.confirm_reset_until.is_some_and(|until| now < until);
    if let Some(until) = tab.confirm_reset_until.filter(|_| confirming) {
        ui.ctx().request_repaint_after(until - now);
    }
    let version = concat!("Version ", env!("CARGO_PKG_VERSION"));
    card(ui, "Color Picker", version, |ui| {
        let path = cx
            .status
            .settings_path
            .as_deref()
            .unwrap_or("Not saved (no settings folder)");
        row(ui, "Settings file", path, None, |ui| {
            if widgets::pbtn(ui, Some(Icon::Folder), "Open folder", None).clicked() {
                cx.actions.push(Action::OpenSettingsFolder);
            }
        });
        row(
            ui,
            "Reset everything",
            "Settings, history and palette back to defaults.",
            None,
            |ui| {
                let text = if confirming {
                    "Click to confirm"
                } else {
                    "Reset…"
                };
                if widgets::pbtn(ui, Some(Icon::Reset), text, Some(p().danger)).clicked() {
                    if confirming {
                        tab.confirm_reset_until = None;
                        cx.actions.push(Action::ResetAll);
                    } else {
                        tab.confirm_reset_until = Some(Instant::now() + CONFIRM_TIME);
                    }
                }
            },
        );
    });
    card(ui, "Source", "Open source on GitHub.", |ui| {
        let name = REPO_URL.trim_start_matches("https://github.com/");
        row(ui, name, "Code, releases and issues.", None, |ui| {
            if widgets::pbtn(ui, Some(Icon::ExternalLink), "GitHub", None).clicked() {
                cx.actions.push(Action::OpenUrl(REPO_URL.to_owned()));
            }
        });
    });
}

fn picking(ui: &mut Ui, tab: &mut SettingsTab, cx: &mut Cx<'_, '_>) {
    card(
        ui,
        "Hotkey",
        "Works anywhere, even with the window in the tray.",
        |ui| {
            let p = p();
            let label = cx
                .settings
                .hotkey
                .map_or_else(|| "None".to_owned(), Hotkey::label);
            let (text, color, hint, hint_color) = if tab.recording {
                (
                    "Press a shortcut…".to_owned(),
                    p.text,
                    "Esc cancels. Ctrl, Alt, Shift or Win plus a key.",
                    None,
                )
            } else if let (Some(error), Some(_)) = (cx.status.hotkey_error, cx.settings.hotkey) {
                (label, p.danger, error, Some(p.danger))
            } else {
                (
                    label,
                    p.text,
                    "Starts a pick. Esc or right-click cancels.",
                    None,
                )
            };
            row(ui, "Pick hotkey", hint, hint_color, |ui| {
                let edit = if tab.recording { "Cancel" } else { "Edit" };
                if widgets::pbtn(ui, Some(Icon::Keyboard), edit, None).clicked() {
                    tab.recording = !tab.recording;
                    cx.actions.push(Action::HotkeyRecording(tab.recording));
                }
                if widgets::pbtn(ui, Some(Icon::Clear), "", None)
                    .on_hover_text("No hotkey")
                    .clicked()
                    && cx.settings.hotkey.is_some()
                {
                    cx.settings.hotkey = None;
                    if tab.recording {
                        tab.recording = false;
                        cx.actions.push(Action::HotkeyRecording(false));
                    }
                    cx.changed();
                }
                widgets::field(ui, &text, color, 150.0);
            });
        },
    );
    card(ui, "Sampling", "What a click picks.", |ui| {
        row(
            ui,
            "Sample",
            "One pixel, or the average of a square.",
            None,
            |ui| {
                let options = SampleSize::ALL.map(|s| (s, s.label()));
                if widgets::segmented(ui, &mut cx.settings.sample, &options, cx.accent) {
                    cx.changed();
                }
            },
        );
        row(
            ui,
            "Loupe zoom",
            "The mouse wheel changes it while picking.",
            None,
            |ui| {
                let labels = ZOOM_LEVELS.map(|z| format!("{z}×"));
                let options: Vec<(u8, &str)> = ZOOM_LEVELS
                    .iter()
                    .zip(&labels)
                    .map(|(&z, l)| (z, l.as_str()))
                    .collect();
                if widgets::segmented(ui, &mut cx.settings.zoom, &options, cx.accent) {
                    cx.changed();
                }
            },
        );
    });
}

fn copying(ui: &mut Ui, cx: &mut Cx<'_, '_>) {
    card(
        ui,
        "Copy on pick",
        "After each pick from the screen or an image.",
        |ui| {
            row(
                ui,
                "Copy automatically",
                "Puts the picked color on the clipboard.",
                None,
                |ui| {
                    if widgets::onoff(ui, &mut cx.settings.copy_on_pick) {
                        cx.changed();
                    }
                },
            );
            row(
                ui,
                "Format",
                "How the copied color is written.",
                None,
                |ui| {
                    let options = Format::ALL.map(|f| (f, f.label()));
                    ui.add_enabled_ui(cx.settings.copy_on_pick, |ui| {
                        if widgets::segmented(ui, &mut cx.settings.copy_format, &options, cx.accent)
                        {
                            cx.changed();
                        }
                    });
                },
            );
        },
    );
    card(ui, "Values", "Next to the swatch on every tab.", |ui| {
        row(ui, "HEX style", "Shown and copied this way.", None, |ui| {
            let options = HexStyle::ALL.map(|s| (s, s.label()));
            if widgets::segmented(ui, &mut cx.settings.hex, &options, cx.accent) {
                cx.changed();
            }
        });
        row(ui, "Formats shown", "At least one stays on.", None, |ui| {
            for format in Format::ALL.into_iter().rev() {
                let on = cx.settings.formats.contains(format);
                if widgets::chip(ui, format.label(), on).clicked() {
                    if on && cx.settings.formats.len() == 1 {
                        cx.actions
                            .push(Action::Toast("Keep at least one format".into()));
                    } else {
                        cx.settings.formats.set(format, !on);
                        cx.changed();
                    }
                }
            }
        });
    });
}

fn appearance(ui: &mut Ui, tab: &mut SettingsTab, cx: &mut Cx<'_, '_>) {
    card(ui, "Theme & accent", "Applied to all tabs.", |ui| {
        row(ui, "Theme", "Dark, light, or follow Windows.", None, |ui| {
            let options: Vec<(ThemeMode, &str)> =
                ThemeMode::ALL.iter().map(|&t| (t, t.label())).collect();
            if widgets::segmented(ui, &mut cx.settings.theme, &options, cx.accent) {
                cx.changed();
            }
        });
        row(
            ui,
            "Accent",
            "A color per tab, or one color everywhere.",
            None,
            |ui| {
                let options: Vec<(AccentMode, &str)> =
                    AccentMode::ALL.iter().map(|&a| (a, a.label())).collect();
                if widgets::segmented(ui, &mut cx.settings.accent, &options, cx.accent) {
                    cx.changed();
                }
            },
        );
        if cx.settings.accent == AccentMode::Single {
            if tab.accent_synced != Some(cx.settings.accent_color) {
                tab.accent_text = HexStyle::default().format(cx.settings.accent_color);
                tab.accent_synced = Some(cx.settings.accent_color);
            }
            row(
                ui,
                "Accent color",
                "Type it, or take the current color.",
                None,
                |ui| {
                    if widgets::pbtn(ui, Some(Icon::Pipette), "Use current", None).clicked() {
                        cx.settings.accent_color = cx.status.color;
                        cx.changed();
                    }
                    let id = ui.make_persistent_id("accent-hex");
                    if widgets::boxed_text_edit(ui, id, &mut tab.accent_text, 96.0).changed()
                        && let Some(color) = Rgb::from_hex(&tab.accent_text)
                    {
                        cx.settings.accent_color = color;
                        tab.accent_synced = Some(color);
                        cx.changed();
                    }
                    widgets::swatch(
                        ui,
                        Vec2::splat(CTRL_H),
                        cx.settings.accent_color,
                        Sense::hover(),
                    );
                },
            );
        }
    });
    card(
        ui,
        "Interface",
        "Size of everything, and the bottom bar.",
        |ui| {
            row(
                ui,
                "Interface scale",
                "Text, controls and the window together.",
                None,
                |ui| {
                    if widgets::stepper(ui, &mut cx.settings.scale, &SCALES) {
                        cx.changed();
                    }
                },
            );
            row(
                ui,
                "Status bar",
                "Off: messages show in the title instead.",
                None,
                |ui| {
                    if widgets::onoff(ui, &mut cx.settings.status_bar) {
                        cx.changed();
                    }
                },
            );
        },
    );
    card(ui, "Background", "Behind every tab.", |ui| {
        let name = cx
            .settings
            .background
            .as_deref()
            .and_then(|path| std::path::Path::new(path).file_name())
            .map_or_else(|| "None".to_owned(), |n| n.to_string_lossy().into_owned());
        row(ui, "Background image", &name, None, |ui| {
            if cx.settings.background.is_some()
                && widgets::pbtn(ui, Some(Icon::Clear), "", None)
                    .on_hover_text("Remove the image")
                    .clicked()
            {
                cx.actions.push(Action::ClearBackground);
            }
            if widgets::pbtn(ui, Some(Icon::ImagePlus), "Choose…", None).clicked() {
                cx.actions.push(Action::ChooseBackground);
            }
            if let Some((texture, size)) = cx.status.background {
                let (rect, _) = ui.allocate_exact_size(vec2(54.0, CTRL_H), Sense::hover());
                let uv = cover_uv(size, rect.size());
                ui.painter().image(texture, rect, uv, Color32::WHITE);
                ui.painter().rect_stroke(
                    rect,
                    6.0,
                    Stroke::new(1.0, p().border_subtle),
                    StrokeKind::Inside,
                );
            }
        });
        row(
            ui,
            "Image opacity",
            "How strongly the image shows.",
            None,
            |ui| {
                if widgets::percent_slider(
                    ui,
                    &mut cx.settings.background_opacity,
                    0..=100,
                    cx.accent,
                ) {
                    cx.changed();
                }
            },
        );
        row(
            ui,
            "Panel opacity",
            "Pills, cards and bars over the image.",
            None,
            |ui| {
                let min = super::MIN_PANEL_OPACITY;
                if widgets::percent_slider(ui, &mut cx.settings.panel_opacity, min..=100, cx.accent)
                {
                    cx.changed();
                }
            },
        );
    });
    card(
        ui,
        "Icon & title",
        "Tray, taskbar and the window title.",
        |ui| {
            row(
                ui,
                "Tray and taskbar icon",
                "Changes right away.",
                None,
                |ui| {
                    for &style in IconStyle::ALL.iter().rev() {
                        if icon_tile(ui, style, cx.settings.tray_icon == style, cx.accent) {
                            cx.settings.tray_icon = style;
                            cx.changed();
                        }
                    }
                },
            );
            row(
                ui,
                "Fancy title",
                "A script font for the title (Segoe Script).",
                None,
                |ui| {
                    if widgets::onoff(ui, &mut cx.settings.fancy_title) {
                        cx.changed();
                    }
                },
            );
        },
    );
}

fn window(ui: &mut Ui, cx: &mut Cx<'_, '_>) {
    card(ui, "Window", "Where and how it stays.", |ui| {
        row(
            ui,
            "Always on top",
            "Same as the pin in the title bar.",
            None,
            |ui| {
                if widgets::onoff(ui, &mut cx.settings.always_on_top) {
                    cx.changed();
                }
            },
        );
        row(
            ui,
            "Remember position",
            "Reopens where you left it.",
            None,
            |ui| {
                if widgets::onoff(ui, &mut cx.settings.remember_position) {
                    cx.changed();
                }
            },
        );
    });
    card(ui, "Tray & startup", "Keeps the hotkey working.", |ui| {
        let hint = match cx.settings.tray {
            TrayMode::Off => "No tray icon; closing quits the app.",
            TrayMode::Close => "X hides to the tray instead of quitting.",
            TrayMode::Minimize => "Minimize hides to the tray; X quits.",
        };
        row(ui, "Tray", hint, None, |ui| {
            let options = TrayMode::ALL.map(|t| (t, t.label()));
            if widgets::segmented(ui, &mut cx.settings.tray, &options, cx.accent) {
                cx.changed();
            }
        });
        row(
            ui,
            "Start with Windows",
            "Starts hidden in the tray when you sign in.",
            None,
            |ui| {
                let mut on = cx.status.autostart;
                if widgets::onoff(ui, &mut on) {
                    cx.actions.push(Action::SetAutostart(on));
                }
            },
        );
    });
}

/// A tile with a small preview of a tray icon style. Returns `true` when
/// clicked while not selected.
fn icon_tile(ui: &mut Ui, style: IconStyle, selected: bool, accent: Color32) -> bool {
    let p = p();
    let (rect, response) = ui.allocate_exact_size(vec2(64.0, 46.0), Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    ui.painter().rect_filled(rect, 8.0, p.base);
    let border = if selected {
        Stroke::new(2.0, accent)
    } else if response.hovered() {
        Stroke::new(1.0, p.border)
    } else {
        Stroke::new(1.0, p.border_subtle)
    };
    ui.painter()
        .rect_stroke(rect, 8.0, border, StrokeKind::Inside);
    let icon = Rect::from_center_size(pos2(rect.center().x, rect.top() + 16.0), Vec2::splat(22.0));
    crate::ui::app_icon::paint_preview(ui.painter(), icon, style);
    ui.painter().text(
        pos2(rect.center().x, rect.bottom() - 9.0),
        Align2::CENTER_CENTER,
        style.label(),
        theme::regular(11.0),
        if selected { p.text } else { p.text_dim },
    );
    response.clicked() && !selected
}

/// UV coordinates that crop an image of `size` to fill `target` (cover).
pub fn cover_uv(size: Vec2, target: Vec2) -> Rect {
    if size.x <= 0.0 || size.y <= 0.0 || target.x <= 0.0 || target.y <= 0.0 {
        return Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    }
    let scale = (target.x / size.x).max(target.y / size.y);
    let visible = target / scale / size;
    let min = (Vec2::splat(1.0) - visible) / 2.0;
    Rect::from_min_size(min.to_pos2(), visible)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_crops_the_longer_side() {
        // A wide image in a square: the middle third is shown.
        let uv = cover_uv(vec2(300.0, 100.0), vec2(50.0, 50.0));
        assert!((uv.min.x - 1.0 / 3.0).abs() < 1e-5 && (uv.max.x - 2.0 / 3.0).abs() < 1e-5);
        assert_eq!((uv.min.y, uv.max.y), (0.0, 1.0));
        // Degenerate sizes show the whole image.
        assert_eq!(
            cover_uv(Vec2::ZERO, vec2(1.0, 1.0)),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))
        );
    }
}
