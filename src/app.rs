//! Application state and user interface.

use std::time::{Duration, Instant};

use eframe::egui;

use crate::color::Rgb;
use crate::platform;

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
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
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
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 120.0),
            egui::Sense::hover(),
        );
        let painter = ui.painter();
        painter.rect_filled(rect, 8.0, self.color);

        let text_color = if self.color.luminance() > 0.5 {
            egui::Color32::BLACK
        } else {
            egui::Color32::WHITE
        };
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            self.color.to_hex(),
            egui::FontId::monospace(22.0),
            text_color,
        );
    }

    fn pick_controls(&mut self, ui: &mut egui::Ui) {
        if platform::SCREEN_PICKING_SUPPORTED {
            ui.horizontal(|ui| {
                let picking = self.pending_pick.is_some();
                if ui
                    .add_enabled(!picking, egui::Button::new("Pick from screen"))
                    .clicked()
                {
                    self.start_pick();
                }
                if picking && ui.button("Cancel").clicked() {
                    self.cancel_pick();
                }
            });

            if self.pending_pick.is_some() {
                ui.label(
                    egui::RichText::new("Picking in 2 seconds; move the cursor over the color.")
                        .small()
                        .weak(),
                );
            }
        } else {
            ui.label("Picking from the screen is only available on Windows.");
        }
    }

    fn formats(&mut self, ui: &mut egui::Ui) {
        let rgb = self.color.to_rgb_string();
        let hsl = self.color.to_hsl_string();

        egui::Grid::new("formats")
            .num_columns(3)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label("HEX");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.hex_input)
                        .desired_width(130.0)
                        .font(egui::TextStyle::Monospace),
                );
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
                if ui.button("Copy").clicked() {
                    let hex = self.color.to_hex();
                    ui.ctx().copy_text(hex.clone());
                    self.status = format!("Copied {hex}.");
                }
                ui.end_row();

                ui.label("RGB");
                ui.label(egui::RichText::new(&rgb).monospace());
                if ui.button("Copy").clicked() {
                    ui.ctx().copy_text(rgb.clone());
                    self.status = format!("Copied {rgb}.");
                }
                ui.end_row();

                ui.label("HSL");
                ui.label(egui::RichText::new(&hsl).monospace());
                if ui.button("Copy").clicked() {
                    ui.ctx().copy_text(hsl.clone());
                    self.status = format!("Copied {hsl}.");
                }
                ui.end_row();
            });
    }
}

impl eframe::App for ColorPickerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_pick(ui.ctx());

        egui::CentralPanel::default_margins().show(ui, |ui| {
            ui.add_space(4.0);
            ui.heading("Color Picker");
            ui.add_space(8.0);

            self.swatch(ui);
            ui.add_space(8.0);
            self.pick_controls(ui);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            self.formats(ui);
            ui.add_space(8.0);

            ui.label(egui::RichText::new(&self.status).small().weak());
        });
    }
}
