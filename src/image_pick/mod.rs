//! Picking colors from an image: the Image tab's state. Files decode on a
//! background thread; `view` has the zoom/pan math and `load` the decoding.

mod load;
pub mod ui;
mod view;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use eframe::egui::{
    self, ColorImage, TextureFilter, TextureHandle, TextureOptions, TextureWrapMode,
};
use image::RgbaImage;

use load::Picture;
pub use load::pasted_path;
use view::View;

use crate::platform::{self, MainWindow};

/// The Image tab: the open image, how it is shown, and what is loading.
#[derive(Default)]
pub struct ImageTab {
    shown: Option<Shown>,
    view: View,
    /// Fit the image to the canvas on the next frame (new image, Fit).
    needs_fit: bool,
    /// Keep fitting while the canvas changes size (the window growing for
    /// this tab), until the user zooms or pans.
    auto_fit: bool,
    loading: Option<(String, Receiver<Result<Picture, String>>)>,
    dialog: Option<Receiver<Option<PathBuf>>>,
    error: Option<String>,
    checker: Option<TextureHandle>,
    /// The hovered pixel and its color, from the last frame.
    hover: Option<([u32; 2], crate::color::Rgb)>,
    /// Canvas size from the last frame, for the zoom buttons.
    canvas: egui::Vec2,
    /// Frames left to wait for clipboard contents after "Paste".
    awaiting_paste: u8,
}

struct Shown {
    name: String,
    pixels: RgbaImage,
    texture: TextureHandle,
}

impl ImageTab {
    /// Starts decoding `path` in the background.
    pub fn open_path(&mut self, ctx: &egui::Context, path: PathBuf) {
        let name = file_name(&path);
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(load::open(&path));
            ctx.request_repaint();
        });
        self.loading = Some((name, rx));
        self.error = None;
    }

    pub fn open_pasted(&mut self, ctx: &egui::Context, image: &ColorImage) {
        match load::from_pasted(image) {
            Some(picture) => self.show_picture(ctx, picture),
            None => self.error = Some("The pasted image is empty".to_owned()),
        }
    }

    /// Opens the system's Open dialog, unless it is already open.
    pub fn browse(&mut self, ctx: &egui::Context, owner: Option<MainWindow>) {
        if self.dialog.is_none() {
            self.dialog = Some(platform::open_image_dialog(ctx, owner));
        }
    }

    /// Waits a couple of frames for the clipboard after "Paste".
    pub fn expect_paste(&mut self) {
        self.awaiting_paste = 3;
    }

    /// Whether "Paste" is still waiting; counts down one frame.
    pub fn paste_timed_out(&mut self) -> bool {
        match self.awaiting_paste {
            0 => false,
            1 => {
                self.awaiting_paste = 0;
                true
            }
            n => {
                self.awaiting_paste = n - 1;
                false
            }
        }
    }

    pub fn paste_handled(&mut self) {
        self.awaiting_paste = 0;
    }

    pub fn waiting_for_paste(&self) -> bool {
        self.awaiting_paste > 0
    }

    /// Picks up dialog results and finished decodes.
    pub fn poll(&mut self, ctx: &egui::Context) {
        if let Some(dialog) = &self.dialog {
            match dialog.try_recv() {
                Ok(Some(path)) => {
                    self.dialog = None;
                    self.open_path(ctx, path);
                }
                Ok(None) | Err(TryRecvError::Disconnected) => self.dialog = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some((_, loading)) = &self.loading {
            match loading.try_recv() {
                Ok(Ok(picture)) => {
                    self.loading = None;
                    self.show_picture(ctx, picture);
                }
                Ok(Err(error)) => {
                    self.loading = None;
                    self.error = Some(error);
                }
                Err(TryRecvError::Disconnected) => {
                    self.loading = None;
                    self.error = Some("Couldn't open the image".to_owned());
                }
                Err(TryRecvError::Empty) => {}
            }
        }
    }

    fn show_picture(&mut self, ctx: &egui::Context, picture: Picture) {
        let texture = upload(ctx, &picture.pixels);
        self.shown = Some(Shown {
            name: picture.name,
            pixels: picture.pixels,
            texture,
        });
        self.needs_fit = true;
        self.auto_fit = true;
        self.error = None;
        self.hover = None;
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// Uploads `pixels` as a texture. Images larger than the GPU allows are
/// shown scaled down; picking still reads the full-size pixels.
fn upload(ctx: &egui::Context, pixels: &RgbaImage) -> TextureHandle {
    let max = ctx.input(|i| i.max_texture_side).max(1) as u32;
    let (w, h) = pixels.dimensions();
    let scaled;
    let pixels = if w > max || h > max {
        let k = max as f32 / w.max(h) as f32;
        let (sw, sh) = (
            ((w as f32 * k) as u32).clamp(1, max),
            ((h as f32 * k) as u32).clamp(1, max),
        );
        scaled = image::imageops::resize(pixels, sw, sh, image::imageops::FilterType::Triangle);
        &scaled
    } else {
        pixels
    };
    let image = ColorImage::from_rgba_unmultiplied(
        [pixels.width() as usize, pixels.height() as usize],
        pixels.as_raw(),
    );
    ctx.load_texture(
        "picked-image",
        image,
        TextureOptions {
            magnification: TextureFilter::Nearest,
            minification: TextureFilter::Linear,
            mipmap_mode: Some(TextureFilter::Linear),
            ..TextureOptions::LINEAR
        },
    )
}

/// A tiny repeating checkerboard shown behind transparent pixels.
fn checker_texture(ctx: &egui::Context) -> TextureHandle {
    let dark = egui::Color32::from_gray(22);
    let light = egui::Color32::from_gray(34);
    let image = ColorImage::new([2, 2], vec![light, dark, dark, light]);
    ctx.load_texture(
        "checker",
        image,
        TextureOptions {
            magnification: TextureFilter::Nearest,
            minification: TextureFilter::Nearest,
            wrap_mode: TextureWrapMode::Repeat,
            mipmap_mode: None,
        },
    )
}
