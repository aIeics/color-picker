//! The app icon, drawn in code so there is no icon file to ship: a color
//! wheel ring. Used for the taskbar, Alt+Tab and the tray.

use crate::color::{Hsl, Rgb};

/// RGBA pixels (straight alpha) of a `size`×`size` icon.
pub fn pixels(size: u32) -> Vec<u8> {
    const SUB: u32 = 4; // 4×4 samples per pixel for smooth edges
    let s = size as f32;
    let center = s / 2.0;
    let outer = s * 0.47;
    let inner = s * 0.22;

    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let (mut r, mut g, mut b, mut covered) = (0.0f32, 0.0f32, 0.0f32, 0u32);
            for sy in 0..SUB {
                for sx in 0..SUB {
                    let px = x as f32 + (sx as f32 + 0.5) / SUB as f32 - center;
                    let py = y as f32 + (sy as f32 + 0.5) / SUB as f32 - center;
                    let d = px.hypot(py);
                    if d > outer || d < inner {
                        continue;
                    }
                    // Hue around the ring, red at the top, clockwise.
                    let hue = px.atan2(-py).to_degrees().rem_euclid(360.0);
                    let c = Rgb::from_hsl(Hsl {
                        h: hue,
                        s: 0.9,
                        l: 0.55,
                    });
                    r += f32::from(c.r);
                    g += f32::from(c.g);
                    b += f32::from(c.b);
                    covered += 1;
                }
            }
            if covered == 0 {
                out.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                let n = covered as f32;
                let alpha = covered as f32 / (SUB * SUB) as f32;
                out.extend_from_slice(&[
                    (r / n).round() as u8,
                    (g / n).round() as u8,
                    (b / n).round() as u8,
                    (alpha * 255.0).round() as u8,
                ]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_with_transparent_middle_and_corners() {
        let size = 32;
        let icon = pixels(size);
        assert_eq!(icon.len(), (size * size * 4) as usize);
        let alpha = |x: u32, y: u32| icon[((y * size + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0, "corner");
        assert_eq!(alpha(16, 16), 0, "middle");
        assert_eq!(alpha(16, 3), 255, "ring");
    }
}
