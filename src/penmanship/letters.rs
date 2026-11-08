//! The geometry of the penmanship mode: the guide sheets of Spencerian and Copperplate, the
//! drills and the small letters, built from the principles of the *Theory of Spencerian
//! Penmanship* (1874, public domain): the straight line on the main slant, the right and the
//! left curves on the connective slant and the extended loop.
//!
//! A letter is drawn upright, its x-height 1 and its stems vertical, then sheared to the slant
//! of the style. The connective upstrokes of 30° become a rise of about one space for one space
//! to the right once upright, as the 1874 measurements give them.
use bevy::math::Vec2;

/// The writing styles, their sheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Style {
    #[default]
    Spencerian,
    Copperplate,
}

impl Style {
    pub const ALL: [Style; 2] = [Style::Spencerian, Style::Copperplate];

    pub fn name(self) -> &'static str {
        match self {
            Style::Spencerian => "Spencerian",
            Style::Copperplate => "Copperplate",
        }
    }

    pub fn metrics(self) -> Metrics {
        match self {
            // 52°, a 2:1:2 sheet: the loops 3 spaces high, the descenders 2 below.
            Style::Spencerian => Metrics { slant: 52.0, ascender: 3.0, half_ascender: 2.0, descender: -2.0, half_descender: -1.5 },
            // 55°, the 3:2:3 sheet: the ascenders and the descenders 1.5 x-heights.
            Style::Copperplate => Metrics { slant: 55.0, ascender: 2.5, half_ascender: 1.75, descender: -1.5, half_descender: -0.75 },
        }
    }
}

/// The lines of a sheet in x-heights above the base line, and the main slant in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub slant: f32,
    pub ascender: f32,
    pub half_ascender: f32,
    pub descender: f32,
    pub half_descender: f32,
}

impl Metrics {
    /// From the upright drawing to the slant: the stems lean right.
    pub fn shear(&self, p: Vec2) -> Vec2 {
        Vec2::new(p.x + p.y / self.slant.to_radians().tan(), p.y)
    }

    /// The heights of the drawing: the loops and the descenders of the Spencerian figures are
    /// moved to the lines of the style.
    fn fit(&self, p: Vec2) -> Vec2 {
        let y = if p.y > 2.0 {
            self.half_ascender + (p.y - 2.0) * (self.ascender - self.half_ascender)
        } else if p.y > 1.0 {
            1.0 + (p.y - 1.0) * (self.half_ascender - 1.0)
        } else if p.y < -1.5 {
            self.half_descender - (p.y + 1.5) * 2.0 * (self.descender - self.half_descender)
        } else if p.y < 0.0 {
            p.y / -1.5 * self.half_descender
        } else {
            p.y
        };
        Vec2::new(p.x, y)
    }
}

/// A stroke to write in one movement, or a dot (the i, the j) touched anywhere near it.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    /// In x-heights, upright, the base line at 0.
    pub points: Vec<Vec2>,
    pub dot: bool,
    /// Where the counts of the 1874 method begin, indices in `points`: the first point, then
    /// each turn of the movement, the 1-2-3 of the copybooks.
    pub counts: Vec<usize>,
}

/// A figure to write: a letter, or a row of a drill.
#[derive(Debug, Clone, PartialEq)]
pub struct Figure {
    pub name: &'static str,
    /// How to write it, the count of the 1874 method.
    pub how: &'static str,
    pub strokes: Vec<Stroke>,
}

impl Figure {
    /// The strokes on the sheet of a style: fitted to its lines and sheared to its slant.
    pub fn on_sheet(&self, style: Style) -> Vec<Stroke> {
        let m = style.metrics();
        self.strokes
            .iter()
            .map(|s| Stroke { points: s.points.iter().map(|p| m.shear(m.fit(*p))).collect(), dot: s.dot, counts: s.counts.clone() })
            .collect()
    }
}

/// A lesson: its name and its figures.
pub struct Lesson {
    pub name: &'static str,
    pub figures: Vec<Figure>,
}

/// The points of a cubic Bézier curve after its start.
const SAMPLES: usize = 16;

/// Draws a stroke with curves and lines, upright. Each principle (a curve, a straight line, a
/// loop) is one count, marked where it begins.
struct Pen {
    points: Vec<Vec2>,
    counts: Vec<usize>,
}

impl Pen {
    fn at(x: f32, y: f32) -> Pen {
        Pen { points: vec![Vec2::new(x, y)], counts: vec![0] }
    }

    fn here(&self) -> Vec2 {
        *self.points.last().expect("a pen has a point")
    }

    /// A count begins here.
    fn mark(mut self) -> Pen {
        let at = self.points.len() - 1;
        if self.counts.last() != Some(&at) {
            self.counts.push(at);
        }
        self
    }

    /// A straight line, one count.
    fn line(self, x: f32, y: f32) -> Pen {
        self.mark().seg_line(x, y)
    }

    /// A curve, one count.
    fn curve(self, c1: (f32, f32), c2: (f32, f32), end: (f32, f32)) -> Pen {
        self.mark().seg_curve(c1, c2, end)
    }

    fn seg_curve(mut self, c1: (f32, f32), c2: (f32, f32), end: (f32, f32)) -> Pen {
        let (p0, c1, c2, p3) = (self.here(), Vec2::from(c1), Vec2::from(c2), Vec2::from(end));
        for i in 1..=SAMPLES {
            let t = i as f32 / SAMPLES as f32;
            let u = 1.0 - t;
            self.points.push(u * u * u * p0 + 3.0 * u * u * t * c1 + 3.0 * u * t * t * c2 + t * t * t * p3);
        }
        self
    }

    fn seg_line(mut self, x: f32, y: f32) -> Pen {
        let (from, to) = (self.here(), Vec2::new(x, y));
        let n = ((to - from).length() * 8.0).ceil().max(1.0) as usize;
        for i in 1..=n {
            self.points.push(from.lerp(to, i as f32 / n as f32));
        }
        self
    }

    /// Principle 2, the right curve, up to `(x, y)`: it leaves the line low and rises at the
    /// end, its belly down and to the right.
    fn right_curve(self, x: f32, y: f32) -> Pen {
        let a = self.here();
        let d = Vec2::new(x, y) - a;
        self.mark().seg_curve((a.x + 0.5 * d.x, a.y + 0.04 * d.y), (x - 0.3 * d.x, y - 0.45 * d.y), (x, y))
    }

    /// Principle 3, the left curve, up and over to the top of a stem at `x`: it rises first and
    /// turns at the head line, a short turn into the downstroke.
    fn overturn(self, x: f32, top: f32) -> Pen {
        let a = self.here();
        let d = x - a.x;
        self.mark().seg_curve((a.x + 0.1 * d, a.y + 0.6 * (top - a.y)), (x - 0.35 * d, top + 0.04), (x, top - 0.2))
    }

    /// Principle 1 down to `y`, ending with a short turn into the next upstroke.
    fn down_turn(self, y: f32) -> Pen {
        let x = self.here().x;
        self.mark().seg_line(x, y + 0.22).seg_curve((x, y + 0.04), (x + 0.06, y), (x + 0.24, y + 0.04))
    }

    /// Principle 1 down to `y`, an angle at the end.
    fn down(self, y: f32) -> Pen {
        let x = self.here().x;
        self.mark().seg_line(x, y)
    }

    /// The oval of a, d, g, q: from the head line at `right`, down the left side and up the
    /// right one back to the head line, counterclockwise.
    fn oval(self, left: f32, right: f32) -> Pen {
        let a = self.here();
        let mid = 0.5 * (left + right);
        self.mark()
            .seg_curve((a.x - 0.25, 1.06), (left, 0.85), (left, 0.45))
            .seg_curve((left, 0.08), (mid - 0.1, -0.03), (mid + 0.05, 0.02))
            .seg_curve((right - 0.05, 0.08), (right, 0.6), (right, 1.0))
    }

    /// Principle 4, the extended loop: up to the top line from the base, a short turn and down
    /// on the main slant, crossing the upstroke one space above the base. Ends at the base.
    fn loop_up(self, stem: f32) -> Pen {
        let a = self.here();
        self.mark()
            .seg_curve((a.x + 0.55, a.y + 0.4), (stem + 0.45, 2.3), (stem + 0.4, 2.8))
            .seg_curve((stem + 0.35, 3.08), (stem, 3.08), (stem, 2.7))
            .line(stem, 0.0)
    }

    /// The loop below the base: down the stem to the descender line, a turn to the left and up
    /// crossing the stem at the base, to the head line at `to`.
    fn loop_down(self, to: f32) -> Pen {
        let x = self.here().x;
        self.line(x, -1.7)
            .mark()
            .seg_curve((x, -2.08), (x - 0.4, -2.08), (x - 0.4, -1.8))
            .seg_curve((x - 0.4, -1.0), (x + 0.1, -0.2), (x + 0.4, 0.2))
            .seg_curve((x + 0.6, 0.45), (to - 0.3, 0.85), (to, 1.0))
    }

    /// The finish of b, o, v, w: a dot and a short curve to the right on the head line.
    fn finish(self) -> Pen {
        let a = self.here();
        self.mark()
            .seg_curve((a.x - 0.12, a.y - 0.1), (a.x - 0.08, a.y - 0.22), (a.x, a.y - 0.12))
            .seg_curve((a.x + 0.15, a.y - 0.02), (a.x + 0.35, a.y - 0.05), (a.x + 0.55, a.y - 0.15))
    }

    fn done(self) -> Stroke {
        Stroke { points: self.points, dot: false, counts: self.counts }
    }
}

fn dot(x: f32, y: f32) -> Stroke {
    Stroke { points: vec![Vec2::new(x, y), Vec2::new(x - 0.04, y - 0.06)], dot: true, counts: Vec::new() }
}

/// The cross of the t, a short line on the head of its stem.
fn bar(x0: f32, x1: f32, y: f32) -> Stroke {
    Pen::at(x0, y).line(x1, y).done()
}

fn figure(name: &'static str, how: &'static str, strokes: Vec<Stroke>) -> Figure {
    Figure { name, how, strokes }
}

/// The small letters, in the order of the copybooks.
pub fn letter(c: char) -> Figure {
    match c {
        'i' => figure("i", "right curve up, straight down, right curve up · 1-2-3, then the dot", vec![
            Pen::at(0.0, 0.0).right_curve(0.95, 1.0).down_turn(0.0).right_curve(2.0, 1.0).done(),
            dot(0.95, 1.7),
        ]),
        'u' => figure("u", "right curve, straight, right curve, straight, right curve · 1-2-3-4-5", vec![Pen::at(0.0, 0.0)
            .right_curve(0.95, 1.0)
            .down_turn(0.0)
            .right_curve(2.1, 1.0)
            .down_turn(0.0)
            .right_curve(3.15, 1.0)
            .done()]),
        'w' => figure("w", "as the u, a third right curve up, then the dot and the finish", vec![Pen::at(0.0, 0.0)
            .right_curve(0.95, 1.0)
            .down_turn(0.0)
            .right_curve(2.1, 1.0)
            .down_turn(0.0)
            .right_curve(3.1, 1.0)
            .finish()
            .done()]),
        'n' => figure("n", "left curve over, straight down; again; right curve up · 1-2-3-4-5", vec![Pen::at(0.0, 0.0)
            .overturn(0.55, 1.0)
            .down(0.0)
            .overturn(1.55, 1.0)
            .down_turn(0.0)
            .right_curve(2.6, 1.0)
            .done()]),
        'm' => figure("m", "three left curves over and straight down, a right curve up · 1-2-3-4-5-6-7", vec![Pen::at(0.0, 0.0)
            .overturn(0.55, 1.0)
            .down(0.0)
            .overturn(1.5, 1.0)
            .down(0.0)
            .overturn(2.45, 1.0)
            .down_turn(0.0)
            .right_curve(3.5, 1.0)
            .done()]),
        'v' => figure("v", "left curve over, straight down, right curve up, the dot and the finish", vec![Pen::at(0.0, 0.0)
            .overturn(0.55, 1.0)
            .down_turn(0.0)
            .right_curve(1.6, 1.0)
            .finish()
            .done()]),
        'x' => figure("x", "left curve over, right curve down and up; then the crossing, up on the connective slant", vec![
            Pen::at(0.0, 0.0)
                .overturn(0.55, 1.0)
                .curve((0.75, 0.6), (0.7, 0.0), (0.95, 0.02))
                .right_curve(1.9, 1.0)
                .done(),
            Pen::at(0.05, 0.05).line(1.2, 0.95).done(),
        ]),
        'o' => figure("o", "left curve up, the oval down and around, the dot and the finish", vec![Pen::at(0.0, 0.0)
            .curve((0.1, 0.6), (0.45, 1.0), (0.85, 1.0))
            .curve((0.45, 1.05), (0.25, 0.75), (0.25, 0.45))
            .curve((0.25, 0.08), (0.45, -0.03), (0.6, 0.02))
            .curve((0.8, 0.08), (0.9, 0.6), (0.85, 1.0))
            .finish()
            .done()]),
        'a' => figure("a", "right curve up, the oval, straight down, right curve up · 1-2-3-4-5", vec![Pen::at(0.0, 0.0)
            .right_curve(1.0, 1.0)
            .oval(0.35, 1.0)
            .down_turn(0.0)
            .right_curve(2.1, 1.0)
            .done()]),
        'e' => figure("e", "right curve up to a small loop, left curve down, right curve up · 1-2-3", vec![Pen::at(0.0, 0.0)
            .curve((0.5, 0.2), (1.0, 0.85), (0.72, 1.0))
            .curve((0.45, 1.1), (0.3, 0.6), (0.35, 0.35))
            .curve((0.4, 0.05), (0.6, 0.0), (0.75, 0.04))
            .right_curve(1.7, 1.0)
            .done()]),
        'c' => figure("c", "right curve up, the hook, left curve down, right curve up", vec![Pen::at(0.0, 0.0)
            .curve((0.45, 0.2), (0.85, 0.75), (0.9, 0.95))
            .curve((0.95, 1.05), (0.75, 1.08), (0.72, 0.92))
            .curve((0.5, 1.05), (0.25, 0.75), (0.3, 0.4))
            .curve((0.35, 0.05), (0.6, 0.0), (0.75, 0.04))
            .right_curve(1.7, 1.0)
            .done()]),
        'r' => figure("r", "right curve up to 1¼, a notch, straight down, right curve up", vec![Pen::at(0.0, 0.0)
            .right_curve(0.85, 1.25)
            .curve((0.95, 1.12), (1.1, 1.05), (1.25, 1.0))
            .down_turn(0.0)
            .right_curve(2.3, 1.0)
            .done()]),
        's' => figure("s", "right curve up to 1¼, the curve down to the base, the dot, right curve up", vec![Pen::at(0.0, 0.0)
            .right_curve(0.85, 1.25)
            .curve((1.25, 0.8), (1.2, 0.05), (0.75, 0.03))
            .curve((0.55, 0.02), (0.55, 0.25), (0.8, 0.12))
            .right_curve(1.8, 1.0)
            .done()]),
        't' => figure("t", "right curve up to 2, straight down, right curve up, then the cross", vec![
            Pen::at(0.0, 0.0).right_curve(0.95, 2.0).down_turn(0.0).right_curve(2.0, 1.0).done(),
            bar(0.6, 1.6, 1.5),
        ]),
        'd' => figure("d", "the oval, up the stem to 2 and straight down, right curve up", vec![Pen::at(0.0, 0.0)
            .right_curve(1.0, 1.0)
            .oval(0.35, 1.0)
            .line(1.0, 2.0)
            .down_turn(0.0)
            .right_curve(2.1, 1.0)
            .done()]),
        'p' => figure("p", "right curve up to 2, straight down to 1½ below, back up, left curve over, down, right curve up", vec![Pen::at(0.0, 0.0)
            .right_curve(0.95, 2.0)
            .down(-1.5)
            .line(0.95, 0.0)
            .overturn(1.95, 1.0)
            .down_turn(0.0)
            .right_curve(3.0, 1.0)
            .done()]),
        'q' => figure("q", "the oval, straight down to 1½ below, the turn and up to the head line", vec![Pen::at(0.0, 0.0)
            .right_curve(1.0, 1.0)
            .oval(0.35, 1.0)
            .down(-1.5)
            .curve((1.0, -1.75), (1.3, -1.6), (1.4, -1.2))
            .curve((1.5, -0.6), (1.6, 0.4), (2.2, 1.0))
            .done()]),
        'h' => figure("h", "the loop up to 3 and down, left curve over, straight down, right curve up", vec![Pen::at(0.0, 0.0)
            .loop_up(0.75)
            .overturn(1.75, 1.0)
            .down_turn(0.0)
            .right_curve(2.8, 1.0)
            .done()]),
        'k' => figure("k", "the loop, left curve over to a small knot, straight down, right curve up", vec![Pen::at(0.0, 0.0)
            .loop_up(0.75)
            .overturn(1.6, 1.15)
            .curve((1.6, 0.75), (1.2, 0.6), (1.3, 0.55))
            .curve((1.6, 0.5), (1.75, 0.45), (1.75, 0.3))
            .down_turn(0.0)
            .right_curve(2.8, 1.0)
            .done()]),
        'l' => figure("l", "right curve up to the loop, straight down, right curve up · 1-2", vec![Pen::at(0.0, 0.0)
            .loop_up(0.75)
            .curve((0.75, 0.04), (0.81, 0.0), (0.99, 0.04))
            .right_curve(1.9, 1.0)
            .done()]),
        'b' => figure("b", "the loop, straight down, right curve up, the dot and the finish", vec![Pen::at(0.0, 0.0)
            .loop_up(0.75)
            .curve((0.75, 0.04), (0.81, 0.0), (0.99, 0.04))
            .right_curve(1.7, 1.0)
            .finish()
            .done()]),
        'j' => figure("j", "right curve up, straight down to the loop below, up through the base; the dot", vec![
            Pen::at(0.0, 0.0).right_curve(0.95, 1.0).loop_down(1.9).done(),
            dot(0.95, 1.7),
        ]),
        'y' => figure("y", "left curve over, down, right curve up, straight down to the loop below", vec![Pen::at(0.0, 0.0)
            .overturn(0.55, 1.0)
            .down_turn(0.0)
            .right_curve(1.55, 1.0)
            .loop_down(2.5)
            .done()]),
        'g' => figure("g", "the oval, straight down to the loop below, up through the base", vec![Pen::at(0.0, 0.0)
            .right_curve(1.0, 1.0)
            .oval(0.35, 1.0)
            .loop_down(2.0)
            .done()]),
        'z' => figure("z", "left curve over, down to a small turn on the base, the loop below", vec![Pen::at(0.0, 0.0)
            .overturn(0.55, 1.0)
            .line(0.55, 0.15)
            .curve((0.55, -0.05), (0.85, -0.05), (0.85, 0.1))
            .curve((0.85, 0.25), (0.6, 0.2), (0.8, 0.0))
            .loop_down(1.9)
            .done()]),
        'f' => figure("f", "the loop up to 3, straight down to 2 below, the loop back to the base, right curve up", vec![Pen::at(0.0, 0.0)
            .loop_up(0.75)
            .line(0.75, -1.7)
            .curve((0.75, -2.08), (1.15, -2.0), (1.15, -1.5))
            .curve((1.15, -0.9), (0.95, -0.25), (0.75, 0.0))
            .curve((1.0, 0.15), (1.6, 0.6), (1.9, 1.0))
            .done()]),
        _ => figure("?", "", Vec::new()),
    }
}

/// A row of push-pulls: straight lines up and down the main slant, retraced.
/// The push-pull: from the base up the slant to the head line and back down the same line, one
/// movement.
fn push_pulls() -> Figure {
    let strokes = (0..6)
        .map(|i| {
            let x = i as f32 * 0.45;
            Pen::at(x, 0.0).line(x, 1.0).line(x, 0.0).done()
        })
        .collect();
    figure("push-pulls", "from the dot, up the slant to the head line and back down the same line, one movement · 1-2", strokes)
}

/// Ovals one space wide and two high, once around from the top: counterclockwise, the direct
/// oval of the capitals O E D C, or clockwise, the reversed oval.
fn ovals(direct: bool) -> Figure {
    let strokes = (0..3)
        .map(|i| {
            let cx = i as f32 * 1.25 + 0.5;
            let turns = 1.0;
            let n = 40;
            let points = (0..=n)
                .map(|k| {
                    let t = k as f32 / n as f32 * turns * std::f32::consts::TAU;
                    // From the top, to the left for the direct oval.
                    let a = if direct { t } else { -t };
                    Vec2::new(cx - 0.5 * a.sin(), 1.0 + a.cos())
                })
                .collect();
            Stroke { points, dot: false, counts: vec![0] }
        })
        .collect();
    if direct {
        figure("direct ovals", "from the dot at the top, down the left side and around back to the top · counterclockwise", strokes)
    } else {
        figure("reversed ovals", "from the dot at the top, down the right side and around back to the top · clockwise", strokes)
    }
}

/// The three first principles, each alone.
fn principles() -> Vec<Figure> {
    vec![
        figure("principle 1: straight line", "straight down on the main slant", (0..4)
            .map(|i| Pen::at(i as f32 * 0.6, 1.0).line(i as f32 * 0.6, 0.0).done())
            .collect()),
        figure("principle 2: right curve", "up from the base, the belly down and to the right", (0..3)
            .map(|i| Pen::at(i as f32 * 1.2, 0.0).right_curve(i as f32 * 1.2 + 0.95, 1.0).done())
            .collect()),
        figure("principle 3: left curve", "up from the base, the belly up and to the left", (0..3)
            .map(|i| {
                let x = i as f32 * 1.2;
                Pen::at(x, 0.0).curve((x + 0.1, 0.6), (x + 0.5, 1.0), (x + 0.95, 1.0)).done()
            })
            .collect()),
    ]
}

fn extended_loops() -> Figure {
    let strokes = (0..3).map(|i| Pen::at(i as f32 * 1.3, 0.0).loop_up(i as f32 * 1.3 + 0.75).done()).collect();
    figure("principle 4: extended loop", "right curve up to 3, a short turn, straight down crossing at 1", strokes)
}

/// The lessons, in the order of the copybooks: the warm-up drills, the principles, the short
/// letters, the semi-extended ones and the loops.
pub fn lessons() -> Vec<Lesson> {
    let letters = |s: &str| s.chars().map(letter).collect::<Vec<_>>();
    vec![
        Lesson { name: "Push-pulls", figures: vec![push_pulls()] },
        Lesson { name: "Direct and reversed ovals", figures: vec![ovals(true), ovals(false)] },
        Lesson { name: "The principles 1, 2, 3", figures: principles() },
        Lesson { name: "i u w", figures: letters("iuw") },
        Lesson { name: "n m v x", figures: letters("nmvx") },
        Lesson { name: "o a e c", figures: letters("oaec") },
        Lesson { name: "r s", figures: letters("rs") },
        Lesson { name: "t d p q", figures: letters("tdpq") },
        Lesson { name: "The extended loop", figures: vec![extended_loops()] },
        Lesson { name: "h k l b", figures: letters("hklb") },
        Lesson { name: "j y g z f", figures: letters("jygzf") },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_counts_begin_at_the_start_and_at_the_turns() {
        let push_pull = &push_pulls().strokes[0];
        // Up, then down: the 1 at the base, the 2 at the head line.
        assert_eq!(push_pull.counts.len(), 2);
        assert_eq!(push_pull.points[push_pull.counts[0]].y, 0.0);
        assert_eq!(push_pull.points[push_pull.counts[1]].y, 1.0);
        // The i: right curve, straight down, right curve · 1-2-3.
        let i = letter('i');
        assert_eq!(i.strokes[0].counts.len(), 3, "{:?}", i.strokes[0].counts);
        for stroke in lessons().iter().flat_map(|l| &l.figures).flat_map(|f| &f.strokes) {
            assert!(stroke.counts.iter().all(|c| *c < stroke.points.len()));
            assert!(stroke.counts.windows(2).all(|w| w[0] < w[1]));
        }
    }

    #[test]
    fn every_lesson_has_figures_with_strokes() {
        let lessons = lessons();
        assert_eq!(lessons.len(), 11);
        let letters: usize = lessons.iter().flat_map(|l| &l.figures).filter(|f| f.name.len() == 1).count();
        assert_eq!(letters, 26);
        for figure in lessons.iter().flat_map(|l| &l.figures) {
            assert!(!figure.strokes.is_empty(), "{}", figure.name);
            assert!(figure.strokes.iter().all(|s| s.points.len() >= 2), "{}", figure.name);
        }
    }

    #[test]
    fn the_sheet_leans_at_the_slant() {
        let m = Style::Spencerian.metrics();
        let top = m.shear(Vec2::new(0.0, 1.0));
        let angle = top.y.atan2(top.x).to_degrees();
        assert!((angle - 52.0).abs() < 0.01);
        // The loops of the Spencerian reach its top line, the Copperplate ones its own.
        let l = letter('l');
        let top = |style: Style| l.on_sheet(style)[0].points.iter().map(|p| p.y).fold(f32::MIN, f32::max);
        assert!((top(Style::Spencerian) - 3.0).abs() < 0.15);
        assert!((top(Style::Copperplate) - 2.5).abs() < 0.15);
        // The i stays within the x-height, its dot above.
        let i = letter('i').on_sheet(Style::Spencerian);
        assert!(i[0].points.iter().all(|p| p.y > -0.05 && p.y < 1.05));
        assert!(i[1].dot);
    }
}
