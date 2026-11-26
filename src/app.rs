//! Application state and user interface.

use std::time::{Duration, Instant};

use eframe::egui;

use crate::color::Rgb;
use crate::platform;
use crate::theme::{self, Kind};

/// How long the user has to move the cursor over the target color after
/// pressing "Pick from screen".
const PICK_DELAY: Duration = Duration::from_secs(2);

const INITIAL_COLOR: Rgb = Rgb::new(0x3B, 0x82, 0xF6);

pub struct ColorPickerApp {
    color: Rgb,
    hex_input: String,
    status: String,
    pending_pick: Option<Instant>,
}

impl ColorPickerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        Self {
            color: INITIAL_COLOR,
            hex_input: INITIAL_COLOR.to_hex(),
            status: "Ready.".to_owned(),
            pending_pick: None,
        }
    }

    fn set_color(&mut self, color: Rgb) {
        self.color = color;
        self.hex_input = color.to_hex();
    }

    fn start_pick(&mut self) {
        self.pending_pick = Some(Instant::now() + PICK_DELAY);
        self.status = "Move the cursor over the color to pick.".to_owned();
    }

    fn cancel_pick(&mut self) {
        self.pending_pick = None;
        self.status = "Pick cancelled.".to_owned();
    }

    /// Finishes a pending screen pick once its delay has elapsed. Requests a
    /// repaint at the deadline so the app stays idle while waiting.
    fn poll_pick(&mut self, ctx: &egui::Context) {
        let Some(deadline) = self.pending_pick else {
            return;
        };

        let now = Instant::now();
        if now < deadline {
            ctx.request_repaint_after(deadline - now);
            return;
        }

        self.pending_pick = None;
        match platform::color_at_cursor() {
            Some(color) => {
                self.set_color(color);
                self.status = format!("Picked {}.", color.to_hex());
            }
            None => self.status = "Could not read the color under the cursor.".to_owned(),
        }
    }

    fn swatch(&self, ui: &mut egui::Ui) {
        theme::section_label(ui, "Preview");
        theme::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 100.0),
                egui::Sense::hover(),
            );
            ui.painter().rect_filled(rect, 12.0, self.color);

            let text_color = if self.color.luminance() > 0.5 {
                egui::Color32::BLACK
            } else {
                egui::Color32::WHITE
            };
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                self.color.to_hex(),
                theme::bold(24.0),
                text_color,
            );
        });
    }

    fn pick_controls(&mut self, ui: &mut egui::Ui) {
        theme::section_label(ui, "Pick");
        theme::card()
            .inner_margin(egui::Margin::symmetric(14, 12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                if !platform::SCREEN_PICKING_SUPPORTED {
                    ui.label("Picking from the screen is only available on Windows.");
                    return;
                }

                let picking = self.pending_pick.is_some();
                ui.horizontal(|ui| {
                    let pick = ui
                        .scope(|ui| {
                            if picking {
                                ui.disable();
                            }
                            theme::button_sized(ui, "Pick from screen", Kind::Primary, 148.0)
                        })
                        .inner;
                    if !picking && pick.clicked() {
                        self.start_pick();
                    }
                    if picking && theme::button_sized(ui, "Cancel", Kind::Ghost, 84.0).clicked() {
                        self.cancel_pick();
                    }
                });

                if picking {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Move the cursor over the color to pick.")
                            .size(12.0)
                            .color(theme::TEXT_DIM),
                    );
                }
            });
    }

    fn formats(&mut self, ui: &mut egui::Ui) {
        theme::section_label(ui, "Formats");
        theme::card()
            .inner_margin(egui::Margin::symmetric(14, 12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                let label_width = 44.0;
                let button_width = 64.0;
                let gap = ui.spacing().item_spacing.x;
                let value_width =
                    (ui.available_width() - label_width - button_width - 2.0 * gap).max(80.0);

                let rgb = self.color.to_rgb_string();
                let hsl = self.color.to_hsl_string();

                ui.horizontal(|ui| {
                    ui.add_sized(
                        [label_width, 30.0],
                        egui::Label::new(egui::RichText::new("HEX").color(theme::TEXT_DIM))
                            .truncate(),
                    );
                    let response =
                        theme::text_field(ui, &mut self.hex_input, "#RRGGBB", value_width);
                    if response.changed() {
                        match Rgb::from_hex(&self.hex_input) {
                            Some(color) => {
                                self.color = color;
                                self.status = "Color updated.".to_owned();
                            }
                            None => {
                                self.status = "Type a hex color such as #3B82F6.".to_owned();
                            }
                        }
                    }
                    if theme::button_sized(ui, "Copy", Kind::Ghost, button_width).clicked() {
                        let hex = self.color.to_hex();
                        ui.ctx().copy_text(hex.clone());
                        self.status = format!("Copied {hex}.");
                    }
                });
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [label_width, 30.0],
                        egui::Label::new(egui::RichText::new("RGB").color(theme::TEXT_DIM))
                            .truncate(),
                    );
                    ui.add_sized(
                        [value_width, 30.0],
                        egui::Label::new(egui::RichText::new(&rgb).color(theme::TEXT)).truncate(),
                    );
                    if theme::button_sized(ui, "Copy", Kind::Ghost, button_width).clicked() {
                        ui.ctx().copy_text(rgb.clone());
                        self.status = format!("Copied {rgb}.");
                    }
                });
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [label_width, 30.0],
                        egui::Label::new(egui::RichText::new("HSL").color(theme::TEXT_DIM))
                            .truncate(),
                    );
                    ui.add_sized(
                        [value_width, 30.0],
                        egui::Label::new(egui::RichText::new(&hsl).color(theme::TEXT)).truncate(),
                    );
                    if theme::button_sized(ui, "Copy", Kind::Ghost, button_width).clicked() {
                        ui.ctx().copy_text(hsl.clone());
                        self.status = format!("Copied {hsl}.");
                    }
                });
            });
    }
}

impl eframe::App for ColorPickerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_pick(ui.ctx());

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme::BG)
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        theme::column(ui, theme::MAX_WIDTH, |ui| {
                            ui.heading("Color Picker");
                            ui.add_space(10.0);

                            self.swatch(ui);
                            ui.add_space(12.0);
                            self.pick_controls(ui);
                            ui.add_space(12.0);
                            self.formats(ui);
                            ui.add_space(8.0);

                            ui.label(
                                egui::RichText::new(&self.status)
                                    .size(12.0)
                                    .color(theme::TEXT_DIM),
                            );
                            ui.add_space(4.0);
                        });
                    });
            });
    }
}
