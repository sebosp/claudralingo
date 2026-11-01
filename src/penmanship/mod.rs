//! The penmanship mode: Spencerian script and Copperplate on their guide sheets, the horizontal
//! lines of the heights (2:1:2 for the Spencerian, 3:2:3 for the Copperplate) and the slant
//! lines (52° and 55°). Each figure of a lesson, a drill or a letter, is written five times as
//! the copybooks ask: over the model, over the faded model, then three times on the empty
//! sheet. The strokes are compared with the model as the hanzi strokes are, see
//! [`crate::hanzi::matcher`], and the slant of the downstrokes is measured against the sheet.
//!
//! The model shows the shades of the downstrokes; the ink of the pen follows its pressure later,
//! when the pens are read on Wayland, see TODO.md.
pub mod letters;

use bevy::asset::RenderAssetUsages;
use bevy::color::{ColorToComponents, Srgba};
use bevy::feathers::controls::*;
use bevy::feathers::display::{caption, label, label_dim};
use bevy::feathers::theme::UiTheme;
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
const HEIGHT: u32 = 800;
/// One space, the x-height, in pixels of the image.
const UNIT: f32 = 130.0;
/// The writes of a figure: over the model, over the faded model, then on the empty sheet.
const WRITES: usize = 5;
/// The misses on a stroke before the model shows it again.
const MISSES_BEFORE_GLIMPSE: u32 = 3;
const GLIMPSE_SECONDS: f32 = 2.2;
const WRONG_SECONDS: f32 = 0.9;
/// The share of the strokes right on the first try to open the next lesson.
const GOAL: f64 = 75.0;
const CANVAS_MAX: f32 = 900.0;

/// The progress of the penmanship, whatever the keyboard.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PenmanshipProgress {
    pub style: Style,
    /// The lesson, 0 based, and the last one open.
    pub lesson: usize,
    pub unlocked: usize,
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
struct Practice {
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
    /// The stroke shown being written after the misses, its time left.
    glimpse: f32,
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
#[derive(Component, Default, Clone)]
struct StyleCaption;

pub struct PenmanshipPlugin;

impl Plugin for PenmanshipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Practice>().add_systems(Startup, setup).add_systems(Update, show_root).add_systems(
            Update,
            (start_lesson, keys, tick, draw_sheet, update_texts, fit_sheet)
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
            --
            @label("") SlantText
            --
            Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                column_gap: px(8),
                row_gap: px(8),
            }
            Children [
                @FeathersButton {
                    @caption: bsn! { @caption("Show me") }
                }
                on(show_me)
                --
                @FeathersButton {
                    @caption: bsn! { @caption("Start over") }
                }
                on(start_over)
                --
                @FeathersButton {
                    @caption: bsn! { @caption("") StyleCaption }
                }
                on(switch_style)
            ]
            --
            @label("") StatusText
        ]
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
fn to_pixels(strokes: &[letters::Stroke], style: Style) -> Vec<letters::Stroke> {
    let m = style.metrics();
    let (min, max) = strokes
        .iter()
        .flat_map(|s| &s.points)
        .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.x), hi.max(p.x)));
    let center = 0.5 * (min + max);
    let base = baseline(&m);
    strokes
        .iter()
        .map(|s| letters::Stroke {
            points: s.points.iter().map(|p| Vec2::new(WIDTH as f32 / 2.0 + (p.x - center) * UNIT, base - p.y * UNIT)).collect(),
            dot: s.dot,
        })
        .collect()
}

/// The base line in pixels: the band of the sheet in the middle of the image.
fn baseline(m: &letters::Metrics) -> f32 {
    let band = (m.ascender - m.descender) * UNIT;
    (HEIGHT as f32 - band) / 2.0 + m.ascender * UNIT
}

impl Practice {
    fn begin_figure(&mut self, figure: usize) {
        let Some(lesson) = self.lesson.and_then(|l| all_lessons().get(l)) else {
            return;
        };
        self.figure = figure;
        self.model = lesson.figures.get(figure).map_or(Vec::new(), |f| to_pixels(&f.on_sheet(self.style), self.style));
        self.write = 0;
        self.slants.clear();
        self.begin_write();
    }

    fn begin_write(&mut self) {
        self.stroke = 0;
        self.misses = 0;
        self.written.clear();
        self.ink.clear();
        self.glimpse = 0.0;
        self.dirty = true;
    }

    fn begin_lesson(&mut self, lesson: usize, style: Style) {
        self.lesson = Some(lesson);
        self.style = style;
        self.first_try = 0;
        self.total = 0;
        self.done = false;
        self.message.clear();
        self.wrong = None;
        self.begin_figure(0);
    }

    fn stage(&self) -> Stage {
        Stage::of_write(self.write)
    }
}

fn start_lesson(progress: Res<Progress>, mut practice: ResMut<Practice>) {
    let lesson = progress.penmanship.lesson.min(n_lessons() - 1);
    if practice.lesson != Some(lesson) || practice.style != progress.penmanship.style {
        practice.begin_lesson(lesson, progress.penmanship.style);
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
        } else if key.typed == Some(' ') {
            practice.glimpse = GLIMPSE_SECONDS;
            practice.dirty = true;
        }
    }
}

fn tick(time: Res<Time>, mut practice: ResMut<Practice>) {
    let dt = time.delta_secs();
    if practice.glimpse > 0.0 {
        practice.glimpse = (practice.glimpse - dt).max(0.0);
        practice.dirty = true;
    }
    if let Some((_, t)) = practice.wrong.as_mut() {
        *t -= dt;
        if *t <= 0.0 {
            practice.wrong = None;
        }
        practice.dirty = true;
    }
}

fn sheet_point(position: Vec2, window: &Window, sheet: &Query<(&ComputedNode, &UiGlobalTransform), With<SheetNode>>) -> Option<Vec2> {
    let (node, transform) = sheet.iter().next()?;
    let size = node.size();
    if size.x <= 0.0 {
        return None;
    }
    let physical = position * window.scale_factor();
    let top_left = transform.affine().translation - size / 2.0;
    Some((physical - top_left) / size * Vec2::new(WIDTH as f32, HEIGHT as f32))
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
    if let Some(p) = sheet_point(press.pointer.position, &window, &sheet) {
        practice.ink = vec![p];
        practice.drawing = true;
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
    if let Some(p) = sheet_point(drag.pointer.position, &window, &sheet)
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
        dot_matches(&ink, &target.points)
    } else {
        let medians: Vec<Vec<Vec2>> = practice.model.iter().map(|s| s.points.clone()).collect();
        let options = Options { leniency: 0.8, outline_visible: practice.stage() != Stage::Blank };
        match matcher::stroke_matches(&ink, &medians, practice.stroke, options) {
            Outcome::Match => true,
            Outcome::Backwards => {
                practice.message = "The right stroke, the other way round: start at the dot".into();
                false
            }
            _ => false,
        }
    };
    if !ok {
        practice.misses += 1;
        if practice.misses >= MISSES_BEFORE_GLIMPSE {
            practice.glimpse = GLIMPSE_SECONDS;
            practice.misses = 0;
            practice.message = "Watch it once more, then write it".into();
        }
        practice.wrong = Some((ink, WRONG_SECONDS));
        return;
    }
    practice.total += 1;
    if practice.misses == 0 && practice.glimpse <= 0.0 {
        practice.first_try += 1;
    }
    if !target.dot
        && let Some(slant) = downstroke_slant(&ink)
    {
        practice.slants.push(slant);
    }
    practice.message.clear();
    practice.misses = 0;
    practice.written.push(ink);
    practice.stroke += 1;
    if practice.stroke < practice.model.len() {
        return;
    }
    // The figure written: again, the next figure, or the end of the lesson.
    practice.write += 1;
    if practice.write < WRITES {
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
    practice.message = if quality >= GOAL {
        if lesson + 1 < n_lessons() {
            progress.penmanship.unlocked = progress.penmanship.unlocked.max(lesson + 1);
            format!("{quality:.0}% of the strokes on the first try. Enter: the next lesson")
        } else {
            format!("{quality:.0}% of the strokes on the first try: the small letters are done")
        }
    } else {
        format!("{quality:.0}% of the strokes on the first try, {GOAL:.0}% opens the next lesson. Start over to write it again")
    };
    progress.save();
}

fn show_me(_: On<Activate>, mut practice: ResMut<Practice>, mut focus: ResMut<InputFocus>) {
    practice.glimpse = GLIMPSE_SECONDS;
    practice.dirty = true;
    focus.clear();
}

fn start_over(_: On<Activate>, mut practice: ResMut<Practice>, mut focus: ResMut<InputFocus>) {
    practice.lesson = None;
    focus.clear();
}

fn switch_style(_: On<Activate>, mut progress: ResMut<Progress>, mut focus: ResMut<InputFocus>) {
    progress.penmanship.style = match progress.penmanship.style {
        Style::Spencerian => Style::Copperplate,
        Style::Copperplate => Style::Spencerian,
    };
    progress.save();
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
    }
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
fn ruling(pixmap: &mut Pixmap, style: Style, c: &Colors) {
    pixmap.fill(tiny_skia::Color::from_rgba8(0, 0, 0, 0));
    let [r, g, b, _] = Srgba::from(c.paper).to_u8_array();
    pixmap.fill(tiny_skia::Color::from_rgba8(r, g, b, 255));
    let m = style.metrics();
    let base = baseline(&m);
    let w = WIDTH as f32;
    let y = |h: f32| base - h * UNIT;
    let strong = paint(c.line, 0.75);
    let light = paint(c.line, 0.4);
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
    let slant = paint(c.slant, 0.8);
    let lean = 1.0 / m.slant.to_radians().tan();
    let (top, bottom) = (y(m.ascender), y(m.descender));
    let rise = (m.ascender - m.descender) * lean * UNIT;
    let mut x = -rise;
    while x < w + rise {
        line(pixmap, Vec2::new(x, bottom), Vec2::new(x + rise, top), 1.0, &slant, None);
        x += 0.5 * UNIT;
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
    ruling(pixmap, practice.style, &c);
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
    // The next stroke: highlighted over the model, its start marked on the faded one.
    if let Some(next) = practice.model.get(practice.stroke) {
        if stage == Stage::Trace {
            shaded(pixmap, &next.points, practice.style, &paint(c.next, 0.55), 1.0);
        }
        if stage != Stage::Blank
            && let Some(start) = next.points.first()
        {
            polyline(pixmap, &[*start], 12.0, &paint(c.next, 0.9));
        }
        if practice.glimpse > 0.0 {
            // Written before the eyes in the first second, then it stays and fades.
            let t = ((GLIMPSE_SECONDS - practice.glimpse) / 1.0).min(1.0);
            let n = ((next.points.len() as f32 * t).ceil() as usize).clamp(1, next.points.len());
            let alpha = (practice.glimpse / 0.6).min(1.0);
            shaded(pixmap, &next.points[..n], practice.style, &paint(c.next, alpha), 1.0);
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
        Query<&mut Text, With<StatusText>>,
        Query<&mut Text, With<StyleCaption>>,
    )>,
) {
    if !practice.is_changed() && !progress.is_changed() {
        return;
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
            format!("{}: {} · {stage}", lesson.name, figure.name)
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
    let caption = format!("{} sheet", match practice.style {
        Style::Spencerian => "Copperplate",
        Style::Copperplate => "Spencerian",
    });
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
    for mut t in &mut texts.p3() {
        set(&mut t, &status);
    }
    for mut t in &mut texts.p4() {
        set(&mut t, &caption);
    }
}

/// The sheet as wide as the window allows, under the toolbar.
fn fit_sheet(window: Single<&Window>, scale: Res<UiScale>, mut nodes: Query<&mut Node, With<SheetNode>>) {
    let size = window.size() / scale.0.max(0.1);
    let ratio = HEIGHT as f32 / WIDTH as f32;
    let width = (size.x - 32.0).min((size.y - 260.0).max(200.0) / ratio).clamp(240.0, CANVAS_MAX).floor();
    for mut node in &mut nodes {
        if node.width != px(width) {
            node.width = px(width);
            node.height = px((width * ratio).floor());
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
            let model = to_pixels(&figure.on_sheet(Style::Spencerian), Style::Spencerian);
            let medians: Vec<Vec<Vec2>> = model.iter().map(|s| s.points.clone()).collect();
            for (i, stroke) in model.iter().enumerate().filter(|(_, s)| !s.dot) {
                let outcome = matcher::stroke_matches(&stroke.points, &medians, i, Options { leniency: 0.8, outline_visible: false });
                assert_eq!(outcome, Outcome::Match, "{} stroke {i}", figure.name);
            }
            // Within the image.
            assert!(model.iter().flat_map(|s| &s.points).all(|p| p.x > 0.0 && p.x < WIDTH as f32 && p.y > 0.0 && p.y < HEIGHT as f32), "{}", figure.name);
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
