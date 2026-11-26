//! Line icons drawn with the painter, so no icon font or image files are
//! needed. Each icon is drawn on a 24×24 grid and scaled to its rect, in the
//! style of the BlurAutoClicker title bar (2-unit strokes, round ends).

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2, vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Settings,
    Crosshair,
    Image,
    Sliders,
    History,
    Palette,
    Pin,
    Minimize,
    Close,
    Copy,
    Keyboard,
    Clear,
    ChevronDown,
    Folder,
    Paste,
    Plus,
    Minus,
    Fit,
    Trash,
    Check,
}

/// Paints `icon` centered in `rect`, as large as its shorter side.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let pen = Pen::new(painter, rect, color);
    match icon {
        Icon::Settings => {
            for k in 0..8 {
                let angle = k as f32 * std::f32::consts::FRAC_PI_4;
                let dir = vec2(angle.cos(), angle.sin());
                let inner = Pos2::new(12.0, 12.0) + dir * 6.8;
                let outer = Pos2::new(12.0, 12.0) + dir * 9.8;
                pen.thick(inner, outer, 3.4);
            }
            pen.circle(12.0, 12.0, 7.0);
            pen.circle(12.0, 12.0, 2.8);
        }
        Icon::Crosshair => {
            pen.circle(12.0, 12.0, 7.0);
            pen.line(&[(12.0, 1.5), (12.0, 6.0)]);
            pen.line(&[(12.0, 18.0), (12.0, 22.5)]);
            pen.line(&[(1.5, 12.0), (6.0, 12.0)]);
            pen.line(&[(18.0, 12.0), (22.5, 12.0)]);
            pen.dot(12.0, 12.0, 1.7);
        }
        Icon::Image => {
            pen.rect(3.0, 4.0, 21.0, 20.0, 2.5);
            pen.circle(8.5, 9.5, 1.8);
            pen.line(&[(21.0, 15.5), (16.0, 10.5), (5.5, 20.0)]);
        }
        Icon::Sliders => {
            pen.line(&[(21.0, 5.0), (14.0, 5.0)]);
            pen.line(&[(10.0, 5.0), (3.0, 5.0)]);
            pen.line(&[(21.0, 12.0), (12.0, 12.0)]);
            pen.line(&[(8.0, 12.0), (3.0, 12.0)]);
            pen.line(&[(21.0, 19.0), (16.0, 19.0)]);
            pen.line(&[(12.0, 19.0), (3.0, 19.0)]);
            pen.line(&[(14.0, 2.5), (14.0, 7.5)]);
            pen.line(&[(8.0, 9.5), (8.0, 14.5)]);
            pen.line(&[(16.0, 16.5), (16.0, 21.5)]);
        }
        Icon::History => {
            // An arc from 9 o'clock round the bottom and back up to the top
            // left (screen angles, y down), an arrow head there, clock hands.
            let arc: Vec<(f32, f32)> = (0..=28)
                .map(|i| {
                    let rad = (180.0 - i as f32 * (315.0 / 28.0)).to_radians();
                    (12.0 + 9.0 * rad.cos(), 12.0 + 9.0 * rad.sin())
                })
                .collect();
            pen.line(&arc);
            pen.line(&[(3.0, 3.5), (3.0, 8.0), (7.5, 8.0)]);
            pen.line(&[(12.0, 7.0), (12.0, 12.0), (15.5, 14.0)]);
        }
        Icon::Palette => {
            pen.rect(3.5, 3.5, 10.5, 10.5, 1.8);
            pen.rect(13.5, 3.5, 20.5, 10.5, 1.8);
            pen.rect(3.5, 13.5, 10.5, 20.5, 1.8);
            pen.rect(13.5, 13.5, 20.5, 20.5, 1.8);
        }
        Icon::Pin => {
            pen.line(&[(12.0, 16.0), (12.0, 21.5)]);
            pen.closed(&[
                (9.0, 3.5),
                (15.0, 3.5),
                (15.0, 10.0),
                (18.5, 13.5),
                (18.5, 16.0),
                (5.5, 16.0),
                (5.5, 13.5),
                (9.0, 10.0),
            ]);
        }
        Icon::Minimize | Icon::Minus => pen.line(&[(6.0, 12.0), (18.0, 12.0)]),
        Icon::Close => {
            pen.line(&[(6.5, 6.5), (17.5, 17.5)]);
            pen.line(&[(17.5, 6.5), (6.5, 17.5)]);
        }
        Icon::Clear => {
            pen.line(&[(8.0, 8.0), (16.0, 16.0)]);
            pen.line(&[(16.0, 8.0), (8.0, 16.0)]);
        }
        Icon::Copy => {
            pen.rect(8.5, 8.5, 20.5, 20.5, 2.0);
            pen.line(&[
                (5.0, 15.5),
                (3.5, 15.5),
                (3.5, 3.5),
                (15.5, 3.5),
                (15.5, 5.0),
            ]);
        }
        Icon::Keyboard => {
            pen.rect(2.5, 6.0, 21.5, 18.0, 2.0);
            for x in [6.5, 10.2, 13.8, 17.5] {
                pen.dot(x, 10.0, 1.0);
            }
            pen.line(&[(8.0, 14.0), (16.0, 14.0)]);
        }
        Icon::ChevronDown => pen.line(&[(6.5, 9.5), (12.0, 15.0), (17.5, 9.5)]),
        Icon::Folder => pen.closed(&[
            (3.0, 6.0),
            (9.5, 6.0),
            (11.5, 8.5),
            (21.0, 8.5),
            (21.0, 19.0),
            (3.0, 19.0),
        ]),
        Icon::Paste => {
            pen.rect(5.0, 4.5, 19.0, 21.5, 2.0);
            pen.rect(9.0, 2.5, 15.0, 6.5, 1.0);
        }
        Icon::Plus => {
            pen.line(&[(12.0, 5.0), (12.0, 19.0)]);
            pen.line(&[(5.0, 12.0), (19.0, 12.0)]);
        }
        Icon::Fit => {
            pen.line(&[(3.5, 8.5), (3.5, 3.5), (8.5, 3.5)]);
            pen.line(&[(15.5, 3.5), (20.5, 3.5), (20.5, 8.5)]);
            pen.line(&[(20.5, 15.5), (20.5, 20.5), (15.5, 20.5)]);
            pen.line(&[(8.5, 20.5), (3.5, 20.5), (3.5, 15.5)]);
        }
        Icon::Trash => {
            pen.line(&[(3.5, 6.5), (20.5, 6.5)]);
            pen.line(&[(9.0, 6.5), (9.0, 3.5), (15.0, 3.5), (15.0, 6.5)]);
            pen.line(&[(5.5, 6.5), (6.5, 21.0), (17.5, 21.0), (18.5, 6.5)]);
        }
        Icon::Check => pen.line(&[(5.0, 12.5), (10.0, 17.5), (19.0, 7.0)]),
    }
}

/// Draws on the 24×24 grid, scaled into the target rect.
struct Pen<'a> {
    painter: &'a Painter,
    origin: Pos2,
    scale: f32,
    stroke: Stroke,
}

impl<'a> Pen<'a> {
    fn new(painter: &'a Painter, rect: Rect, color: Color32) -> Self {
        let scale = rect.width().min(rect.height()) / 24.0;
        Self {
            painter,
            origin: rect.center() - Vec2::splat(12.0 * scale),
            scale,
            stroke: Stroke::new((2.0 * scale).max(1.2), color),
        }
    }

    fn p(&self, (x, y): (f32, f32)) -> Pos2 {
        self.origin + vec2(x, y) * self.scale
    }

    /// An open polyline with round ends.
    fn line(&self, points: &[(f32, f32)]) {
        let points: Vec<Pos2> = points.iter().map(|&p| self.p(p)).collect();
        let cap = self.stroke.width / 2.0;
        if let (Some(&first), Some(&last)) = (points.first(), points.last()) {
            self.painter.circle_filled(first, cap, self.stroke.color);
            self.painter.circle_filled(last, cap, self.stroke.color);
        }
        self.painter.add(Shape::line(points, self.stroke));
    }

    fn closed(&self, points: &[(f32, f32)]) {
        let points = points.iter().map(|&p| self.p(p)).collect();
        self.painter.add(Shape::closed_line(points, self.stroke));
    }

    /// A short, wide segment (gear teeth), in grid units.
    fn thick(&self, from: Pos2, to: Pos2, width: f32) {
        let stroke = Stroke::new(width * self.scale, self.stroke.color);
        self.painter
            .line_segment([self.p((from.x, from.y)), self.p((to.x, to.y))], stroke);
    }

    fn circle(&self, x: f32, y: f32, r: f32) {
        self.painter
            .circle_stroke(self.p((x, y)), r * self.scale, self.stroke);
    }

    fn dot(&self, x: f32, y: f32, r: f32) {
        self.painter
            .circle_filled(self.p((x, y)), r * self.scale, self.stroke.color);
    }

    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, radius: f32) {
        let rect = Rect::from_min_max(self.p((x0, y0)), self.p((x1, y1)));
        self.painter
            .rect_stroke(rect, radius * self.scale, self.stroke, StrokeKind::Middle);
    }
}
