//! Where the image sits on the canvas: zoom and offset, and the math
//! between canvas points and image pixels.

use eframe::egui::{Vec2, vec2};

/// `zoom` canvas points per image pixel; `offset` is the image's top-left
/// corner relative to the canvas's top-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub zoom: f32,
    pub offset: Vec2,
}

impl Default for View {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset: Vec2::ZERO,
        }
    }
}

impl View {
    pub const MIN_ZOOM: f32 = 0.01;
    pub const MAX_ZOOM: f32 = 96.0;

    /// The whole image, centered and as large as fits.
    pub fn fit(image: Vec2, canvas: Vec2) -> Self {
        if image.x <= 0.0 || image.y <= 0.0 {
            return Self::default();
        }
        let zoom = (canvas.x / image.x)
            .min(canvas.y / image.y)
            .clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        Self {
            zoom,
            offset: (canvas - image * zoom) / 2.0,
        }
    }

    /// Zooms by `factor`, keeping the image point under `anchor` (canvas
    /// coordinates) where it is.
    pub fn zoom_at(self, anchor: Vec2, factor: f32) -> Self {
        let zoom = (self.zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        let k = zoom / self.zoom;
        Self {
            zoom,
            offset: anchor - (anchor - self.offset) * k,
        }
    }

    /// The image point (in pixels, fractional) under a canvas point.
    pub fn to_image(self, canvas_point: Vec2) -> Vec2 {
        (canvas_point - self.offset) / self.zoom
    }

    /// The canvas point of an image point.
    pub fn to_canvas(self, image_point: Vec2) -> Vec2 {
        self.offset + image_point * self.zoom
    }

    /// The pixel under a canvas point, if it is on the image.
    pub fn pixel_at(self, canvas_point: Vec2, size: [u32; 2]) -> Option<[u32; 2]> {
        let p = self.to_image(canvas_point);
        let inside = p.x >= 0.0 && p.y >= 0.0 && p.x < size[0] as f32 && p.y < size[1] as f32;
        inside.then(|| [p.x as u32, p.y as u32])
    }

    /// Keeps at least `margin` points of the image inside the canvas, so it
    /// can't be dragged out of sight.
    pub fn clamp_to(self, image: Vec2, canvas: Vec2, margin: f32) -> Self {
        let size = image * self.zoom;
        let min = vec2(margin - size.x, margin - size.y);
        let max = vec2(canvas.x - margin, canvas.y - margin);
        Self {
            offset: self.offset.max(min).min(max),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_centers_and_scales() {
        let view = View::fit(vec2(200.0, 100.0), vec2(400.0, 400.0));
        assert_eq!(view.zoom, 2.0);
        assert_eq!(view.offset, vec2(0.0, 100.0));

        let big = View::fit(vec2(4000.0, 1000.0), vec2(400.0, 400.0));
        assert_eq!(big.zoom, 0.1);
        assert_eq!(big.offset, vec2(0.0, 150.0));
    }

    #[test]
    fn fit_of_empty_image_is_default() {
        assert_eq!(View::fit(Vec2::ZERO, vec2(10.0, 10.0)), View::default());
    }

    #[test]
    fn zoom_keeps_anchor_still() {
        let view = View {
            zoom: 2.0,
            offset: vec2(30.0, -10.0),
        };
        let anchor = vec2(123.0, 77.0);
        let before = view.to_image(anchor);
        let zoomed = view.zoom_at(anchor, 3.0);
        assert_eq!(zoomed.zoom, 6.0);
        let after = zoomed.to_image(anchor);
        assert!((before - after).length() < 1e-4, "{before:?} vs {after:?}");
    }

    #[test]
    fn zoom_is_clamped() {
        let view = View::default().zoom_at(Vec2::ZERO, 1e9);
        assert_eq!(view.zoom, View::MAX_ZOOM);
        let view = View::default().zoom_at(Vec2::ZERO, 1e-9);
        assert_eq!(view.zoom, View::MIN_ZOOM);
    }

    #[test]
    fn pixel_lookup() {
        let view = View {
            zoom: 10.0,
            offset: vec2(5.0, 5.0),
        };
        assert_eq!(view.pixel_at(vec2(5.0, 5.0), [4, 3]), Some([0, 0]));
        assert_eq!(view.pixel_at(vec2(44.9, 34.9), [4, 3]), Some([3, 2]));
        assert_eq!(view.pixel_at(vec2(45.0, 10.0), [4, 3]), None);
        assert_eq!(view.pixel_at(vec2(4.9, 10.0), [4, 3]), None);
        assert_eq!(view.to_canvas(vec2(1.0, 1.0)), vec2(15.0, 15.0));
    }

    #[test]
    fn clamp_keeps_image_in_reach() {
        let view = View {
            zoom: 1.0,
            offset: vec2(-1000.0, 5000.0),
        };
        let clamped = view.clamp_to(vec2(100.0, 100.0), vec2(300.0, 200.0), 20.0);
        assert_eq!(clamped.offset, vec2(-80.0, 180.0));
    }
}
