//! Whether a stroke drawn by the user is the expected stroke of a character: a port of
//! `strokeMatches.ts` and `geometry.ts` of Hanzi Writer (MIT, David Chanin). The points are in
//! the coordinates of Make Me a Hanzi, a 1024 box, the strokes compared with their medians.
use bevy::math::Vec2;

/// -1 to 1, smaller is more lenient.
const COSINE_SIMILARITY_THRESHOLD: f32 = 0.0;
/// Bigger is more lenient.
const START_AND_END_DIST_THRESHOLD: f32 = 250.0;
const FRECHET_THRESHOLD: f32 = 0.4;
/// Smaller is more lenient.
const MIN_LEN_THRESHOLD: f32 = 0.35;
const AVERAGE_DISTANCE_THRESHOLD: f32 = 350.0;
const SHAPE_FIT_ROTATIONS: [f32; 5] = [
    std::f32::consts::PI / 16.0,
    std::f32::consts::PI / 32.0,
    0.0,
    -std::f32::consts::PI / 32.0,
    -std::f32::consts::PI / 16.0,
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    pub leniency: f32,
    /// The outline of the character is shown, the strokes must be closer to it.
    pub outline_visible: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { leniency: 1.0, outline_visible: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Match,
    /// The right stroke, drawn from its end to its start.
    Backwards,
    /// A later stroke of the character, out of order.
    OtherStroke(usize),
    Miss,
}

/// Compares the user's stroke with the stroke `stroke` of the medians of a character.
pub fn stroke_matches(user: &[Vec2], medians: &[Vec<Vec2>], stroke: usize, options: Options) -> Outcome {
    let points = strip_duplicates(user);
    if points.len() < 2 || stroke >= medians.len() {
        return Outcome::Miss;
    }
    let data = match_data(&points, &medians[stroke], stroke, options, true);
    if !data.is_match {
        if data.backwards {
            return Outcome::Backwards;
        }
        // Not this one: a later stroke, drawn out of order?
        let later = (stroke + 1..medians.len())
            .map(|i| (i, match_data(&points, &medians[i], i, options, false)))
            .filter(|(_, d)| d.is_match)
            .min_by(|a, b| a.1.avg_dist.total_cmp(&b.1.avg_dist));
        return later.map_or(Outcome::Miss, |(i, _)| Outcome::OtherStroke(i));
    }
    // A better match among the strokes not drawn yet: the user probably drew the wrong stroke,
    // the leniency is reduced rather than rejecting it.
    let closest = (stroke + 1..medians.len())
        .map(|i| match_data(&points, &medians[i], i, options, false))
        .filter(|d| d.is_match)
        .map(|d| d.avg_dist)
        .fold(data.avg_dist, f32::min);
    if closest < data.avg_dist {
        let adjustment = (0.6 * (closest + data.avg_dist)) / (2.0 * data.avg_dist);
        let strict = Options { leniency: options.leniency * adjustment, ..options };
        let again = match_data(&points, &medians[stroke], stroke, strict, true);
        return if again.is_match {
            Outcome::Match
        } else if again.backwards {
            Outcome::Backwards
        } else {
            Outcome::Miss
        };
    }
    Outcome::Match
}

struct MatchData {
    is_match: bool,
    backwards: bool,
    avg_dist: f32,
}

fn match_data(points: &[Vec2], median: &[Vec2], stroke_num: usize, options: Options, check_backwards: bool) -> MatchData {
    let leniency = options.leniency;
    let avg_dist = average_distance(median, points);
    let dist_mod = if options.outline_visible || stroke_num > 0 { 0.5 } else { 1.0 };
    let within = avg_dist <= AVERAGE_DISTANCE_THRESHOLD * dist_mod * leniency;
    if !within {
        return MatchData { is_match: false, backwards: false, avg_dist };
    }
    let is_match = start_and_end_matches(points, median, leniency)
        && direction_matches(points, median)
        && shape_fit(points, median, leniency)
        && length_matches(points, median, leniency);
    if check_backwards && !is_match {
        let reversed: Vec<Vec2> = points.iter().rev().copied().collect();
        if match_data(&reversed, median, stroke_num, options, false).is_match {
            return MatchData { is_match: false, backwards: true, avg_dist };
        }
    }
    MatchData { is_match, backwards: false, avg_dist }
}

fn strip_duplicates(points: &[Vec2]) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::with_capacity(points.len());
    for p in points {
        if out.last() != Some(p) {
            out.push(*p);
        }
    }
    out
}

fn length(points: &[Vec2]) -> f32 {
    points.windows(2).map(|w| w[0].distance(w[1])).sum()
}

fn vectors(points: &[Vec2]) -> Vec<Vec2> {
    points.windows(2).map(|w| w[1] - w[0]).collect()
}

/// The mean distance of the points to the nearest point of the median.
fn average_distance(median: &[Vec2], points: &[Vec2]) -> f32 {
    let total: f32 = points
        .iter()
        .map(|p| median.iter().map(|m| m.distance(*p)).fold(f32::INFINITY, f32::min))
        .sum();
    total / points.len() as f32
}

fn start_and_end_matches(points: &[Vec2], median: &[Vec2], leniency: f32) -> bool {
    let (Some(first), Some(last)) = (median.first(), median.last()) else {
        return false;
    };
    let threshold = START_AND_END_DIST_THRESHOLD * leniency;
    first.distance(points[0]) <= threshold && last.distance(points[points.len() - 1]) <= threshold
}

fn cosine_similarity(a: Vec2, b: Vec2) -> f32 {
    a.dot(b) / a.length() / b.length()
}

fn direction_matches(points: &[Vec2], median: &[Vec2]) -> bool {
    let strokes = vectors(median);
    let edges = vectors(points);
    if edges.is_empty() || strokes.is_empty() {
        return false;
    }
    let total: f32 = edges
        .iter()
        .map(|e| strokes.iter().map(|s| cosine_similarity(*s, *e)).fold(f32::NEG_INFINITY, f32::max))
        .sum();
    total / edges.len() as f32 > COSINE_SIMILARITY_THRESHOLD
}

fn length_matches(points: &[Vec2], median: &[Vec2], leniency: f32) -> bool {
    (leniency * (length(points) + 25.0)) / (length(median) + 25.0) >= MIN_LEN_THRESHOLD
}

fn shape_fit(a: &[Vec2], b: &[Vec2], leniency: f32) -> bool {
    let a = normalize_curve(a);
    let b = normalize_curve(b);
    let min = SHAPE_FIT_ROTATIONS
        .iter()
        .map(|theta| frechet_dist(&a, &rotate(&b, *theta)))
        .fold(f32::INFINITY, f32::min);
    min <= FRECHET_THRESHOLD * leniency
}

fn rotate(curve: &[Vec2], theta: f32) -> Vec<Vec2> {
    let (s, c) = theta.sin_cos();
    curve.iter().map(|p| Vec2::new(c * p.x - s * p.y, s * p.x + c * p.y)).collect()
}

/// The discrete Fréchet distance.
fn frechet_dist(a: &[Vec2], b: &[Vec2]) -> f32 {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut prev: Vec<f32> = Vec::new();
    for (i, p) in long.iter().enumerate() {
        let mut cur: Vec<f32> = Vec::with_capacity(short.len());
        for (j, q) in short.iter().enumerate() {
            let d = p.distance(*q);
            let v = match (i, j) {
                (0, 0) => d,
                (_, 0) => prev[0].max(d),
                (0, _) => cur[j - 1].max(d),
                _ => prev[j].min(prev[j - 1]).min(cur[j - 1]).max(d),
            };
            cur.push(v);
        }
        prev = cur;
    }
    prev.last().copied().unwrap_or(f32::INFINITY)
}

/// Translated to its mean and scaled by its ends, resampled and subdivided.
fn normalize_curve(curve: &[Vec2]) -> Vec<Vec2> {
    let outlined = outline_curve(curve, 30);
    let mean = outlined.iter().copied().sum::<Vec2>() / outlined.len() as f32;
    let translated: Vec<Vec2> = outlined.iter().map(|p| *p - mean).collect();
    let first = translated[0];
    let last = translated[translated.len() - 1];
    let scale = ((first.length_squared() + last.length_squared()) / 2.0).sqrt().max(f32::EPSILON);
    subdivide_curve(&translated.iter().map(|p| *p / scale).collect::<Vec<_>>(), 0.05)
}

/// `numPoints` points spaced evenly along the curve.
fn outline_curve(curve: &[Vec2], n: usize) -> Vec<Vec2> {
    let segment = length(curve) / (n - 1) as f32;
    let mut out = vec![curve[0]];
    let mut remaining: std::collections::VecDeque<Vec2> = curve[1..].iter().copied().collect();
    for _ in 0..n.saturating_sub(2) {
        let mut last = *out.last().unwrap_or(&curve[0]);
        let mut left = segment;
        while let Some(next) = remaining.front().copied() {
            let d = last.distance(next);
            if d < left {
                left -= d;
                last = next;
                remaining.pop_front();
            } else {
                out.push(extend_point_on_line(last, next, left - d));
                break;
            }
        }
        if remaining.is_empty() {
            out.push(last);
        }
    }
    out.push(curve[curve.len() - 1]);
    out
}

/// The point on the line of `p1` and `p2`, `dist` beyond `p2`.
fn extend_point_on_line(p1: Vec2, p2: Vec2, dist: f32) -> Vec2 {
    let v = p2 - p1;
    let len = v.length();
    if len == 0.0 {
        return p2;
    }
    p2 + v * (dist / len)
}

fn subdivide_curve(curve: &[Vec2], max_len: f32) -> Vec<Vec2> {
    let mut out = vec![curve[0]];
    for p in &curve[1..] {
        let prev = *out.last().unwrap_or(p);
        let seg = prev.distance(*p);
        if seg > max_len {
            let n = (seg / max_len).ceil() as usize;
            let step = seg / n as f32;
            for i in 0..n {
                out.push(extend_point_on_line(*p, prev, -step * (i + 1) as f32));
            }
        } else {
            out.push(*p);
        }
    }
    out
}

/// The error of a stroke, in the units of the 1024 box, at which its accuracy is 0%.
const ZERO_ACCURACY_ERROR: f32 = 200.0;

/// How close a stroke accepted is to the target, 0 to 100%: both resampled along their length,
/// the mean distance between their points in order, and the distances of their ends. A careful
/// trace inside the outline is above 90%, a stroke a stroke's width off about 70%.
pub fn accuracy(user: &[Vec2], median: &[Vec2]) -> f32 {
    let points = strip_duplicates(user);
    if points.len() < 2 || median.len() < 2 {
        return 0.0;
    }
    let a = outline_curve(&points, 32);
    let b = outline_curve(median, 32);
    let along = a.iter().zip(&b).map(|(p, q)| p.distance(*q)).sum::<f32>() / a.len() as f32;
    let ends = (a[0].distance(b[0]) + a[a.len() - 1].distance(b[b.len() - 1])) / 2.0;
    let error = 0.7 * along + 0.3 * ends;
    (100.0 * (1.0 - error / ZERO_ACCURACY_ERROR)).clamp(0.0, 100.0)
}

/// In words, for the message after a stroke.
pub fn accuracy_word(accuracy: f32) -> &'static str {
    match accuracy {
        a if a >= 90.0 => "excellent",
        a if a >= 78.0 => "good",
        a if a >= 62.0 => "fair, closer to the outline",
        _ => "rough, follow the outline",
    }
}

/// The stroke the user most likely drew, any stroke of the character: for the hints.
pub fn closest_stroke(user: &[Vec2], medians: &[Vec<Vec2>]) -> Option<usize> {
    let points = strip_duplicates(user);
    if points.len() < 2 {
        return None;
    }
    (0..medians.len()).min_by(|a, b| average_distance(&medians[*a], &points).total_cmp(&average_distance(&medians[*b], &points)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 十: a horizontal stroke, then a vertical one, in the 1024 box.
    fn shi() -> Vec<Vec<Vec2>> {
        vec![
            vec![Vec2::new(150.0, 400.0), Vec2::new(500.0, 405.0), Vec2::new(870.0, 400.0)],
            vec![Vec2::new(510.0, 760.0), Vec2::new(512.0, 400.0), Vec2::new(510.0, 20.0)],
        ]
    }

    fn wobbly(median: &[Vec2]) -> Vec<Vec2> {
        // A hand drawn line: many points with a little noise.
        let mut out = Vec::new();
        for w in median.windows(2) {
            for k in 0..10 {
                let t = k as f32 / 10.0;
                let noise = Vec2::new((k as f32 * 1.7).sin() * 12.0, (k as f32 * 2.3).cos() * 12.0);
                out.push(w[0].lerp(w[1], t) + noise);
            }
        }
        out.push(*median.last().unwrap());
        out
    }

    #[test]
    fn the_strokes_in_order() {
        let medians = shi();
        let o = Options::default();
        assert_eq!(stroke_matches(&wobbly(&medians[0]), &medians, 0, o), Outcome::Match);
        assert_eq!(stroke_matches(&wobbly(&medians[1]), &medians, 1, o), Outcome::Match);
    }

    #[test]
    fn backwards_and_out_of_order() {
        let medians = shi();
        let o = Options::default();
        let mut reversed = wobbly(&medians[0]);
        reversed.reverse();
        assert_eq!(stroke_matches(&reversed, &medians, 0, o), Outcome::Backwards);
        assert_eq!(stroke_matches(&wobbly(&medians[1]), &medians, 0, o), Outcome::OtherStroke(1));
        assert_eq!(closest_stroke(&wobbly(&medians[1]), &medians), Some(1));
    }

    #[test]
    fn accuracies() {
        let medians = shi();
        let exact = accuracy(&medians[0], &medians[0]);
        assert!(exact > 99.0, "{exact}");
        let wobbly = accuracy(&wobbly(&medians[0]), &medians[0]);
        assert!((85.0..99.0).contains(&wobbly), "{wobbly}");
        // A stroke's width below the target.
        let low: Vec<Vec2> = medians[0].iter().map(|p| *p - Vec2::new(0.0, 80.0)).collect();
        let low = accuracy(&low, &medians[0]);
        assert!(low < wobbly && low > 40.0, "{low}");
    }

    #[test]
    fn a_scribble_misses() {
        let medians = shi();
        let scribble = vec![Vec2::new(100.0, 900.0), Vec2::new(120.0, 880.0), Vec2::new(90.0, 860.0)];
        assert_eq!(stroke_matches(&scribble, &medians, 0, Options::default()), Outcome::Miss);
        assert_eq!(stroke_matches(&[Vec2::ZERO], &medians, 0, Options::default()), Outcome::Miss);
    }
}
