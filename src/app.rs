//! The app: the window chrome (title bar, status bar), the tabs, and the
//! state they share. Tabs draw from read-only state and push [`Action`]s,
//! which are applied here after drawing. Events from the platform thread
//! (hotkey picks, tray clicks) are handled in `logic`, which also runs while
//! the window is hidden.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Color32, Layout, Rect, Stroke, StrokeKind, TextureHandle, TextureOptions,
    UiBuilder, Vec2, ViewportCommand, WindowLevel, pos2, vec2,
};

use crate::action::Action;
use crate::color::Rgb;
use crate::editor::{self, Editor};
use crate::history::{self, History};
use crate::image_pick::{self, ImageTab};
use crate::palette::{self, Added, Palette};
use crate::platform::{self, Command, Event, MainWindow, Service};
use crate::screen::{self, PickStyle, ScreenPick};
use crate::settings::{self, AccentMode, Settings, ThemeMode, TrayMode};
use crate::store::{self, Saved};
use crate::ui::icons::Icon;
use crate::ui::theme::{self, Look, p};
use crate::ui::{app_icon, details, titlebar, widgets};

/// Window width in points (the interface scale multiplies it); the height
/// follows the open tab.
pub const WIDTH: f32 = 820.0;
/// Space around the tab content.
const MARGIN: f32 = 12.0;
/// The Image tab makes the window this tall (without the status bar).
const IMAGE_HEIGHT: f32 = 640.0;
/// The Settings tab's height (without the status bar).
const SETTINGS_HEIGHT: f32 = titlebar::HEIGHT + 520.0;
/// How long a message shows.
const TOAST_TIME: Duration = Duration::from_millis(2200);
/// Settings are written at most this often while something keeps changing.
const SAVE_DELAY: Duration = Duration::from_millis(800);
/// The background image is kept at most this wide or tall.
const BACKGROUND_MAX: u32 = 2048;

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
            Self::Screen => Icon::Pipette,
            Self::Image => Icon::Image,
            Self::Editor => Icon::Sliders,
            Self::History => Icon::History,
            Self::Palette => Icon::Palette,
        }
    }

    /// The tab's accent: its own color, or the one the user chose for all.
    pub fn color(self) -> Color32 {
        if let Some(accent) = theme::single_accent() {
            return accent;
        }
        let p = p();
        match self {
            Self::Settings | Self::Screen => p.green,
            Self::Image => p.blue,
            Self::Editor => p.amber,
            Self::History => p.teal,
            Self::Palette => p.violet,
        }
    }
}

/// Window height for the Screen tab, used before the first frame measures
/// the real content.
pub fn initial_height(settings: &Settings) -> f32 {
    titlebar::HEIGHT
        + MARGIN
        + details::height(settings.formats.len(), 1)
        + MARGIN
        + status_height(settings)
}

fn status_height(settings: &Settings) -> f32 {
    if settings.status_bar {
        titlebar::STATUS_HEIGHT
    } else {
        0.0
    }
}

struct Toast {
    text: String,
    until: Instant,
}

/// The image behind every tab.
#[derive(Default)]
struct Background {
    /// The path the texture (or the load in progress) is for.
    path: Option<String>,
    texture: Option<(TextureHandle, Vec2)>,
    loading: Option<Receiver<Option<image::RgbaImage>>>,
}

pub struct ColorPickerApp {
    tab: Tab,
    color: Rgb,
    settings: Settings,
    /// The settings as the window and the platform thread last got them.
    applied: Settings,
    history: History,
    palette: Palette,
    window_pos: Option<[i32; 2]>,
    window: Option<MainWindow>,
    /// Windows 10 has no rounded corners or border; draw a border there.
    draw_border: bool,
    /// Whether the app starts with Windows (read from the system).
    autostart: bool,
    /// "Exit" was chosen in the tray: let the close through.
    quitting: bool,
    toast: Option<Toast>,
    /// Unsaved changes, and when they were made.
    dirty_since: Option<Instant>,
    /// The size we asked the window to take, until it has it.
    requested_size: Option<Vec2>,
    /// The palette in use is light (resolves the System theme).
    light: bool,
    background: Background,
    /// The Open dialog for a background image, while it is open.
    background_dialog: Option<Receiver<Option<PathBuf>>>,
    screen: ScreenPick,
    image: ImageTab,
    editor: Editor,
    palette_tab: palette::ui::PaletteTab,
    settings_tab: settings::ui::SettingsTab,
    /// The platform thread: hotkey, screen picking and tray. `None` where
    /// unsupported or if it failed to start.
    service: Option<Service>,
    /// Started hidden in the tray: where the window goes when first shown.
    restore_pos: Option<[i32; 2]>,
    /// Frames left in which to hide the window (eframe shows it after its
    /// first frame, whatever the viewport builder says).
    hide_frames: u8,
}

impl ColorPickerApp {
    /// `startup`: launched by Windows at sign-in (the window may be hidden).
    pub fn new(cc: &eframe::CreationContext<'_>, saved: Saved, startup: bool) -> Self {
        let ctx = &cc.egui_ctx;
        theme::install_fonts(ctx);
        let settings = saved.settings;
        let light = resolve_light(ctx, settings.theme);
        theme::set(ctx, look(&settings, light));
        ctx.set_zoom_factor(f32::from(settings.scale) / 100.0);

        let window = MainWindow::from_handle(cc);
        let rounded = window.is_some_and(MainWindow::round_corners);
        if let Some(window) = window {
            window.set_border_color(p().frame);
        }
        if let (Some(window), true, Some(pos)) =
            (window, settings.remember_position, saved.window_pos)
        {
            window.move_to(pos);
        }

        let service = Service::start(
            ctx.clone(),
            PickStyle::from_settings(&settings, light),
            settings.hotkey,
        );
        if let Some(service) = &service {
            service.send(Command::SetTrayIcon(settings.tray_icon));
            service.send(Command::SetTray(settings.tray != TrayMode::Off));
        }
        let (mut restore_pos, mut hide_frames) = (None, 0);
        if startup {
            if settings.tray == TrayMode::Off || service.is_none() {
                // No tray to come back from: start minimized instead.
                ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
            } else {
                if let Some(window) = window {
                    restore_pos = window.position();
                    window.move_offscreen();
                }
                hide_frames = 3;
                ctx.request_repaint();
            }
        }

        let autostart = platform::autostart_enabled();
        if autostart {
            // Repoints the entry at this .exe only if its target is gone.
            platform::repair_autostart();
        }

        let mut app = Self {
            tab: Tab::Screen,
            color: saved.color,
            applied: settings.clone(),
            settings,
            history: History::new(saved.history),
            palette: Palette::new(saved.palette),
            window_pos: saved.window_pos,
            window,
            draw_border: !rounded,
            autostart,
            quitting: false,
            toast: None,
            dirty_since: None,
            requested_size: None,
            light,
            background: Background::default(),
            background_dialog: None,
            screen: ScreenPick::default(),
            image: ImageTab::default(),
            editor: Editor::default(),
            palette_tab: palette::ui::PaletteTab::default(),
            settings_tab: settings::ui::SettingsTab::default(),
            service,
            restore_pos,
            hide_frames,
        };
        app.load_background(ctx);
        app
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

    fn remember_position(&mut self) {
        // Never shown since a hidden start: the window is parked off-screen.
        if self.restore_pos.is_some() {
            return;
        }
        if self.settings.remember_position
            && let Some(pos) = self.window.and_then(MainWindow::position)
        {
            self.window_pos = Some(pos);
        }
    }

    fn toast(&mut self, text: impl Into<String>) {
        self.toast = Some(Toast {
            text: text.into(),
            until: Instant::now() + TOAST_TIME,
        });
    }

    /// The current message, if one is still showing.
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

    /// Resizes the window to `size` points if it isn't that size yet.
    fn fit_size(&mut self, ctx: &egui::Context, size: Vec2) {
        let size = vec2(size.x.round(), size.y.round());
        let current = ctx.input(|i| i.viewport().inner_rect.map(|r| r.size()));
        let fits =
            current.is_some_and(|c| (c.x - size.x).abs() < 1.0 && (c.y - size.y).abs() < 1.0);
        if fits {
            if self.requested_size.take().is_some()
                && let Some(window) = self.window
            {
                window.keep_on_screen();
            }
        } else if self.requested_size != Some(size) {
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
            self.requested_size = Some(size);
        }
    }

    fn has_tray(&self) -> bool {
        self.service.is_some() && self.settings.tray != TrayMode::Off
    }

    fn hide_to_tray(&mut self, ctx: &egui::Context) {
        self.remember_position();
        self.save_now();
        ctx.send_viewport_cmd(ViewportCommand::Visible(false));
    }

    fn close_clicked(&mut self, ctx: &egui::Context) {
        if self.has_tray() && self.settings.tray == TrayMode::Close {
            self.hide_to_tray(ctx);
        } else {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    fn minimize_clicked(&mut self, ctx: &egui::Context) {
        if self.has_tray() && self.settings.tray == TrayMode::Minimize {
            self.hide_to_tray(ctx);
        } else {
            ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
        }
    }

    /// Brings the window back: visible, not minimized, in front.
    fn show_window(&mut self, ctx: &egui::Context) {
        self.hide_frames = 0;
        if let (Some(window), Some(pos)) = (self.window, self.restore_pos.take()) {
            window.move_to(pos);
        }
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
    }

    /// Re-applies the theme, accent and panel opacity (also when Windows
    /// switches between light and dark while the theme is System).
    fn apply_look(&mut self, ctx: &egui::Context) {
        self.light = resolve_light(ctx, self.settings.theme);
        theme::set(ctx, look(&self.settings, self.light));
        if let Some(window) = self.window {
            window.set_border_color(p().frame);
        }
    }

    /// Sends what changed in the settings to the window and the platform
    /// thread, and saves them.
    fn apply_settings(&mut self, ctx: &egui::Context) {
        let (new, old) = (self.settings.clone(), self.applied.clone());
        if new.always_on_top != old.always_on_top {
            let level = if new.always_on_top {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            };
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(level));
        }
        if (new.theme, new.accent, new.accent_color, new.panel_opacity)
            != (old.theme, old.accent, old.accent_color, old.panel_opacity)
        {
            self.apply_look(ctx);
        }
        if new.scale != old.scale {
            ctx.set_zoom_factor(f32::from(new.scale) / 100.0);
        }
        if new.background != old.background {
            self.load_background(ctx);
        }
        if new.tray_icon != old.tray_icon {
            ctx.send_viewport_cmd(ViewportCommand::Icon(Some(Arc::new(window_icon(
                new.tray_icon,
            )))));
        }
        if let Some(service) = &self.service {
            if new.hotkey != old.hotkey && !self.settings_tab.recording {
                service.send(Command::SetHotkey(new.hotkey));
            }
            let style = PickStyle::from_settings(&new, self.light);
            if style != PickStyle::from_settings(&old, self.light) {
                service.send(Command::SetStyle(style));
            }
            if new.tray_icon != old.tray_icon {
                service.send(Command::SetTrayIcon(new.tray_icon));
            }
            if (new.tray == TrayMode::Off) != (old.tray == TrayMode::Off) {
                service.send(Command::SetTray(new.tray != TrayMode::Off));
            }
        }
        self.applied = new;
        self.mark_dirty();
    }

    /// Starts decoding the background image (or drops it), off the UI thread.
    fn load_background(&mut self, ctx: &egui::Context) {
        let path = self.settings.background.clone();
        if path == self.background.path {
            return;
        }
        self.background = Background {
            path: path.clone(),
            ..Background::default()
        };
        let Some(path) = path else {
            return;
        };
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let image = image::open(&path).ok().map(|image| {
                let image = image.to_rgba8();
                let (w, h) = image.dimensions();
                if w.max(h) > BACKGROUND_MAX {
                    let k = BACKGROUND_MAX as f32 / w.max(h) as f32;
                    image::imageops::resize(
                        &image,
                        ((w as f32 * k) as u32).max(1),
                        ((h as f32 * k) as u32).max(1),
                        image::imageops::FilterType::Triangle,
                    )
                } else {
                    image
                }
            });
            let _ = tx.send(image);
            ctx.request_repaint();
        });
        self.background.loading = Some(rx);
    }

    fn poll_background(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.background.loading {
            match rx.try_recv() {
                Ok(Some(image)) => {
                    let size = vec2(image.width() as f32, image.height() as f32);
                    let color = egui::ColorImage::from_rgba_unmultiplied(
                        [image.width() as usize, image.height() as usize],
                        image.as_raw(),
                    );
                    let texture = ctx.load_texture("background", color, TextureOptions::LINEAR);
                    self.background.texture = Some((texture, size));
                    self.background.loading = None;
                }
                Ok(None) | Err(TryRecvError::Disconnected) => {
                    self.background.loading = None;
                    self.toast("Couldn't open the background image");
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(dialog) = &self.background_dialog {
            match dialog.try_recv() {
                Ok(Some(path)) => {
                    self.background_dialog = None;
                    self.settings.background = Some(path.display().to_string());
                    self.apply_settings(ctx);
                }
                Ok(None) | Err(TryRecvError::Disconnected) => self.background_dialog = None,
                Err(TryRecvError::Empty) => {}
            }
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
                if self.settings.copy_on_pick {
                    let format = self.settings.copy_format;
                    ctx.copy_text(format.copy_text(color, self.settings.hex));
                    self.toast(format!("Picked {hex} · copied {}", format.label()));
                } else {
                    self.toast(format!("Picked {hex}"));
                }
            }
            Action::Copy(text) => {
                ctx.copy_text(text.clone());
                self.toast(format!("Copied {text}"));
            }
            Action::StartScreenPick => match &self.service {
                Some(service) => service.send(Command::StartPick),
                None => self.toast("Picking from the screen isn't available here"),
            },
            Action::CancelScreenPick => {
                if let Some(service) = &self.service {
                    service.send(Command::CancelPick);
                }
            }
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
            Action::SettingsChanged => self.apply_settings(ctx),
            Action::HotkeyRecording(recording) => {
                // While recording, the hotkey must not fire, or pressing it
                // again would start a pick instead of reaching the field.
                let hotkey = if recording {
                    None
                } else {
                    self.settings.hotkey
                };
                if let Some(service) = &self.service {
                    service.send(Command::SetHotkey(hotkey));
                }
                self.applied.hotkey = hotkey;
                if recording {
                    self.screen.hotkey_error = None;
                }
            }
            Action::SetAutostart(on) => match platform::set_autostart(on) {
                Ok(()) => {
                    self.autostart = on;
                    self.toast(if on {
                        "Starts with Windows, in the tray"
                    } else {
                        "Won't start with Windows"
                    });
                }
                Err(error) => self.toast(error),
            },
            Action::OpenUrl(url) => platform::open_url(&url),
            Action::OpenSettingsFolder => {
                if let Some(dir) = store::path().as_deref().and_then(std::path::Path::parent) {
                    let _ = std::fs::create_dir_all(dir);
                    platform::open_url(&dir.display().to_string());
                }
            }
            Action::ResetAll => {
                let defaults = Saved::default();
                self.settings = defaults.settings;
                self.color = defaults.color;
                self.history = History::default();
                self.palette = Palette::default();
                self.window_pos = None;
                self.apply_settings(ctx);
                self.toast("Everything is back to defaults");
            }
            Action::ChooseBackground => {
                if self.background_dialog.is_none() {
                    self.background_dialog = Some(platform::open_image_dialog(ctx, self.window));
                }
            }
            Action::ClearBackground => {
                self.settings.background = None;
                self.apply_settings(ctx);
            }
            Action::Toast(text) => self.toast(text),
        }
    }

    fn handle_event(&mut self, ctx: &egui::Context, event: Event) {
        match event {
            Event::HotkeyStatus(error) => self.screen.hotkey_error = error,
            Event::PickStarted => self.screen.picking = true,
            Event::Picked(color) => {
                self.screen.picking = false;
                self.apply(ctx, Action::Picked(color));
            }
            Event::PickCancelled => self.screen.picking = false,
            Event::ZoomChanged(zoom) => {
                self.settings.zoom = zoom;
                self.applied.zoom = zoom;
                self.mark_dirty();
            }
            Event::ShowRequested => self.show_window(ctx),
            Event::ExitRequested => {
                self.quitting = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    /// Opens dropped image files and pasted images (or pasted paths to
    /// images) in the Image tab, from any tab.
    fn handle_drop_and_paste(&mut self, ctx: &egui::Context) {
        let (dropped, image, text) = ctx.input(|i| {
            let dropped = i.raw.dropped_files.first().map(|f| f.path().to_path_buf());
            let mut image = None;
            let mut text = None;
            for event in &i.events {
                match event {
                    egui::Event::PasteImage(pasted) => image = Some(pasted.clone()),
                    egui::Event::Paste(pasted) => text = Some(pasted.clone()),
                    _ => {}
                }
            }
            (dropped, image, text)
        });
        // Text pasted into a focused field belongs to that field.
        let text = text.filter(|_| ctx.memory(|m| m.focused().is_none()));

        if let Some(path) = dropped {
            self.tab = Tab::Image;
            self.image.open_path(ctx, path);
        } else if let Some(image) = image {
            self.tab = Tab::Image;
            self.image.paste_handled();
            self.image.open_pasted(ctx, &image);
        } else if let Some(path) = text.as_deref().and_then(image_pick::pasted_path) {
            self.tab = Tab::Image;
            self.image.paste_handled();
            self.image.open_path(ctx, path);
        } else if text.is_some() {
            if self.image.paste_timed_out() || self.tab == Tab::Image {
                self.image.paste_handled();
                self.toast("No image on the clipboard");
            }
        } else if self.image.paste_timed_out() {
            self.toast("No image on the clipboard");
        }
        if self.image.waiting_for_paste() {
            ctx.request_repaint();
        }
    }

    /// What the status bar says when there is no message.
    fn hint(&self) -> (Icon, String, Option<Color32>) {
        match self.tab {
            Tab::Screen if self.screen.picking => (
                Icon::Pipette,
                "Click to pick · Esc cancels · arrows nudge · wheel zooms".into(),
                None,
            ),
            Tab::Screen => match (&self.screen.hotkey_error, self.settings.hotkey) {
                (Some(_), Some(hotkey)) => (
                    Icon::Keyboard,
                    format!(
                        "{} is used by another app. Change it in Settings → Picking.",
                        hotkey.label()
                    ),
                    Some(p().danger),
                ),
                (_, Some(hotkey)) => (
                    Icon::Keyboard,
                    format!("Press {} anywhere to pick a color", hotkey.label()),
                    None,
                ),
                (_, None) => (
                    Icon::Keyboard,
                    "Set a pick hotkey in Settings → Picking".into(),
                    None,
                ),
            },
            Tab::Image => (
                Icon::Image,
                "Scroll to zoom · drag to pan · click to pick".into(),
                None,
            ),
            Tab::Editor => (
                Icon::Sliders,
                "Scroll over a slider to change it by one".into(),
                None,
            ),
            Tab::History => (
                Icon::History,
                "Right-click a color to copy, save or remove it".into(),
                None,
            ),
            Tab::Palette => (
                Icon::Palette,
                "Right-click a color to copy or remove it".into(),
                None,
            ),
            Tab::Settings => (
                Icon::Info,
                "Settings are saved as you change them".into(),
                None,
            ),
        }
    }

    fn body(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        match self.tab {
            Tab::Screen => screen::show(ui, &self.screen, self.color, &self.settings, actions),
            Tab::Image => image_pick::ui::show(
                ui,
                &mut self.image,
                self.color,
                &self.settings,
                self.window,
                actions,
            ),
            Tab::Editor => {
                editor::ui::show(ui, &mut self.editor, self.color, &self.settings, actions);
            }
            Tab::History => {
                history::ui::show(ui, &self.history, self.color, &self.settings, actions);
            }
            Tab::Palette => palette::ui::show(
                ui,
                &mut self.palette_tab,
                &self.palette,
                self.color,
                &self.settings,
                actions,
            ),
            Tab::Settings => {}
        }
    }
}

/// Whether the palette should be light for `mode`.
fn resolve_light(ctx: &egui::Context, mode: ThemeMode) -> bool {
    match mode {
        ThemeMode::Dark => false,
        ThemeMode::Light => true,
        ThemeMode::System => ctx.system_theme() == Some(egui::Theme::Light),
    }
}

fn look(settings: &Settings, light: bool) -> Look {
    Look {
        light,
        accent: (settings.accent == AccentMode::Single).then(|| settings.accent_color.into()),
        panel_opacity: f32::from(settings.panel_opacity) / 100.0,
    }
}

/// The taskbar icon for a style.
pub fn window_icon(style: settings::IconStyle) -> egui::IconData {
    egui::IconData {
        rgba: app_icon::pixels(64, style),
        width: 64,
        height: 64,
    }
}

impl eframe::App for ColorPickerApp {
    /// Runs before every frame and also while the window is hidden, so
    /// hotkey picks and tray clicks work without the window.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.hide_frames > 0 {
            self.hide_frames -= 1;
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
            ctx.request_repaint();
        }
        while let Some(event) = self.service.as_ref().and_then(Service::poll) {
            self.handle_event(ctx, event);
        }
        self.image.poll(ctx);
        self.poll_background(ctx);
        if self.settings.theme == ThemeMode::System
            && resolve_light(ctx, ThemeMode::System) != self.light
        {
            self.apply_look(ctx);
            if let Some(service) = &self.service {
                service.send(Command::SetStyle(PickStyle::from_settings(
                    &self.settings,
                    self.light,
                )));
            }
        }

        // Alt+F4 or "Close window" on the taskbar: same as the X button.
        if ctx.input(|i| i.viewport().close_requested())
            && !self.quitting
            && self.has_tray()
            && self.settings.tray == TrayMode::Close
        {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.hide_to_tray(ctx);
        }
        self.save_when_settled(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let full = ui.max_rect();
        let painter = ui.painter().clone();
        let palette = p();
        painter.rect_filled(full, 0.0, palette.base);
        if let Some((texture, size)) = &self.background.texture {
            let uv = settings::ui::cover_uv(*size, full.size());
            let alpha = f32::from(self.settings.background_opacity) / 100.0;
            painter.image(texture.id(), full, uv, Color32::WHITE.gamma_multiply(alpha));
        }
        let mut actions = Vec::new();
        let status_h = status_height(&self.settings);
        let toast = self.toast_text(&ctx);

        let bar = Rect::from_min_size(full.min, vec2(full.width(), titlebar::HEIGHT));
        let title = titlebar::Title {
            tab: self.tab,
            pinned: self.settings.always_on_top,
            line: (self.tab != Tab::Settings).then(|| self.tab.color()),
            message: toast.as_deref().filter(|_| status_h == 0.0),
            fancy: self.settings.fancy_title,
        };
        let clicked = titlebar::show(ui, bar, &title);
        if let Some(tab) = clicked.tab {
            self.tab = tab;
        }
        if self.tab != Tab::Settings && self.settings_tab.recording {
            self.settings_tab.recording = false;
            actions.push(Action::HotkeyRecording(false));
        }
        if clicked.pin {
            self.settings.always_on_top = !self.settings.always_on_top;
            actions.push(Action::SettingsChanged);
        }
        if clicked.minimize {
            self.minimize_clicked(&ctx);
        }
        if clicked.close {
            self.close_clicked(&ctx);
        }

        self.handle_drop_and_paste(&ctx);

        let bottom_limit = (full.bottom() - status_h).max(bar.bottom() + 1.0);
        let height = if self.tab == Tab::Settings {
            let rect = Rect::from_min_max(
                pos2(full.left(), bar.bottom()),
                pos2(full.right(), bottom_limit),
            );
            let background = self
                .background
                .texture
                .as_ref()
                .map(|(texture, size)| (texture.id(), *size));
            let status = settings::ui::Status {
                hotkey_error: self.screen.hotkey_error.as_deref(),
                autostart: self.autostart,
                color: self.color,
                settings_path: store::path().map(|p| p.display().to_string()),
                background,
            };
            settings::ui::show(
                ui,
                rect,
                &mut self.settings_tab,
                &mut self.settings,
                &status,
                &mut actions,
            );
            SETTINGS_HEIGHT
        } else {
            let top = bar.bottom() + MARGIN;
            let body_rect = Rect::from_min_max(
                pos2(full.left() + MARGIN, top),
                pos2(
                    full.right() - MARGIN,
                    (bottom_limit - MARGIN).max(top + 1.0),
                ),
            );
            let mut body = ui.new_child(
                UiBuilder::new()
                    .max_rect(body_rect)
                    .layout(Layout::top_down(Align::Min)),
            );
            body.spacing_mut().item_spacing = vec2(widgets::GAP, widgets::GAP);
            self.body(&mut body, &mut actions);
            match self.tab {
                Tab::Image => IMAGE_HEIGHT,
                _ => body.min_rect().bottom() - full.top() + MARGIN,
            }
        };

        if status_h > 0.0 {
            let rect = Rect::from_min_max(pos2(full.left(), full.bottom() - status_h), full.max);
            let accent = if self.tab == Tab::Settings {
                palette.text_dim
            } else {
                self.tab.color()
            };
            match &toast {
                Some(text) => titlebar::status_bar(
                    ui,
                    rect,
                    Icon::CircleCheck,
                    accent,
                    text,
                    palette.text_muted,
                ),
                None => {
                    let (icon, text, color) = self.hint();
                    let icon_color = color.unwrap_or(accent);
                    titlebar::status_bar(
                        ui,
                        rect,
                        icon,
                        icon_color,
                        &text,
                        color.unwrap_or(palette.text_muted),
                    );
                }
            }
        }

        // Dragging files over the window: say where they will go.
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            painter.rect_filled(full, 0.0, palette.base.gamma_multiply(0.88));
            painter.text(
                full.center(),
                egui::Align2::CENTER_CENTER,
                "Drop to open in the Image tab",
                theme::bold(16.0),
                palette.text,
            );
        }
        if self.draw_border {
            painter.rect_stroke(
                full,
                0.0,
                Stroke::new(1.0, palette.border),
                StrokeKind::Inside,
            );
        }

        self.fit_size(&ctx, vec2(WIDTH, height + status_h));

        for action in actions {
            self.apply(&ctx, action);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.remember_position();
        self.save_now();
    }
}
