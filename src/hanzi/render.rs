//! Draws a character of the hanzi mode into an image with tiny-skia: the practice grid (米字格),
//! the outlines of the strokes as the stage shows them, the strokes drawn and the user's ink, the
//! brush guide of the next stroke, and the small images of the breakdown of a word.
use bevy::color::{Color, ColorToComponents, Srgba};
use bevy::math::Vec2;
use tiny_skia::{FillRule, LineCap, LineJoin, Mask, Paint, PathBuilder, Pixmap, PixmapPaint, Stroke, StrokeDash, Transform};

use super::brush::Guide;
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
    /// The next stroke written before the eyes, the glimpse after the misses: the part of it
    /// up to `t` (0 to 1) of its median, with its opacity.
    Reveal(f32, f32),
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
    /// The path and the presses of the brush guide.
    pub guide: Color,
}

/// What the canvas shows besides the character.
#[derive(Default)]
pub struct Frame<'a> {
    pub looks: &'a [Look],
    /// The stroke being drawn, in pixels, and its widths with the brush, empty without.
    pub ink: &'a [Vec2],
    pub ink_widths: &'a [f32],
    /// The last rejected stroke, fading with its alpha.
    pub wrong: Option<(&'a [Vec2], f32)>,
    /// The last accepted brush stroke, fading over the character, to compare its thickness.
    pub echo: Option<(&'a [Vec2], &'a [f32], f32)>,
    /// The brush guide of the next stroke.
    pub guide: Option<&'a Guide>,
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

fn grid(pixmap: &mut Pixmap, colors: &Colors, ts: Transform) {
    let s = SIZE as f32;
    let mut stroke = Stroke { width: 2.0, ..Default::default() };
    let border = tiny_skia::Rect::from_xywh(2.0, 2.0, s - 4.0, s - 4.0).map(PathBuilder::from_rect);
    if let Some(border) = border {
        pixmap.stroke_path(&border, &paint(colors.grid, 1.0), &stroke, ts, None);
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
            pixmap.stroke_path(&path, &paint(colors.grid, 0.8), &stroke, ts, None);
        }
    }
}

fn clear(pixmap: &mut Pixmap, colors: &Colors) {
    let [r, g, b, _] = Srgba::from(colors.paper).to_f32_array();
    pixmap.fill(tiny_skia::Color::from_rgba(r, g, b, 1.0).unwrap_or(tiny_skia::Color::WHITE));
}

fn strokes(pixmap: &mut Pixmap, glyph: &Glyph, looks: &[Look], colors: &Colors, ts: Transform) {
    for (i, (ops, look)) in glyph.outlines.iter().zip(looks).enumerate() {
        let Some(path) = outline_path(ops) else {
            continue;
        };
        let (paint, mask) = match *look {
            Look::Hidden => continue,
            Look::Outline(alpha) => (paint(colors.outline, alpha), None),
            Look::Next(alpha) => (paint(colors.next, alpha), None),
            Look::Reveal(t, alpha) => {
                (paint(colors.next, alpha), glyph.medians.get(i).and_then(|m| reveal_mask(pixmap, m, t, ts)))
            }
            Look::Done => (paint(colors.ink, 1.0), None),
        };
        pixmap.fill_path(&path, &paint, FillRule::Winding, ts, mask.as_ref());
    }
}

/// The part of a stroke written up to `t` of its median: a wide band along the median, cut
/// square where the brush is, a little longer at the start for the outline before the median.
fn reveal_mask(pixmap: &Pixmap, median: &[Vec2], t: f32, ts: Transform) -> Option<Mask> {
    if t >= 1.0 || median.len() < 2 {
        return None;
    }
    let points: Vec<Vec2> = median.iter().map(|p| to_pixels(*p)).collect();
    let total: f32 = points.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut left = total * t.max(0.0);
    let back = (points[0] - points[1]).normalize_or_zero();
    let mut part = vec![points[0] + back * 30.0, points[0]];
    for w in points.windows(2) {
        let d = w[0].distance(w[1]);
        if d >= left {
            part.push(w[0].lerp(w[1], left / d.max(1e-6)));
            break;
        }
        left -= d;
        part.push(w[1]);
    }
    let band = Stroke { width: 140.0, line_cap: LineCap::Butt, line_join: LineJoin::Round, ..Default::default() };
    let path = polyline(&part)?.stroke(&band, 1.0)?;
    let mut mask = Mask::new(pixmap.width(), pixmap.height())?;
    mask.fill_path(&path, FillRule::Winding, true, ts);
    Some(mask)
}

/// The ink of a brush: discs at the points joined by quads, their widths varying, drawn opaque
/// on a layer then laid with the alpha, the overlaps do not darken.
fn brush_ink(pixmap: &mut Pixmap, points: &[Vec2], widths: &[f32], color: Color, alpha: f32) {
    let (Some(first), Some(mut layer)) = (points.first(), Pixmap::new(pixmap.width(), pixmap.height())) else {
        return;
    };
    let width = |i: usize| widths.get(i).copied().unwrap_or(super::brush::INK_MIN) / 2.0;
    let mut pb = PathBuilder::new();
    pb.push_circle(first.x, first.y, width(0));
    for (i, w) in points.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        let n = (b - a).normalize_or_zero().perp();
        pb.push_circle(b.x, b.y, width(i + 1));
        if n != Vec2::ZERO {
            let corners = [a + n * width(i), b + n * width(i + 1), b - n * width(i + 1), a - n * width(i)];
            pb.move_to(corners[0].x, corners[0].y);
            for c in &corners[1..] {
                pb.line_to(c.x, c.y);
            }
            pb.close();
        }
    }
    if let Some(path) = pb.finish() {
        layer.fill_path(&path, &paint(color, 1.0), FillRule::Winding, Transform::identity(), None);
        pixmap.draw_pixmap(0, 0, layer.as_ref(), &PixmapPaint { opacity: alpha, ..Default::default() }, Transform::identity(), None);
    }
}

/// The guide of the brush, like the stroke charts of calligraphy: the path with an arrow, a ring
/// as wide as the stroke where the brush presses, and the hook of the tip going in against the
/// direction (藏锋).
pub fn guide(pixmap: &mut Pixmap, guide: &Guide, colors: &Colors, ts: Transform) {
    let points: Vec<Vec2> = guide.path.iter().map(|p| to_pixels(*p)).collect();
    if points.len() < 2 {
        return;
    }
    let scale = SIZE as f32 / BOX;
    let line = Stroke { width: 3.0, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() };
    let color = paint(colors.guide, 0.95);
    if let Some(path) = polyline(&points) {
        pixmap.stroke_path(&path, &color, &line, ts, None);
    }
    // The arrow at the end.
    let end = points[points.len() - 1];
    let dir = (end - points[points.len().saturating_sub(4)]).normalize_or_zero();
    if dir != Vec2::ZERO {
        let back = end - dir * 18.0;
        let side = dir.perp() * 10.0;
        if let Some(path) = polyline(&[back + side, end, back - side]) {
            pixmap.stroke_path(&path, &color, &line, ts, None);
        }
    }
    // The tip goes in against the direction, toward the upper left, then turns into the stroke.
    let start = points[0];
    let first = (points[points.len().min(4) - 1] - start).normalize_or_zero();
    if guide.hidden_entry && first != Vec2::ZERO {
        // The side of the stroke toward the top of the image, or its left for a vertical.
        let mut side = first.perp();
        if side.y > 0.1 || (side.y.abs() <= 0.1 && side.x > 0.0) {
            side = -side;
        }
        let r = 22.0;
        let from = start - first * r * 0.6 + side * r;
        let control = start - first * r * 0.9 + side * r * 0.1;
        let mut pb = PathBuilder::new();
        pb.move_to(from.x, from.y);
        pb.quad_to(control.x, control.y, start.x + first.x * 6.0, start.y + first.y * 6.0);
        if let Some(path) = pb.finish() {
            let thin = Stroke { width: 2.0, dash: StrokeDash::new(vec![4.0, 3.0], 0.0), ..line.clone() };
            pixmap.stroke_path(&path, &color, &thin, ts, None);
        }
    }
    // The presses.
    let ring = Stroke { width: 2.5, ..Default::default() };
    for (center, width) in &guide.presses {
        let c = to_pixels(*center);
        if let Some(path) = PathBuilder::from_circle(c.x, c.y, (width * scale / 2.0).max(6.0)) {
            pixmap.stroke_path(&path, &color, &ring, ts, None);
        }
    }
    // A dot where it starts.
    if let Some(path) = PathBuilder::from_circle(start.x, start.y, 5.0) {
        pixmap.fill_path(&path, &color, FillRule::Winding, ts, None);
    }
}

/// The character with the looks of its strokes, the ink of the stroke being drawn, of the last
/// rejected stroke and of the last brush stroke, and the brush guide.
pub fn draw(pixmap: &mut Pixmap, glyph: &Glyph, frame: &Frame, colors: &Colors) {
    let ts = Transform::identity();
    clear(pixmap, colors);
    grid(pixmap, colors, ts);
    strokes(pixmap, glyph, frame.looks, colors, ts);
    if let Some((points, widths, alpha)) = frame.echo {
        brush_ink(pixmap, points, widths, colors.user, 0.7 * alpha);
    }
    if let Some(g) = frame.guide {
        guide(pixmap, g, colors, ts);
    }
    let pen = Stroke { width: 22.0, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() };
    if let Some((points, alpha)) = frame.wrong
        && let Some(path) = polyline(points)
    {
        pixmap.stroke_path(&path, &paint(colors.wrong, alpha), &pen, ts, None);
    }
    if !frame.ink_widths.is_empty() {
        brush_ink(pixmap, frame.ink, frame.ink_widths, colors.user, 0.85);
    } else if let Some(path) = polyline(frame.ink) {
        pixmap.stroke_path(&path, &paint(colors.user, 0.85), &pen, ts, None);
    }
}

/// A stroke of the character in a small image: the strokes before it in ink, it in the color of
/// the next stroke, the ones after faint, and its brush guide when given.
pub fn draw_stroke(pixmap: &mut Pixmap, glyph: &Glyph, stroke: usize, guide_of: Option<&Guide>, colors: &Colors) {
    let s = pixmap.width() as f32 / SIZE as f32;
    let ts = Transform::from_scale(s, s);
    clear(pixmap, colors);
    grid(pixmap, colors, ts);
    let looks: Vec<Look> = (0..glyph.outlines.len())
        .map(|i| match i.cmp(&stroke) {
            std::cmp::Ordering::Less => Look::Done,
            std::cmp::Ordering::Equal => Look::Next(if guide_of.is_some() { 0.45 } else { 1.0 }),
            std::cmp::Ordering::Greater => Look::Outline(0.18),
        })
        .collect();
    strokes(pixmap, glyph, &looks, colors, ts);
    if let Some(g) = guide_of {
        guide(pixmap, g, colors, ts);
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
            guide: Color::srgb(1.0, 0.0, 0.0),
        };
        let mut pixmap = Pixmap::new(SIZE, SIZE).expect("pixmap");
        let frame = Frame { looks: &[Look::Done, Look::Outline(0.3)], ..Default::default() };
        draw(&mut pixmap, glyph, &frame, &colors);
        let dark = pixmap.pixels().iter().filter(|p| p.red() < 50).count();
        assert!(dark > 1000, "{dark}");
        // The glimpse: half of the second stroke is less than all of it.
        let blue = |pixmap: &Pixmap| pixmap.pixels().iter().filter(|p| p.blue() > 200 && p.red() < 80).count();
        let half = Frame { looks: &[Look::Done, Look::Reveal(0.5, 1.0)], ..Default::default() };
        draw(&mut pixmap, glyph, &half, &colors);
        let n_half = blue(&pixmap);
        let whole = Frame { looks: &[Look::Done, Look::Reveal(1.0, 1.0)], ..Default::default() };
        draw(&mut pixmap, glyph, &whole, &colors);
        let n_whole = blue(&pixmap);
        assert!(n_half > 500 && n_half < n_whole * 3 / 4, "{n_half} of {n_whole}");
        // A small image of the second stroke, with its guide in red.
        let mut tile = Pixmap::new(SIZE, SIZE).expect("tile");
        let profile = super::super::brush::profile(&glyph.outlines[1], &glyph.medians[1]);
        let g = super::super::brush::guide(&profile, glyph.kinds[1]);
        draw_stroke(&mut tile, glyph, 1, Some(&g), &colors);
        let red = tile.pixels().iter().filter(|p| p.red() > 200 && p.green() < 80 && p.blue() < 80).count();
        assert!(red > 50, "{red}");
    }
}
