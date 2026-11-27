//! Lucide icons (the set BlurAutoClicker uses), drawn from their SVG path
//! data on a 24×24 grid. Parsed once, then stroked with round ends and
//! joins at any size, color and rotation.

use std::sync::OnceLock;

use eframe::egui::{Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

use super::lucide as l;
use super::svg::{self, Subpath};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Settings,
    Pipette,
    Image,
    Sliders,
    History,
    Palette,
    Copy,
    Keyboard,
    Clear,
    Folder,
    Paste,
    Plus,
    Minus,
    Fit,
    Trash,
    Check,
    Crosshair,
    AppWindow,
    Sun,
    Info,
    ExternalLink,
    CircleCheck,
    ImagePlus,
    Reset,
    Eye,
    Hash,
    PanelBottom,
}

impl Icon {
    const ALL: [Self; 27] = [
        Self::Settings,
        Self::Pipette,
        Self::Image,
        Self::Sliders,
        Self::History,
        Self::Palette,
        Self::Copy,
        Self::Keyboard,
        Self::Clear,
        Self::Folder,
        Self::Paste,
        Self::Plus,
        Self::Minus,
        Self::Fit,
        Self::Trash,
        Self::Check,
        Self::Crosshair,
        Self::AppWindow,
        Self::Sun,
        Self::Info,
        Self::ExternalLink,
        Self::CircleCheck,
        Self::ImagePlus,
        Self::Reset,
        Self::Eye,
        Self::Hash,
        Self::PanelBottom,
    ];

    fn data(self) -> &'static [&'static str] {
        match self {
            Self::Settings => l::SETTINGS,
            Self::Pipette => l::PIPETTE,
            Self::Image => l::IMAGE,
            Self::Sliders => l::SLIDERS_HORIZONTAL,
            Self::History => l::ROTATE_CCW_CLOCK,
            Self::Palette => l::PALETTE,
            Self::Copy => l::COPY,
            Self::Keyboard => l::KEYBOARD,
            Self::Clear => l::X,
            Self::Folder => l::FOLDER_OPEN,
            Self::Paste => l::CLIPBOARD_PASTE,
            Self::Plus => l::PLUS,
            Self::Minus => l::MINUS,
            Self::Fit => l::SCAN,
            Self::Trash => l::TRASH,
            Self::Check => l::CHECK,
            Self::Crosshair => l::CROSSHAIR,
            Self::AppWindow => l::APP_WINDOW,
            Self::Sun => l::SUN,
            Self::Info => l::INFO,
            Self::ExternalLink => l::EXTERNAL_LINK,
            Self::CircleCheck => l::CIRCLE_CHECK,
            Self::ImagePlus => l::IMAGE_PLUS,
            Self::Reset => l::ROTATE_CCW,
            Self::Eye => l::EYE,
            Self::Hash => l::HASH,
            Self::PanelBottom => l::PANEL_BOTTOM,
        }
    }

    /// The icon's polylines on the 24×24 grid, parsed on first use.
    pub fn geometry(self) -> &'static [Subpath] {
        static CACHE: [OnceLock<Vec<Subpath>>; 27] = [const { OnceLock::new() }; 27];
        let index = Self::ALL.iter().position(|&i| i == self).unwrap_or(0);
        CACHE[index].get_or_init(|| self.data().iter().flat_map(|d| svg::parse(d)).collect())
    }
}

/// Paints `icon` centered in `rect`, as large as its shorter side.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    paint_ex(painter, rect, icon, color, 2.0, 0.0);
}

/// Like [`paint`], with the stroke width in grid units (Lucide uses 2) and a
/// rotation in radians around the center.
pub fn paint_ex(painter: &Painter, rect: Rect, icon: Icon, color: Color32, width: f32, angle: f32) {
    let grid = Grid::new(rect, angle);
    let stroke = Stroke::new((width * grid.scale).max(1.0), color);
    for path in icon.geometry() {
        let points: Vec<Pos2> = path.points.iter().map(|&p| grid.map(p)).collect();
        stroke_path(painter, points, path.closed, stroke);
    }
}

/// The pin button: Lucide's pin body, filled, and a needle that grows from
/// nothing to full length as `needle` goes from 0 to 1 (pinned).
pub fn paint_pin(painter: &Painter, rect: Rect, color: Color32, needle: f32) {
    static BODY: OnceLock<Vec<Pos2>> = OnceLock::new();
    let body = BODY.get_or_init(|| {
        let mut points: Vec<Pos2> = svg::parse(l::PIN_BODY)
            .into_iter()
            .flat_map(|p| p.points)
            .collect();
        points.dedup_by(|a, b| (*a - *b).length() < 1e-3);
        if points.len() > 1 && (points[0] - points[points.len() - 1]).length() < 1e-3 {
            points.pop();
        }
        points
    });
    let grid = Grid::new(rect, 0.0);
    let needle = needle.clamp(0.0, 1.0);
    let drop = vec2(0.0, needle * grid.scale);
    let mapped: Vec<Pos2> = body.iter().map(|&p| grid.map(p) + drop).collect();
    let mut mesh = Mesh::default();
    for &p in &mapped {
        mesh.colored_vertex(p, color);
    }
    for [a, b, c] in svg::triangulate(&mapped) {
        mesh.add_triangle(a as u32, b as u32, c as u32);
    }
    painter.add(Shape::mesh(mesh));
    if needle > 0.01 {
        let top = grid.map(pos2(12.0, 17.0)) + drop;
        let bottom = top + vec2(0.0, 5.0 * grid.scale * needle);
        let half = grid.scale;
        painter.rect_filled(
            Rect::from_min_max(pos2(top.x - half, top.y), pos2(top.x + half, bottom.y)),
            0.0,
            color,
        );
    }
}

/// The minimize button: a 10×2 bar.
pub fn paint_minimize(painter: &Painter, rect: Rect, color: Color32) {
    painter.rect_filled(
        Rect::from_center_size(rect.center(), vec2(10.0, 2.0)),
        0.0,
        color,
    );
}

/// The close button: a 10×10 cross, 2 wide.
pub fn paint_close(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let stroke = Stroke::new(2.0, color);
    painter.line_segment([c + vec2(-4.5, -4.5), c + vec2(4.5, 4.5)], stroke);
    painter.line_segment([c + vec2(4.5, -4.5), c + vec2(-4.5, 4.5)], stroke);
}

/// Maps the 24×24 grid into a rect, with an optional rotation.
struct Grid {
    center: Pos2,
    scale: f32,
    sin: f32,
    cos: f32,
}

impl Grid {
    fn new(rect: Rect, angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            center: rect.center(),
            scale: rect.width().min(rect.height()) / 24.0,
            sin,
            cos,
        }
    }

    fn map(&self, p: Pos2) -> Pos2 {
        let v = (p - pos2(12.0, 12.0)) * self.scale;
        self.center
            + vec2(
                self.cos * v.x - self.sin * v.y,
                self.sin * v.x + self.cos * v.y,
            )
    }
}

/// Strokes a polyline with round ends and round sharp corners, like SVG's
/// round line caps and joins (egui draws mitered joins).
fn stroke_path(painter: &Painter, points: Vec<Pos2>, closed: bool, stroke: Stroke) {
    let n = points.len();
    if n < 2 {
        return;
    }
    let radius = stroke.width / 2.0;
    for i in 0..n {
        let end = !closed && (i == 0 || i == n - 1);
        let sharp = !end && {
            let prev = points[(i + n - 1) % n];
            let next = points[(i + 1) % n];
            let (a, b) = (points[i] - prev, next - points[i]);
            let (la, lb) = (a.length(), b.length());
            la > 1e-4 && lb > 1e-4 && a.dot(b) / (la * lb) < 0.75
        };
        if end || sharp {
            painter.circle_filled(points[i], radius, stroke.color);
        }
    }
    if closed {
        painter.add(Shape::closed_line(points, stroke));
    } else {
        painter.add(Shape::line(points, stroke));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_parses_inside_the_grid() {
        for icon in Icon::ALL {
            let paths = icon.geometry();
            assert!(!paths.is_empty(), "{icon:?} has no paths");
            for p in paths.iter().flat_map(|p| &p.points) {
                assert!(
                    (-0.5..=24.5).contains(&p.x) && (-0.5..=24.5).contains(&p.y),
                    "{icon:?} point {p:?} is off the grid"
                );
            }
        }
    }

    #[test]
    fn pin_body_triangulates() {
        let points: Vec<Pos2> = svg::parse(l::PIN_BODY)
            .into_iter()
            .flat_map(|p| p.points)
            .collect();
        assert!(points.len() > 10);
        assert!(svg::triangulate(&points).len() >= points.len() / 2);
    }
}
