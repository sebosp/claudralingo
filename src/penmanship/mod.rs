//! The penmanship mode: Spencerian script and Copperplate on their guide sheets, the horizontal
//! lines of the heights (2:1:2 for the Spencerian, 3:2:3 for the Copperplate) and the slant
//! lines (52° and 55°). Each figure of a lesson, a drill or a letter, is written five times as
//! the copybooks ask: over the model, over the faded model, then three times on the empty
//! sheet. The strokes are compared with the model as the hanzi strokes are, see
//! [`crate::hanzi::matcher`], and the slant of the downstrokes is measured against the sheet.
//!
//! The first write of a figure shows each stroke being written before it is asked, a miss says
//! what went wrong (the start, the end, the length, the shape); a stroke shown after misses or
//! on request takes the first try away.
//!
//! The model shows the shades of the downstrokes; the ink of the pen follows its pressure later,
//! when the pens are read on Wayland, see TODO.md.
pub mod letters;

use bevy::asset::RenderAssetUsages;
use bevy::color::{ColorToComponents, Srgba};
use bevy::feathers::controls::*;
use bevy::feathers::display::{caption, label, label_dim};
use bevy::feathers::theme::{ThemeTextColor, UiTheme};
use bevy::feathers::tokens as feathers_tokens;
use bevy::input_focus::InputFocus;
use bevy::picking::events::{PointerDrag, PointerDragEnd, PointerPress, PointerRelease};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui_widgets::Activate;
use serde::{Deserialize, Serialize};
use tiny_skia::{LineCap, Paint, PathBuilder, Pixmap, Stroke as SkStroke, StrokeDash, Transform};

use crate::game::KeyPressed;
use crate::hanzi::matcher::{self, Options, Outcome};
use crate::klavaro::UPSYM;
use crate::progress::{ExerciseMode, Progress};
use crate::theme::tokens;
use letters::{Lesson, Style};

/// The image of the sheet, in pixels: the strokes are compared in these coordinates.
const WIDTH: u32 = 1024;
/// The height of a sheet of one row, the start; more rows make it taller, see [`sheet_height`].
const HEIGHT: u32 = 800;
/// Above the first row and under the last, and between the rows.
const SHEET_MARGIN: f32 = 75.0;
const ROW_GAP: f32 = 40.0;
/// One space, the x-height, in pixels of the image.
const UNIT: f32 = 130.0;
/// The writes of a figure: over the model, over the faded model, then on the empty sheet.
/// The writes of a figure, unless the settings say otherwise.
const WRITES: usize = 5;
/// The misses on a stroke before the model shows it again.
const MISSES_BEFORE_GLIMPSE: u32 = 3;
/// The Hint button appears after this many misses on a stroke, or after a pause.
const MISSES_FOR_HINT: u32 = 2;
const HINT_AFTER_SECONDS: f32 = 8.0;
/// The stroke is written before the eyes: the pen takes `DEMO_DRAW` seconds, the line stays a
/// moment and fades. The figure is demonstrated whole when it starts, stroke after stroke.
const GLIMPSE_SECONDS: f32 = 2.4;
const DEMO_DRAW: f32 = 1.2;
const DEMO_FADE: f32 = 0.5;
/// The counts of a figure shown on the sheet, at most.
const MAX_COUNTS: usize = 8;
const WRONG_SECONDS: f32 = 1.4;
/// The share of the strokes right on the first try to open the next lesson.
const GOAL: f64 = 75.0;
/// The strokes are compared scaled up: the matcher's distances are those of a hanzi filling
/// its 1024 box, a letter is an x-height of 130 px. At 2.5 the start and the end must be
/// within about one x-height of the model's, the stroke on average within three quarters.
const MATCH_SCALE: f32 = 2.5;
/// The leniency of the matcher over the model and on the empty sheet: the figures are long
/// strokes written in one movement, looser than the hanzi strokes (1.0) on the shape; the
/// models scaled by 15%, squeezed, moved 40 px and jittered 14 px all pass, see the
/// calibration test.
const LENIENCY: f32 = 1.4;
const LENIENCY_BLANK: f32 = 1.5;

/// The points in the coordinates of the matcher.
fn to_match(points: &[Vec2]) -> Vec<Vec2> {
    points.iter().map(|p| *p * MATCH_SCALE).collect()
}
const CANVAS_MAX: f32 = 900.0;

/// The progress of the penmanship, whatever the keyboard.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PenmanshipProgress {
    pub style: Style,
    /// The lesson, 0 based, and the last one open.
    pub lesson: usize,
    pub unlocked: usize,
    /// The writes of a figure, 0 for the five of the course; the rows of the pad, 0 for one:
    /// the writes go down the rows, the sheet clears when they are full.
    pub writes: u32,
    pub rows: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Stage {
    #[default]
    Trace,
    Faded,
    Blank,
}

impl Stage {
    fn of_write(write: usize) -> Stage {
        match write {
            0 => Stage::Trace,
            1 => Stage::Faded,
            _ => Stage::Blank,
        }
    }
}

#[derive(Resource, Default)]
pub struct Practice {
    /// The lesson, the figure and the write being drawn, `None` before the start.
    lesson: Option<usize>,
    style: Style,
    figure: usize,
    write: usize,
    stroke: usize,
    misses: u32,
    /// The strokes on the sheet of the figure, in pixels.
    model: Vec<letters::Stroke>,
    ink: Vec<Vec2>,
    drawing: bool,
    /// The strokes accepted of the current write, in pixels.
    written: Vec<Vec<Vec2>>,
    wrong: Option<(Vec<Vec2>, f32)>,
    /// The stroke shown being written, its time left: the demonstration of the first write,
    /// or the help after the misses and on request.
    glimpse: f32,
    /// The stroke was shown on request or after misses: not a first try.
    helped: bool,
    /// The figure demonstrated when it starts: the stroke being written before the eyes.
    intro: Option<usize>,
    /// The lesson done: passed or not, the verdict in green or amber.
    passed: Option<bool>,
    /// The settings of the lesson: the writes of a figure and the rows of the pad.
    writes: usize,
    rows: usize,
    /// The strokes of the figure on the sheet of the style, in x-heights; `model` has them in
    /// the pixels of the row of the write.
    figure_strokes: Vec<letters::Stroke>,
    /// The seconds without a stroke nor a demonstration: the Hint button appears after a pause.
    idle: f32,
    /// The strokes of the lesson, right on the first try.
    first_try: u32,
    total: u32,
    /// The slant of the downstrokes of the figure, in degrees, with their lengths.
    slants: Vec<(f32, f32)>,
    message: String,
    done: bool,
    dirty: bool,
}

#[derive(Resource)]
struct Canvas {
    image: Handle<Image>,
    pixmap: Pixmap,
}

#[derive(Component, Default, Clone)]
pub struct PenmanshipRoot;
#[derive(Component, Default, Clone)]
struct SheetNode;
#[derive(Component, Default, Clone)]
struct TitleText;
#[derive(Component, Default, Clone)]
struct HowText;
#[derive(Component, Default, Clone)]
struct SlantText;
#[derive(Component, Default, Clone)]
struct StatusText;

/// Shown after misses or a pause.
#[derive(Component, Default, Clone)]
struct HintButton;
/// The bar of the writes of the lesson, as wide as the sheet.
#[derive(Component, Default, Clone)]
struct PenBar;
/// The counts of the stroke demonstrated, over the sheet.
#[derive(Component, Default, Clone)]
struct CountLabel(usize);

pub struct PenmanshipPlugin;

impl Plugin for PenmanshipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Practice>().add_systems(Startup, setup).add_systems(Update, show_root).add_systems(
            Update,
            (start_lesson, keys, tick, draw_sheet, update_texts, place_counts, fit_sheet)
                .chain()
                .after(crate::game::GameSystems)
                .run_if(|progress: Res<Progress>| progress.mode == ExerciseMode::Penmanship),
        );
    }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = Image::new_fill(
        Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    let pixmap = Pixmap::new(WIDTH, HEIGHT).expect("the sheet");
    commands.insert_resource(Canvas { image: images.add(image), pixmap });
}

/// The view: the lesson, how to write the figure and the slant beside the sheet.
pub fn view() -> impl Scene {
    bsn! {
        Node {
            display: Display::None,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(8),
            width: percent(100),
            margin: UiRect::top(px(6)),
            padding: UiRect::horizontal(px(8)),
        }
        PenmanshipRoot
        Children [
            @label("") TitleText
            --
            @label_dim("") HowText
            --
            @crate::ui::steps_bar()
            Node {
                width: px(CANVAS_MAX),
            }
            PenBar
            --
            Node {
                width: px(CANVAS_MAX),
                height: px(CANVAS_MAX * HEIGHT as f32 / WIDTH as f32),
                border_radius: BorderRadius::all(px(8)),
            }
            ImageNode
            SheetNode
            on(pen_down)
            on(pen_move)
            on(pen_up)
            on(pen_drag_end)
            Children [
                @count_label(0) -- @count_label(1) -- @count_label(2) -- @count_label(3)
                -- @count_label(4) -- @count_label(5) -- @count_label(6) -- @count_label(7)
            ]
            --
            @label("") SlantText
            --
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                column_gap: px(8),
            }
            Children [
                @FeathersButton {
                    @caption: bsn! { @caption("Hint") }
                }
                HintButton
                Node { display: Display::None }
                on(show_me)
                --
                @FeathersButton {
                    @caption: bsn! { @crate::icons::icon(crate::icons::Icon::Print) }
                }
                on(crate::print::open)
            ]
            --
            @label("") StatusText
        ]
    }
}

/// A count of the demonstration, placed over the sheet where it begins.
fn count_label(index: usize) -> impl Scene {
    bsn! {
        @label("")
        TextFont { font_size: FontSize::Px(20.0) }
        ThemeTextColor(tokens::CHAR_WRONG)
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
        }
        CountLabel({index})
        Pickable::IGNORE
    }
}

fn show_root(progress: Res<Progress>, mut roots: Query<&mut Node, With<PenmanshipRoot>>) {
    if !progress.is_changed() {
        return;
    }
    let display = if progress.mode == ExerciseMode::Penmanship { Display::Flex } else { Display::None };
    for mut node in &mut roots {
        if node.display != display {
            node.display = display;
        }
    }
}

/// The lessons, computed once.
fn all_lessons() -> &'static [Lesson] {
    static LESSONS: std::sync::OnceLock<Vec<Lesson>> = std::sync::OnceLock::new();
    LESSONS.get_or_init(letters::lessons)
}

pub fn n_lessons() -> usize {
    all_lessons().len()
}

pub fn lesson_name(lesson: usize) -> &'static str {
    all_lessons().get(lesson).map_or("", |l| l.name)
}

/// From the sheet, x-heights with y up, to the pixels of the image, the figure centered.
fn to_pixels(strokes: &[letters::Stroke], style: Style, row: usize) -> Vec<letters::Stroke> {
    let m = style.metrics();
    let (min, max) = strokes
        .iter()
        .flat_map(|s| &s.points)
        .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.x), hi.max(p.x)));
    let center = 0.5 * (min + max);
    let base = baseline(&m, row);
    strokes
        .iter()
        .map(|s| letters::Stroke {
            points: s.points.iter().map(|p| Vec2::new(WIDTH as f32 / 2.0 + (p.x - center) * UNIT, base - p.y * UNIT)).collect(),
            dot: s.dot,
            counts: s.counts.clone(),
        })
        .collect()
}

/// The band of a row, from the ascender line to the descender line, in pixels.
fn band(m: &letters::Metrics) -> f32 {
    (m.ascender - m.descender) * UNIT
}

/// The height of the sheet with its rows.
pub fn sheet_height(style: Style, rows: usize) -> u32 {
    let rows = rows.max(1) as f32;
    (2.0 * SHEET_MARGIN + rows * band(&style.metrics()) + (rows - 1.0) * ROW_GAP).ceil() as u32
}

/// The base line of a row in pixels.
fn baseline(m: &letters::Metrics, row: usize) -> f32 {
    SHEET_MARGIN + row as f32 * (band(m) + ROW_GAP) + m.ascender * UNIT
}

impl Practice {
    fn begin_figure(&mut self, figure: usize) {
        let Some(lesson) = self.lesson.and_then(|l| all_lessons().get(l)) else {
            return;
        };
        self.figure = figure;
        self.figure_strokes = lesson.figures.get(figure).map_or(Vec::new(), |f| f.on_sheet(self.style));
        self.write = 0;
        self.slants.clear();
        self.begin_write();
        // The whole figure written before the eyes, then it is yours: F5 shows it again.
        self.intro = (!self.model.is_empty()).then_some(0);
        self.glimpse = if self.intro.is_some() { GLIMPSE_SECONDS } else { 0.0 };
        self.message = "Watch the figure being written, then write it yourself from the dot, one movement per stroke. F5 shows it again, Hint one stroke.".into();
    }

    fn begin_write(&mut self) {
        self.stroke = 0;
        self.misses = 0;
        self.helped = false;
        self.idle = 0.0;
        // The write goes on its row of the pad, the sheet clears once the rows are full.
        let row = self.row();
        self.model = to_pixels(&self.figure_strokes, self.style, row);
        if row == 0 {
            self.written.clear();
        }
        self.ink.clear();
        self.glimpse = 0.0;
        self.intro = None;
        self.dirty = true;
    }

    /// The stroke being written before the eyes: the one of the demonstration, else the next
    /// one shown after misses or on request.
    fn demo_stroke(&self) -> Option<&letters::Stroke> {
        if self.glimpse <= 0.0 {
            return None;
        }
        self.model.get(self.intro.unwrap_or(self.stroke))
    }

    /// The stroke written before the eyes on request: that write is not a first try.
    fn show_stroke(&mut self) {
        self.glimpse = GLIMPSE_SECONDS;
        self.helped = true;
        self.idle = 0.0;
        self.dirty = true;
    }

    /// The Hint button and the Space key: after misses on the stroke or a pause.
    pub fn hint_shown(&self) -> bool {
        self.lesson.is_some() && !self.done && self.intro.is_none() && (self.misses >= MISSES_FOR_HINT || self.idle >= HINT_AFTER_SECONDS)
    }

    pub fn is_done(&self) -> bool {
        self.done
    }

    /// The writes of the lesson as steps: the figures times the writes of each, the stroke
    /// within the write in progress.
    fn steps(&self) -> crate::ui::Steps {
        let n_figures = self.lesson.and_then(|l| all_lessons().get(l)).map_or(0, |l| l.figures.len());
        let strokes = self.model.len().max(1) as f32;
        let writes = self.writes.max(1);
        let done = if self.done {
            (n_figures * writes) as f32
        } else {
            (self.figure * writes + self.write) as f32 + self.stroke as f32 / strokes
        };
        crate::ui::Steps { total: n_figures * writes, done }
    }

    fn begin_lesson(&mut self, lesson: usize, pen: &PenmanshipProgress) {
        self.lesson = Some(lesson);
        self.style = pen.style;
        self.writes = if pen.writes == 0 { WRITES } else { pen.writes as usize };
        self.rows = pen.rows.max(1) as usize;
        self.first_try = 0;
        self.total = 0;
        self.done = false;
        self.passed = None;
        self.message.clear();
        self.wrong = None;
        self.begin_figure(0);
    }

    fn stage(&self) -> Stage {
        Stage::of_write(self.write)
    }

    /// The row of the pad of the write.
    fn row(&self) -> usize {
        self.write % self.rows.max(1)
    }

    /// The height of the sheet, with its rows.
    fn height(&self) -> u32 {
        sheet_height(self.style, self.rows)
    }

    /// The lesson being written, for the printable page.
    pub fn lesson(&self) -> Option<&'static letters::Lesson> {
        self.lesson.and_then(|l| all_lessons().get(l))
    }

    pub fn style(&self) -> Style {
        self.style
    }
}

/// The lesson of the progress, from its start: when it changes, and again on the circling
/// arrow or F5.
fn start_lesson(progress: Res<Progress>, mut practice: ResMut<Practice>, mut restart: MessageReader<crate::game::NewExercise>) {
    if restart.read().count() > 0 {
        practice.lesson = None;
    }
    let pen = &progress.penmanship;
    let lesson = pen.lesson.min(n_lessons() - 1);
    let (writes, rows) = (if pen.writes == 0 { WRITES } else { pen.writes as usize }, pen.rows.max(1) as usize);
    if practice.lesson != Some(lesson) || practice.style != pen.style || practice.writes != writes || practice.rows != rows {
        practice.begin_lesson(lesson, pen);
    }
}

/// Enter goes to the next lesson once done.
fn keys(mut keys: MessageReader<KeyPressed>, mut practice: ResMut<Practice>, mut progress: ResMut<Progress>) {
    for key in keys.read() {
        if key.typed == Some(UPSYM) && practice.done {
            let next = (progress.penmanship.lesson + 1).min(n_lessons() - 1);
            if next <= progress.penmanship.unlocked || !progress.lock_lessons {
                progress.penmanship.lesson = next;
            }
            progress.save();
            practice.lesson = None;
        } else if key.typed == Some(' ') && practice.hint_shown() {
            practice.show_stroke();
        }
    }
}

fn tick(time: Res<Time>, mut practice: ResMut<Practice>) {
    let dt = time.delta_secs();
    if practice.glimpse > 0.0 {
        practice.glimpse = (practice.glimpse - dt).max(0.0);
        practice.dirty = true;
        // The demonstration goes on with the next stroke as this one fades.
        if let Some(i) = practice.intro
            && practice.glimpse <= DEMO_FADE
        {
            if i + 1 < practice.model.len() {
                practice.intro = Some(i + 1);
                practice.glimpse = GLIMPSE_SECONDS;
            } else if practice.glimpse <= 0.0 {
                practice.intro = None;
            }
        }
    } else if !practice.drawing && !practice.done && practice.idle < HINT_AFTER_SECONDS {
        practice.idle += dt;
    }
    if let Some((_, t)) = practice.wrong.as_mut() {
        *t -= dt;
        if *t <= 0.0 {
            practice.wrong = None;
        }
        practice.dirty = true;
    }
}

fn sheet_point(position: Vec2, window: &Window, sheet: &Query<(&ComputedNode, &UiGlobalTransform), With<SheetNode>>, height: u32) -> Option<Vec2> {
    let (node, transform) = sheet.iter().next()?;
    let size = node.size();
    if size.x <= 0.0 {
        return None;
    }
    let physical = position * window.scale_factor();
    let top_left = transform.affine().translation - size / 2.0;
    Some((physical - top_left) / size * Vec2::new(WIDTH as f32, height as f32))
}

fn pen_down(
    press: On<PointerPress>,
    window: Single<&Window>,
    sheet: Query<(&ComputedNode, &UiGlobalTransform), With<SheetNode>>,
    mut practice: ResMut<Practice>,
) {
    if press.button != PointerButton::Primary || practice.done {
        return;
    }
    if let Some(p) = sheet_point(press.pointer.position, &window, &sheet, practice.height()) {
        // The eager hand: the demonstration stops.
        if practice.intro.take().is_some() {
            practice.glimpse = 0.0;
        }
        practice.ink = vec![p];
        practice.drawing = true;
        practice.idle = 0.0;
        practice.dirty = true;
    }
}

fn pen_move(
    drag: On<PointerDrag>,
    window: Single<&Window>,
    sheet: Query<(&ComputedNode, &UiGlobalTransform), With<SheetNode>>,
    mut practice: ResMut<Practice>,
) {
    if !practice.drawing {
        return;
    }
    if let Some(p) = sheet_point(drag.pointer.position, &window, &sheet, practice.height())
        && practice.ink.last().is_some_and(|last| last.distance(p) >= 2.0)
    {
        practice.ink.push(p);
        practice.dirty = true;
    }
}

fn pen_up(_: On<PointerRelease>, mut practice: ResMut<Practice>, mut progress: ResMut<Progress>) {
    finish_stroke(&mut practice, &mut progress);
}

fn pen_drag_end(_: On<PointerDragEnd>, mut practice: ResMut<Practice>, mut progress: ResMut<Progress>) {
    finish_stroke(&mut practice, &mut progress);
}

/// The stroke of the model the ink lies on: the mean distance of its points to the strokes, the
/// nearest. The push-pulls are six lines alike a space apart, a line drawn on the wrong one is
/// told.
fn nearest_stroke(ink: &[Vec2], medians: &[Vec<Vec2>]) -> Option<usize> {
    let distance = |median: &[Vec2]| -> f32 {
        let total: f32 =
            ink.iter().map(|p| median.iter().map(|m| m.distance(*p)).fold(f32::MAX, f32::min)).sum();
        total / ink.len().max(1) as f32
    };
    (0..medians.len()).filter(|i| medians[*i].len() >= 2).min_by(|a, b| distance(&medians[*a]).total_cmp(&distance(&medians[*b])))
}

/// The slant of the downstrokes of a stroke in pixels (y down), in degrees from the base line,
/// with their length: the parts going down more than sideways.
fn downstroke_slant(points: &[Vec2]) -> Option<(f32, f32)> {
    let (mut sum, mut length) = (0.0, 0.0);
    for w in points.windows(2) {
        let d = w[1] - w[0];
        let len = d.length();
        if len > 0.0 && d.y > 0.0 && d.y > 1.2 * d.x.abs() {
            // Down and to the left leans right, as the sheet: under 90°.
            sum += (d.y).atan2(-d.x).to_degrees() * len;
            length += len;
        }
    }
    (length > UNIT * 0.4).then(|| (sum / length, length))
}

/// A dot is touched near its place, a short touch.
fn dot_matches(ink: &[Vec2], dot: &[Vec2]) -> bool {
    let center = ink.iter().copied().sum::<Vec2>() / ink.len() as f32;
    let length: f32 = ink.windows(2).map(|w| w[0].distance(w[1])).sum();
    center.distance(dot[0]) < 0.4 * UNIT && length < 0.6 * UNIT
}

fn finish_stroke(practice: &mut Practice, progress: &mut Progress) {
    if !practice.drawing {
        return;
    }
    practice.drawing = false;
    practice.dirty = true;
    let ink = std::mem::take(&mut practice.ink);
    let Some(target) = practice.model.get(practice.stroke).cloned() else {
        return;
    };
    let ok = if target.dot {
        let ok = dot_matches(&ink, &target.points);
        if !ok {
            practice.message = "The dot: a short touch above the stem".into();
        }
        ok
    } else {
        let medians: Vec<Vec<Vec2>> = practice.model.iter().map(|s| to_match(&s.points)).collect();
        let scaled = to_match(&ink);
        let blank = practice.stage() == Stage::Blank;
        let options = Options { leniency: if blank { LENIENCY_BLANK } else { LENIENCY }, outline_visible: !blank };
        match matcher::stroke_matches(&scaled, &medians, practice.stroke, options) {
            Outcome::Match => true,
            Outcome::Backwards => {
                practice.message = "The right stroke, the other way round: start at the dot".into();
                false
            }
            _ => {
                use matcher::Problem;
                let nearest = nearest_stroke(&scaled, &medians);
                practice.message = match matcher::diagnose(&scaled, &medians[practice.stroke], practice.stroke, options) {
                    // Another slant, another stroke of the figure: the dot marks the one asked.
                    _ if nearest.is_some_and(|n| n != practice.stroke) => {
                        format!("That is stroke {} of the figure, the dot marks stroke {}", nearest.unwrap_or(0) + 1, practice.stroke + 1)
                    }
                    Some(Problem::Far) if blank => "Far from where the stroke goes: Space shows it".into(),
                    Some(Problem::Far) => "Far from the model: write over the blue stroke".into(),
                    Some(Problem::Start) => "Start at the dot".into(),
                    Some(Problem::End) => "It ends elsewhere: follow the stroke to its end".into(),
                    Some(Problem::Direction) => "The other way round: start at the dot and follow the stroke".into(),
                    Some(Problem::Short) => "Too short: the whole stroke in one movement, without lifting the pen".into(),
                    Some(Problem::Shape) => "Not quite the shape: Space shows it written".into(),
                    None => "Not quite: Space shows it written".into(),
                };
                false
            }
        }
    };
    if !ok {
        practice.misses += 1;
        if practice.misses >= MISSES_BEFORE_GLIMPSE {
            practice.show_stroke();
            practice.misses = 0;
            practice.message = "Watch it once more, then write it".into();
        }
        practice.wrong = Some((ink, WRONG_SECONDS));
        return;
    }
    practice.total += 1;
    if practice.misses == 0 && !practice.helped {
        practice.first_try += 1;
    }
    if !target.dot
        && let Some(slant) = downstroke_slant(&ink)
    {
        practice.slants.push(slant);
    }
    practice.message.clear();
    practice.misses = 0;
    practice.helped = false;
    practice.written.push(ink);
    practice.stroke += 1;
    if practice.stroke < practice.model.len() {
        return;
    }
    // The figure written: again, the next figure, or the end of the lesson.
    practice.write += 1;
    if practice.write < practice.writes {
        practice.begin_write();
        return;
    }
    let n_figures = practice.lesson.and_then(|l| all_lessons().get(l)).map_or(0, |l| l.figures.len());
    if practice.figure + 1 < n_figures {
        let next = practice.figure + 1;
        practice.begin_figure(next);
        return;
    }
    end_lesson(practice, progress);
}

fn end_lesson(practice: &mut Practice, progress: &mut Progress) {
    practice.done = true;
    let quality = 100.0 * practice.first_try as f64 / practice.total.max(1) as f64;
    let lesson = practice.lesson.unwrap_or(0);
    let now = crate::progress::now_unix_seconds();
    progress.current().sessions.push(crate::progress::Session {
        mode: ExerciseMode::Penmanship,
        lesson: Some(lesson + 1),
        accuracy: quality,
        velocity: 0.0,
        fluidness: 0.0,
        finished_at: now,
        elapsed: 0.0,
        quality: Some(quality),
    });
    practice.passed = Some(quality >= GOAL);
    practice.message = if quality >= GOAL {
        if lesson + 1 < n_lessons() {
            progress.penmanship.unlocked = progress.penmanship.unlocked.max(lesson + 1);
            format!("Lesson complete: {quality:.0}% of the strokes on the first try. Enter goes on to the next lesson")
        } else {
            format!("Lesson complete: {quality:.0}% of the strokes on the first try. The small letters are done!")
        }
    } else {
        format!("Below the goal: {quality:.0}% of the strokes on the first try, {GOAL:.0}% opens the next lesson. F5 writes it again")
    };
    progress.save();
}

fn show_me(_: On<Activate>, mut practice: ResMut<Practice>, mut focus: ResMut<InputFocus>) {
    practice.show_stroke();
    focus.clear();
}

struct Colors {
    paper: Color,
    line: Color,
    slant: Color,
    model: Color,
    next: Color,
    ink: Color,
    user: Color,
    wrong: Color,
    /// The stroke written before the eyes and its counts, as the brush guide of the hanzi.
    demo: Color,
}

fn colors(theme: &UiTheme) -> Colors {
    Colors {
        paper: theme.color(&tokens::KEY_BG),
        line: theme.color(&feathers_tokens::TEXT_DIM),
        slant: theme.color(&tokens::KEY_BORDER),
        model: theme.color(&feathers_tokens::TEXT_DIM),
        next: theme.color(&tokens::CURSOR),
        ink: theme.color(&feathers_tokens::TEXT_MAIN),
        user: theme.color(&tokens::SPARK_OK),
        wrong: theme.color(&tokens::CHAR_WRONG),
        demo: theme.color(&tokens::CHAR_WRONG),
    }
}

/// The part of a stroke up to `t` (0 to 1) of its length, the last point interpolated: the pen
/// of the demonstration.
fn along(points: &[Vec2], t: f32) -> Vec<Vec2> {
    let total: f32 = points.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut left = total * t.clamp(0.0, 1.0);
    let mut part: Vec<Vec2> = points.first().copied().into_iter().collect();
    for w in points.windows(2) {
        let d = w[0].distance(w[1]);
        if d >= left {
            part.push(w[0].lerp(w[1], if d > 0.0 { left / d } else { 1.0 }));
            return part;
        }
        left -= d;
        part.push(w[1]);
    }
    part
}

/// Where each count of a stroke begins, as a fraction of its length.
fn count_fractions(stroke: &letters::Stroke) -> Vec<(f32, Vec2)> {
    let mut lengths = vec![0.0f32];
    for w in stroke.points.windows(2) {
        lengths.push(lengths.last().unwrap_or(&0.0) + w[0].distance(w[1]));
    }
    let total = lengths.last().copied().unwrap_or(0.0).max(1e-6);
    stroke.counts.iter().filter_map(|i| Some((lengths.get(*i)? / total, *stroke.points.get(*i)?))).collect()
}

fn paint(color: Color, alpha: f32) -> Paint<'static> {
    let [r, g, b, a] = Srgba::from(color).to_f32_array();
    let mut paint = Paint::default();
    paint.set_color_rgba8((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, (a * alpha * 255.0) as u8);
    paint.anti_alias = true;
    paint
}

fn line(pixmap: &mut Pixmap, from: Vec2, to: Vec2, width: f32, paint: &Paint, dash: Option<f32>) {
    let mut pb = PathBuilder::new();
    pb.move_to(from.x, from.y);
    pb.line_to(to.x, to.y);
    if let Some(path) = pb.finish() {
        let stroke = SkStroke {
            width,
            line_cap: LineCap::Round,
            dash: dash.and_then(|d| StrokeDash::new(vec![d, d * 1.5], 0.0)),
            ..Default::default()
        };
        pixmap.stroke_path(&path, paint, &stroke, Transform::identity(), None);
    }
}

/// The ruling of the sheet: the base and head lines, the ascender and descender lines, their
/// halves dotted, and the slant lines every half space.
fn ruling(pixmap: &mut Pixmap, style: Style, rows: usize, c: &Colors) {
    pixmap.fill(tiny_skia::Color::from_rgba8(0, 0, 0, 0));
    let [r, g, b, _] = Srgba::from(c.paper).to_u8_array();
    pixmap.fill(tiny_skia::Color::from_rgba8(r, g, b, 255));
    let m = style.metrics();
    let w = WIDTH as f32;
    let strong = paint(c.line, 0.75);
    let light = paint(c.line, 0.4);
    let slant = paint(c.slant, 0.8);
    let lean = 1.0 / m.slant.to_radians().tan();
    for row in 0..rows.max(1) {
        let base = baseline(&m, row);
        let y = |h: f32| base - h * UNIT;
        for (h, p, width, dash) in [
            (0.0, &strong, 2.0, None),
            (1.0, &strong, 1.5, None),
            (m.ascender, &light, 1.2, None),
            (m.descender, &light, 1.2, None),
            (m.half_ascender, &light, 1.0, Some(4.0)),
            (m.half_descender, &light, 1.0, Some(4.0)),
        ] {
            line(pixmap, Vec2::new(0.0, y(h)), Vec2::new(w, y(h)), width, p, dash);
        }
        // The slant lines across the band, every half space.
        let (top, bottom) = (y(m.ascender), y(m.descender));
        let rise = (m.ascender - m.descender) * lean * UNIT;
        let mut x = -rise;
        while x < w + rise {
            line(pixmap, Vec2::new(x, bottom), Vec2::new(x + rise, top), 1.0, &slant, None);
            x += 0.5 * UNIT;
        }
    }
}

/// A stroke of the model with its shades: thin up, thick down as a flexible nib writes.
fn shaded(pixmap: &mut Pixmap, points: &[Vec2], style: Style, paint: &Paint, scale: f32) {
    let (hair, shade) = match style {
        Style::Spencerian => (2.5, 9.0),
        Style::Copperplate => (2.5, 13.0),
    };
    for w in points.windows(2) {
        let d = w[1] - w[0];
        let len = d.length();
        if len <= 0.0 {
            continue;
        }
        let down = (d.y / len).max(0.0);
        line(pixmap, w[0], w[1], scale * (hair + (shade - hair) * down * down), paint, None);
    }
}

fn polyline(pixmap: &mut Pixmap, points: &[Vec2], width: f32, paint: &Paint) {
    let mut pb = PathBuilder::new();
    let Some(first) = points.first() else {
        return;
    };
    pb.move_to(first.x, first.y);
    for p in &points[1..] {
        pb.line_to(p.x, p.y);
    }
    if points.len() == 1 {
        pb.line_to(first.x + 0.5, first.y + 0.5);
    }
    if let Some(path) = pb.finish() {
        let stroke = SkStroke { width, line_cap: LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
        pixmap.stroke_path(&path, paint, &stroke, Transform::identity(), None);
    }
}

fn draw_sheet(
    mut practice: ResMut<Practice>,
    mut canvas: ResMut<Canvas>,
    theme: Res<UiTheme>,
    mut images: ResMut<Assets<Image>>,
    mut nodes: Query<&mut ImageNode, With<SheetNode>>,
) {
    for mut node in &mut nodes {
        if node.image != canvas.image {
            node.image = canvas.image.clone();
            practice.dirty = true;
        }
    }
    if !practice.dirty && !theme.is_changed() {
        return;
    }
    practice.dirty = false;
    let c = colors(&theme);
    let Canvas { image, pixmap } = &mut *canvas;
    // The sheet grows with the rows of the pad.
    let height = practice.height();
    if pixmap.height() != height {
        if let Some(taller) = Pixmap::new(WIDTH, height) {
            *pixmap = taller;
        }
        if let Some(mut image) = images.get_mut(&*image) {
            image.resize(Extent3d { width: WIDTH, height, depth_or_array_layers: 1 });
        }
    }
    ruling(pixmap, practice.style, practice.rows, &c);
    let stage = practice.stage();
    let model_alpha = match stage {
        Stage::Trace => 0.45,
        Stage::Faded => 0.16,
        Stage::Blank => 0.0,
    };
    for (i, stroke) in practice.model.iter().enumerate() {
        if i < practice.stroke {
            continue;
        }
        if model_alpha > 0.0 {
            shaded(pixmap, &stroke.points, practice.style, &paint(c.model, model_alpha), 1.0);
        }
    }
    // The next stroke: highlighted over the model, its start marked in every write, on the
    // empty sheet too: which slant it is.
    if let Some(next) = practice.model.get(practice.stroke) {
        if stage == Stage::Trace {
            shaded(pixmap, &next.points, practice.style, &paint(c.next, 0.55), 1.0);
        }
        if let Some(start) = next.points.first() {
            polyline(pixmap, &[*start], 12.0, &paint(c.next, 0.9));
        }
    }
    if let Some(demo) = practice.demo_stroke() {
        // The pen writes it before the eyes, the line stays a moment and fades.
        let t = ((GLIMPSE_SECONDS - practice.glimpse) / DEMO_DRAW).min(1.0);
        let alpha = (practice.glimpse / DEMO_FADE).min(1.0);
        let part = along(&demo.points, t);
        polyline(pixmap, &part, 5.0, &paint(c.demo, 0.9 * alpha));
        if let Some(tip) = part.last() {
            polyline(pixmap, &[*tip], 18.0, &paint(c.demo, alpha));
            polyline(pixmap, &[*tip], 7.0, &paint(c.paper, alpha));
        }
    }
    for stroke in &practice.written {
        polyline(pixmap, stroke, 4.0, &paint(c.ink, 0.95));
    }
    if let Some((points, t)) = &practice.wrong {
        polyline(pixmap, points, 4.0, &paint(c.wrong, (t / WRONG_SECONDS).clamp(0.0, 1.0)));
    }
    if !practice.ink.is_empty() {
        polyline(pixmap, &practice.ink, 4.0, &paint(c.user, 1.0));
    }
    if let Some(mut image) = images.get_mut(image) {
        image.data = Some(pixmap.data().to_vec());
    }
}

#[allow(clippy::type_complexity)]
fn update_texts(
    practice: Res<Practice>,
    progress: Res<Progress>,
    mut texts: ParamSet<(
        Query<&mut Text, With<TitleText>>,
        Query<&mut Text, With<HowText>>,
        Query<&mut Text, With<SlantText>>,
        Query<(Entity, &mut Text, &mut TextFont), With<StatusText>>,
    )>,
    mut hints: Query<&mut Node, With<HintButton>>,
    mut bars: Query<&mut crate::ui::Steps, With<PenBar>>,
    mut verdict: Local<Option<Option<bool>>>,
    mut commands: Commands,
) {
    if !practice.is_changed() && !progress.is_changed() {
        return;
    }
    let hint = if practice.hint_shown() { Display::Flex } else { Display::None };
    for mut node in &mut hints {
        if node.display != hint {
            node.display = hint;
        }
    }
    let steps = practice.steps();
    for mut bar in &mut bars {
        if *bar != steps {
            *bar = steps;
        }
    }
    let lesson = practice.lesson.and_then(|l| all_lessons().get(l));
    let figure = lesson.and_then(|l| l.figures.get(practice.figure));
    let m = practice.style.metrics();
    let title = match (lesson, figure) {
        (Some(lesson), Some(figure)) => {
            let stage = match practice.stage() {
                Stage::Trace => "over the model".to_string(),
                Stage::Faded => "over the faded model".to_string(),
                Stage::Blank => format!("on the empty sheet, {} of 3", practice.write - 1),
            };
            if lesson.name.eq_ignore_ascii_case(figure.name) { format!("{} · {stage}", lesson.name) } else { format!("{}: {} · {stage}", lesson.name, figure.name) }
        }
        _ => String::new(),
    };
    let how = figure.map_or(String::new(), |f| f.how.to_string());
    let slant = if practice.slants.is_empty() {
        format!("{} sheet: slant {:.0}°, the dot marks where the stroke starts", practice.style.name(), m.slant)
    } else {
        let total: f32 = practice.slants.iter().map(|(_, l)| l).sum();
        let mean = practice.slants.iter().map(|(a, l)| a * l).sum::<f32>() / total;
        let advice = if (mean - m.slant).abs() <= 4.0 {
            "right on the slant"
        } else if mean > m.slant {
            "too upright, lean the downstrokes more"
        } else {
            "leaning too much, stand the downstrokes up"
        };
        format!("Your downstrokes: {mean:.0}°, the sheet {:.0}°: {advice}", m.slant)
    };
    let status = practice.message.clone();
    let set = |text: &mut Text, value: &str| {
        if text.0 != value {
            text.0 = value.to_string();
        }
    };
    for mut t in &mut texts.p0() {
        set(&mut t, &title);
    }
    for mut t in &mut texts.p1() {
        set(&mut t, &how);
    }
    for mut t in &mut texts.p2() {
        set(&mut t, &slant);
    }
    // The verdict of the lesson in green or amber, bigger; the messages of the strokes plain.
    for (entity, mut t, mut font) in &mut texts.p3() {
        set(&mut t, &status);
        let size = FontSize::Px(if practice.passed.is_some() { 18.0 } else { 14.0 });
        if font.font_size != size {
            font.font_size = size;
        }
        if *verdict != Some(practice.passed) {
            let token = match practice.passed {
                Some(true) => tokens::GRADE_GOOD,
                Some(false) => tokens::NEAR_MISS,
                None => feathers_tokens::TEXT_MAIN,
            };
            commands.entity(entity).insert(ThemeTextColor(token));
        }
    }
    *verdict = Some(practice.passed);
}

/// The sheet as wide as the window allows, under the toolbar.
fn fit_sheet(
    window: Single<&Window>,
    scale: Res<UiScale>,
    practice: Res<Practice>,
    mut nodes: Query<&mut Node, (With<SheetNode>, Without<PenBar>)>,
    mut bars: Query<&mut Node, With<PenBar>>,
) {
    let size = window.size() / scale.0.max(0.1);
    let ratio = practice.height() as f32 / WIDTH as f32;
    let width = (size.x - 32.0).min((size.y - 260.0).max(200.0) / ratio).clamp(240.0, CANVAS_MAX).floor();
    let height = px((width * ratio).floor());
    for mut node in &mut nodes {
        if node.width != px(width) || node.height != height {
            node.width = px(width);
            node.height = height;
        }
    }
    for mut node in &mut bars {
        if node.width != px(width) {
            node.width = px(width);
        }
    }
}

/// The counts of the stroke demonstrated appear over the sheet as the pen reaches them.
fn place_counts(practice: Res<Practice>, mut labels: Query<(&CountLabel, &mut Node, &mut Text)>) {
    if !practice.is_changed() {
        return;
    }
    let shown: Vec<(f32, Vec2)> = match practice.demo_stroke() {
        Some(stroke) => {
            let t = ((GLIMPSE_SECONDS - practice.glimpse) / DEMO_DRAW).min(1.0);
            count_fractions(stroke).into_iter().filter(|(at, _)| *at <= t + 1e-3).take(MAX_COUNTS).collect()
        }
        _ => Vec::new(),
    };
    for (CountLabel(i), mut node, mut text) in &mut labels {
        match shown.get(*i) {
            Some((_, p)) => {
                node.display = Display::Flex;
                node.left = percent(100.0 * p.x / WIDTH as f32);
                node.top = percent(100.0 * p.y / practice.height() as f32);
                // Beside the point, to the left and a little below: off the stroke.
                node.margin = UiRect { left: px(-30.0), top: px(-12.0), ..default() };
                let label = format!("{}", i + 1);
                if text.0 != label {
                    text.0 = label;
                }
            }
            None => node.display = Display::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_model_matches_itself_and_not_another_letter() {
        let lessons = letters::lessons();
        for figure in lessons.iter().flat_map(|l| &l.figures) {
            let model = to_pixels(&figure.on_sheet(Style::Spencerian), Style::Spencerian, 0);
            let medians: Vec<Vec<Vec2>> = model.iter().map(|s| to_match(&s.points)).collect();
            for (i, stroke) in model.iter().enumerate().filter(|(_, s)| !s.dot) {
                let outcome = matcher::stroke_matches(&to_match(&stroke.points), &medians, i, Options { leniency: LENIENCY_BLANK, outline_visible: false });
                assert_eq!(outcome, Outcome::Match, "{} stroke {i}", figure.name);
            }
            // Within the image.
            assert!(model.iter().flat_map(|s| &s.points).all(|p| p.x > 0.0 && p.x < WIDTH as f32 && p.y > 0.0 && p.y < sheet_height(Style::Spencerian, 1) as f32), "{}", figure.name);
            // The second row of a pad of three sits a band lower.
            let lower = to_pixels(&figure.on_sheet(Style::Spencerian), Style::Spencerian, 1);
            let shift = lower[0].points[0].y - model[0].points[0].y;
            assert!((shift - (band(&Style::Spencerian.metrics()) + ROW_GAP)).abs() < 0.01);
            assert!(lower.iter().flat_map(|s| &s.points).all(|p| p.y < sheet_height(Style::Spencerian, 3) as f32));
        }
    }

    #[test]
    fn the_slant_of_a_downstroke() {
        let m = Style::Spencerian.metrics();
        let top = Vec2::new(m.shear(Vec2::new(0.0, 1.0)).x * UNIT, 0.0);
        let points = [top, Vec2::new(0.0, UNIT)];
        let (angle, _) = downstroke_slant(&points).unwrap();
        assert!((angle - 52.0).abs() < 0.5, "{angle}");
        // A backhand stroke is over 90°.
        let (angle, _) = downstroke_slant(&[Vec2::ZERO, Vec2::new(30.0, UNIT)]).unwrap();
        assert!(angle > 90.0);
    }
}

#[cfg(test)]
mod calibration {
    use super::*;

    /// A hand-drawn stroke, in the pixels of the sheet: the model scaled about its center (in
    /// x and in y), moved and jittered.
    fn hand(points: &[Vec2], scale: Vec2, offset: Vec2, jitter: f32, seed: f32) -> Vec<Vec2> {
        let center = points.iter().copied().sum::<Vec2>() / points.len() as f32;
        let mut out = Vec::new();
        for (i, w) in points.windows(2).enumerate() {
            for k in 0..4 {
                let t = k as f32 / 4.0;
                let p = w[0].lerp(w[1], t);
                let n = (i * 4 + k) as f32;
                let noise = Vec2::new((n * 0.7 + seed).sin(), (n * 1.1 + seed).cos()) * jitter;
                out.push(center + (p - center) * scale + offset + noise);
            }
        }
        out.push(center + (points[points.len() - 1] - center) * scale + offset);
        out
    }

    fn variants() -> Vec<(Vec2, Vec2, f32)> {
        vec![
            (Vec2::splat(1.0), Vec2::ZERO, 4.0),
            (Vec2::splat(0.9), Vec2::new(15.0, -10.0), 6.0),
            (Vec2::splat(1.1), Vec2::new(-15.0, 12.0), 6.0),
            (Vec2::splat(0.85), Vec2::new(30.0, 20.0), 10.0),
            (Vec2::splat(1.15), Vec2::new(-25.0, -30.0), 10.0),
            (Vec2::splat(1.0), Vec2::new(40.0, 0.0), 14.0),
            // Squeezed and stretched: the proportions of a hand.
            (Vec2::new(0.8, 1.1), Vec2::new(10.0, 10.0), 8.0),
            (Vec2::new(1.2, 0.9), Vec2::new(-10.0, 20.0), 8.0),
        ]
    }

    #[test]
    #[ignore = "prints the match rates of simulated hand strokes per leniency"]
    fn match_rates() {
        let lessons = letters::lessons();
        let variants = variants();
        for outline in [true, false] {
            for leniency in [1.0, 1.2, 1.4, 1.6, 1.8] {
                let (mut n, mut ok) = (0, 0);
                let mut worst: Vec<(String, usize)> = Vec::new();
                for figure in lessons.iter().flat_map(|l| &l.figures) {
                    let model = to_pixels(&figure.on_sheet(Style::Spencerian), Style::Spencerian, 0);
                    let medians: Vec<Vec<Vec2>> = model.iter().map(|s| to_match(&s.points)).collect();
                    for (i, stroke) in model.iter().enumerate() {
                        if stroke.dot {
                            continue;
                        }
                        let mut misses = 0;
                        for (v, (scale, offset, jitter)) in variants.iter().enumerate() {
                            n += 1;
                            let ink = to_match(&hand(&stroke.points, *scale, *offset, *jitter, v as f32));
                            let options = Options { leniency, outline_visible: outline };
                            if matcher::stroke_matches(&ink, &medians, i, options) == Outcome::Match {
                                ok += 1;
                            } else {
                                misses += 1;
                            }
                        }
                        if misses > 0 {
                            worst.push((format!("{} s{}", figure.name, i + 1), misses));
                        }
                    }
                }
                worst.sort_by_key(|w| std::cmp::Reverse(w.1));
                let shown: Vec<String> = worst.iter().take(8).map(|(f, m)| format!("{f}:{m}")).collect();
                println!("outline {outline} leniency {leniency}: {ok}/{n} matched; most missed {}", shown.join(" "));
            }
        }
        println!("(the simulated strokes are the model scaled 0.8-1.2, moved up to 40 px, jittered 4-14 px)");
    }

    #[test]
    fn hand_strokes_pass_and_a_half_stroke_says_why() {
        let lessons = letters::lessons();
        for figure in lessons.iter().flat_map(|l| &l.figures) {
            let model = to_pixels(&figure.on_sheet(Style::Spencerian), Style::Spencerian, 0);
            let medians: Vec<Vec<Vec2>> = model.iter().map(|s| to_match(&s.points)).collect();
            for (i, stroke) in model.iter().enumerate().filter(|(_, s)| !s.dot) {
                for (v, (scale, offset, jitter)) in variants().iter().enumerate() {
                    let ink = to_match(&hand(&stroke.points, *scale, *offset, *jitter, v as f32));
                    for (leniency, outline) in [(LENIENCY, true), (LENIENCY_BLANK, false)] {
                        let options = Options { leniency, outline_visible: outline };
                        assert_eq!(matcher::stroke_matches(&ink, &medians, i, options), Outcome::Match, "{} stroke {i} variant {v}", figure.name);
                    }
                }
            }
        }
        // A push-pull that only goes up: it ends at the top, not back at the base. One far from
        // the model, one started a space away.
        let push_pull = to_pixels(&lessons[0].figures[0].on_sheet(Style::Spencerian), Style::Spencerian, 0);
        let model = to_match(&push_pull[0].points);
        let options = Options { leniency: LENIENCY, outline_visible: true };
        let half = to_match(&push_pull[0].points[..push_pull[0].points.len() / 2 + 1]);
        assert_eq!(matcher::diagnose(&half, &model, 0, options), Some(matcher::Problem::End));
        let far: Vec<Vec2> = push_pull[0].points.iter().map(|p| (*p + Vec2::new(0.0, 3.0 * UNIT)) * MATCH_SCALE).collect();
        assert_eq!(matcher::diagnose(&far, &model, 0, options), Some(matcher::Problem::Far));
        let shifted: Vec<Vec2> = push_pull[0].points.iter().map(|p| (*p + Vec2::new(1.3 * UNIT, 0.0)) * MATCH_SCALE).collect();
        assert!(matches!(matcher::diagnose(&shifted, &model, 0, options), Some(matcher::Problem::Far | matcher::Problem::Start)));
        // The fifth line drawn when the fourth is asked: not accepted, and named.
        let medians: Vec<Vec<Vec2>> = push_pull.iter().map(|s| to_match(&s.points)).collect();
        let fifth = to_match(&hand(&push_pull[4].points, Vec2::ONE, Vec2::ZERO, 2.0, 1.0));
        assert_ne!(matcher::stroke_matches(&fifth, &medians, 3, options), Outcome::Match);
        assert_eq!(nearest_stroke(&fifth, &medians), Some(4));
        let fourth = to_match(&hand(&push_pull[3].points, Vec2::ONE, Vec2::ZERO, 2.0, 1.0));
        assert_eq!(matcher::stroke_matches(&fourth, &medians, 3, options), Outcome::Match);
        assert_eq!(nearest_stroke(&fourth, &medians), Some(3));
    }
}
