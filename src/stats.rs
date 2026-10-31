//! The statistics panel, of the subject of the first menu, the touch typing or the hanzi: today
//! and the streak of days with the daily goal met, the calendars of this month and the previous
//! one (a day is a success when a lesson was passed), the words per minute and the accuracy of
//! the last 30 days (the accuracy of the strokes and the strokes right on the first try for the
//! hanzi), the exercises by type (by HSK level), and the keys (the words).
//!
//! The charts follow the dataviz method: one measure per chart and one axis, thin lines with
//! dots for the days, the goal as a dashed reference line, the values in text colors, a readout
//! of the day under the pointer. They are drawn with tiny-skia, as the hanzi.
use std::collections::BTreeMap;

use bevy::asset::RenderAssetUsages;
use bevy::color::{ColorToComponents, Srgba};
use bevy::feathers::containers::{group, group_body, group_header};
use bevy::feathers::controls::*;
use bevy::feathers::display::{caption, label, label_dim};
use bevy::feathers::theme::{ThemeTextColor, UiTheme};
use bevy::feathers::tokens as feathers_tokens;
use bevy::input_focus::InputFocus;
use bevy::picking::events::{PointerClick, PointerMove};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui_widgets::Activate;
use tiny_skia::{FillRule, LineCap, Paint, PathBuilder, Pixmap, Stroke, StrokeDash, Transform};

use crate::progress::{ExerciseMode, Progress, Session, Subject, now_unix_seconds};
use crate::theme::tokens;
use crate::ui::{Panel, RankingText};

const DAYS: usize = 30;
const CHART_W: u32 = 640;
const CHART_H: u32 = 140;
const SECONDS_PER_DAY: u64 = 86_400;

#[derive(Component, Default, Clone)]
pub struct StatsOverlay;
#[derive(Component, Default, Clone)]
struct TodayText;
#[derive(Component, Default, Clone)]
struct Calendars;
#[derive(Component, Default, Clone)]
struct ModeBars;
/// A chart: 0 the words per minute, 1 the accuracy.
#[derive(Component, Default, Clone)]
struct Chart(usize);
/// The titles of the charts, of the exercises and of the rankings, they change with the subject.
#[derive(Component, Default, Clone)]
struct Title(usize);
/// The labels of a chart: its top and bottom values, and the readout of the day under the
/// pointer.
#[derive(Component, Default, Clone)]
struct ChartLabel(usize, u8);

/// The values of the days, for the readouts.
#[derive(Resource, Default)]
struct Days {
    subject: Subject,
    first: u64,
    wpm: Vec<Option<f64>>,
    accuracy: Vec<Option<f64>>,
}

#[derive(Resource)]
struct ChartImages([Handle<Image>; 2]);

pub struct StatsPlugin;

impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Days>().add_systems(Startup, setup).add_systems(
            Update,
            (show, fill.run_if(resource_changed::<Panel>)).chain(),
        );
    }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = || {
        Image::new_fill(
            Extent3d { width: CHART_W, height: CHART_H, depth_or_array_layers: 1 },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        )
    };
    commands.insert_resource(ChartImages([images.add(image()), images.add(image())]));
}

pub fn overlay() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            display: Display::None,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        GlobalZIndex(50)
        StatsOverlay
        Children [
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
            }
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6))
            on(close)
            --
            @group()
            Node {
                width: px(900),
                max_width: percent(96),
                max_height: percent(94),
            }
            Children [
                @group_header() Children [ @label("Stats") Title(0) ]
                --
                @group_body()
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    min_height: px(0),
                    overflow: Overflow::scroll_y(),
                }
                bevy::ui_widgets::ScrollArea
                Children [
                    @label("") TodayText
                    --
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(32),
                    }
                    Calendars
                    --
                    @chart(0)
                    --
                    @chart(1)
                    --
                    @label("") Title(3)
                    --
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                    }
                    ModeBars
                    --
                    @label("") Title(4)
                    --
                    @label_dim("") RankingText
                    --
                    @FeathersButton {
                        @caption: bsn! { @caption("Close") }
                        @variant: ButtonVariant::Primary,
                    }
                    on(close_button)
                ]
            ]
        ]
    }
}

fn chart(index: usize) -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
        }
        Children [
            @label("") Title({index + 1})
            --
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
            }
            Children [
                Node {
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    width: px(40),
                    height: px(CHART_H as f32),
                }
                Children [
                    @label_dim("") ChartLabel({index}, 0)
                    --
                    @label_dim("") ChartLabel({index}, 1)
                ]
                --
                Node {
                    width: px(CHART_W as f32),
                    height: px(CHART_H as f32),
                }
                ImageNode
                Chart({index})
                on(readout)
            ]
            --
            @label_dim("") ChartLabel({index}, 2)
        ]
    }
}

fn close(_: On<PointerClick>, mut panel: ResMut<Panel>) {
    *panel = Panel::None;
}

fn close_button(_: On<Activate>, mut panel: ResMut<Panel>, mut focus: ResMut<InputFocus>) {
    *panel = Panel::None;
    focus.clear();
}

fn show(panel: Res<Panel>, mut overlays: Query<&mut Node, With<StatsOverlay>>) {
    if !panel.is_changed() {
        return;
    }
    for mut node in &mut overlays {
        node.display = if *panel == Panel::Stats { Display::Flex } else { Display::None };
    }
}

/// The days since the UNIX epoch, in UTC.
fn day_of(seconds: u64) -> u64 {
    seconds / SECONDS_PER_DAY
}

/// The year, the month (1 to 12) and the day (1 to 31) of a day since the UNIX epoch, the
/// civil calendar of Howard Hinnant.
pub fn civil(days: u64) -> (i64, u32, u32) {
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// The day since the UNIX epoch of a date.
pub fn days_from_civil(y: i64, m: u32, d: u32) -> u64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe - 719_468) as u64
}

/// `Oct 27`, or `Oct 27 2025` in another year, of a time in seconds since the UNIX epoch.
pub fn short_date(seconds: u64) -> String {
    let (y, m, d) = civil(day_of(seconds));
    let (this_year, _, _) = civil(day_of(now_unix_seconds()));
    let month = &MONTHS[m as usize - 1][..3];
    if y == this_year { format!("{month} {d}") } else { format!("{month} {d} {y}") }
}

const MONTHS: [&str; 12] =
    ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

/// What a day holds.
#[derive(Debug, Clone, Default)]
pub struct Day {
    pub seconds: f64,
    pub passed: bool,
    pub exercises: usize,
}

/// The sessions of a subject, of all the keyboards.
fn sessions(progress: &Progress, subject: Subject) -> impl Iterator<Item = &Session> {
    progress.layouts.values().flat_map(|l| &l.sessions).filter(move |s| s.mode.subject() == subject)
}

/// The days with sessions of a subject.
pub fn days(progress: &Progress, subject: Subject) -> BTreeMap<u64, Day> {
    let mut days: BTreeMap<u64, Day> = BTreeMap::new();
    for session in sessions(progress, subject) {
        let day = days.entry(day_of(session.finished_at)).or_default();
        day.seconds += session.elapsed;
        day.exercises += 1;
        day.passed |= session.accuracy >= crate::game::goal(session.mode);
    }
    days
}

/// The days in a row up to today with the daily goal met, today counting when met.
pub fn streak(days: &BTreeMap<u64, Day>, today: u64, goal_minutes: u32) -> usize {
    let met = |d: u64| days.get(&d).is_some_and(|day| day.seconds >= goal_minutes as f64 * 60.0);
    let mut day = if met(today) { today } else { today.saturating_sub(1) };
    let mut streak = 0;
    while met(day) {
        streak += 1;
        if day == 0 {
            break;
        }
        day -= 1;
    }
    streak
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn fill(
    panel: Res<Panel>,
    progress: Res<Progress>,
    theme: Res<UiTheme>,
    charts: Res<ChartImages>,
    mut images: ResMut<Assets<Image>>,
    mut nodes: Query<(&mut ImageNode, &Chart)>,
    mut texts: Query<(&mut Text, Option<&TodayText>, Option<&ChartLabel>, Option<&Title>)>,
    hanzi: Res<crate::hanzi::data::HanziData>,
    calendars: Query<Entity, With<Calendars>>,
    bars: Query<Entity, With<ModeBars>>,
    mut stored: ResMut<Days>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    if *panel != Panel::Stats {
        return;
    }
    let now = now_unix_seconds();
    let today = day_of(now);
    let subject = progress.mode.subject();
    let typing = subject == Subject::Typing;
    let days = days(&progress, subject);
    let goal = progress.daily_goal_minutes;
    let minutes = days.get(&today).map_or(0.0, |d| d.seconds / 60.0);
    let streak = streak(&days, today, goal);
    let first = today + 1 - DAYS as u64;
    let recent: Vec<&Session> = sessions(&progress, subject).filter(|s| day_of(s.finished_at) >= first).collect();
    // The averages of the days.
    let average = |day: u64, value: &dyn Fn(&Session) -> Option<f64>| -> Option<f64> {
        let values: Vec<f64> = recent.iter().filter(|s| day_of(s.finished_at) == day).filter_map(|s| value(s)).collect();
        (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
    };
    // The first chart: the words per minute, or the accuracy of the strokes.
    let wpm: Vec<Option<f64>> = (first..=today)
        .map(|d| {
            average(d, &|s: &Session| {
                if typing { (s.mode.is_typing() && s.velocity > 0.0).then_some(s.velocity) } else { s.quality }
            })
        })
        .collect();
    let accuracy: Vec<Option<f64>> = (first..=today).map(|d| average(d, &|s: &Session| Some(s.accuracy))).collect();
    let best_wpm = wpm.iter().flatten().copied().fold(0.0, f64::max);
    let today_text = format!(
        "Today {minutes:.0} of {goal} minutes{} · streak {streak} day{} · {} {} in 30 days · best day {best_wpm:.0}{}",
        if minutes >= goal as f64 { ", goal met" } else { "" },
        if streak == 1 { "" } else { "s" },
        recent.len(),
        if typing { "exercises" } else { "words" },
        if typing { " WPM" } else { "% stroke accuracy" },
    );
    // The scales: the words per minute from 0, the accuracies from 50% or lower.
    let wpm_max = (best_wpm.max(10.0) / 10.0).ceil() * 10.0;
    let lowest = |values: &[Option<f64>]| values.iter().flatten().copied().fold(100.0, f64::min).min(80.0).floor().max(0.0);
    let scales = if typing { [(0.0, wpm_max), (lowest(&accuracy), 100.0)] } else { [(lowest(&wpm), 100.0), (lowest(&accuracy), 100.0)] };
    let titles = if typing {
        [
            "Stats · Touch typing",
            "Words per minute, average of the day",
            "Accuracy, average of the day, the goal of 95% dashed",
            "Exercises of the last 30 days, by type",
            "Your keys",
        ]
    } else {
        [
            "Stats · Hanzi",
            "Stroke accuracy, how close the strokes were to the target, average of the day",
            "Strokes right on the first try, a stroke shown counts as two misses, the goal of 95% dashed",
            "Words of the last 30 days, by HSK level",
            "Your words",
        ]
    };
    for (mut text, today_label, chart_label, title) in &mut texts {
        if let Some(Title(i)) = title {
            text.0 = titles[*i].into();
        } else if today_label.is_some() {
            text.0.clone_from(&today_text);
        } else if let Some(ChartLabel(chart, which)) = chart_label {
            let (min, max) = scales[*chart];
            let unit = if *chart == 0 && typing { "" } else { "%" };
            text.0 = match which {
                0 => format!("{max:.0}{unit}"),
                1 => format!("{min:.0}{unit}"),
                _ => {
                    let (y, m, d) = civil(first);
                    let (y2, m2, d2) = civil(today);
                    format!("{} {d} {y} to {} {d2} {y2}, point at a day for its value", &MONTHS[m as usize - 1][..3], &MONTHS[m2 as usize - 1][..3])
                }
            };
        }
    }
    let line = theme.color(&tokens::CURSOR);
    let grid = theme.color(&tokens::KEY_BORDER);
    let reference = theme.color(&feathers_tokens::TEXT_DIM);
    for (i, values) in [&wpm, &accuracy].into_iter().enumerate() {
        let goal_line = (i == 1).then_some(95.0);
        let pixmap = draw_chart(values, scales[i], goal_line, line, grid, reference);
        if let Some(mut image) = images.get_mut(&charts.0[i]) {
            image.data = Some(pixmap.data().to_vec());
        }
    }
    for (mut node, Chart(i)) in &mut nodes {
        node.image = charts.0[*i].clone();
    }
    *stored = Days { subject, first, wpm, accuracy };

    // The calendars of this month and the previous one.
    let font = |size: f32| TextFont {
        font: bevy::text::FontSource::Handle(asset_server.load(bevy::feathers::constants::fonts::REGULAR)),
        font_size: FontSize::Px(size),
        ..default()
    };
    let (y, m, _) = civil(today);
    let previous = if m == 1 { (y - 1, 12) } else { (y, m - 1) };
    let success = theme.color(&tokens::CHAR_CORRECT);
    let empty = theme.color(&tokens::KEY_BG);
    let outline = theme.color(&tokens::CURSOR);
    for root in &calendars {
        commands.entity(root).despawn_children();
        for (year, month) in [previous, (y, m)] {
            let column = commands
                .spawn((Node { flex_direction: FlexDirection::Column, row_gap: px(4), ..default() }, ChildOf(root)))
                .id();
            commands.spawn((
                Text::new(format!("{} {year}", MONTHS[month as usize - 1])),
                font(14.0),
                ThemeTextColor(feathers_tokens::TEXT_MAIN),
                ChildOf(column),
            ));
            let grid_node = commands
                .spawn((
                    Node {
                        display: Display::Grid,
                        grid_template_columns: RepeatedGridTrack::px::<Vec<RepeatedGridTrack>>(7, 28.0),
                        column_gap: px(3),
                        row_gap: px(3),
                        ..default()
                    },
                    ChildOf(column),
                ))
                .id();
            for name in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
                commands.spawn((Text::new(name), font(11.0), ThemeTextColor(feathers_tokens::TEXT_DIM), ChildOf(grid_node)));
            }
            let start = days_from_civil(year, month, 1);
            // 1970-01-01 was a Thursday: Monday is 0.
            let weekday = ((start + 3) % 7) as usize;
            for _ in 0..weekday {
                commands.spawn((Node { width: px(28), height: px(24), ..default() }, ChildOf(grid_node)));
            }
            let next = if month == 12 { days_from_civil(year + 1, 1, 1) } else { days_from_civil(year, month + 1, 1) };
            for day in start..next {
                let info = days.get(&day);
                let passed = info.is_some_and(|d| d.passed);
                let cell = commands
                    .spawn((
                        Node {
                            width: px(28),
                            height: px(24),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(if passed { success.with_alpha(0.85) } else { empty }),
                        BorderColor::all(if day == today { outline } else { Color::NONE }),
                        ChildOf(grid_node),
                    ))
                    .id();
                let token = if passed {
                    tokens::KEY_TEXT_ON_HINT
                } else if day > today {
                    feathers_tokens::TEXT_DIM
                } else {
                    feathers_tokens::TEXT_MAIN
                };
                commands.spawn((Text::new(format!("{}", day - start + 1)), font(11.0), ThemeTextColor(token), ChildOf(cell)));
            }
        }
        // The legend: the color is not alone, the words say it.
        let legend = commands
            .spawn((Node { flex_direction: FlexDirection::Column, row_gap: px(6), margin: UiRect::top(px(24)), ..default() }, ChildOf(root)))
            .id();
        for (color, text) in [(success, "a lesson passed"), (empty, "no lesson passed"), (outline, "today, outlined")] {
            let row = commands
                .spawn((Node { flex_direction: FlexDirection::Row, column_gap: px(6), align_items: AlignItems::Center, ..default() }, ChildOf(legend)))
                .id();
            // Today is outlined, as in the calendar.
            let outlined = color == outline;
            commands.spawn((
                Node {
                    width: px(14),
                    height: px(14),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(if outlined { empty } else { color }),
                BorderColor::all(if outlined { outline } else { Color::NONE }),
                ChildOf(row),
            ));
            commands.spawn((Text::new(text), font(12.0), ThemeTextColor(feathers_tokens::TEXT_DIM), ChildOf(row)));
        }
    }

    // The exercises by type: one hue, the bars sorted, the counts in text.
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for s in &recent {
        let name = match s.lesson.and_then(|l| hanzi.levels.get(l.wrapping_sub(1))) {
            Some(level) if s.mode == ExerciseMode::Hanzi => format!("HSK {}", level.hsk),
            _ => s.mode.name().to_string(),
        };
        *counts.entry(name).or_default() += 1;
    }
    let mut counts: Vec<(String, usize)> = counts.into_iter().collect();
    counts.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    let most = counts.first().map_or(1, |(_, n)| *n).max(1);
    for root in &bars {
        commands.entity(root).despawn_children();
        if counts.is_empty() {
            let none = if typing { "No exercise yet." } else { "No word yet." };
            commands.spawn((Text::new(none), font(12.0), ThemeTextColor(feathers_tokens::TEXT_DIM), ChildOf(root)));
        }
        for (name, n) in &counts {
            let row = commands
                .spawn((Node { flex_direction: FlexDirection::Row, column_gap: px(8), align_items: AlignItems::Center, ..default() }, ChildOf(root)))
                .id();
            commands.spawn((
                Text::new(name.clone()),
                font(12.0),
                ThemeTextColor(feathers_tokens::TEXT_MAIN),
                Node { width: px(200), ..default() },
                ChildOf(row),
            ));
            commands.spawn((
                Node {
                    width: px(420.0 * *n as f32 / most as f32),
                    height: px(12),
                    border_radius: BorderRadius::right(px(4)),
                    ..default()
                },
                BackgroundColor(line),
                ChildOf(row),
            ));
            commands.spawn((Text::new(n.to_string()), font(12.0), ThemeTextColor(feathers_tokens::TEXT_DIM), ChildOf(row)));
        }
    }
}

fn paint(color: Color, alpha: f32) -> Paint<'static> {
    let [r, g, b, a] = Srgba::from(color).to_f32_array();
    let mut paint = Paint::default();
    paint.set_color_rgba8((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, (a * alpha * 255.0) as u8);
    paint.anti_alias = true;
    paint
}

/// A line of the days with a dot on each day with a value, a recessive grid, the goal dashed.
fn draw_chart(values: &[Option<f64>], (min, max): (f64, f64), goal: Option<f64>, line: Color, grid: Color, reference: Color) -> Pixmap {
    let mut pixmap = Pixmap::new(CHART_W, CHART_H).expect("chart");
    let (w, h) = (CHART_W as f32, CHART_H as f32);
    let pad = 8.0;
    let x_of = |i: usize| pad + (w - 2.0 * pad) * i as f32 / (values.len().max(2) - 1) as f32;
    let y_of = |v: f64| pad + (h - 2.0 * pad) * (1.0 - ((v - min) / (max - min).max(1e-9)) as f32);
    let thin = Stroke { width: 1.0, ..Default::default() };
    for k in 0..=2 {
        let y = pad + (h - 2.0 * pad) * k as f32 / 2.0;
        let mut pb = PathBuilder::new();
        pb.move_to(pad, y);
        pb.line_to(w - pad, y);
        if let Some(path) = pb.finish() {
            pixmap.stroke_path(&path, &paint(grid, 0.8), &thin, Transform::identity(), None);
        }
    }
    if let Some(goal) = goal {
        let y = y_of(goal);
        let mut pb = PathBuilder::new();
        pb.move_to(pad, y);
        pb.line_to(w - pad, y);
        let dashed = Stroke { width: 1.5, dash: StrokeDash::new(vec![6.0, 5.0], 0.0), ..Default::default() };
        if let Some(path) = pb.finish() {
            pixmap.stroke_path(&path, &paint(reference, 0.9), &dashed, Transform::identity(), None);
        }
    }
    // The line, broken on the days without practice.
    let stroke = Stroke { width: 2.0, line_cap: LineCap::Round, ..Default::default() };
    let mut pb = PathBuilder::new();
    let mut drawing = false;
    for (i, v) in values.iter().enumerate() {
        match v {
            Some(v) => {
                let (x, y) = (x_of(i), y_of(*v));
                if drawing {
                    pb.line_to(x, y);
                } else {
                    pb.move_to(x, y);
                    drawing = true;
                }
            }
            None => drawing = false,
        }
    }
    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, &paint(line, 1.0), &stroke, Transform::identity(), None);
    }
    for (i, v) in values.iter().enumerate() {
        if let Some(v) = v
            && let Some(dot) = PathBuilder::from_circle(x_of(i), y_of(*v), 4.0)
        {
            pixmap.fill_path(&dot, &paint(line, 1.0), FillRule::Winding, Transform::identity(), None);
        }
    }
    pixmap
}

/// The value of the day under the pointer.
fn readout(
    moved: On<PointerMove>,
    charts: Query<(&Chart, &ComputedNode, &UiGlobalTransform)>,
    window: Single<&Window>,
    days: Res<Days>,
    mut labels: Query<(&mut Text, &ChartLabel)>,
) {
    let Ok((Chart(chart), node, transform)) = charts.get(moved.entity) else {
        return;
    };
    let size = node.size();
    let left = transform.affine().translation.x - size.x / 2.0;
    let x = (moved.pointer.position.x * window.scale_factor() - left) / size.x.max(1.0);
    let values = if *chart == 0 { &days.wpm } else { &days.accuracy };
    if values.is_empty() {
        return;
    }
    let i = ((x * (values.len() - 1) as f32).round().max(0.0) as usize).min(values.len() - 1);
    let (y, m, d) = civil(days.first + i as u64);
    let value = match values[i] {
        Some(v) if *chart == 0 && days.subject == Subject::Typing => format!("{v:.1} words per minute"),
        Some(v) if *chart == 0 => format!("{v:.1}% stroke accuracy"),
        Some(v) if days.subject == Subject::Typing => format!("{v:.1}% accuracy"),
        Some(v) => format!("{v:.1}% of the strokes right on the first try"),
        None => "no practice".into(),
    };
    for (mut text, ChartLabel(c, which)) in &mut labels {
        if c == chart && *which == 2 {
            text.0 = format!("{} {d} {y}: {value}", &MONTHS[m as usize - 1][..3]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        let d = days_from_civil(2026, 10, 29);
        assert_eq!(civil(d), (2026, 10, 29));
        // A Thursday: Monday is 0.
        assert_eq!((d + 3) % 7, 3);
        assert_eq!(civil(days_from_civil(2024, 2, 29)), (2024, 2, 29));
    }

    #[test]
    fn streaks() {
        let mut days = BTreeMap::new();
        for d in [8, 9, 10] {
            days.insert(d, Day { seconds: 700.0, passed: true, exercises: 1 });
        }
        // Today (11) not met yet: the streak of the days before.
        assert_eq!(streak(&days, 11, 10), 3);
        assert_eq!(streak(&days, 10, 10), 3);
        assert_eq!(streak(&days, 13, 10), 0);
    }
}
