//! The app: custom title bar, tabs, and the state they share. Tabs draw
//! from read-only state and push [`Action`]s, which are applied here after
//! drawing.

use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Layout, Rect, Stroke, StrokeKind, UiBuilder, ViewportCommand, WindowLevel, pos2,
    vec2,
};

use crate::action::Action;
use crate::color::Rgb;
use crate::history::{self, History};
use crate::palette::{self, Added, Palette};
use crate::platform::MainWindow;
use crate::screen::{self, ScreenPick};
use crate::settings::Settings;
use crate::store::{self, Saved};
use crate::ui::icons::Icon;
use crate::ui::{details, theme, titlebar, widgets};

/// Fixed window width; the height follows the open tab.
pub const WIDTH: f32 = 820.0;
/// Space around the tab content.
const MARGIN: f32 = 12.0;
/// The Image tab makes the window this tall to show the image.
const IMAGE_HEIGHT: f32 = 640.0;
/// How long a message replaces the title.
const TOAST_TIME: Duration = Duration::from_millis(1800);
/// Settings are written at most this often while something keeps changing.
const SAVE_DELAY: Duration = Duration::from_millis(800);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Settings,
    Screen,
    Image,
    Editor,
    History,
    Palette,
}

impl Tab {
    pub const ALL: [Self; 6] = [
        Self::Settings,
        Self::Screen,
        Self::Image,
        Self::Editor,
        Self::History,
        Self::Palette,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Settings => "Settings",
            Self::Screen => "Pick from screen",
            Self::Image => "Pick from image",
            Self::Editor => "Color editor",
            Self::History => "History",
            Self::Palette => "Palette",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Self::Settings => Icon::Settings,
            Self::Screen => Icon::Crosshair,
            Self::Image => Icon::Image,
            Self::Editor => Icon::Sliders,
            Self::History => Icon::History,
            Self::Palette => Icon::Palette,
        }
    }
}

/// Window height for the Screen tab, used before the first frame measures
/// the real content.
pub fn initial_height(settings: &Settings) -> f32 {
    titlebar::HEIGHT + MARGIN + details::height(settings.formats.len(), true) + MARGIN
}

struct Toast {
    text: String,
    until: Instant,
}

pub struct ColorPickerApp {
    tab: Tab,
    color: Rgb,
    settings: Settings,
    history: History,
    palette: Palette,
    window_pos: Option<[i32; 2]>,
    window: Option<MainWindow>,
    /// Windows 10 has no rounded corners or border; draw a border there.
    draw_border: bool,
    toast: Option<Toast>,
    /// Unsaved changes, and when they were made.
    dirty_since: Option<Instant>,
    /// The height we asked the window to take, until it has it.
    requested_height: Option<f32>,
    screen: ScreenPick,
    palette_tab: palette::ui::PaletteTab,
}

impl ColorPickerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, saved: Saved) -> Self {
        theme::apply(&cc.egui_ctx);
        let window = MainWindow::from_handle(cc);
        let rounded = window.is_some_and(MainWindow::round_corners);
        if let (Some(window), true, Some(pos)) =
            (window, saved.settings.remember_position, saved.window_pos)
        {
            window.move_to(pos);
        }

        Self {
            tab: Tab::Screen,
            color: saved.color,
            history: History::new(saved.history),
            palette: Palette::new(saved.palette),
            settings: saved.settings,
            window_pos: saved.window_pos,
            window,
            draw_border: !rounded,
            toast: None,
            dirty_since: None,
            requested_height: None,
            screen: ScreenPick::default(),
            palette_tab: palette::ui::PaletteTab::default(),
        }
    }

    fn saved(&self) -> Saved {
        Saved {
            settings: self.settings.clone(),
            color: self.color,
            history: self.history.colors().to_vec(),
            palette: self.palette.colors().to_vec(),
            window_pos: self.window_pos,
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty_since.get_or_insert_with(Instant::now);
    }

    /// Saves once changes have settled for [`SAVE_DELAY`], so dragging a
    /// slider doesn't write the file every frame.
    fn save_when_settled(&mut self, ctx: &egui::Context) {
        let Some(since) = self.dirty_since else {
            return;
        };
        let waited = since.elapsed();
        if waited >= SAVE_DELAY {
            self.save_now();
        } else {
            ctx.request_repaint_after(SAVE_DELAY - waited);
        }
    }

    fn save_now(&mut self) {
        self.dirty_since = None;
        // A failed save (read-only folder, full disk) must not stop the app.
        let _ = store::save(&self.saved());
    }

    fn toast(&mut self, text: impl Into<String>) {
        self.toast = Some(Toast {
            text: text.into(),
            until: Instant::now() + TOAST_TIME,
        });
    }

    /// The message to show instead of the title, if one is still current.
    fn toast_text(&mut self, ctx: &egui::Context) -> Option<String> {
        let toast = self.toast.as_ref()?;
        let left = toast.until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            self.toast = None;
            return None;
        }
        ctx.request_repaint_after(left);
        Some(toast.text.clone())
    }

    /// Resizes the window to `height` points if it isn't that tall yet.
    fn fit_height(&mut self, ctx: &egui::Context, height: f32) {
        let height = height.round();
        let current = ctx.input(|i| i.viewport().inner_rect.map(|r| r.height().round()));
        if current.is_some_and(|current| (current - height).abs() < 1.0) {
            if self.requested_height.take().is_some()
                && let Some(window) = self.window
            {
                window.keep_on_screen();
            }
        } else if self.requested_height != Some(height) {
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(WIDTH, height)));
            self.requested_height = Some(height);
        }
    }

    fn apply(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::SetColor(color) => {
                self.color = color;
                self.mark_dirty();
            }
            Action::Picked(color) => {
                self.color = color;
                self.history.push(color);
                self.mark_dirty();
                let hex = self.settings.hex.format(color);
                match self.settings.copy_on_pick {
                    Some(format) => {
                        ctx.copy_text(format.copy_text(color, self.settings.hex));
                        self.toast(format!("Picked {hex} · copied {}", format.label()));
                    }
                    None => self.toast(format!("Picked {hex}")),
                }
            }
            Action::Copy(text) => {
                ctx.copy_text(text.clone());
                self.toast(format!("Copied {text}"));
            }
            Action::StartScreenPick | Action::CancelScreenPick => {}
            Action::AddToPalette(color) => {
                let message = match self.palette.add(color) {
                    Added::Yes => {
                        self.mark_dirty();
                        "Saved to palette"
                    }
                    Added::AlreadyThere => "Already in the palette",
                    Added::Full => "The palette is full",
                };
                self.toast(message);
            }
            Action::RemoveFromPalette(index) => {
                self.palette.remove(index);
                self.mark_dirty();
            }
            Action::ClearPalette => {
                self.palette.clear();
                self.mark_dirty();
                self.toast("Palette cleared");
            }
            Action::RemoveFromHistory(index) => {
                self.history.remove(index);
                self.mark_dirty();
            }
            Action::ClearHistory => {
                self.history.clear();
                self.mark_dirty();
                self.toast("History cleared");
            }
            Action::SettingsChanged => {
                self.mark_dirty();
                let level = if self.settings.always_on_top {
                    WindowLevel::AlwaysOnTop
                } else {
                    WindowLevel::Normal
                };
                ctx.send_viewport_cmd(ViewportCommand::WindowLevel(level));
            }
            Action::Toast(text) => self.toast(text),
        }
    }
}

impl eframe::App for ColorPickerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let full = ui.max_rect();
        let painter = ui.painter().clone();
        painter.rect_filled(full, 0.0, theme::BG);
        let mut actions = Vec::new();

        let bar = Rect::from_min_size(full.min, vec2(full.width(), titlebar::HEIGHT));
        let toast = self.toast_text(&ctx);
        let clicked = titlebar::show(
            ui,
            bar,
            self.tab,
            self.settings.always_on_top,
            toast.as_deref(),
        );
        if let Some(tab) = clicked.tab {
            self.tab = tab;
        }
        if clicked.pin {
            self.settings.always_on_top = !self.settings.always_on_top;
            actions.push(Action::SettingsChanged);
        }
        if clicked.minimize {
            ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
        }
        if clicked.close {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }

        let top = bar.bottom() + MARGIN;
        let body_rect = Rect::from_min_max(
            pos2(full.left() + MARGIN, top),
            pos2(full.right() - MARGIN, full.bottom().max(top + 1.0)),
        );
        let mut body = ui.new_child(
            UiBuilder::new()
                .max_rect(body_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        body.spacing_mut().item_spacing = vec2(widgets::GAP, widgets::GAP);
        match self.tab {
            Tab::Screen => screen::show(
                &mut body,
                &self.screen,
                self.color,
                &self.settings,
                &mut actions,
            ),
            Tab::History => history::ui::show(
                &mut body,
                &self.history,
                self.color,
                &self.settings,
                &mut actions,
            ),
            Tab::Palette => palette::ui::show(
                &mut body,
                &mut self.palette_tab,
                &self.palette,
                self.color,
                &self.settings,
                &mut actions,
            ),
            Tab::Image | Tab::Editor | Tab::Settings => {
                let width = body.available_width();
                widgets::pill(&mut body, width, |p| {
                    let width = p.remaining();
                    p.value("Coming soon", theme::TEXT_DIM, width);
                });
            }
        }
        let bottom = body.min_rect().bottom();

        if self.draw_border {
            painter.rect_stroke(
                full,
                0.0,
                Stroke::new(1.0, theme::BORDER),
                StrokeKind::Inside,
            );
        }

        let height = match self.tab {
            Tab::Image => IMAGE_HEIGHT,
            _ => bottom - full.top() + MARGIN,
        };
        self.fit_height(&ctx, height);

        for action in actions {
            self.apply(&ctx, action);
        }
        self.save_when_settled(&ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.settings.remember_position
            && let Some(pos) = self.window.and_then(MainWindow::position)
        {
            self.window_pos = Some(pos);
        }
        self.save_now();
    }
}
