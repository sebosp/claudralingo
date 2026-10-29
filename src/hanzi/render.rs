//! Draws a character of the hanzi mode into an image with tiny-skia: the practice grid (米字格),
//! the outlines of the strokes as the stage shows them, the strokes drawn and the user's ink.
use bevy::color::{Color, ColorToComponents, Srgba};
use bevy::math::Vec2;
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, StrokeDash, Transform};

use super::data::{BASELINE, BOX, Glyph, PathOp};

/// The side of the image in pixels.
pub const SIZE: u32 = 512;

/// How a stroke of the character shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Look {
    Hidden,
    /// The outline to follow, with its opacity.
    Outline(f32),
    /// The next stroke, highlighted.
    Next(f32),
    /// Drawn already, in ink.
    Done,
}

pub struct Colors {
    pub paper: Color,
    pub grid: Color,
    pub outline: Color,
    pub next: Color,
    pub ink: Color,
    pub user: Color,
    pub wrong: Color,
}

fn paint(color: Color, alpha: f32) -> Paint<'static> {
    let [r, g, b, a] = Srgba::from(color).to_f32_array();
    let mut paint = Paint::default();
    paint.set_color_rgba8((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, (a * alpha * 255.0) as u8);
    paint.anti_alias = true;
    paint
}

/// From the 1024 box of Make Me a Hanzi, its y upwards, to the pixels of the image.
pub fn to_pixels(p: Vec2) -> Vec2 {
    let s = SIZE as f32 / BOX;
    Vec2::new(p.x * s, (BASELINE - p.y) * s)
}

/// From the pixels of the image to the 1024 box.
pub fn from_pixels(p: Vec2) -> Vec2 {
    let s = SIZE as f32 / BOX;
    Vec2::new(p.x / s, BASELINE - p.y / s)
}

fn outline_path(ops: &[PathOp]) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    for op in ops {
        match *op {
            PathOp::Move(p) => {
                let p = to_pixels(p);
                pb.move_to(p.x, p.y);
            }
            PathOp::Line(p) => {
                let p = to_pixels(p);
                pb.line_to(p.x, p.y);
            }
            PathOp::Quad(c, p) => {
                let (c, p) = (to_pixels(c), to_pixels(p));
                pb.quad_to(c.x, c.y, p.x, p.y);
            }
            PathOp::Cubic(c1, c2, p) => {
                let (c1, c2, p) = (to_pixels(c1), to_pixels(c2), to_pixels(p));
                pb.cubic_to(c1.x, c1.y, c2.x, c2.y, p.x, p.y);
            }
            PathOp::Close => pb.close(),
        }
    }
    pb.finish()
}

fn polyline(points: &[Vec2]) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    let first = points.first()?;
    pb.move_to(first.x, first.y);
    if points.len() == 1 {
        pb.line_to(first.x + 0.1, first.y);
    }
    for p in &points[1..] {
        pb.line_to(p.x, p.y);
    }
    pb.finish()
}

fn grid(pixmap: &mut Pixmap, colors: &Colors) {
    let s = SIZE as f32;
    let mut stroke = Stroke { width: 2.0, ..Default::default() };
    let border = tiny_skia::Rect::from_xywh(2.0, 2.0, s - 4.0, s - 4.0).map(PathBuilder::from_rect);
    if let Some(border) = border {
        pixmap.stroke_path(&border, &paint(colors.grid, 1.0), &stroke, Transform::identity(), None);
    }
    stroke.width = 1.5;
    stroke.dash = StrokeDash::new(vec![10.0, 8.0], 0.0);
    for (a, b) in [
        ((0.0, s / 2.0), (s, s / 2.0)),
        ((s / 2.0, 0.0), (s / 2.0, s)),
        ((0.0, 0.0), (s, s)),
        ((s, 0.0), (0.0, s)),
    ] {
        if let Some(path) = polyline(&[Vec2::new(a.0, a.1), Vec2::new(b.0, b.1)]) {
            pixmap.stroke_path(&path, &paint(colors.grid, 0.8), &stroke, Transform::identity(), None);
        }
    }
}

/// The character with the looks of its strokes, the ink of the stroke being drawn and of the
/// last rejected stroke, in pixels, the rejected one fading with `wrong_alpha`.
pub fn draw(
    pixmap: &mut Pixmap,
    glyph: &Glyph,
    looks: &[Look],
    ink: &[Vec2],
    wrong: Option<(&[Vec2], f32)>,
    colors: &Colors,
) {
    let [r, g, b, _] = Srgba::from(colors.paper).to_f32_array();
    pixmap.fill(tiny_skia::Color::from_rgba(r, g, b, 1.0).unwrap_or(tiny_skia::Color::WHITE));
    grid(pixmap, colors);
    for (ops, look) in glyph.outlines.iter().zip(looks) {
        let Some(path) = outline_path(ops) else {
            continue;
        };
        let paint = match *look {
            Look::Hidden => continue,
            Look::Outline(alpha) => paint(colors.outline, alpha),
            Look::Next(alpha) => paint(colors.next, alpha),
            Look::Done => paint(colors.ink, 1.0),
        };
        pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }
    let pen = Stroke { width: 22.0, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() };
    if let Some((points, alpha)) = wrong
        && let Some(path) = polyline(points)
    {
        pixmap.stroke_path(&path, &paint(colors.wrong, alpha), &pen, Transform::identity(), None);
    }
    if let Some(path) = polyline(ink) {
        pixmap.stroke_path(&path, &paint(colors.user, 0.85), &pen, Transform::identity(), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_round_trip() {
        let p = Vec2::new(300.0, 120.0);
        assert!((from_pixels(to_pixels(p)) - p).length() < 1e-3);
        // The top of the box is the top of the image.
        assert!(to_pixels(Vec2::new(0.0, BASELINE)).y.abs() < 1e-3);
    }

    #[test]
    fn draws_a_character() {
        let data = super::super::data::HanziData::load();
        let glyph = data.glyph('人').expect("人");
        let colors = Colors {
            paper: Color::WHITE,
            grid: Color::srgb(0.8, 0.8, 0.8),
            outline: Color::srgb(0.5, 0.5, 0.5),
            next: Color::srgb(0.0, 0.5, 1.0),
            ink: Color::BLACK,
            user: Color::srgb(0.0, 0.0, 1.0),
            wrong: Color::srgb(1.0, 0.0, 0.0),
        };
        let mut pixmap = Pixmap::new(SIZE, SIZE).expect("pixmap");
        draw(&mut pixmap, glyph, &[Look::Done, Look::Outline(0.3)], &[], None, &colors);
        let dark = pixmap.pixels().iter().filter(|p| p.red() < 50).count();
        assert!(dark > 1000, "{dark}");
    }
}
