//! A small reader for SVG path data, enough for the Lucide icons: lines,
//! cubic and quadratic curves and elliptical arcs, absolute and relative.
//! Curves are flattened into polylines the painter can stroke or fill.

use std::f32::consts::{PI, TAU};

use eframe::egui::{Pos2, Vec2, pos2, vec2};

/// One continuous run of the path.
#[derive(Clone, Debug, PartialEq)]
pub struct Subpath {
    pub points: Vec<Pos2>,
    pub closed: bool,
}

/// Parses path data such as `M3 12a9 9 0 1 0 9-9`. Stops quietly at the
/// first thing it can't read, keeping what it has.
pub fn parse(data: &str) -> Vec<Subpath> {
    let mut reader = Reader {
        bytes: data.as_bytes(),
        at: 0,
    };
    let mut paths = Vec::new();
    let mut points: Vec<Pos2> = Vec::new();
    let mut pos = Pos2::ZERO;
    let mut start = Pos2::ZERO;
    // The last control points, for the smooth curve commands S and T.
    let mut cubic_ctrl: Option<Pos2> = None;
    let mut quad_ctrl: Option<Pos2> = None;
    let mut command = 0u8;

    let flush = |points: &mut Vec<Pos2>, paths: &mut Vec<Subpath>, closed: bool| {
        if points.len() > 1 {
            paths.push(Subpath {
                points: std::mem::take(points),
                closed,
            });
        } else {
            points.clear();
        }
    };

    while let Some(next) = reader.command(command) {
        command = next;
        let relative = command.is_ascii_lowercase();
        let origin = if relative { pos } else { Pos2::ZERO };
        let upper = command.to_ascii_uppercase();
        if upper != b'C' && upper != b'S' {
            cubic_ctrl = None;
        }
        if upper != b'Q' && upper != b'T' {
            quad_ctrl = None;
        }
        match upper {
            b'M' => {
                let Some(p) = reader.point(origin) else { break };
                flush(&mut points, &mut paths, false);
                points.push(p);
                pos = p;
                start = p;
                // Pairs after a move are line-tos.
                command = if relative { b'l' } else { b'L' };
            }
            b'L' => {
                let Some(p) = reader.point(origin) else { break };
                points.push(p);
                pos = p;
            }
            b'H' => {
                let Some(x) = reader.number() else { break };
                pos = pos2(if relative { pos.x + x } else { x }, pos.y);
                points.push(pos);
            }
            b'V' => {
                let Some(y) = reader.number() else { break };
                pos = pos2(pos.x, if relative { pos.y + y } else { y });
                points.push(pos);
            }
            b'C' => {
                let (Some(c1), Some(c2), Some(p)) = (
                    reader.point(origin),
                    reader.point(origin),
                    reader.point(origin),
                ) else {
                    break;
                };
                cubic(&mut points, pos, c1, c2, p);
                cubic_ctrl = Some(c2);
                pos = p;
            }
            b'S' => {
                let (Some(c2), Some(p)) = (reader.point(origin), reader.point(origin)) else {
                    break;
                };
                let c1 = cubic_ctrl.map_or(pos, |c| pos + (pos - c));
                cubic(&mut points, pos, c1, c2, p);
                cubic_ctrl = Some(c2);
                pos = p;
            }
            b'Q' => {
                let (Some(c), Some(p)) = (reader.point(origin), reader.point(origin)) else {
                    break;
                };
                quadratic(&mut points, pos, c, p);
                quad_ctrl = Some(c);
                pos = p;
            }
            b'T' => {
                let Some(p) = reader.point(origin) else { break };
                let c = quad_ctrl.map_or(pos, |c| pos + (pos - c));
                quadratic(&mut points, pos, c, p);
                quad_ctrl = Some(c);
                pos = p;
            }
            b'A' => {
                let (Some(rx), Some(ry), Some(rotation), Some(large), Some(sweep), Some(p)) = (
                    reader.number(),
                    reader.number(),
                    reader.number(),
                    reader.flag(),
                    reader.flag(),
                    reader.point(origin),
                ) else {
                    break;
                };
                arc(&mut points, pos, rx, ry, rotation, large, sweep, p);
                pos = p;
            }
            b'Z' => {
                flush(&mut points, &mut paths, true);
                pos = start;
                points.push(pos);
            }
            _ => break,
        }
    }
    flush(&mut points, &mut paths, false);
    paths
}

fn cubic(points: &mut Vec<Pos2>, p0: Pos2, c1: Pos2, c2: Pos2, p1: Pos2) {
    const STEPS: usize = 10;
    for i in 1..=STEPS {
        let t = i as f32 / STEPS as f32;
        let u = 1.0 - t;
        let v = p0.to_vec2() * (u * u * u)
            + c1.to_vec2() * (3.0 * u * u * t)
            + c2.to_vec2() * (3.0 * u * t * t)
            + p1.to_vec2() * (t * t * t);
        points.push(v.to_pos2());
    }
}

fn quadratic(points: &mut Vec<Pos2>, p0: Pos2, c: Pos2, p1: Pos2) {
    const STEPS: usize = 8;
    for i in 1..=STEPS {
        let t = i as f32 / STEPS as f32;
        let u = 1.0 - t;
        let v = p0.to_vec2() * (u * u) + c.to_vec2() * (2.0 * u * t) + p1.to_vec2() * (t * t);
        points.push(v.to_pos2());
    }
}

/// An elliptical arc, converted from SVG's endpoint form to a center and
/// angles (SVG 1.1, appendix F.6.5).
#[allow(clippy::too_many_arguments)]
fn arc(
    points: &mut Vec<Pos2>,
    p0: Pos2,
    rx: f32,
    ry: f32,
    rotation_deg: f32,
    large: bool,
    sweep: bool,
    p1: Pos2,
) {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < 1e-6 || ry < 1e-6 || (p0 - p1).length() < 1e-6 {
        points.push(p1);
        return;
    }
    let phi = rotation_deg.to_radians();
    let (sin, cos) = phi.sin_cos();
    let rotate = |v: Vec2, s: f32, c: f32| vec2(c * v.x - s * v.y, s * v.x + c * v.y);

    let half = (p0 - p1) / 2.0;
    let p = rotate(half, -sin, cos);
    let lambda = (p.x * p.x) / (rx * rx) + (p.y * p.y) / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let num = rx * rx * ry * ry - rx * rx * p.y * p.y - ry * ry * p.x * p.x;
    let den = rx * rx * p.y * p.y + ry * ry * p.x * p.x;
    let mut k = if den > 0.0 {
        (num / den).max(0.0).sqrt()
    } else {
        0.0
    };
    if large == sweep {
        k = -k;
    }
    let c_prime = vec2(k * rx * p.y / ry, -k * ry * p.x / rx);
    let center = rotate(c_prime, sin, cos) + (p0.to_vec2() + p1.to_vec2()) / 2.0;

    let angle = |u: Vec2, v: Vec2| (u.x * v.y - u.y * v.x).atan2(u.x * v.x + u.y * v.y);
    let u = vec2((p.x - c_prime.x) / rx, (p.y - c_prime.y) / ry);
    let v = vec2((-p.x - c_prime.x) / rx, (-p.y - c_prime.y) / ry);
    let theta = angle(vec2(1.0, 0.0), u);
    let mut delta = angle(u, v) % TAU;
    if !sweep && delta > 0.0 {
        delta -= TAU;
    } else if sweep && delta < 0.0 {
        delta += TAU;
    }

    let steps = ((delta.abs() / (PI / 12.0)).ceil() as usize).max(2);
    for i in 1..=steps {
        let t = theta + delta * i as f32 / steps as f32;
        let local = vec2(rx * t.cos(), ry * t.sin());
        points.push((rotate(local, sin, cos) + center).to_pos2());
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn skip_separators(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| b.is_ascii_whitespace() || *b == b',')
        {
            self.at += 1;
        }
    }

    /// The next command letter, or `last` again when a number follows
    /// (implicit repetition). `None` at the end.
    fn command(&mut self, last: u8) -> Option<u8> {
        self.skip_separators();
        let b = *self.bytes.get(self.at)?;
        if b.is_ascii_alphabetic() {
            self.at += 1;
            Some(b)
        } else if (b.is_ascii_digit() || b == b'-' || b == b'+' || b == b'.') && last != 0 {
            Some(last)
        } else {
            None
        }
    }

    fn number(&mut self) -> Option<f32> {
        self.skip_separators();
        let start = self.at;
        let bytes = self.bytes;
        let mut i = self.at;
        if matches!(bytes.get(i), Some(b'-' | b'+')) {
            i += 1;
        }
        let digits = |i: &mut usize| {
            let from = *i;
            while bytes.get(*i).is_some_and(u8::is_ascii_digit) {
                *i += 1;
            }
            *i > from
        };
        let whole = digits(&mut i);
        let mut fraction = false;
        if bytes.get(i) == Some(&b'.') {
            i += 1;
            fraction = digits(&mut i);
        }
        if !whole && !fraction {
            return None;
        }
        if matches!(bytes.get(i), Some(b'e' | b'E')) {
            let mut j = i + 1;
            if matches!(bytes.get(j), Some(b'-' | b'+')) {
                j += 1;
            }
            if digits(&mut j) {
                i = j;
            }
        }
        self.at = i;
        std::str::from_utf8(&bytes[start..i]).ok()?.parse().ok()
    }

    fn point(&mut self, origin: Pos2) -> Option<Pos2> {
        let x = self.number()?;
        let y = self.number()?;
        Some(origin + vec2(x, y))
    }

    /// An arc flag: a single `0` or `1`, which may run into the next number.
    fn flag(&mut self) -> Option<bool> {
        self.skip_separators();
        let b = *self.bytes.get(self.at)?;
        self.at += 1;
        match b {
            b'0' => Some(false),
            b'1' => Some(true),
            _ => None,
        }
    }
}

/// Splits a simple polygon (no holes) into triangles, for filled icons.
/// Returns indices into `points`.
pub fn triangulate(points: &[Pos2]) -> Vec<[usize; 3]> {
    let n = points.len();
    if n < 3 {
        return Vec::new();
    }
    // Work counter-clockwise (in y-down screen space: positive area).
    let area: f32 = (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    let mut order: Vec<usize> = (0..n).collect();
    if area < 0.0 {
        order.reverse();
    }
    let cross = |o: Pos2, a: Pos2, b: Pos2| (a - o).x * (b - o).y - (a - o).y * (b - o).x;
    let inside = |p: Pos2, a: Pos2, b: Pos2, c: Pos2| {
        cross(a, b, p) >= 0.0 && cross(b, c, p) >= 0.0 && cross(c, a, p) >= 0.0
    };

    let mut triangles = Vec::with_capacity(n - 2);
    let mut guard = 0;
    while order.len() > 3 && guard < n * n {
        guard += 1;
        let m = order.len();
        let mut clipped = false;
        for i in 0..m {
            let (ia, ib, ic) = (order[(i + m - 1) % m], order[i], order[(i + 1) % m]);
            let (a, b, c) = (points[ia], points[ib], points[ic]);
            if cross(a, b, c) <= 1e-7 {
                continue; // reflex or flat corner
            }
            let blocked = order
                .iter()
                .any(|&j| j != ia && j != ib && j != ic && inside(points[j], a, b, c));
            if !blocked {
                triangles.push([ia, ib, ic]);
                order.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped {
            // Degenerate input: drop a flat corner and keep going.
            order.remove(0);
        }
    }
    if order.len() == 3 {
        triangles.push([order[0], order[1], order[2]]);
    }
    triangles
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: Pos2, b: Pos2) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn lines_and_implicit_repeats() {
        let paths = parse("M3 3v5h5");
        assert_eq!(paths.len(), 1);
        assert_eq!(
            paths[0].points,
            [pos2(3.0, 3.0), pos2(3.0, 8.0), pos2(8.0, 8.0)]
        );

        let paths = parse("M1 1 2 2 3 3");
        assert_eq!(paths[0].points.len(), 3, "pairs after M are line-tos");
    }

    #[test]
    fn compact_numbers() {
        let paths = parse("M.5.5l9 9M9.5.5l-9 9");
        assert_eq!(paths.len(), 2);
        assert!(near(paths[0].points[1], pos2(9.5, 9.5)));
        assert!(near(paths[1].points[1], pos2(0.5, 9.5)));
        let paths = parse("M1e1 2E0L-1.5-2");
        assert!(near(paths[0].points[0], pos2(10.0, 2.0)));
        assert!(near(paths[0].points[1], pos2(-1.5, -2.0)));
    }

    #[test]
    fn arcs_stay_on_the_circle_and_end_where_asked() {
        // A full circle of radius 3 around (12, 12), as the converter writes it.
        let paths = parse("M9 12a3 3 0 1 0 6 0a3 3 0 1 0 -6 0z");
        assert_eq!(paths.len(), 1);
        assert!(paths[0].closed);
        for p in &paths[0].points {
            assert!(
                ((*p - pos2(12.0, 12.0)).length() - 3.0).abs() < 1e-3,
                "{p:?}"
            );
        }
        // Compact flags: "a2 2 0 011-1" means flags 0 and 1, then 1 -1.
        let paths = parse("M0 0a2 2 0 011-1");
        assert!(near(*paths[0].points.last().unwrap(), pos2(1.0, -1.0)));
    }

    #[test]
    fn curves_end_at_their_end_points() {
        let paths = parse("M0 0C1 2 3 2 4 0S7-2 8 0Q9 2 10 0T12 0");
        assert!(near(*paths[0].points.last().unwrap(), pos2(12.0, 0.0)));
        assert!(
            paths[0].points.iter().any(|p| p.y > 1.0),
            "curved, not straight"
        );
    }

    #[test]
    fn close_returns_to_start() {
        let paths = parse("M2 2h4v4zm1 1h1");
        assert_eq!(paths.len(), 2);
        assert!(paths[0].closed);
        assert_eq!(
            paths[1].points[0],
            pos2(3.0, 3.0),
            "m after z is relative to the start"
        );
    }

    #[test]
    fn junk_does_not_panic() {
        for data in [
            "", "Z", "M", "M1", "L1 2", "M1 2 x", "M1 2a1", "Mq", "M1 2C",
        ] {
            let _ = parse(data);
        }
    }

    #[test]
    fn triangulates_concave_polygons() {
        // An L shape: 6 corners, 4 triangles, covering its area of 3.
        let l = [
            pos2(0.0, 0.0),
            pos2(2.0, 0.0),
            pos2(2.0, 1.0),
            pos2(1.0, 1.0),
            pos2(1.0, 2.0),
            pos2(0.0, 2.0),
        ];
        let triangles = triangulate(&l);
        assert_eq!(triangles.len(), 4);
        let area: f32 = triangles
            .iter()
            .map(|t| {
                let (a, b, c) = (l[t[0]], l[t[1]], l[t[2]]);
                ((b - a).x * (c - a).y - (b - a).y * (c - a).x).abs() / 2.0
            })
            .sum();
        assert!((area - 3.0).abs() < 1e-4, "{area}");
    }
}
