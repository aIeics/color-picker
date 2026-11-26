//! The Image tab: a toolbar, the image canvas and the current color.
//! Scroll zooms, drag pans, click picks.

use eframe::egui::{
    Align2, Color32, CursorIcon, PointerButton, Rect, Sense, Stroke, StrokeKind, Ui, Vec2,
    ViewportCommand, pos2, vec2,
};

use super::view::View;
use super::{ImageTab, checker_texture, load};
use crate::action::Action;
use crate::color::{HexStyle, Rgb};
use crate::platform::MainWindow;
use crate::settings::Settings;
use crate::ui::icons::{self, Icon};
use crate::ui::{details, theme, widgets};

pub fn show(
    ui: &mut Ui,
    tab: &mut ImageTab,
    color: Rgb,
    settings: &Settings,
    window: Option<MainWindow>,
    actions: &mut Vec<Action>,
) {
    let ctx = ui.ctx().clone();
    tab.poll(&ctx);
    toolbar(ui, tab, settings.hex, window);

    let details_h = details::height(settings.formats.len(), 0);
    let width = ui.available_width();
    let height = (ui.available_height() - details_h - widgets::GAP).max(120.0);
    canvas(ui, tab, vec2(width, height), settings, window, actions);

    details::show(ui, color, settings, actions, None);
}

fn toolbar(ui: &mut Ui, tab: &mut ImageTab, hex: HexStyle, window: Option<MainWindow>) {
    let ctx = ui.ctx().clone();
    let width = ui.available_width();
    let open_w = widgets::button_width(ui, true, "Open");
    let paste_w = widgets::button_width(ui, true, "Paste");
    let files_w = open_w + 9.0 + paste_w + 8.0;
    let fit_w = widgets::button_width(ui, true, "Fit");
    let zoom_w = widgets::SEG_H * 2.0 + 64.0 + 9.0 + fit_w + 8.0;
    let info_w = width - files_w - zoom_w - 2.0 * widgets::GAP;

    ui.horizontal(|ui| {
        widgets::pill(ui, files_w, |p| {
            if p.button(Some(Icon::Folder), "Open", open_w)
                .on_hover_text("Open an image file")
                .clicked()
            {
                tab.browse(&ctx, window);
            }
            p.divider();
            if p.button(Some(Icon::Paste), "Paste", paste_w)
                .on_hover_text("Paste an image (Ctrl+V)")
                .clicked()
            {
                ctx.send_viewport_cmd(ViewportCommand::RequestPaste);
                ctx.request_repaint();
                tab.expect_paste();
            }
        });

        widgets::pill(ui, info_w, |p| {
            let (swatch, text) = info_text(tab, hex);
            p.space(6.0);
            if let Some(swatch) = swatch {
                let (rect, _) = p.ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                widgets::paint_swatch(p.ui.painter(), rect, swatch, 4.0);
            }
            let width = p.remaining();
            let color = if tab.error.is_some() {
                theme::DANGER
            } else {
                theme::LABEL
            };
            p.value(&text, color, width);
        });

        widgets::pill(ui, zoom_w, |p| {
            let has_image = tab.shown.is_some();
            p.ui.add_enabled_ui(has_image, |ui| {
                let mut p = widgets::Pill { ui };
                let center = tab.canvas / 2.0;
                if p.icon_button(Icon::Minus, "Zoom out").clicked() {
                    tab.view = tab.view.zoom_at(center, 0.8);
                }
                let percent = tab.view.zoom * p.ui.ctx().pixels_per_point() * 100.0;
                let text = if !has_image {
                    "–".to_owned()
                } else if percent < 10.0 {
                    format!("{percent:.1}%")
                } else {
                    format!("{percent:.0}%")
                };
                p.value(&text, theme::TEXT, 64.0);
                if p.icon_button(Icon::Plus, "Zoom in").clicked() {
                    tab.view = tab.view.zoom_at(center, 1.25);
                }
                p.divider();
                if p.button(Some(Icon::Fit), "Fit", fit_w)
                    .on_hover_text("Show the whole image")
                    .clicked()
                {
                    tab.needs_fit = true;
                }
            });
        });
    });
}

/// What the middle toolbar pill says: the hovered pixel, the image, or why
/// there is none.
fn info_text(tab: &ImageTab, hex: HexStyle) -> (Option<Rgb>, String) {
    if let Some((name, _)) = &tab.loading {
        return (None, format!("Opening {name}…"));
    }
    if let Some(error) = &tab.error {
        return (None, error.clone());
    }
    match (&tab.shown, tab.hover) {
        (Some(_), Some(([x, y], color))) => {
            (Some(color), format!("{}   x {x}, y {y}", hex.format(color)))
        }
        (Some(shown), None) => (
            None,
            format!(
                "{} · {} × {}",
                shown.name,
                shown.pixels.width(),
                shown.pixels.height()
            ),
        ),
        (None, _) => (None, "No image yet".to_owned()),
    }
}

fn canvas(
    ui: &mut Ui,
    tab: &mut ImageTab,
    size: Vec2,
    settings: &Settings,
    window: Option<MainWindow>,
    actions: &mut Vec<Action>,
) {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    widgets::paint_pill(ui.painter(), rect);
    let inner = rect.shrink(1.0);
    tab.canvas = inner.size();
    let painter = ui.painter_at(inner);

    let Some(shown) = &tab.shown else {
        tab.hover = None;
        let (line1, line2) = if tab.loading.is_some() {
            ("Opening…", "")
        } else {
            (
                "Open, paste (Ctrl+V) or drop an image here",
                "PNG, JPEG, BMP, GIF or WebP",
            )
        };
        let icon = Rect::from_center_size(inner.center() - vec2(0.0, 34.0), Vec2::splat(28.0));
        icons::paint(&painter, icon, Icon::Image, theme::TEXT_FAINT);
        painter.text(
            inner.center(),
            Align2::CENTER_CENTER,
            line1,
            theme::bold(15.0),
            theme::LABEL,
        );
        painter.text(
            inner.center() + vec2(0.0, 24.0),
            Align2::CENTER_CENTER,
            line2,
            theme::regular(12.5),
            theme::TEXT_DIM,
        );
        if response.clicked() && tab.loading.is_none() {
            let ctx = ui.ctx().clone();
            tab.browse(&ctx, window);
        }
        return;
    };

    let image_size = vec2(shown.pixels.width() as f32, shown.pixels.height() as f32);
    let pixel_count = [shown.pixels.width(), shown.pixels.height()];
    if tab.needs_fit {
        tab.view = View::fit(image_size, inner.size());
        tab.needs_fit = false;
    }

    // Zoom around the pointer with the wheel (or Ctrl+wheel, or a pinch).
    if let Some(pointer) = response.hover_pos() {
        let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        let factor = (scroll * 0.004).exp() * pinch;
        if (factor - 1.0).abs() > 1e-4 {
            tab.view = tab.view.zoom_at(pointer - inner.min, factor);
        }
    }
    if response.dragged_by(PointerButton::Primary) || response.dragged_by(PointerButton::Middle) {
        tab.view.offset += response.drag_delta();
    }
    tab.view = tab.view.clamp_to(image_size, inner.size(), 24.0);
    let view = tab.view;

    let image_rect = Rect::from_min_size(inner.min + view.offset, image_size * view.zoom);
    let checker = tab
        .checker
        .get_or_insert_with(|| checker_texture(ui.ctx()))
        .id();
    let visible = image_rect.intersect(inner);
    if visible.is_positive() {
        let uv = Rect::from_min_max(
            pos2(visible.min.x / 16.0, visible.min.y / 16.0),
            pos2(visible.max.x / 16.0, visible.max.y / 16.0),
        );
        painter.image(checker, visible, uv, Color32::WHITE);
    }
    painter.image(
        shown.texture.id(),
        image_rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );

    // The hovered pixel: its color (as a click would pick it) and, when
    // zoomed in, an outline around the sampled square.
    let side = settings.sample.side();
    let hovered = response
        .hover_pos()
        .and_then(|pos| view.pixel_at(pos - inner.min, pixel_count));
    let hover = hovered.and_then(|px| load::sample(&shown.pixels, px, side).map(|c| (px, c)));
    if hover != tab.hover {
        tab.hover = hover;
        ui.ctx().request_repaint();
    }
    if let Some(([x, y], _)) = hover {
        let half = (side / 2) as f32;
        let min = inner.min + view.to_canvas(vec2(x as f32 - half, y as f32 - half));
        let square = Rect::from_min_size(min, Vec2::splat(view.zoom * side as f32));
        if square.width() >= 6.0 {
            painter.rect_stroke(
                square,
                0.0,
                Stroke::new(1.0, Color32::WHITE),
                StrokeKind::Inside,
            );
            painter.rect_stroke(
                square,
                0.0,
                Stroke::new(1.0, Color32::BLACK),
                StrokeKind::Outside,
            );
        }
        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
    }
    if response.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    }
    if response.clicked()
        && let Some((_, color)) = hover
    {
        actions.push(Action::Picked(color));
    }
}
