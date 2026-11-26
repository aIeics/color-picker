//! Turning files, pasted images and pasted paths into pixels.

use std::path::{Path, PathBuf};

use eframe::egui::ColorImage;
use image::RgbaImage;

use crate::color::Rgb;

/// File types the Open dialog offers and pasted paths may point to.
pub const EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "bmp", "gif", "webp"];

/// A decoded image and the name to show for it.
pub struct Picture {
    pub name: String,
    pub pixels: RgbaImage,
}

impl Picture {
    pub fn size(&self) -> [u32; 2] {
        [self.pixels.width(), self.pixels.height()]
    }
}

/// Reads and decodes an image file (GIF: its first frame).
pub fn open(path: &Path) -> Result<Picture, String> {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let bytes = std::fs::read(path).map_err(|e| format!("Can't read {name}: {e}"))?;
    decode(&bytes, name)
}

pub fn decode(bytes: &[u8], name: String) -> Result<Picture, String> {
    let image = image::load_from_memory(bytes).map_err(|e| format!("Can't open {name}: {e}"))?;
    let pixels = image.to_rgba8();
    if pixels.width() == 0 || pixels.height() == 0 {
        return Err(format!("{name} is empty"));
    }
    Ok(Picture { name, pixels })
}

/// An image pasted from the clipboard.
pub fn from_pasted(image: &ColorImage) -> Option<Picture> {
    let [w, h] = image.size;
    let bytes = image
        .pixels
        .iter()
        .flat_map(|c| c.to_srgba_unmultiplied())
        .collect();
    let pixels = RgbaImage::from_raw(w as u32, h as u32, bytes)?;
    (w > 0 && h > 0).then(|| Picture {
        name: "Pasted image".to_owned(),
        pixels,
    })
}

/// A pasted path to an image file (Explorer's "Copy as path" adds quotes).
pub fn pasted_path(text: &str) -> Option<PathBuf> {
    let text = text.trim().trim_matches('"').trim();
    if text.is_empty() || text.contains('\n') {
        return None;
    }
    let path = PathBuf::from(text);
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    (EXTENSIONS.contains(&extension.as_str()) && path.is_file()).then_some(path)
}

/// The average color of the `side`×`side` square around pixel (`x`, `y`),
/// clipped to the image. Alpha is ignored.
pub fn sample(pixels: &RgbaImage, [x, y]: [u32; 2], side: u32) -> Option<Rgb> {
    let (w, h) = pixels.dimensions();
    if x >= w || y >= h {
        return None;
    }
    let half = side.max(1) / 2;
    let xs = x.saturating_sub(half)..=(x + half).min(w - 1);
    let ys = y.saturating_sub(half)..=(y + half).min(h - 1);
    Rgb::average(ys.flat_map(|py| {
        xs.clone().map(move |px| {
            let [r, g, b, _] = pixels.get_pixel(px, py).0;
            Rgb::new(r, g, b)
        })
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checker() -> RgbaImage {
        RgbaImage::from_fn(4, 4, |x, y| {
            if (x + y) % 2 == 0 {
                image::Rgba([255, 255, 255, 255])
            } else {
                image::Rgba([0, 0, 0, 0])
            }
        })
    }

    #[test]
    fn samples_single_pixels() {
        let pixels = checker();
        assert_eq!(sample(&pixels, [0, 0], 1), Some(Rgb::new(255, 255, 255)));
        assert_eq!(sample(&pixels, [1, 0], 1), Some(Rgb::BLACK));
        assert_eq!(sample(&pixels, [4, 0], 1), None);
    }

    #[test]
    fn averages_and_clips_at_edges() {
        let pixels = checker();
        // 3×3 around (1, 1): 5 white, 4 black.
        assert_eq!(sample(&pixels, [1, 1], 3), Some(Rgb::new(142, 142, 142)));
        // 5×5 at the corner only covers the 3×3 that exists: 5 white of 9.
        assert_eq!(sample(&pixels, [0, 0], 5), Some(Rgb::new(142, 142, 142)));
    }

    #[test]
    fn decodes_png_and_rejects_junk() {
        let mut png = Vec::new();
        checker()
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let picture = decode(&png, "a.png".into()).unwrap();
        assert_eq!(picture.size(), [4, 4]);
        assert!(decode(b"not an image", "x.png".into()).is_err());
    }

    #[test]
    fn pasted_text_that_is_not_an_image_path() {
        assert_eq!(pasted_path(""), None);
        assert_eq!(pasted_path("#3B82F6"), None);
        assert_eq!(pasted_path("\"C:\\nope\\missing.png\""), None);
        assert_eq!(pasted_path("a.png\nb.png"), None);
    }
}
