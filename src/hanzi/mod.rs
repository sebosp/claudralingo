//! The hanzi mode: the words of HSK 1 to 4, one word at a time, in levels of ten words. Each
//! character is written stroke by stroke with a pen, a mouse or a finger, in four steps: over
//! its whole outline, over a faded outline, filling the strokes taken out of it, and on the
//! empty grid. The strokes are compared with the strokes of Make Me a Hanzi as Hanzi Writer
//! does, see [`matcher`]. Then the pinyin of the word is typed, with the letters of the keyboard
//! or the nine keys of a phone (T9), and the word picked among the candidates, see [`pinyin`].
//! The words are spoken, the recordings of audio-cmn are embedded. With the brush setting the
//! next stroke shows where the brush presses and lifts and the ink is thick or thin, see
//! [`brush`]; the Strokes button breaks the word down into its strokes, see [`breakdown`].
pub mod breakdown;
pub mod brush;
pub mod data;
pub mod matcher;
pub mod pinyin;
pub mod render;
pub mod strokes;

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::feathers::containers::flex_spacer;
use bevy::feathers::controls::*;
use bevy::feathers::display::{caption, label, label_dim};
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeTextColor, UiTheme};
use bevy::feathers::tokens as feathers_tokens;
use bevy::input_focus::InputFocus;
use bevy::picking::events::{PointerClick, PointerDrag, PointerDragEnd, PointerPress, PointerRelease};
use bevy::input::touch::{ForceTouch, Touches};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::text::FontSource;
use bevy::ui_widgets::Activate;

use crate::game::KeyPressed;
use crate::klavaro::{BACKSPACE, UPSYM};
use crate::progress::{ExerciseMode, HanziWordStats, Progress};
use crate::theme::tokens;
use data::HanziData;
use matcher::{Options, Outcome};
use pinyin::{Candidate, Ime};
use render::{Look, SIZE};

/// The misses on a stroke before its hint shows.
const MISSES_BEFORE_HINT: u32 = 3;
const HINT_SECONDS: f32 = 1.6;
const WRONG_SECONDS: f32 = 0.6;
/// The last brush stroke stays over the character, to compare its thickness.
const ECHO_SECONDS: f32 = 1.4;
/// The misses of a step that repeat it.
const MISSES_TO_REPEAT: u32 = 3;
/// The size of the drawing area, in logical pixels.
const CANVAS: f32 = 380.0;
const WORD_FONT: f32 = 64.0;

/// The steps of a word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stage {
    /// Over the whole outline, the next stroke highlighted.
    #[default]
    Trace,
    /// Over a faded outline.
    Faded,
    /// Some strokes are written already, the others are taken out.
    Partial,
    /// On the empty grid.
    Memory,
    /// The pinyin typed and the word picked.
    Pinyin,
}

impl Stage {
    const ALL: [Stage; 5] = [Stage::Trace, Stage::Faded, Stage::Partial, Stage::Memory, Stage::Pinyin];

    fn number(self) -> usize {
        Stage::ALL.iter().position(|s| *s == self).unwrap_or(0) + 1
    }

    fn next(self) -> Option<Stage> {
        Stage::ALL.get(self.number()).copied()
    }

    fn instructions(self) -> &'static str {
        match self {
            Stage::Trace => "trace the strokes in order, over the outline",
            Stage::Faded => "trace the strokes over the faded outline",
            Stage::Partial => "fill in the missing strokes",
            Stage::Memory => "write the word from memory",
            Stage::Pinyin => "type the pinyin, then pick the word",
        }
    }
}

/// The word being practiced.
#[derive(Resource, Default)]
pub struct Practice {
    /// The index of the word in [`HanziData::words`].
    pub word: usize,
    pub stage: Stage,
    /// The character of the word being written.
    pub char_index: usize,
    /// The next stroke of the character.
    pub stroke: usize,
    misses_on_stroke: u32,
    stage_misses: u32,
    word_misses: u32,
    word_strokes: u32,
    /// The stroke being drawn, in the pixels of the image.
    ink: Vec<Vec2>,
    /// Its widths with the brush, empty without.
    ink_widths: Vec<f32>,
    /// When its last point was drawn, in seconds.
    last_point: f64,
    /// The last stroke accepted with the brush and its widths, fading.
    echo: Option<(Vec<Vec2>, Vec<f32>, f32)>,
    drawing: bool,
    /// The last stroke not accepted, fading.
    wrong: Option<(Vec<Vec2>, f32)>,
    /// The hint of the next stroke, its seconds left.
    hint: f32,
    pub message: String,
    /// The letters typed, or the digits of the nine keys.
    pub input: String,
    pub candidates: Vec<Candidate>,
    pub level_done: bool,
    /// The image needs drawing again.
    dirty: bool,
    started: bool,
    /// When the word started, seconds since the UNIX epoch.
    begun: u64,
}

/// The image of the drawing area.
#[derive(Resource)]
struct Canvas {
    image: Handle<Image>,
    pixmap: tiny_skia::Pixmap,
}

#[derive(Resource)]
pub struct HanziFont(pub Handle<Font>);

/// The recordings waiting to be played, one after the other: the syllables of a word.
#[derive(Resource, Default)]
struct Speaker {
    /// The word to say on the next frame.
    pending: Option<String>,
    queue: VecDeque<Handle<AudioSource>>,
    playing: Option<Entity>,
    cache: HashMap<String, Vec<Handle<AudioSource>>>,
}

#[derive(Resource)]
struct InputMethod(Option<Ime>);

#[derive(Component, Default, Clone)]
pub struct HanziRoot;
#[derive(Component, Default, Clone)]
struct WordText;
#[derive(Component, Default, Clone)]
struct PinyinText;
#[derive(Component, Default, Clone)]
struct MeaningText;
#[derive(Component, Default, Clone)]
struct StageText;
#[derive(Component, Default, Clone)]
struct MessageText;
#[derive(Component, Default, Clone)]
struct CanvasNode;
#[derive(Component, Default, Clone)]
struct PinyinPanel;
#[derive(Component, Default, Clone)]
struct InputText;
#[derive(Component, Default, Clone)]
struct CandidatesRow;
#[derive(Component, Default, Clone)]
struct Keypad;
#[derive(Component, Default, Clone)]
struct NextLevelButton;
/// The kind of the next stroke and, with the brush, how to write it.
#[derive(Component, Default, Clone)]
struct KindText;
/// A key of the nine keys: a digit, `<` for Backspace, ` ` to pick the first candidate, `x` to
/// clear.
#[derive(Component, Default, Clone)]
struct T9Key(char);

pub struct HanziPlugin;

impl Plugin for HanziPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(breakdown::BreakdownPlugin)
            .insert_resource(HanziData::load())
            .init_resource::<Practice>()
            .init_resource::<Speaker>()
            .insert_resource(InputMethod(None))
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (start_word, keys, auto_strokes, fade, draw_canvas, update_view, update_kind_text, update_candidates, speak, set_fonts)
                    .chain()
                    .after(crate::game::GameSystems)
                    .run_if(|progress: Res<Progress>| progress.mode == ExerciseMode::Hanzi),
            );
    }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut fonts: ResMut<Assets<Font>>) {
    let image = Image::new_fill(
        Extent3d { width: SIZE, height: SIZE, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    let pixmap = tiny_skia::Pixmap::new(SIZE, SIZE).expect("the canvas");
    commands.insert_resource(Canvas { image: images.add(image), pixmap });
    commands.insert_resource(HanziFont(fonts.add(Font::from_bytes(data::FONT.to_vec()))));
}

/// The view, in the column of the window, shown in the hanzi mode.
pub fn view() -> impl Scene {
    bsn! {
        Node {
            display: Display::None,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(10),
            margin: UiRect::top(px(6)),
        }
        HanziRoot
        Children [
            // The word, its pinyin and its meaning.
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(18),
                padding: UiRect::axes(px(18), px(8)),
                border_radius: BorderRadius::all(px(10)),
                min_width: px(520),
            }
            ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
            Children [
                Text("") WordText
                ThemeTextColor(feathers_tokens::TEXT_MAIN)
                --
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    max_width: px(420),
                }
                Children [
                    @label("") PinyinText
                    --
                    @label_dim("") MeaningText
                ]
                --
                @flex_spacer()
                --
                @FeathersButton {
                    @caption: bsn! { @caption("Listen") }
                }
                on(listen)
            ]
            --
            @label("") StageText
            --
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Start,
                column_gap: px(16),
            }
            Children [
                Node {
                    width: px(CANVAS),
                    height: px(CANVAS),
                    border_radius: BorderRadius::all(px(8)),
                }
                ImageNode
                CanvasNode
                on(pen_down)
                on(pen_move)
                on(pen_up)
                on(pen_drag_end)
                --
                // The buttons and, in its step, the pinyin beside the drawing.
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    width: px(300),
                }
                Children [
                    Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: px(8),
                        row_gap: px(8),
                    }
                    Children [
                        @FeathersButton {
                            @caption: bsn! { @caption("Hint") }
                        }
                        on(show_hint)
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("Start over") }
                        }
                        on(restart_word)
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("Strokes") }
                        }
                        on(breakdown::open)
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("Next level") }
                            @variant: ButtonVariant::Primary,
                        }
                        NextLevelButton
                        Node { display: Display::None }
                        on(next_level)
                    ]
                    --
                    Text("") KindText
                    ThemeTextColor(feathers_tokens::TEXT_MAIN)
                    --
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Start,
                        row_gap: px(8),
                    }
                    PinyinPanel
                    Children [
                        @label("") InputText
                        --
                        Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: px(6),
                            row_gap: px(6),
                            min_height: px(64),
                        }
                        CandidatesRow
                        --
                        Node {
                            display: Display::Grid,
                            grid_template_columns: RepeatedGridTrack::px::<Vec<RepeatedGridTrack>>(3, 94.0),
                            column_gap: px(6),
                            row_gap: px(6),
                        }
                        Keypad
                        Children [
                            @t9_key('1', "1  '") -- @t9_key('2', "2  abc") -- @t9_key('3', "3  def")
                            -- @t9_key('4', "4  ghi") -- @t9_key('5', "5  jkl") -- @t9_key('6', "6  mno")
                            -- @t9_key('7', "7  pqrs") -- @t9_key('8', "8  tuv") -- @t9_key('9', "9  wxyz")
                            -- @t9_key('<', "⌫") -- @t9_key(' ', "Pick") -- @t9_key('x', "Clear")
                        ]
                    ]
                ]
            ]
            --
            @label("") MessageText
            TextLayout { justify: Justify::Center }
        ]
    }
}

fn t9_key(key: char, text: &'static str) -> impl Scene {
    bsn! {
        @FeathersButton {
            @caption: bsn! { @caption(text) }
        }
        T9Key({key})
        Node { height: px(44) }
        on(on_t9_key)
    }
}

/// The CJK font on the texts of hanzi, the feathers font does not have them.
fn set_fonts(font: Res<HanziFont>, mut texts: Query<&mut TextFont, Added<WordText>>) {
    for mut text in &mut texts {
        text.font = FontSource::Handle(font.0.clone());
        text.font_size = FontSize::Px(WORD_FONT);
    }
}

fn hanzi_font(font: &HanziFont, size: f32) -> TextFont {
    TextFont { font: FontSource::Handle(font.0.clone()), font_size: FontSize::Px(size), ..default() }
}

/// The stroke in the partial step: every other stroke is taken out, at least one.
fn taken_out(stroke: usize, n_strokes: usize, word: usize) -> bool {
    n_strokes < 2 || stroke % 2 == (word + 1) % 2
}

impl Practice {
    fn begin(&mut self, word: usize) {
        *self = Practice { word, dirty: true, started: true, begun: crate::progress::now_unix_seconds(), ..default() };
        self.message = "A new word: listen, then trace its strokes.".into();
    }

    fn word<'a>(&self, data: &'a HanziData) -> &'a data::Word {
        &data.words[self.word.min(data.words.len() - 1)]
    }

    fn current_char(&self, data: &HanziData) -> Option<char> {
        self.word(data).word.chars().nth(self.char_index)
    }

    /// Skips the strokes written already in the partial step.
    fn skip_given(&mut self, data: &HanziData) {
        let Some(glyph) = self.current_char(data).and_then(|c| data.glyph(c)) else {
            return;
        };
        let n = glyph.medians.len();
        while self.stage == Stage::Partial && self.stroke < n && !taken_out(self.stroke, n, self.word) {
            self.stroke += 1;
        }
    }

    fn start_stage(&mut self, stage: Stage, data: &HanziData) {
        self.stage = stage;
        self.char_index = 0;
        self.stroke = 0;
        self.misses_on_stroke = 0;
        self.stage_misses = 0;
        self.input.clear();
        self.candidates.clear();
        self.dirty = true;
        self.skip_given(data);
    }

    /// The looks of the strokes of the current character.
    fn looks(&self, n: usize) -> Vec<Look> {
        (0..n)
            .map(|i| {
                let given = self.stage == Stage::Partial && !taken_out(i, n, self.word);
                if i < self.stroke || given {
                    return Look::Done;
                }
                if i == self.stroke && self.hint > 0.0 {
                    return Look::Next(0.75 * (self.hint / HINT_SECONDS).min(1.0));
                }
                match self.stage {
                    Stage::Trace if i == self.stroke => Look::Next(0.5),
                    Stage::Trace => Look::Outline(0.35),
                    Stage::Faded => Look::Outline(0.12),
                    _ => Look::Hidden,
                }
            })
            .collect()
    }
}

/// A new word when the mode starts, and the level of the progress.
fn start_word(
    mut practice: ResMut<Practice>,
    progress: Res<Progress>,
    data: Res<HanziData>,
    mut speaker: ResMut<Speaker>,
    mut restart: MessageReader<crate::game::NewExercise>,
) {
    // Restart, or the mode just picked.
    if restart.read().count() > 0 {
        practice.started = false;
    }
    let level = progress.hanzi.level.min(data.levels.len() - 1);
    // Another level picked, with ‹ › or the lessons.
    if practice.started && (data.level_of(practice.word) == level || practice.level_done && !progress.is_changed()) {
        return;
    }
    if practice.started && practice.level_done && data.level_of(practice.word) == level {
        return;
    }
    let words = &data.levels[level].words;
    let word = (words.start + progress.hanzi.word).min(words.end - 1);
    practice.begin(word);
    speaker.say(&data.words[word].word);
}

impl Speaker {
    fn say(&mut self, word: &str) {
        // The handles are made by `speak`, it has the assets.
        self.pending = Some(word.to_string());
    }
}

/// Plays the recordings one after the other.
fn speak(
    mut speaker: ResMut<Speaker>,
    data: Res<HanziData>,
    progress: Res<Progress>,
    mut sources: ResMut<Assets<AudioSource>>,
    players: Query<(), With<AudioPlayer>>,
    mut commands: Commands,
) {
    if let Some(word) = speaker.pending.take()
        && progress.sound
    {
        let handles = if let Some(cached) = speaker.cache.get(&word).filter(|h| !h.is_empty()) {
            cached.clone()
        } else {
            let handles: Vec<Handle<AudioSource>> = data
                .audio(&word)
                .into_iter()
                .map(|bytes| sources.add(AudioSource { bytes: Arc::from(bytes) }))
                .collect();
            speaker.cache.insert(word.clone(), handles.clone());
            handles
        };
        if let Some(playing) = speaker.playing.take() {
            commands.entity(playing).try_despawn();
        }
        speaker.queue = handles.into();
    }
    let busy = speaker.playing.is_some_and(|e| players.contains(e));
    if !busy && let Some(next) = speaker.queue.pop_front() {
        let entity = commands.spawn((AudioPlayer(next), PlaybackSettings::DESPAWN)).id();
        speaker.playing = Some(entity);
    } else if !busy {
        speaker.playing = None;
    }
}

fn listen(_: On<Activate>, practice: Res<Practice>, data: Res<HanziData>, mut speaker: ResMut<Speaker>, mut focus: ResMut<InputFocus>) {
    speaker.say(&practice.word(&data).word);
    focus.clear();
}

fn show_hint(_: On<Activate>, mut practice: ResMut<Practice>, mut focus: ResMut<InputFocus>) {
    if practice.stage == Stage::Pinyin {
        practice.message = "The pinyin is under the word in the first steps, listen to it.".into();
    } else {
        practice.hint = HINT_SECONDS;
        practice.dirty = true;
    }
    focus.clear();
}

fn restart_word(_: On<Activate>, mut practice: ResMut<Practice>, data: Res<HanziData>, mut speaker: ResMut<Speaker>, mut focus: ResMut<InputFocus>) {
    let word = practice.word;
    practice.begin(word);
    speaker.say(&data.words[word].word);
    focus.clear();
}

fn next_level(_: On<Activate>, mut practice: ResMut<Practice>, mut progress: ResMut<Progress>, data: Res<HanziData>, mut focus: ResMut<InputFocus>) {
    let level = progress.hanzi.level + 1;
    go_to_level(&mut practice, &mut progress, &data, level);
    focus.clear();
}

/// Starts a level of the hanzi mode, its first word.
pub fn go_to_level(practice: &mut Practice, progress: &mut Progress, data: &HanziData, level: usize) {
    progress.hanzi.level = level.min(data.levels.len() - 1);
    progress.hanzi.word = 0;
    progress.save();
    practice.started = false;
}

/// The point of the pointer in the pixels of the image.
fn canvas_point(
    position: Vec2,
    window: &Window,
    canvas: &Query<(&ComputedNode, &UiGlobalTransform), With<CanvasNode>>,
) -> Option<Vec2> {
    let (node, transform) = canvas.iter().next()?;
    let size = node.size();
    if size.x <= 0.0 {
        return None;
    }
    let physical = position * window.scale_factor();
    let top_left = transform.affine().translation - size / 2.0;
    Some((physical - top_left) / size * SIZE as f32)
}

/// The pressure of a pen or a finger, 0 to 1, when the device tells it.
fn pressure(touches: &Touches) -> Option<f32> {
    touches.iter().find_map(|t| t.force()).map(|force| match force {
        ForceTouch::Normalized(f) => f as f32,
        ForceTouch::Calibrated { force, max_possible_force, .. } => (force / max_possible_force.max(1e-6)) as f32,
    })
}

#[allow(clippy::too_many_arguments)]
fn pen_down(
    press: On<PointerPress>,
    window: Single<&Window>,
    canvas: Query<(&ComputedNode, &UiGlobalTransform), With<CanvasNode>>,
    progress: Res<Progress>,
    touches: Res<Touches>,
    time: Res<Time<Real>>,
    mut practice: ResMut<Practice>,
) {
    if press.button != PointerButton::Primary || practice.stage == Stage::Pinyin || practice.level_done {
        return;
    }
    if let Some(p) = canvas_point(press.pointer.position, &window, &canvas) {
        practice.ink = vec![p];
        practice.ink_widths.clear();
        if progress.brush {
            // The tip rests as it touches down: the widest ink, or the pressure.
            let width = pressure(&touches).map_or(brush::width_of_speed(0.0, None), brush::width_of_pressure);
            practice.ink_widths.push(width);
        }
        practice.last_point = time.elapsed_secs_f64();
        practice.drawing = true;
        practice.dirty = true;
    }
}

fn pen_move(
    drag: On<PointerDrag>,
    window: Single<&Window>,
    canvas: Query<(&ComputedNode, &UiGlobalTransform), With<CanvasNode>>,
    touches: Res<Touches>,
    time: Res<Time<Real>>,
    mut practice: ResMut<Practice>,
) {
    if !practice.drawing {
        return;
    }
    if let Some(p) = canvas_point(drag.pointer.position, &window, &canvas)
        && let Some(last) = practice.ink.last().copied()
        && last.distance(p) >= 2.0
    {
        if !practice.ink_widths.is_empty() {
            let now = time.elapsed_secs_f64();
            let dt = (now - practice.last_point).max(1.0 / 240.0) as f32;
            practice.last_point = now;
            let width = match pressure(&touches) {
                Some(pressure) => brush::width_of_pressure(pressure),
                None => brush::width_of_speed(last.distance(p) / dt, practice.ink_widths.last().copied()),
            };
            practice.ink_widths.push(width);
        }
        practice.ink.push(p);
        practice.dirty = true;
    }
}

fn pen_up(_: On<PointerRelease>, mut practice: ResMut<Practice>, data: Res<HanziData>, mut speaker: ResMut<Speaker>) {
    finish_stroke(&mut practice, &data, &mut speaker);
}

fn pen_drag_end(_: On<PointerDragEnd>, mut practice: ResMut<Practice>, data: Res<HanziData>, mut speaker: ResMut<Speaker>) {
    finish_stroke(&mut practice, &data, &mut speaker);
}

/// Compares the stroke drawn with the expected one.
fn finish_stroke(practice: &mut Practice, data: &HanziData, speaker: &mut Speaker) {
    if !practice.drawing {
        return;
    }
    practice.drawing = false;
    practice.dirty = true;
    let ink = std::mem::take(&mut practice.ink);
    let widths = std::mem::take(&mut practice.ink_widths);
    if ink.len() < 2 {
        return;
    }
    let Some(glyph) = practice.current_char(data).and_then(|c| data.glyph(c)) else {
        return;
    };
    let points: Vec<Vec2> = ink.iter().map(|p| render::from_pixels(*p)).collect();
    let options = Options {
        leniency: if practice.stage == Stage::Memory { 1.15 } else { 1.0 },
        outline_visible: matches!(practice.stage, Stage::Trace | Stage::Faded),
    };
    let stroke = practice.stroke;
    let n = glyph.medians.len();
    match matcher::stroke_matches(&points, &glyph.medians, stroke, options) {
        Outcome::Match => {
            practice.word_strokes += 1;
            practice.misses_on_stroke = 0;
            practice.hint = 0.0;
            practice.stroke += 1;
            practice.skip_given(data);
            practice.message.clear();
            // With the brush: its thickness against the stroke's, the ink stays a moment.
            if !widths.is_empty() {
                let target = brush::profile(&glyph.outlines[stroke], &glyph.medians[stroke]);
                if let Some(advice) = brush::advice(&target, &brush::along(&ink, &widths, 24)) {
                    practice.message = format!("{} {advice}", glyph.kinds.get(stroke).map_or("", |k| k.hanzi));
                }
                practice.echo = Some((ink, widths, ECHO_SECONDS));
            }
            if practice.stroke >= n {
                next_char(practice, data, speaker);
            }
        }
        outcome => {
            practice.misses_on_stroke += 1;
            practice.stage_misses += 1;
            practice.word_misses += 1;
            practice.wrong = Some((ink, WRONG_SECONDS));
            practice.message = match outcome {
                Outcome::Backwards => "The right stroke, drawn backwards: start at its other end.".to_string(),
                Outcome::OtherStroke(other)
                    if !(practice.stage == Stage::Partial && !taken_out(other, n, practice.word)) =>
                {
                    format!("That is stroke {}, stroke {} comes first: the order matters.", other + 1, stroke + 1)
                }
                _ => format!("Not quite: stroke {} of {n}.", stroke + 1),
            };
            if practice.misses_on_stroke >= MISSES_BEFORE_HINT {
                practice.hint = HINT_SECONDS;
                practice.message.push_str(" Here it is.");
            }
        }
    }
}

fn next_char(practice: &mut Practice, data: &HanziData, speaker: &mut Speaker) {
    practice.char_index += 1;
    practice.stroke = 0;
    practice.skip_given(data);
    if practice.char_index < practice.word(data).word.chars().count() {
        return;
    }
    // The step is done: again with many misses, else the next one.
    let stage = practice.stage;
    if practice.stage_misses >= MISSES_TO_REPEAT && stage != Stage::Trace {
        practice.start_stage(stage, data);
        practice.message = format!("{} misses: once more, slowly.", MISSES_TO_REPEAT);
        return;
    }
    let next = stage.next().unwrap_or(Stage::Pinyin);
    practice.start_stage(next, data);
    practice.message = match next {
        Stage::Pinyin => "Now the pinyin: type it and pick the word.".into(),
        _ => "Well done.".into(),
    };
    if next == Stage::Memory {
        speaker.say(&practice.word(data).word);
    }
}

/// The keys: the pinyin in its step, Enter for the next level, Space to listen.
#[allow(clippy::too_many_arguments)]
fn keys(
    mut pressed: MessageReader<KeyPressed>,
    mut practice: ResMut<Practice>,
    mut progress: ResMut<Progress>,
    data: Res<HanziData>,
    mut ime: ResMut<InputMethod>,
    mut speaker: ResMut<Speaker>,
    panel: Res<crate::ui::Panel>,
) {
    if *panel != crate::ui::Panel::None {
        pressed.clear();
        return;
    }
    let t9 = progress.pinyin_keypad;
    for key in pressed.read() {
        let Some(c) = key.typed else {
            continue;
        };
        if practice.level_done {
            if c == UPSYM {
                let level = progress.hanzi.level + 1;
                go_to_level(&mut practice, &mut progress, &data, level);
            }
            continue;
        }
        if practice.stage != Stage::Pinyin {
            if c == ' ' {
                speaker.say(&practice.word(&data).word);
            }
            continue;
        }
        let pick = match c {
            ' ' | UPSYM => Some(0),
            '1'..='8' if !t9 => Some(c as usize - '1' as usize),
            _ => None,
        };
        if let Some(i) = pick {
            pick_candidate(&mut practice, &mut progress, &data, &mut speaker, i);
            continue;
        }
        match c {
            BACKSPACE => {
                practice.input.pop();
            }
            '2'..='9' if t9 => practice.input.push(c),
            'a'..='z' | 'A'..='Z' if !t9 => practice.input.push(c.to_ascii_lowercase()),
            // ü
            'ü' if !t9 => practice.input.push('v'),
            _ => continue,
        }
        refresh_candidates(&mut practice, &data, &mut ime, t9);
    }
}

fn refresh_candidates(practice: &mut Practice, data: &HanziData, ime: &mut InputMethod, t9: bool) {
    let ime = ime.0.get_or_insert_with(|| Ime::new(data));
    practice.candidates =
        if practice.input.is_empty() { Vec::new() } else { ime.candidates(data, &practice.input, t9) };
}

#[allow(clippy::too_many_arguments)]
fn on_t9_key(
    act: On<Activate>,
    keys: Query<&T9Key>,
    mut practice: ResMut<Practice>,
    mut progress: ResMut<Progress>,
    data: Res<HanziData>,
    mut ime: ResMut<InputMethod>,
    mut speaker: ResMut<Speaker>,
    mut focus: ResMut<InputFocus>,
) {
    focus.clear();
    let Ok(T9Key(key)) = keys.get(act.entity) else {
        return;
    };
    if practice.stage != Stage::Pinyin {
        return;
    }
    match key {
        '2'..='9' => practice.input.push(*key),
        '<' => {
            practice.input.pop();
        }
        'x' => practice.input.clear(),
        ' ' => {
            pick_candidate(&mut practice, &mut progress, &data, &mut speaker, 0);
            return;
        }
        _ => return,
    }
    // The keypad types digits, whatever the setting.
    refresh_candidates(&mut practice, &data, &mut ime, true);
}

/// The word picked: the right one ends the word.
fn pick_candidate(practice: &mut Practice, progress: &mut Progress, data: &HanziData, speaker: &mut Speaker, i: usize) {
    let Some(candidate) = practice.candidates.get(i).cloned() else {
        return;
    };
    let word = practice.word(data).clone();
    if candidate.word != word.word {
        practice.word_misses += 1;
        practice.message = format!("That is {} ({}), not the word: try again.", candidate.word, candidate.pinyin);
        let stats = progress.hanzi.words.entry(word.word.clone()).or_default();
        stats.pinyin_misses += 1;
        return;
    }
    speaker.say(&word.word);
    let stats: &mut HanziWordStats = progress.hanzi.words.entry(word.word.clone()).or_default();
    stats.practiced += 1;
    stats.strokes += practice.word_strokes;
    stats.misses += practice.word_misses;
    // A session for the statistics of the day: the accuracy of the strokes.
    let now = crate::progress::now_unix_seconds();
    let total = (practice.word_strokes + practice.word_misses).max(1);
    let session = crate::progress::Session {
        mode: ExerciseMode::Hanzi,
        lesson: Some(data.level_of(practice.word) + 1),
        accuracy: 100.0 * practice.word_strokes as f64 / total as f64,
        velocity: 0.0,
        fluidness: 0.0,
        finished_at: now,
        elapsed: now.saturating_sub(practice.begun).min(600) as f64,
    };
    progress.current().sessions.push(session);
    let level = data.level_of(practice.word);
    let range = data.levels[level].words.clone();
    if practice.word + 1 < range.end {
        progress.hanzi.level = level;
        progress.hanzi.word = practice.word + 1 - range.start;
        progress.save();
        let next = practice.word + 1;
        practice.begin(next);
        practice.message = format!("{} ({}) learned. The next word.", word.word, word.pinyin);
        // The word just picked is spoken first, the next one with Listen.
    } else {
        let misses: u32 = range.clone().filter_map(|w| progress.hanzi.words.get(&data.words[w].word)).map(|s| s.misses).sum();
        progress.hanzi.levels_done.insert(level, misses);
        progress.hanzi.word = range.len() - 1;
        progress.save();
        practice.level_done = true;
        practice.candidates.clear();
        practice.input.clear();
        practice.message = format!(
            "Level {} done, {} words! Enter or Next level for the next ten.",
            level + 1,
            range.len()
        );
    }
}

/// For the screenshots of the snapshot harness: `CLAUDRALINGO_HANZI_STROKES` strokes written
/// on their medians, one every few frames, and `CLAUDRALINGO_HANZI_INPUT` typed in the pinyin
/// step.
fn auto_strokes(
    mut practice: ResMut<Practice>,
    mut progress: ResMut<Progress>,
    data: Res<HanziData>,
    mut speaker: ResMut<Speaker>,
    mut ime: ResMut<InputMethod>,
    mut frame: Local<u32>,
    mut done: Local<usize>,
) {
    let Some(strokes) = std::env::var("CLAUDRALINGO_HANZI_STROKES").ok().and_then(|n| n.parse::<usize>().ok()) else {
        return;
    };
    *frame += 1;
    if *frame < 60 || !frame.is_multiple_of(3) {
        return;
    }
    if *done < strokes && practice.stage != Stage::Pinyin {
        let Some(glyph) = practice.current_char(&data).and_then(|c| data.glyph(c)).cloned() else {
            return;
        };
        let stroke = practice.stroke.min(glyph.medians.len() - 1);
        practice.ink = glyph.medians[stroke].iter().map(|p| render::to_pixels(*p)).collect();
        // With the brush, as thick as the stroke: its profile.
        if progress.brush {
            let profile = brush::profile(&glyph.outlines[stroke], &glyph.medians[stroke]);
            let scale = SIZE as f32 / data::BOX;
            practice.ink = profile.points.iter().map(|p| render::to_pixels(*p)).collect();
            practice.ink_widths = profile.widths.iter().map(|w| (w * scale).max(brush::INK_MIN)).collect();
        }
        practice.drawing = true;
        finish_stroke(&mut practice, &data, &mut speaker);
        *done += 1;
    } else if practice.stage == Stage::Pinyin
        && practice.input.is_empty()
        && let Ok(input) = std::env::var("CLAUDRALINGO_HANZI_INPUT")
    {
        practice.input = input;
        let t9 = progress.pinyin_keypad;
        refresh_candidates(&mut practice, &data, &mut ime, t9);
        progress.set_changed();
    }
}

fn fade(time: Res<Time>, mut practice: ResMut<Practice>) {
    let dt = time.delta_secs();
    if practice.hint > 0.0 {
        practice.hint = (practice.hint - dt).max(0.0);
        practice.dirty = true;
    }
    if let Some((_, left)) = practice.wrong.as_mut() {
        *left -= dt;
        let gone = *left <= 0.0;
        practice.dirty = true;
        if gone {
            practice.wrong = None;
        }
    }
    if let Some((_, _, left)) = practice.echo.as_mut() {
        *left -= dt;
        let gone = *left <= 0.0;
        practice.dirty = true;
        if gone {
            practice.echo = None;
        }
    }
}

/// The brush guide of the next stroke, for the word, the character and the stroke.
type GuideCache = Option<((usize, usize, usize), brush::Guide)>;

/// The brush guide is shown: the outline is there to follow, or the hint.
fn shows_guide(practice: &Practice, progress: &Progress) -> bool {
    progress.brush
        && !practice.level_done
        && (matches!(practice.stage, Stage::Trace | Stage::Faded) || practice.hint > 0.0 && practice.stage != Stage::Pinyin)
}

#[allow(clippy::too_many_arguments)]
fn draw_canvas(
    mut practice: ResMut<Practice>,
    mut canvas: ResMut<Canvas>,
    data: Res<HanziData>,
    progress: Res<Progress>,
    theme: Res<UiTheme>,
    mut images: ResMut<Assets<Image>>,
    mut nodes: Query<&mut ImageNode, With<CanvasNode>>,
    mut guide_of: Local<GuideCache>,
) {
    for mut node in &mut nodes {
        if node.image != canvas.image {
            node.image = canvas.image.clone();
            practice.dirty = true;
        }
    }
    if !practice.dirty && !theme.is_changed() && !progress.is_changed() {
        return;
    }
    practice.dirty = false;
    let colors = breakdown::colors(&theme);
    let glyph = practice.current_char(&data).and_then(|c| data.glyph(c));
    let Canvas { image, pixmap } = &mut *canvas;
    match glyph {
        Some(glyph) => {
            // In the pinyin step and at the end of a level, the last character is shown whole.
            let n = glyph.medians.len();
            let looks = if practice.stage == Stage::Pinyin || practice.level_done {
                vec![Look::Done; n]
            } else {
                practice.looks(n)
            };
            let wrong = practice.wrong.as_ref().map(|(p, t)| (p.as_slice(), (t / WRONG_SECONDS).clamp(0.0, 1.0)));
            let echo = practice.echo.as_ref().map(|(p, w, t)| (p.as_slice(), w.as_slice(), (t / ECHO_SECONDS).clamp(0.0, 1.0)));
            let key = (practice.word, practice.char_index, practice.stroke);
            let guide = if shows_guide(&practice, &progress) && practice.stroke < n {
                if guide_of.as_ref().is_none_or(|(k, _)| *k != key) {
                    let profile = brush::profile(&glyph.outlines[key.2], &glyph.medians[key.2]);
                    *guide_of = Some((key, brush::guide(&profile, glyph.kinds[key.2])));
                }
                guide_of.as_ref().map(|(_, g)| g)
            } else {
                None
            };
            let frame = render::Frame {
                looks: &looks,
                ink: &practice.ink,
                ink_widths: &practice.ink_widths,
                wrong,
                echo,
                guide,
            };
            render::draw(pixmap, glyph, &frame, &colors);
        }
        None => {
            let empty = data::Glyph { outlines: Vec::new(), medians: Vec::new(), kinds: Vec::new() };
            render::draw(pixmap, &empty, &render::Frame::default(), &colors);
        }
    }
    if let Some(mut image) = images.get_mut(image) {
        image.data = Some(pixmap.data().to_vec());
    }
}

#[allow(clippy::type_complexity)]
fn update_view(
    practice: Res<Practice>,
    progress: Res<Progress>,
    data: Res<HanziData>,
    mut texts: ParamSet<(
        Query<&mut Text, With<WordText>>,
        Query<&mut Text, With<PinyinText>>,
        Query<&mut Text, With<MeaningText>>,
        Query<&mut Text, With<StageText>>,
        Query<&mut Text, With<MessageText>>,
        Query<&mut Text, With<InputText>>,
    )>,
    mut panels: ParamSet<(
        Query<&mut Node, With<PinyinPanel>>,
        Query<&mut Node, With<Keypad>>,
        Query<&mut Node, With<NextLevelButton>>,
    )>,
) {
    if !practice.is_changed() && !progress.is_changed() {
        return;
    }
    let word = practice.word(&data);
    let level = data.level_of(practice.word);
    let info = &data.levels[level];
    let pinyin_step = practice.stage == Stage::Pinyin && !practice.level_done;
    let memory = practice.stage == Stage::Memory && !practice.level_done;
    let set = |text: &mut Text, value: String| {
        if text.0 != value {
            text.0 = value;
        }
    };
    // The word is hidden while it is written from memory, the pinyin while it is typed.
    for mut t in &mut texts.p0() {
        set(&mut t, if memory { "？".repeat(word.word.chars().count()) } else { word.word.clone() });
    }
    for mut t in &mut texts.p1() {
        set(&mut t, if pinyin_step { "…".into() } else { word.pinyin.clone() });
    }
    for mut t in &mut texts.p2() {
        set(&mut t, word.meaning.clone());
    }
    let n_chars = word.word.chars().count();
    let stage = if practice.level_done {
        format!("HSK {} · level {}/{} done", info.hsk, info.part, info.parts)
    } else {
        format!(
            "HSK {} · level {}/{} · word {}/{} · step {} of 5: {}{}",
            info.hsk,
            info.part,
            info.parts,
            practice.word + 1 - info.words.start,
            info.words.len(),
            practice.stage.number(),
            practice.stage.instructions(),
            if n_chars > 1 && !pinyin_step {
                format!(" · character {} of {n_chars}", (practice.char_index + 1).min(n_chars))
            } else {
                String::new()
            }
        )
    };
    for mut t in &mut texts.p3() {
        set(&mut t, stage.clone());
    }
    for mut t in &mut texts.p4() {
        set(&mut t, practice.message.clone());
    }
    let input = if progress.pinyin_keypad {
        let spelled = if practice.input.is_empty() {
            String::new()
        } else {
            pinyin::t9_spellings(&practice.input).first().map(|s| s.letters.clone()).unwrap_or_else(|| "?".into())
        };
        format!("Keys: {}   {spelled}", practice.input)
    } else {
        format!("Pinyin: {}", practice.input)
    };
    for mut t in &mut texts.p5() {
        set(&mut t, input.clone());
    }
    let show = |visible: bool| if visible { Display::Flex } else { Display::None };
    for mut node in &mut panels.p0() {
        node.display = show(pinyin_step);
    }
    for mut node in &mut panels.p1() {
        node.display = if progress.pinyin_keypad { Display::Grid } else { Display::None };
    }
    for mut node in &mut panels.p2() {
        node.display = show(practice.level_done);
    }
}

/// The kind of the next stroke while the outline is there to follow, and with the brush how to
/// write it, its hanzi in the CJK font.
#[allow(clippy::too_many_arguments)]
fn update_kind_text(
    practice: Res<Practice>,
    progress: Res<Progress>,
    data: Res<HanziData>,
    font: Res<HanziFont>,
    asset_server: Res<AssetServer>,
    texts: Query<Entity, With<KindText>>,
    mut shown: Local<Option<String>>,
    mut commands: Commands,
) {
    if !practice.is_changed() && !progress.is_changed() {
        return;
    }
    let glyph = practice.current_char(&data).and_then(|c| data.glyph(c));
    let visible = !practice.level_done
        && practice.stage != Stage::Pinyin
        && (matches!(practice.stage, Stage::Trace | Stage::Faded) || practice.hint > 0.0);
    let text = match glyph.and_then(|g| g.kinds.get(practice.stroke).map(|k| (k, g.kinds.len()))) {
        Some((kind, n)) if visible => {
            let mut text = format!("Stroke {} of {n}: {} {}, {}", practice.stroke + 1, kind.hanzi, kind.pinyin, kind.english);
            if progress.brush {
                text.push_str(&format!(".\n{}", kind.brush));
            }
            text
        }
        _ => String::new(),
    };
    if shown.as_ref() == Some(&text) {
        return;
    }
    let fonts = breakdown::Fonts { latin: asset_server.load(bevy::feathers::constants::fonts::REGULAR), cjk: font.0.clone() };
    for entity in &texts {
        fonts.fill(&mut commands, entity, &text, 15.0, feathers_tokens::TEXT_MAIN);
    }
    *shown = Some(text);
}

/// The candidate buttons, in the CJK font, a click picks one.
fn update_candidates(
    practice: Res<Practice>,
    font: Res<HanziFont>,
    rows: Query<Entity, With<CandidatesRow>>,
    theme: Res<UiTheme>,
    asset_server: Res<AssetServer>,
    mut shown: Local<Vec<Candidate>>,
    mut commands: Commands,
) {
    if *shown == practice.candidates {
        return;
    }
    *shown = practice.candidates.clone();
    for row in &rows {
        commands.entity(row).despawn_children();
        for (i, candidate) in practice.candidates.iter().enumerate() {
            let button = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(px(8), px(4)),
                        border_radius: BorderRadius::all(px(6)),
                        ..default()
                    },
                    BackgroundColor(theme.color(&tokens::KEY_BG)),
                    ChildOf(row),
                ))
                .observe(
                    move |_: On<PointerClick>,
                          mut practice: ResMut<Practice>,
                          mut progress: ResMut<Progress>,
                          data: Res<HanziData>,
                          mut speaker: ResMut<Speaker>| {
                        pick_candidate(&mut practice, &mut progress, &data, &mut speaker, i);
                    },
                )
                .id();
            commands.spawn((
                Text::new(candidate.word.clone()),
                hanzi_font(&font, 30.0),
                ThemeTextColor(feathers_tokens::TEXT_MAIN),
                Pickable::IGNORE,
                ChildOf(button),
            ));
            commands.spawn((
                Text::new(format!("{} {}", i + 1, candidate.pinyin)),
                TextFont {
                    font: FontSource::Handle(asset_server.load(bevy::feathers::constants::fonts::REGULAR)),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                ThemeTextColor(feathers_tokens::TEXT_DIM),
                Pickable::IGNORE,
                ChildOf(button),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steps_of_a_word() {
        let data = HanziData::load();
        let mut practice = Practice::default();
        let mut speaker = Speaker::default();
        // 人: two strokes, drawn on their medians.
        let word = data.words.iter().position(|w| w.word == "人").expect("人");
        practice.begin(word);
        let glyph = data.glyph('人').expect("人").clone();
        let draw = |practice: &mut Practice, speaker: &mut Speaker, stroke: usize| {
            practice.ink = glyph.medians[stroke].iter().map(|p| render::to_pixels(*p)).collect();
            practice.drawing = true;
            finish_stroke(practice, &data, speaker);
        };
        for stage in [Stage::Trace, Stage::Faded] {
            assert_eq!(practice.stage, stage);
            draw(&mut practice, &mut speaker, 0);
            draw(&mut practice, &mut speaker, 1);
        }
        // Partial: one stroke is given, the other taken out.
        assert_eq!(practice.stage, Stage::Partial);
        let expected = practice.stroke;
        assert!(expected < 2);
        draw(&mut practice, &mut speaker, expected);
        assert_eq!(practice.stage, Stage::Memory);
        // Out of order: stroke 2 first.
        draw(&mut practice, &mut speaker, 1);
        assert_eq!(practice.stroke, 0);
        assert!(practice.message.contains("comes first"), "{}", practice.message);
        draw(&mut practice, &mut speaker, 0);
        draw(&mut practice, &mut speaker, 1);
        assert_eq!(practice.stage, Stage::Pinyin);
    }
}
