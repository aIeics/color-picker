//! Color picker: pick colors from the screen or an image.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod action;
mod app;
mod color;
mod editor;
mod history;
mod hotkey;
mod image_pick;
mod palette;
mod platform;
mod screen;
mod settings;
mod store;
mod ui;

use eframe::egui;

fn main() -> eframe::Result {
    // Only one copy runs; launching again shows the running window.
    if !platform::is_first_instance() {
        platform::wake_first_instance();
        return Ok(());
    }
    let saved = store::load();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Color Picker")
        .with_app_id("color-picker")
        .with_inner_size([app::WIDTH, app::initial_height(&saved.settings)])
        .with_resizable(false)
        .with_maximize_button(false)
        .with_decorations(false);
    if saved.settings.always_on_top {
        viewport = viewport.with_always_on_top();
    }

    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "Color Picker",
        options,
        Box::new(move |cc| Ok(Box::new(app::ColorPickerApp::new(cc, saved)))),
    )
}
