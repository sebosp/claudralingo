//! The brush: how thick a stroke of the character is along its median, where the brush presses
//! and where it lifts, and the thickness of the user's ink, from the pressure of a pen or, without
//! one, from the speed: slow is thick, as a brush that rests deposits more ink.
use bevy::math::Vec2;

use super::data::PathOp;
use super::strokes::Kind;

/// The distance between the samples of a median, in the 1024 box.
const STEP: f32 = 12.0;
/// A press: as wide as this part of the widest of the stroke, or more.
const PRESS: f32 = 0.82;
/// A press is this far from the next one at least.
const PRESS_APART: f32 = 90.0;
/// An end thinner than this part of the widest of the stroke is a point: the brush lifts.
const POINT: f32 = 0.5;

/// The thickness of a stroke along its median, in the 1024 box.
#[derive(Debug, Clone, Default)]
pub struct Profile {
    pub points: Vec<Vec2>,
    pub widths: Vec<f32>,
}

/// The guide of a stroke: its path, where to press, and how it starts and ends.
#[derive(Debug, Clone, Default)]
pub struct Guide {
    pub path: Vec<Vec2>,
    /// The centers of the presses and their widths.
    pub presses: Vec<(Vec2, f32)>,
    /// The tip goes in against the direction (藏锋).
    pub hidden_entry: bool,
    pub thin_start: bool,
    pub thin_end: bool,
}

fn flatten(ops: &[PathOp]) -> Vec<Vec<Vec2>> {
    const STEPS: usize = 8;
    let mut contours: Vec<Vec<Vec2>> = Vec::new();
    let mut current: Vec<Vec2> = Vec::new();
    let mut last = Vec2::ZERO;
    for op in ops {
        match *op {
            PathOp::Move(p) => {
                if current.len() > 2 {
                    contours.push(std::mem::take(&mut current));
                }
                current = vec![p];
                last = p;
            }
            PathOp::Line(p) => {
                current.push(p);
                last = p;
            }
            PathOp::Quad(c, p) => {
                for i in 1..=STEPS {
                    let t = i as f32 / STEPS as f32;
                    current.push(last * (1.0 - t) * (1.0 - t) + c * 2.0 * t * (1.0 - t) + p * t * t);
                }
                last = p;
            }
            PathOp::Cubic(c1, c2, p) => {
                for i in 1..=STEPS {
                    let t = i as f32 / STEPS as f32;
                    let u = 1.0 - t;
                    current.push(last * u * u * u + c1 * 3.0 * u * u * t + c2 * 3.0 * u * t * t + p * t * t * t);
                }
                last = p;
            }
            PathOp::Close => {
                if current.len() > 2 {
                    contours.push(std::mem::take(&mut current));
                }
            }
        }
    }
    if current.len() > 2 {
        contours.push(current);
    }
    contours
}

fn edges(contours: &[Vec<Vec2>]) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
    contours.iter().flat_map(|c| c.iter().zip(c.iter().cycle().skip(1)).map(|(a, b)| (*a, *b)))
}

fn inside(contours: &[Vec<Vec2>], p: Vec2) -> bool {
    let mut odd = false;
    for (a, b) in edges(contours) {
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            odd = !odd;
        }
    }
    odd
}

/// The distance from `p` along `dir` to the nearest edge.
fn ray(contours: &[Vec<Vec2>], p: Vec2, dir: Vec2) -> Option<f32> {
    let mut nearest: Option<f32> = None;
    for (a, b) in edges(contours) {
        let e = b - a;
        let denom = dir.perp_dot(e);
        if denom.abs() < 1e-6 {
            continue;
        }
        let t = (a - p).perp_dot(e) / denom;
        let u = (a - p).perp_dot(dir) / denom;
        if t > 0.0 && (0.0..=1.0).contains(&u) && nearest.is_none_or(|n| t < n) {
            nearest = Some(t);
        }
    }
    nearest
}

/// A polyline resampled every `step`.
pub fn resample(points: &[Vec2], step: f32) -> Vec<Vec2> {
    let Some(first) = points.first() else {
        return Vec::new();
    };
    let mut out = vec![*first];
    let mut carried = 0.0;
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = a.distance(b);
        let mut d = step - carried;
        while d <= len {
            out.push(a.lerp(b, d / len));
            d += step;
        }
        carried = len - (d - step);
    }
    if let Some(last) = points.last()
        && out.last().is_some_and(|p| p.distance(*last) > step * 0.3)
    {
        out.push(*last);
    }
    out
}

/// The direction of a polyline at a point, from its neighbors.
fn tangent(points: &[Vec2], i: usize) -> Vec2 {
    let a = points[i.saturating_sub(1)];
    let b = points[(i + 1).min(points.len() - 1)];
    (b - a).normalize_or_zero()
}

/// The thickness of a stroke along its median, across the outline.
pub fn profile(outline: &[PathOp], median: &[Vec2]) -> Profile {
    let contours = flatten(outline);
    let points = resample(median, STEP);
    let widths: Vec<f32> = (0..points.len())
        .map(|i| {
            let p = points[i];
            let n = tangent(&points, i).perp();
            if n == Vec2::ZERO || !inside(&contours, p) {
                return 0.0;
            }
            let side = |d: Vec2| ray(&contours, p, d).unwrap_or(0.0).min(160.0);
            side(n) + side(-n)
        })
        .collect();
    // Smoothed over three samples.
    let smooth = (0..widths.len())
        .map(|i| {
            let r = i.saturating_sub(1)..(i + 2).min(widths.len());
            let n = r.len() as f32;
            widths[r].iter().sum::<f32>() / n
        })
        .collect();
    Profile { points, widths: smooth }
}

impl Profile {
    fn max(&self) -> f32 {
        self.widths.iter().copied().fold(0.0, f32::max)
    }

    /// The mean width of a part of the stroke, from and to fractions of its length.
    pub fn part(&self, from: f32, to: f32) -> f32 {
        part(&self.widths, from, to)
    }
}

/// The mean of a part of the values, from and to fractions of their number.
fn part(values: &[f32], from: f32, to: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let n = values.len();
    let a = ((from * n as f32) as usize).min(n - 1);
    let b = ((to * n as f32).ceil() as usize).clamp(a + 1, n);
    values[a..b].iter().sum::<f32>() / (b - a) as f32
}

/// Where the brush presses: the widest places, apart from each other, and how the stroke starts
/// and ends.
pub fn guide(profile: &Profile, kind: &Kind) -> Guide {
    let max = profile.max();
    let w = &profile.widths;
    let mut presses: Vec<(Vec2, f32)> = Vec::new();
    if max > 0.0 {
        // The local maxima, the widest first.
        let mut peaks: Vec<usize> = (0..w.len())
            .filter(|&i| {
                let r = i.saturating_sub(3)..(i + 4).min(w.len());
                w[i] >= PRESS * max && w[r].iter().all(|x| *x <= w[i])
            })
            .collect();
        peaks.sort_by(|a, b| w[*b].total_cmp(&w[*a]));
        for i in peaks {
            let p = profile.points[i];
            if presses.iter().all(|(q, _)| q.distance(p) >= PRESS_APART) {
                presses.push((p, w[i]));
            }
        }
    }
    let ends = |from: f32, to: f32| max > 0.0 && profile.part(from, to) < POINT * max;
    Guide {
        path: profile.points.clone(),
        presses,
        hidden_entry: kind.hidden_entry(),
        thin_start: ends(0.0, 0.08),
        thin_end: ends(0.92, 1.0),
    }
}

/// The width of the user's ink, in the pixels of the image.
pub const INK_MIN: f32 = 7.0;
pub const INK_MAX: f32 = 36.0;
/// Slower than this is the widest ink, faster than `FAST` the thinnest, in pixels per second.
const SLOW: f32 = 120.0;
const FAST: f32 = 900.0;

/// The width of the ink for a pressure, 0 to 1.
pub fn width_of_pressure(pressure: f32) -> f32 {
    INK_MIN + (INK_MAX - INK_MIN) * pressure.clamp(0.0, 1.0)
}

/// The width of the ink for a speed, eased from the last width: the brush does not jump.
pub fn width_of_speed(speed: f32, last: Option<f32>) -> f32 {
    let t = ((speed - SLOW) / (FAST - SLOW)).clamp(0.0, 1.0);
    let target = INK_MAX - (INK_MAX - INK_MIN) * t;
    match last {
        Some(last) => last + (target - last) * 0.35,
        None => target,
    }
}

/// The widths of a stroke at `n` points evenly along it: the points of the ink are closer where
/// it was drawn slowly.
pub fn along(points: &[Vec2], widths: &[f32], n: usize) -> Vec<f32> {
    if points.len() < 2 || widths.len() != points.len() || n < 2 {
        return widths.to_vec();
    }
    let mut distances = vec![0.0];
    for w in points.windows(2) {
        distances.push(distances[distances.len() - 1] + w[0].distance(w[1]));
    }
    let total = distances[distances.len() - 1].max(1e-3);
    let mut j = 0;
    (0..n)
        .map(|i| {
            let d = total * i as f32 / (n - 1) as f32;
            while j + 2 < distances.len() && distances[j + 1] < d {
                j += 1;
            }
            let span = (distances[j + 1] - distances[j]).max(1e-6);
            let t = ((d - distances[j]) / span).clamp(0.0, 1.0);
            widths[j] + (widths[j + 1] - widths[j]) * t
        })
        .collect()
}

/// How the thickness of the user's stroke differs from the stroke of the character, the
/// biggest difference: a press missing, or a point that should be lifted.
pub fn advice(target: &Profile, widths: &[f32]) -> Option<&'static str> {
    if widths.len() < 4 {
        return None;
    }
    let ratios = |start: f32, mid: f32, end: f32| (start / mid.max(0.1), end / mid.max(0.1));
    let (t_start, t_end) = ratios(target.part(0.0, 0.15), target.part(0.35, 0.65), target.part(0.85, 1.0));
    let (u_start, u_end) = ratios(part(widths, 0.0, 0.15), part(widths, 0.35, 0.65), part(widths, 0.85, 1.0));
    if t_end <= 0.65 && u_end >= 0.95 {
        return Some("Lift toward the end: it thins to a point, speed up as you leave the paper.");
    }
    if t_end >= 1.1 && u_end < 0.95 {
        return Some("Press at the end: slow down and rest the brush before you lift it.");
    }
    if t_start >= 1.1 && u_start < 0.95 {
        return Some("Press at the start: rest the tip a moment as it touches down.");
    }
    if t_start <= 0.7 && u_start >= 1.0 {
        return Some("Start light: touch down quickly, the stroke grows as it goes.");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hanzi::data::HanziData;

    #[test]
    fn resampled_evenly() {
        let points = resample(&[Vec2::ZERO, Vec2::new(100.0, 0.0)], 10.0);
        assert_eq!(points.len(), 11);
        assert!((points[5].x - 50.0).abs() < 1e-3);
    }

    #[test]
    fn the_strokes_of_a_brush() {
        let data = HanziData::load();
        // 人: a 撇 thinning to a point, a 捺 starting thin with a wide foot.
        let ren = data.glyph('人').unwrap();
        let pie = profile(&ren.outlines[0], &ren.medians[0]);
        assert!(pie.widths.iter().all(|w| *w >= 0.0) && pie.max() > 40.0, "{:?}", pie.widths);
        let g = guide(&pie, ren.kinds[0]);
        assert!(g.thin_end && !g.thin_start, "{g:?}");
        let na = profile(&ren.outlines[1], &ren.medians[1]);
        assert!(na.part(0.0, 0.15) < na.part(0.7, 0.9), "{:?}", na.widths);
        // 一: pressed at both ends.
        let yi = data.glyph('一').unwrap();
        let g = guide(&profile(&yi.outlines[0], &yi.medians[0]), yi.kinds[0]);
        assert!(g.hidden_entry && !g.thin_end && !g.presses.is_empty(), "{g:?}");
    }

    #[test]
    fn advice_on_the_thickness() {
        let data = HanziData::load();
        let ren = data.glyph('人').unwrap();
        let pie = profile(&ren.outlines[0], &ren.medians[0]);
        // Even ink on a 撇: lift.
        assert!(advice(&pie, &[20.0; 20]).unwrap().starts_with("Lift"));
        // Thinning ink: right.
        let thinning: Vec<f32> = (0..20).map(|i| 36.0 - i as f32 * 1.5).collect();
        assert_eq!(advice(&pie, &thinning), None);
        assert!(width_of_speed(50.0, None) > width_of_speed(2000.0, None));
        // Evenly along: the crowded points of a slow start count once.
        let points = [Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0), Vec2::new(100.0, 0.0)];
        let even = along(&points, &[30.0, 30.0, 30.0, 10.0], 5);
        assert!(even[2] < 25.0, "{even:?}");
    }
}
