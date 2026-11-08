//! The history of a lesson at its end, above the summary: the words per minute, the accuracy and
//! the fluidness of its last sessions, the one just typed the last point. One small chart per
//! measure, side by side under their columns of the results: they have no common scale. Each
//! has a dashed reference, named at its left end: the speed of the average typist, the
//! accuracy to pass the exercise, Klavaro's goal of fluidness.
use bevy::asset::RenderAssetUsages;
use bevy::feathers::display::label_dim;
use bevy::feathers::theme::UiTheme;
use bevy::feathers::tokens as feathers_tokens;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, StrokeDash, Transform};

use crate::game::Exercise;
use crate::klavaro::tutor::Query as TutorQuery;
use crate::progress::{ExerciseMode, Progress, Session};
use crate::theme::tokens;

/// The sessions shown, the last ones.
const MAX_POINTS: usize = 20;
/// The size of a chart on the screen, drawn twice as big for the sharp edges.
const CHART: (f32, f32) = (180.0, 56.0);
const SCALE: f32 = 2.0;

#[derive(Component, Default, Clone)]
pub struct HistoryRow;
#[derive(Component, Default, Clone)]
struct HistoryChart(usize);
/// The name of the reference of a chart, at the left end of its dashed line: the last point,
/// the session just typed, is at the right.
#[derive(Component, Default, Clone)]
struct HistoryRef(usize);

/// The words per minute of the average typist: 51.56 over 168,000 volunteers, Dhakal, Feit,
/// Kristensson and Oulasvirta, "Observations on Typing from 136 Million Keystrokes", CHI 2018.
pub const AVERAGE_WPM: f64 = 51.56;

/// The reference line of a chart: the average typist, the goal of the exercise, the fluidity
/// Klavaro asks of its fluidness exercises; and its name.
fn reference(i: usize, mode: ExerciseMode) -> (f64, String) {
    match i {
        0 => (AVERAGE_WPM, format!("average typist {AVERAGE_WPM:.0}")),
        1 => {
            let goal = crate::game::goal(mode);
            (goal, format!("goal {goal:.0}%"))
        }
        _ => (crate::klavaro::fluidness::GOAL_FLUIDITY, format!("goal {:.0}%", crate::klavaro::fluidness::GOAL_FLUIDITY)),
    }
}

/// The images of the three charts, drawn again at the end of each exercise.
#[derive(Resource, Default)]
struct Charts(Vec<Handle<Image>>);

pub struct HistoryPlugin;

impl Plugin for HistoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Charts>().add_systems(Startup, make_charts).add_systems(Update, update_history.after(crate::game::GameSystems));
    }
}

const TITLES: [&str; 3] = ["wpm", "accuracy", "fluidness"];

/// The row of the charts, hidden until the end of a lesson with a history.
pub fn view() -> impl Scene {
    bsn! {
        Node {
            display: Display::None,
            width: px(640),
            max_width: percent(96),
            flex_direction: FlexDirection::Row,
        }
        HistoryRow
        Children [ @chart(0) -- @chart(1) -- @chart(2) ]
    }
}

fn chart(i: usize) -> impl Scene {
    bsn! {
        Node {
            flex_basis: px(0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(2),
        }
        Children [
            @label_dim(TITLES[i])
            --
            Node { width: px(CHART.0), height: px(CHART.1), max_width: percent(100) }
            ImageNode
            HistoryChart({i})
            Children [
                Text("")
                TextFont { font_size: FontSize::Px(10.0) }
                Node { position_type: PositionType::Absolute, left: px(2) }
                HistoryRef({i})
            ]
        ]
    }
}

fn blank() -> Image {
    Image::new_fill(
        Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

fn make_charts(mut charts: ResMut<Charts>, mut images: ResMut<Assets<Image>>) {
    charts.0 = (0..3).map(|_| images.add(blank())).collect();
}

/// The lesson key of the sessions of an exercise, as [`crate::game`] logs them.
fn lesson_key(exercise: &Exercise) -> Option<usize> {
    match exercise.mode {
        ExerciseMode::Basic | ExerciseMode::Layers => Some(exercise.lesson),
        ExerciseMode::Fingers => Some(exercise.finger as usize),
        _ => None,
    }
}

/// The last sessions of a lesson, oldest first: the words per minute, the accuracy, the fluidness.
pub fn series(sessions: &[Session], mode: ExerciseMode, lesson: Option<usize>) -> Vec<[f64; 3]> {
    let same: Vec<[f64; 3]> =
        sessions.iter().filter(|s| s.mode == mode && s.lesson == lesson).map(|s| [s.velocity, s.accuracy, s.fluidness]).collect();
    same[same.len().saturating_sub(MAX_POINTS)..].to_vec()
}

#[allow(clippy::too_many_arguments)]
fn update_history(
    exercise: Res<Exercise>,
    progress: Res<Progress>,
    theme: Res<UiTheme>,
    charts: Res<Charts>,
    mut images: ResMut<Assets<Image>>,
    mut rows: Query<&mut Node, With<HistoryRow>>,
    mut nodes: Query<(&HistoryChart, &mut ImageNode)>,
    mut refs: Query<(&HistoryRef, &mut Text, &mut Node, &mut TextColor, &mut TextFont), Without<HistoryRow>>,
    asset_server: Res<AssetServer>,
) {
    if !exercise.is_changed() && !progress.is_changed() && !theme.is_changed() {
        return;
    }
    let ended = progress.mode.is_typing() && exercise.tutor.query == TutorQuery::End && !exercise.words;
    let sessions = progress.layouts.get(progress.layout.key()).map(|l| l.sessions.as_slice()).unwrap_or_default();
    let points = if ended { series(sessions, exercise.mode, lesson_key(&exercise)) } else { Vec::new() };
    // One session is a point, not a history.
    let shown = points.len() >= 2;
    for mut node in &mut rows {
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
    if !shown || !(exercise.is_changed() || theme.is_changed()) {
        return;
    }
    for (HistoryRef(i), mut text, mut node, mut color, mut font) in &mut refs {
        font.font = bevy::text::FontSource::Handle(asset_server.load(bevy::feathers::constants::fonts::REGULAR));
        let values: Vec<f64> = points.iter().map(|p| p[*i]).collect();
        let (value, name) = reference(*i, exercise.mode);
        text.0 = name;
        color.0 = theme.color(&feathers_tokens::TEXT_DIM);
        // Just above the line, below it when the line is near the top.
        let y = line_y(&values, value);
        node.top = px(if y > 14.0 { y - 13.0 } else { y + 2.0 });
    }
    for (HistoryChart(i), mut node) in &mut nodes {
        let values: Vec<f64> = points.iter().map(|p| p[*i]).collect();
        let Some(handle) = charts.0.get(*i) else {
            continue;
        };
        if let Some(pixmap) = draw(&values, Some(reference(*i, exercise.mode).0), &theme)
            && let Some(mut image) = images.get_mut(handle)
        {
            *image = Image::new(
                Extent3d { width: pixmap.width(), height: pixmap.height(), depth_or_array_layers: 1 },
                TextureDimension::D2,
                pixmap.data().to_vec(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
            );
        }
        if node.image != *handle {
            node.image = handle.clone();
        }
    }
}

/// The range of the values on a chart, a little room above and below; at least 4 units tall,
/// a steady speed would be a flat line in the middle. The goal is always in it.
fn range(values: &[f64], goal: Option<f64>) -> (f64, f64) {
    let (mut lo, mut hi) = values.iter().chain(goal.iter()).fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    if hi - lo < 4.0 {
        let middle = (hi + lo) / 2.0;
        (lo, hi) = (middle - 2.0, middle + 2.0);
    }
    let pad = 0.12 * (hi - lo);
    (lo - pad, hi + pad)
}

/// The height of a value on a chart with its reference, in pixels of the screen from the top.
fn line_y(values: &[f64], reference: f64) -> f32 {
    let (lo, hi) = range(values, Some(reference));
    let margin = 6.0;
    CHART.1 - margin - (CHART.1 - 2.0 * margin) * ((reference - lo) / (hi - lo)) as f32
}

fn paint(color: Color) -> Paint<'static> {
    let c = color.to_srgba();
    let mut paint = Paint::default();
    paint.set_color_rgba8((c.red * 255.0) as u8, (c.green * 255.0) as u8, (c.blue * 255.0) as u8, (c.alpha * 255.0) as u8);
    paint.anti_alias = true;
    paint
}

/// A line through the values, the last one marked; the reference dashed, the base of the chart
/// a faint rule.
fn draw(values: &[f64], goal: Option<f64>, theme: &UiTheme) -> Option<Pixmap> {
    let (w, h) = (CHART.0 * SCALE, CHART.1 * SCALE);
    let mut pixmap = Pixmap::new(w as u32, h as u32)?;
    let (lo, hi) = range(values, goal);
    let margin = 6.0 * SCALE;
    let x = |i: usize| margin + (w - 2.0 * margin) * i as f32 / (values.len().max(2) - 1) as f32;
    let y = |v: f64| h - margin - (h - 2.0 * margin) * ((v - lo) / (hi - lo)) as f32;
    let ts = Transform::identity();
    let thin = Stroke { width: 1.0 * SCALE, ..Default::default() };
    let mut rule = PathBuilder::new();
    rule.move_to(0.0, h - 0.5 * SCALE);
    rule.line_to(w, h - 0.5 * SCALE);
    if let Some(path) = rule.finish() {
        pixmap.stroke_path(&path, &paint(theme.color(&tokens::KEY_BORDER)), &thin, ts, None);
    }
    if let Some(goal) = goal {
        let mut pb = PathBuilder::new();
        pb.move_to(0.0, y(goal));
        pb.line_to(w, y(goal));
        let dashed = Stroke { dash: StrokeDash::new(vec![4.0 * SCALE, 3.0 * SCALE], 0.0), ..thin.clone() };
        if let Some(path) = pb.finish() {
            pixmap.stroke_path(&path, &paint(theme.color(&feathers_tokens::TEXT_DIM)), &dashed, ts, None);
        }
    }
    let line_color = theme.color(&tokens::PROGRESS_BAR);
    let mut pb = PathBuilder::new();
    for (i, v) in values.iter().enumerate() {
        if i == 0 { pb.move_to(x(i), y(*v)) } else { pb.line_to(x(i), y(*v)) }
    }
    let line = Stroke { width: 2.0 * SCALE, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() };
    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, &paint(line_color), &line, ts, None);
    }
    // The session just typed: a dot with a ring of the surface around it.
    if let Some(last) = values.last() {
        let (cx, cy) = (x(values.len() - 1), y(*last));
        if let Some(ring) = PathBuilder::from_circle(cx, cy, 6.0 * SCALE) {
            pixmap.fill_path(&ring, &paint(theme.color(&feathers_tokens::WINDOW_BG)), FillRule::Winding, ts, None);
        }
        if let Some(dot) = PathBuilder::from_circle(cx, cy, 4.0 * SCALE) {
            pixmap.fill_path(&dot, &paint(theme.color(&tokens::CURSOR)), FillRule::Winding, ts, None);
        }
    }
    Some(pixmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(mode: ExerciseMode, lesson: Option<usize>, velocity: f64) -> Session {
        Session { elapsed: 60.0, mode, lesson, accuracy: 96.0, velocity, fluidness: 70.0, finished_at: 0, quality: None }
    }

    #[test]
    fn the_history_is_the_last_sessions_of_the_lesson() {
        let mut sessions: Vec<Session> = (0..30).map(|i| session(ExerciseMode::Basic, Some(3), i as f64)).collect();
        sessions.push(session(ExerciseMode::Basic, Some(4), 99.0));
        sessions.push(session(ExerciseMode::Velocity, None, 98.0));
        let points = series(&sessions, ExerciseMode::Basic, Some(3));
        assert_eq!(points.len(), MAX_POINTS);
        assert_eq!(points.first().map(|p| p[0]), Some(10.0));
        assert_eq!(points.last().map(|p| p[0]), Some(29.0));
        assert_eq!(series(&sessions, ExerciseMode::Velocity, None).len(), 1);
    }

    #[test]
    fn every_chart_has_a_reference_inside_it() {
        assert_eq!(reference(0, ExerciseMode::Basic).0, AVERAGE_WPM);
        assert_eq!(reference(1, ExerciseMode::Basic), (95.0, "goal 95%".to_string()));
        assert_eq!(reference(2, ExerciseMode::Basic).1, "goal 70%");
        // A slow learner: the average typist is still on the chart, at its top.
        let y = line_y(&[12.0, 15.0], AVERAGE_WPM);
        assert!(y > 0.0 && y < 12.0, "{y}");
    }

    #[test]
    fn the_range_holds_the_goal_and_is_never_flat() {
        let (lo, hi) = range(&[96.0, 97.0], Some(95.0));
        assert!(lo < 95.0 && hi > 97.0);
        let (lo, hi) = range(&[30.0, 30.0], None);
        assert!(hi - lo >= 4.0 && lo < 30.0 && hi > 30.0);
    }
}
