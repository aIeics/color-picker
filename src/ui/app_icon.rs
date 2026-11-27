//! The app icon, drawn in code so there is no icon file to ship. Three
//! styles (Settings → Appearance): a color wheel ring, a white pipette on a
//! dark tile, or a gray ring. Used for the taskbar, Alt+Tab and the tray.

use std::f32::consts::TAU;

use eframe::egui::{Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2, pos2, vec2};

use super::icons::{self, Icon};
use crate::color::{Hsl, Rgb};
use crate::settings::IconStyle;

/// Ring radii as fractions of the icon size.
const OUTER: f32 = 0.47;
const INNER: f32 = 0.22;
const MONO: [u8; 3] = [158, 158, 158];
const TILE: [u8; 3] = [26, 26, 26];

/// RGBA pixels (straight alpha) of a `size`×`size` icon.
pub fn pixels(size: u32, style: IconStyle) -> Vec<u8> {
    match style {
        IconStyle::Wheel => ring(size, |hue| {
            let c = Rgb::from_hsl(Hsl {
                h: hue,
                s: 0.9,
                l: 0.55,
            });
            [c.r, c.g, c.b]
        }),
        IconStyle::Mono => ring(size, |_| MONO),
        IconStyle::Pipette => pipette(size),
    }
}

/// Supersamples a shape: `sample` gives the color of a point (in pixels,
/// from the icon's center) or `None` outside it.
fn render(size: u32, sample: impl Fn(f32, f32) -> Option<[u8; 3]>) -> Vec<u8> {
    const SUB: u32 = 4;
    let center = size as f32 / 2.0;
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let (mut sum, mut covered) = ([0f32; 3], 0u32);
            for sy in 0..SUB {
                for sx in 0..SUB {
                    let px = x as f32 + (sx as f32 + 0.5) / SUB as f32 - center;
                    let py = y as f32 + (sy as f32 + 0.5) / SUB as f32 - center;
                    if let Some(c) = sample(px, py) {
                        for i in 0..3 {
                            sum[i] += f32::from(c[i]);
                        }
                        covered += 1;
                    }
                }
            }
            if covered == 0 {
                out.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                let n = covered as f32;
                out.extend_from_slice(&[
                    (sum[0] / n).round() as u8,
                    (sum[1] / n).round() as u8,
                    (sum[2] / n).round() as u8,
                    (covered as f32 / (SUB * SUB) as f32 * 255.0).round() as u8,
                ]);
            }
        }
    }
    out
}

/// A ring colored by `color_at(hue)`, red at the top going clockwise.
fn ring(size: u32, color_at: impl Fn(f32) -> [u8; 3]) -> Vec<u8> {
    let s = size as f32;
    render(size, |x, y| {
        let d = x.hypot(y);
        if d > s * OUTER || d < s * INNER {
            return None;
        }
        Some(color_at(x.atan2(-y).to_degrees().rem_euclid(360.0)))
    })
}

/// Lucide's pipette in white on a dark rounded tile.
fn pipette(size: u32) -> Vec<u8> {
    let s = size as f32;
    let half = s * 0.47;
    let radius = s * 0.2;
    let icon = s * 0.68 / 24.0; // grid units to pixels
    let segments: Vec<(Pos2, Pos2)> = Icon::Pipette
        .geometry()
        .iter()
        .flat_map(|path| {
            let mut points = path.points.clone();
            if path.closed
                && let Some(&first) = points.first()
            {
                points.push(first);
            }
            points.windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>()
        })
        .map(|(a, b)| {
            let map = |p: Pos2| pos2((p.x - 12.0) * icon, (p.y - 12.0) * icon);
            (map(a), map(b))
        })
        .collect();
    let width = 2.2 * icon / 2.0;
    render(size, |x, y| {
        // The rounded tile.
        let qx = (x.abs() - (half - radius)).max(0.0);
        let qy = (y.abs() - (half - radius)).max(0.0);
        if qx.hypot(qy) > radius {
            return None;
        }
        let p = pos2(x, y);
        let on_stroke = segments.iter().any(|&(a, b)| distance(p, a, b) <= width);
        Some(if on_stroke { [255, 255, 255] } else { TILE })
    })
}

fn distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = if ab.length_sq() > 0.0 {
        ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p - (a + ab * t)).length()
}

/// A small preview of a style for the Settings tile, drawn with the painter.
pub fn paint_preview(painter: &Painter, rect: Rect, style: IconStyle) {
    let size = rect.width().min(rect.height());
    let center = rect.center();
    match style {
        IconStyle::Wheel | IconStyle::Mono => {
            let (outer, inner) = (size * OUTER, size * INNER);
            let steps = 48;
            let mut mesh = Mesh::default();
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let angle = t * TAU;
                let color = if style == IconStyle::Mono {
                    Color32::from_rgb(MONO[0], MONO[1], MONO[2])
                } else {
                    Rgb::from_hsl(Hsl {
                        h: t * 360.0,
                        s: 0.9,
                        l: 0.55,
                    })
                    .into()
                };
                let dir = vec2(angle.sin(), -angle.cos());
                mesh.colored_vertex(center + dir * outer, color);
                mesh.colored_vertex(center + dir * inner, color);
                if i > 0 {
                    let k = 2 * i as u32;
                    mesh.add_triangle(k - 2, k - 1, k);
                    mesh.add_triangle(k - 1, k, k + 1);
                }
            }
            painter.add(Shape::mesh(mesh));
        }
        IconStyle::Pipette => {
            let tile = Rect::from_center_size(center, Vec2::splat(size * 0.94));
            painter.rect_filled(
                tile,
                size * 0.2,
                Color32::from_rgb(TILE[0], TILE[1], TILE[2]),
            );
            painter.rect_stroke(
                tile,
                size * 0.2,
                Stroke::new(1.0, Color32::from_white_alpha(25)),
                eframe::egui::StrokeKind::Inside,
            );
            icons::paint(
                painter,
                Rect::from_center_size(center, Vec2::splat(size * 0.68)),
                Icon::Pipette,
                Color32::WHITE,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(icon: &[u8], size: u32, x: u32, y: u32) -> u8 {
        icon[((y * size + x) * 4 + 3) as usize]
    }

    #[test]
    fn ring_with_transparent_middle_and_corners() {
        let size = 32;
        for style in [IconStyle::Wheel, IconStyle::Mono] {
            let icon = pixels(size, style);
            assert_eq!(icon.len(), (size * size * 4) as usize);
            assert_eq!(alpha(&icon, size, 0, 0), 0, "corner");
            assert_eq!(alpha(&icon, size, 16, 16), 0, "middle");
            assert_eq!(alpha(&icon, size, 16, 3), 255, "ring");
        }
    }

    #[test]
    fn pipette_tile_has_white_strokes() {
        let size = 48;
        let icon = pixels(size, IconStyle::Pipette);
        assert_eq!(alpha(&icon, size, 0, 0), 0, "rounded corner");
        assert_eq!(alpha(&icon, size, 24, 2), 255, "tile edge");
        let white = icon
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] > 200 && p[3] > 200)
            .count();
        assert!(white > 30, "{white} white pixels");
    }
}
