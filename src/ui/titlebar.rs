//! The custom title bar: tab icons on the left, the title in the middle,
//! pin, minimize and close on the right. Dragging empty space moves the
//! window. Short messages ("Copied …") replace the title for a moment.

use eframe::egui::{
    Align2, Color32, CursorIcon, Id, PointerButton, Rect, Response, Sense, Stroke, Ui, Vec2,
    ViewportCommand, pos2,
};

use super::icons::{self, Icon};
use super::theme;
use super::widgets::{self, ICON};
use crate::app::Tab;

pub const HEIGHT: f32 = 40.0;
const BUTTON: f32 = 32.0;
const GAP: f32 = 4.0;
const PAD: f32 = 8.0;
const CLOSE_HOVER: Color32 = Color32::from_rgb(196, 43, 28);

/// Which title-bar buttons were clicked this frame.
#[derive(Default)]
pub struct Clicked {
    pub tab: Option<Tab>,
    pub pin: bool,
    pub minimize: bool,
    pub close: bool,
}

pub fn show(ui: &mut Ui, rect: Rect, current: Tab, pinned: bool, toast: Option<&str>) -> Clicked {
    let mut clicked = Clicked::default();
    let painter = ui.painter().clone();
    painter.rect_filled(rect, 0.0, theme::SURFACE);
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, theme::BORDER),
    );

    // Added first, so the buttons on top of it win the clicks.
    let drag = ui.interact(rect, Id::unique("title-bar"), Sense::click_and_drag());
    if drag.drag_started_by(PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    let y = rect.center().y - BUTTON / 2.0;
    let mut x = rect.left() + PAD;
    for tab in Tab::ALL {
        let button = Rect::from_min_size(pos2(x, y), Vec2::splat(BUTTON));
        if icon_button(ui, button, tab.icon(), tab == current, tab.name(), false).clicked() {
            clicked.tab = Some(tab);
        }
        x += BUTTON + GAP;
    }
    let tabs_right = x;

    let mut x = rect.right() - PAD - BUTTON;
    let at = |x: f32| Rect::from_min_size(pos2(x, y), Vec2::splat(BUTTON));
    clicked.close = icon_button(ui, at(x), Icon::Close, false, "Close", true).clicked();
    x -= BUTTON + GAP;
    clicked.minimize = icon_button(ui, at(x), Icon::Minimize, false, "Minimize", false).clicked();
    x -= BUTTON + GAP;
    let pin_tip = if pinned {
        "Stop keeping on top"
    } else {
        "Keep on top"
    };
    clicked.pin = icon_button(ui, at(x), Icon::Pin, pinned, pin_tip, false).clicked();

    // The title, or a short message, centered in the space between the groups.
    let (text, font, color) = match toast {
        Some(toast) => (toast, theme::bold(13.5), theme::TEXT),
        None => ("Color Picker", theme::bold(15.5), theme::LABEL),
    };
    let space = (x - tabs_right - 16.0).max(40.0);
    let galley = widgets::fitted(ui, text, font, color, space);
    let center = pos2(rect.center().x, rect.center().y);
    painter.galley(
        Align2::CENTER_CENTER.anchor_size(center, galley.size()).min,
        galley,
        color,
    );
    clicked
}

fn icon_button(
    ui: &mut Ui,
    rect: Rect,
    icon: Icon,
    active: bool,
    tooltip: &str,
    close: bool,
) -> Response {
    let response = ui
        .interact(rect, Id::unique(("title-button", tooltip)), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip);
    let hovered = response.hovered();
    let fill = if close && hovered {
        Some(CLOSE_HOVER)
    } else if active {
        Some(theme::SURFACE_PRESS)
    } else if hovered {
        Some(theme::SURFACE_HI)
    } else {
        None
    };
    let painter = ui.painter();
    if let Some(fill) = fill {
        painter.rect_filled(rect, theme::SEGMENT_RADIUS, fill);
    }
    let color = if active || hovered {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(ICON));
    icons::paint(painter, icon_rect, icon, color);
    response
}
