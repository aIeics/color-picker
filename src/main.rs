//! Color picker: pick colors from the screen or an image.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod color;
mod platform;

use app::ColorPickerApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Color Picker")
            .with_inner_size([380.0, 500.0])
            .with_min_inner_size([320.0, 420.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Color Picker",
        options,
        Box::new(|cc| Ok(Box::new(ColorPickerApp::new(cc)))),
    )
}
