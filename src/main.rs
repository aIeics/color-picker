//! Color picker: pick colors from the screen or an image.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod color;
mod history;
mod hotkey;
mod palette;
mod platform;
mod settings;
mod store;
mod theme;

use app::ColorPickerApp;

/// Fixed window size: a wide rectangle that fits every control without
/// scrolling or overlapping.
const WINDOW_SIZE: [f32; 2] = [640.0, 400.0];

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Color Picker")
            .with_inner_size(WINDOW_SIZE)
            .with_min_inner_size(WINDOW_SIZE)
            .with_max_inner_size(WINDOW_SIZE)
            .with_resizable(false)
            .with_minimize_button(true)
            .with_maximize_button(false)
            .with_close_button(true),
        ..Default::default()
    };

    eframe::run_native(
        "Color Picker",
        options,
        Box::new(|cc| Ok(Box::new(ColorPickerApp::new(cc)))),
    )
}
