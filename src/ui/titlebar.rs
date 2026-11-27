//! The window chrome, after BlurAutoClicker: a title bar with the settings
//! gear and tab icons on the left (the active one tinted in its tab's
//! color), the title in the middle, pin / minimize / close on the right and
//! a 2px accent line under it; and a status bar at the bottom.

use std::f32::consts::FRAC_PI_2;

use eframe::egui::{
    Align2, Color32, CursorIcon, Id, PointerButton, Rect, Response, Sense, Stroke, Ui, Vec2,
    ViewportCommand, pos2,
};

use super::icons::{self, Icon};
use super::theme::{self, p};
use super::widgets;
use crate::app::Tab;

pub const HEIGHT: f32 = 40.0;
pub const STATUS_HEIGHT: f32 = 28.0;
const BUTTON: f32 = 32.0;
const GAP: f32 = 4.0;
const PAD: f32 = 8.0;

/// Which title-bar buttons were clicked this frame.
#[derive(Default)]
pub struct Clicked {
    pub tab: Option<Tab>,
    pub pin: bool,
    pub minimize: bool,
    pub close: bool,
}

/// What the title bar shows.
pub struct Title<'a> {
    pub tab: Tab,
    pub pinned: bool,
    /// The accent line's color; `None` hides it (the Settings tab frames its
    /// content instead).
    pub line: Option<Color32>,
    /// Shown instead of the title (messages, when the status bar is off).
    pub message: Option<&'a str>,
    pub fancy: bool,
}

pub fn show(ui: &mut Ui, rect: Rect, title: &Title<'_>) -> Clicked {
    let p = p();
    let mut clicked = Clicked::default();
    let painter = ui.painter().clone();
    painter.rect_filled(rect, 0.0, p.surface);
    if let Some(line) = title.line {
        painter.rect_filled(
            Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.max),
            0.0,
            line,
        );
    }

    // Added first, so the buttons on top of it win the clicks.
    let drag = ui.interact(rect, Id::unique("title-bar"), Sense::click_and_drag());
    if drag.drag_started_by(PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    let y = rect.top() + (HEIGHT - 2.0 - BUTTON) / 2.0;
    let at = |x: f32| Rect::from_min_size(pos2(x, y), Vec2::splat(BUTTON));

    // The gear, a little apart from the tabs; it turns when hovered.
    let mut x = rect.left() + PAD;
    let gear = button(ui, at(x), "Settings");
    let turn = ui
        .ctx()
        .animate_bool_with_time(gear.id.with("turn"), gear.hovered(), 0.5);
    let gear_active = title.tab == Tab::Settings;
    paint_button_bg(ui, at(x), &gear, gear_active.then_some(p.elevated));
    icons::paint_ex(
        ui.painter(),
        Rect::from_center_size(at(x).center(), Vec2::splat(17.0)),
        Icon::Settings,
        if gear_active || gear.hovered() {
            p.text
        } else {
            p.text_dim
        },
        2.0,
        turn * FRAC_PI_2,
    );
    if gear.clicked() {
        clicked.tab = Some(Tab::Settings);
    }
    x += BUTTON + GAP * 2.0;

    for tab in Tab::ALL.into_iter().filter(|&t| t != Tab::Settings) {
        let r = at(x);
        let response = button(ui, r, tab.name());
        let active = tab == title.tab;
        let color = tab.color();
        paint_button_bg(ui, r, &response, active.then(|| color.gamma_multiply(0.18)));
        let icon_color = if active {
            color
        } else if response.hovered() {
            p.text
        } else {
            p.text_dim
        };
        icons::paint_ex(
            ui.painter(),
            Rect::from_center_size(r.center(), Vec2::splat(18.0)),
            tab.icon(),
            icon_color,
            if active { 2.2 } else { 2.0 },
            0.0,
        );
        if response.clicked() {
            clicked.tab = Some(tab);
        }
        x += BUTTON + GAP;
    }
    let tabs_right = x;

    // Window buttons, right to left.
    let mut x = rect.right() - PAD - BUTTON;
    let close = button(ui, at(x), "Close");
    paint_button_bg(ui, at(x), &close, close.hovered().then_some(p.danger_bg));
    icons::paint_close(
        ui.painter(),
        at(x),
        if close.hovered() {
            p.danger
        } else {
            p.text_dim
        },
    );
    clicked.close = close.clicked();

    x -= BUTTON + GAP;
    let minimize = button(ui, at(x), "Minimize");
    paint_button_bg(ui, at(x), &minimize, None);
    icons::paint_minimize(
        ui.painter(),
        at(x),
        if minimize.hovered() {
            p.text
        } else {
            p.text_dim
        },
    );
    clicked.minimize = minimize.clicked();

    x -= BUTTON + GAP;
    let pin_tip = if title.pinned {
        "Stop keeping on top"
    } else {
        "Keep on top"
    };
    let pin = button(ui, at(x), pin_tip);
    paint_button_bg(ui, at(x), &pin, None);
    let needle = ui
        .ctx()
        .animate_bool_with_time(pin.id.with("needle"), title.pinned, 0.3);
    let pin_color = if title.pinned {
        title.tab.color()
    } else if pin.hovered() {
        p.text
    } else {
        p.text_dim
    };
    icons::paint_pin(
        ui.painter(),
        Rect::from_center_size(at(x).center(), Vec2::splat(16.0)),
        pin_color,
        needle,
    );
    clicked.pin = pin.clicked();

    // The title, or a message, centered between the button groups.
    let (text, font, color) = match title.message {
        Some(message) => (message, theme::bold(14.0), p.text),
        None if title.fancy => ("Color Picker", theme::fancy(19.0), p.text_muted),
        None => ("Color Picker", theme::bold(16.0), p.text_muted),
    };
    let space = (x - tabs_right - 16.0).max(40.0);
    let galley = widgets::fitted(ui, text, font, color, space);
    let center = pos2(rect.center().x, rect.top() + (HEIGHT - 2.0) / 2.0);
    painter.galley(
        Align2::CENTER_CENTER.anchor_size(center, galley.size()).min,
        galley,
        color,
    );
    clicked
}

fn button(ui: &mut Ui, rect: Rect, tooltip: &str) -> Response {
    ui.interact(rect, Id::unique(("title-button", tooltip)), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

/// The rounded square behind a title-bar button: `fill` if given, else the
/// hover color while hovered.
fn paint_button_bg(ui: &Ui, rect: Rect, response: &Response, fill: Option<Color32>) {
    let fill = fill.or_else(|| response.hovered().then(|| p().elevated));
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, theme::SEGMENT_RADIUS, fill);
    }
}

/// The status bar: an icon and a message on the left, the version on the
/// right.
pub fn status_bar(
    ui: &Ui,
    rect: Rect,
    icon: Icon,
    icon_color: Color32,
    text: &str,
    color: Color32,
) {
    let p = p();
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, p.surface);
    painter.hline(
        rect.x_range(),
        rect.top() + 0.5,
        Stroke::new(1.0, p.border_subtle),
    );
    let icon_rect = Rect::from_center_size(
        pos2(rect.left() + 12.0 + 7.5, rect.center().y),
        Vec2::splat(15.0),
    );
    icons::paint(painter, icon_rect, icon, icon_color);
    let version = concat!("v", env!("CARGO_PKG_VERSION"));
    let version_galley =
        painter.layout_no_wrap(version.to_owned(), theme::medium(12.5), p.text_dim);
    let version_w = version_galley.size().x;
    painter.galley(
        pos2(
            rect.right() - 12.0 - version_w,
            rect.center().y - version_galley.size().y / 2.0,
        ),
        version_galley,
        p.text_dim,
    );
    let max = rect.width() - 12.0 - 15.0 - 8.0 - version_w - 24.0;
    let galley = widgets::fitted(ui, text, theme::medium(12.5), color, max);
    painter.galley(
        pos2(
            icon_rect.right() + 8.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        color,
    );
}
