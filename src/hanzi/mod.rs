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

/// The misses on a stroke before it is shown being written, the glimpse.
const MISSES_BEFORE_GLIMPSE: u32 = 3;
/// The glimpse: the stroke written in `GLIMPSE_DRAW` seconds, then it stays and fades.
const GLIMPSE_SECONDS: f32 = 2.8;
const GLIMPSE_DRAW: f32 = 1.0;
const GLIMPSE_FADE: f32 = 0.8;
/// A glimpse counts as this many misses in the accuracy of the word.
const GLIMPSE_PENALTY: u32 = 2;
/// The banner of a new word: where it is in the level, and a word of encouragement.
const BANNER_SECONDS: f32 = 3.5;
const BANNER_FADE: f32 = 1.2;
const ENCOURAGEMENTS: [&str; 10] = [
    "Good luck!",
    "Have fun!",
    "Journey before destination.",
    "One stroke at a time.",
    "Slow is smooth, smooth is fast.",
    "加油！ Keep going!",
    "千里之行，始于足下: a journey of a thousand miles begins with one step.",
    "Breathe, then write.",
    "Every master was once a beginner.",
    "The brush follows the heart.",
];
const WRONG_SECONDS: f32 = 0.6;
/// The last brush stroke stays over the character, to compare its thickness.
const ECHO_SECONDS: f32 = 1.4;
/// The misses of a step that repeat it.
const MISSES_TO_REPEAT: u32 = 3;
/// The size of the drawing area, in logical pixels, it grows with the window, see
/// [`fit_canvas`].
const CANVAS: f32 = 380.0;
const CANVAS_MIN: f32 = 220.0;
const CANVAS_MAX: f32 = 600.0;
/// The column beside the drawing: the word, the steps, the buttons and the pinyin; narrower to
/// keep the drawing beside it.
const SIDE: f32 = 380.0;
const SIDE_MIN: f32 = 280.0;
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
    /// The glimpse of the next stroke, its seconds left.
    glimpse: f32,
    /// The strokes shown this word, the character and the stroke of the last one.
    glimpses: u32,
    glimpsed: Option<(usize, usize)>,
    /// The accuracies of the strokes accepted this word, in percent.
    accuracies: Vec<f32>,
    /// The word is one of the weak words reviewed.
    reviewing: bool,
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
struct SideColumn;
/// The row of the drawing and the column, it wraps in a narrow window.
#[derive(Component, Default, Clone)]
struct DrawingRow;
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
#[derive(Component, Default, Clone)]
struct NextLevelCaption;
/// The banner of a new word over the drawing, fading.
#[derive(Component, Default, Clone)]
struct Banner;
#[derive(Component, Default, Clone)]
struct BannerText;
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
                PostUpdate,
                (
                    cjk_fallback_of_propagated.before(bevy::app::PropagateSet::<TextFont>::default()),
                    cjk_fallback
                        .after(bevy::app::PropagateSet::<TextFont>::default())
                        .before(bevy::ui::UiSystems::Content),
                ),
            )
            .add_systems(
                Update,
                (
                    start_word,
                    keys,
                    auto_strokes,
                    fade,
                    draw_canvas,
                    update_view,
                    update_kind_text,
                    update_candidates,
                    speak,
                    set_fonts,
                    update_banner,
                    fit_canvas,
                )
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

/// The view, in the column of the window, shown in the hanzi mode: the drawing and beside it
/// the word, the step, the buttons and the pinyin; above the drawing when the window is
/// narrow (the row is reversed and wraps).
pub fn view() -> impl Scene {
    bsn! {
        Node {
            display: Display::None,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            width: percent(100),
            margin: UiRect::top(px(6)),
        }
        HanziRoot
        Children [
            Node {
                flex_direction: FlexDirection::RowReverse,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Start,
                column_gap: px(16),
                row_gap: px(10),
                width: percent(100),
                padding: UiRect::horizontal(px(8)),
            }
            DrawingRow
            Children [
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    width: px(SIDE),
                    max_width: percent(100),
                }
                SideColumn
                Children [
                    // The word, its pinyin and its meaning.
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(14),
                        padding: UiRect::axes(px(14), px(8)),
                        border_radius: BorderRadius::all(px(10)),
                    }
                    ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
                    Children [
                        Text("") WordText
                        ThemeTextColor(feathers_tokens::TEXT_MAIN)
                        --
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(4),
                            flex_shrink: 1.0,
                            min_width: px(0),
                        }
                        Children [
                            @label("") PinyinText
                            --
                            @label_dim("") MeaningText
                        ]
                    ]
                    --
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(10),
                    }
                    Children [
                        @FeathersButton {
                            @caption: bsn! { @caption("Listen") }
                        }
                        on(listen)
                        --
                        @crate::ui::setting_checkbox(crate::ui::Setting::Sound, "Speak the words")
                    ]
                    --
                    @label("") StageText
                    --
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
                            @caption: bsn! { @caption("Next level") NextLevelCaption }
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
                    --
                    @label("") MessageText
                ]
                --
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
                Children [
                    // Over the bottom of the drawing, the strokes start at the top; it lets the
                    // pen through.
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(14),
                        left: px(0),
                        width: percent(100),
                        display: Display::None,
                        justify_content: JustifyContent::Center,
                    }
                    Banner
                    Pickable::IGNORE
                    Children [
                        Node {
                            padding: UiRect::axes(px(16), px(8)),
                            border_radius: BorderRadius::all(px(10)),
                            max_width: percent(92),
                        }
                        BackgroundColor(Color::NONE)
                        Pickable::IGNORE
                        Children [
                            Text("") BannerText
                            TextColor(Color::NONE)

                            TextLayout { justify: Justify::Center }
                            Pickable::IGNORE
                        ]
                    ]
                ]
            ]
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

/// The CJK font on the texts of hanzi, the feathers font does not have them; the font of
/// feathers on the banner.
fn set_fonts(
    font: Res<HanziFont>,
    asset_server: Res<AssetServer>,
    mut words: Query<&mut TextFont, (Added<WordText>, Without<BannerText>)>,
    mut banners: Query<&mut TextFont, Added<BannerText>>,
) {
    for mut text in &mut words {
        text.font = FontSource::Handle(font.0.clone());
        text.font_size = FontSize::Px(WORD_FONT);
    }
    for mut text in &mut banners {
        text.font = FontSource::Handle(asset_server.load(bevy::feathers::constants::fonts::REGULAR));
        text.font_size = FontSize::Px(17.0);
    }
}

/// The font with the CJK font after it: the characters it has not are taken from that one. The
/// meanings cite hanzi (`as in 意`), the Fira fonts of feathers have none, they showed boxes.
fn with_cjk(source: &FontSource, cjk: &Handle<Font>) -> Option<FontSource> {
    match source {
        FontSource::Handle(handle) if handle != cjk => {
            Some(FontSource::List(vec![FontSource::Handle(handle.clone()), FontSource::Handle(cjk.clone())]))
        }
        _ => None,
    }
}

/// The fonts that feathers hands down to the texts of its widgets.
fn cjk_fallback_of_propagated(mut fonts: Query<&mut bevy::app::Propagate<TextFont>>, cjk: Option<Res<HanziFont>>) {
    let Some(cjk) = cjk else {
        return;
    };
    for mut propagate in &mut fonts {
        if let Some(font) = with_cjk(&propagate.0.font, &cjk.0) {
            propagate.0.font = font;
        }
    }
}

/// The fonts of the other texts.
fn cjk_fallback(mut fonts: Query<&mut TextFont, Changed<TextFont>>, cjk: Option<Res<HanziFont>>) {
    let Some(cjk) = cjk else {
        return;
    };
    for mut text in &mut fonts {
        if let Some(font) = with_cjk(&text.font, &cjk.0) {
            text.font = font;
        }
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
        let reviewing = self.reviewing;
        *self = Practice { word, reviewing, dirty: true, started: true, begun: crate::progress::now_unix_seconds(), ..default() };
        self.message = "A new word: listen, then trace its strokes.".into();
    }

    /// Shows the next stroke being written. A penalty, once per stroke: the word becomes weak.
    fn show_glimpse(&mut self) {
        self.glimpse = GLIMPSE_SECONDS;
        self.dirty = true;
        let at = (self.char_index, self.stroke);
        if self.glimpsed != Some(at) {
            self.glimpsed = Some(at);
            self.glimpses += 1;
        }
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

    /// The looks of the strokes of the current character; without the motion, the glimpse is
    /// the whole stroke at once.
    fn looks(&self, n: usize, motion: bool) -> Vec<Look> {
        (0..n)
            .map(|i| {
                let given = self.stage == Stage::Partial && !taken_out(i, n, self.word);
                if i < self.stroke || given {
                    return Look::Done;
                }
                if i == self.stroke && self.glimpse > 0.0 {
                    let written = if motion { (GLIMPSE_SECONDS - self.glimpse) / GLIMPSE_DRAW } else { 1.0 };
                    return Look::Reveal(written.min(1.0), 0.85 * (self.glimpse / GLIMPSE_FADE).min(1.0));
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
    // Another level or the review picked, with ‹ › or the menus.
    let (word, reviewing) = target_word(&progress, &data);
    if practice.started && practice.word == word && practice.reviewing == reviewing {
        return;
    }
    practice.reviewing = reviewing;
    practice.begin(word);
    speaker.say(&data.words[word].word);
}

/// The word to write: the one of the review, else the one of the level.
fn target_word(progress: &Progress, data: &HanziData) -> (usize, bool) {
    let hanzi = &progress.hanzi;
    if hanzi.review
        && let Some(word) = hanzi.review_words.get(hanzi.review_word).and_then(|w| data.index_of(w))
    {
        return (word, true);
    }
    let words = &data.levels[hanzi.level.min(data.levels.len() - 1)].words;
    ((words.start + hanzi.word).min(words.end - 1), false)
}

/// The weak words: a stroke of them was shown, the weakest first, then the most missed.
pub fn weak_words(progress: &Progress, data: &HanziData) -> Vec<String> {
    let mut words: Vec<(&String, &HanziWordStats)> =
        progress.hanzi.words.iter().filter(|(w, s)| s.weak > 0 && data.index_of(w).is_some()).collect();
    words.sort_by(|a, b| b.1.weak.cmp(&a.1.weak).then(b.1.misses.cmp(&a.1.misses)));
    words.into_iter().take(data::WORDS_PER_LEVEL).map(|(w, _)| w.clone()).collect()
}

/// Starts the review of the weak words, false without any.
pub fn start_review(progress: &mut Progress, data: &HanziData) -> bool {
    let words = weak_words(progress, data);
    if words.is_empty() {
        return false;
    }
    progress.hanzi.review = true;
    progress.hanzi.review_words = words;
    progress.hanzi.review_word = 0;
    progress.save();
    true
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
    } else if !practice.level_done {
        practice.show_glimpse();
        practice.message = "Watch the stroke written, then write it. The word goes to your weak words.".into();
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
    after_level(&mut practice, &mut progress, &data);
    focus.clear();
}

/// At the end of a level the next one, at the end of the review the level again.
fn after_level(practice: &mut Practice, progress: &mut Progress, data: &HanziData) {
    if progress.hanzi.review {
        progress.hanzi.review = false;
        progress.save();
        practice.started = false;
    } else {
        let level = progress.hanzi.level + 1;
        go_to_level(practice, progress, data, level);
    }
}

/// Starts a level of the hanzi mode, its first word.
pub fn go_to_level(practice: &mut Practice, progress: &mut Progress, data: &HanziData, level: usize) {
    progress.hanzi.level = level.min(data.levels.len() - 1);
    progress.hanzi.word = 0;
    progress.hanzi.review = false;
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
            practice.glimpse = 0.0;
            practice.stroke += 1;
            practice.skip_given(data);
            // How close it was, and with the brush its thickness against the stroke's, the ink
            // stays a moment.
            let accuracy = matcher::accuracy(&points, &glyph.medians[stroke]);
            practice.accuracies.push(accuracy);
            let kind = glyph.kinds.get(stroke).map_or("", |k| k.hanzi);
            practice.message = format!("{kind} {accuracy:.0}%, {}.", matcher::accuracy_word(accuracy));
            if !widths.is_empty() {
                let target = brush::profile(&glyph.outlines[stroke], &glyph.medians[stroke]);
                if let Some(advice) = brush::advice(&target, &brush::along(&ink, &widths, 24)) {
                    practice.message = format!("{} {advice}", practice.message);
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
            if practice.misses_on_stroke >= MISSES_BEFORE_GLIMPSE {
                practice.show_glimpse();
                practice.message.push_str(" Watch it written, then write it: the word goes to your weak words.");
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
                after_level(&mut practice, &mut progress, &data);
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
    // The strokes shown make the word weak, a word written without them less so.
    stats.glimpses += practice.glimpses;
    if practice.glimpses > 0 {
        stats.weak += practice.glimpses;
    } else {
        stats.weak = stats.weak.saturating_sub(1);
    }
    let quality = (!practice.accuracies.is_empty())
        .then(|| practice.accuracies.iter().map(|a| *a as f64).sum::<f64>() / practice.accuracies.len() as f64);
    if let Some(q) = quality {
        stats.quality_sum += q * practice.accuracies.len() as f64;
        stats.quality_n += practice.accuracies.len() as u32;
    }
    // A session for the statistics of the day: the strokes right on the first try, a glimpse
    // counts as misses.
    let now = crate::progress::now_unix_seconds();
    let total = (practice.word_strokes + practice.word_misses + GLIMPSE_PENALTY * practice.glimpses).max(1);
    let session = crate::progress::Session {
        mode: ExerciseMode::Hanzi,
        lesson: Some(data.level_of(practice.word) + 1),
        accuracy: 100.0 * practice.word_strokes as f64 / total as f64,
        velocity: 0.0,
        fluidness: 0.0,
        finished_at: now,
        elapsed: now.saturating_sub(practice.begun).min(600) as f64,
        quality,
    };
    progress.current().sessions.push(session);
    let learned = match quality {
        Some(q) => format!("{} ({}) learned, the strokes {q:.0}% accurate", word.word, word.pinyin),
        None => format!("{} ({}) learned", word.word, word.pinyin),
    };
    if progress.hanzi.review {
        let hanzi = &mut progress.hanzi;
        hanzi.review_word += 1;
        if let Some(next) = hanzi.review_words.get(hanzi.review_word).and_then(|w| data.index_of(w)) {
            progress.save();
            practice.begin(next);
            practice.message = format!("{learned}. The next weak word.");
        } else {
            let n = hanzi.review_words.len();
            progress.save();
            practice.level_done = true;
            practice.candidates.clear();
            practice.input.clear();
            practice.message = format!("{learned}. The {n} weak words reviewed! Enter or Back to the level.");
        }
        return;
    }
    let level = data.level_of(practice.word);
    let range = data.levels[level].words.clone();
    if practice.word + 1 < range.end {
        progress.hanzi.level = level;
        progress.hanzi.word = practice.word + 1 - range.start;
        progress.save();
        let next = practice.word + 1;
        practice.begin(next);
        practice.message = format!("{learned}. The next word.");
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
            "{learned}. Level {} done, {} words! Enter or Next level for the next ten.",
            level + 1,
            range.len()
        );
    }
}

/// For the screenshots of the snapshot harness: `CLAUDRALINGO_HANZI_STROKES` strokes written
/// on their medians, one every few frames, after `CLAUDRALINGO_HANZI_MISSES` scribbles that
/// miss, and `CLAUDRALINGO_HANZI_INPUT` typed in the pinyin step.
#[allow(clippy::too_many_arguments)]
fn auto_strokes(
    mut practice: ResMut<Practice>,
    mut progress: ResMut<Progress>,
    data: Res<HanziData>,
    mut speaker: ResMut<Speaker>,
    mut ime: ResMut<InputMethod>,
    mut frame: Local<u32>,
    mut done: Local<usize>,
    mut missed: Local<usize>,
) {
    let Some(strokes) = std::env::var("CLAUDRALINGO_HANZI_STROKES").ok().and_then(|n| n.parse::<usize>().ok()) else {
        return;
    };
    *frame += 1;
    if *frame < 60 || !frame.is_multiple_of(3) {
        return;
    }
    let misses = std::env::var("CLAUDRALINGO_HANZI_MISSES").ok().and_then(|n| n.parse::<usize>().ok()).unwrap_or(0);
    if *missed < misses {
        practice.ink = vec![Vec2::new(30.0, 470.0), Vec2::new(60.0, 480.0), Vec2::new(40.0, 500.0)];
        practice.drawing = true;
        finish_stroke(&mut practice, &data, &mut speaker);
        *missed += 1;
    } else if *done < strokes && practice.stage != Stage::Pinyin {
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
    if practice.glimpse > 0.0 {
        practice.glimpse = (practice.glimpse - dt).max(0.0);
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

/// The brush guide is shown: the outline is there to follow, or the glimpse.
fn shows_guide(practice: &Practice, progress: &Progress) -> bool {
    progress.brush
        && !practice.level_done
        && (matches!(practice.stage, Stage::Trace | Stage::Faded) || practice.glimpse > 0.0 && practice.stage != Stage::Pinyin)
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
                practice.looks(n, !progress.reduce_motion)
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
        Query<&mut Text, With<NextLevelCaption>>,
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
    // Where the word is in the level shows in the banner when it starts, see `update_banner`.
    let n_chars = word.word.chars().count();
    let stage = if practice.level_done {
        format!("{}: done", place(&progress, &data, &practice, false))
    } else {
        format!(
            "Step {} of 5: {}{}",
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
    for mut t in &mut texts.p6() {
        set(&mut t, if progress.hanzi.review { "Back to the level".into() } else { "Next level".into() });
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

/// Where the word is: `HSK 1 · level 3/15 · word 2/10`, or in the review of the weak words.
pub fn place(progress: &Progress, data: &HanziData, practice: &Practice, with_word: bool) -> String {
    let hanzi = &progress.hanzi;
    if hanzi.review {
        let n = hanzi.review_words.len();
        return if with_word { format!("Weak words · word {}/{n}", (hanzi.review_word + 1).min(n)) } else { "Weak words".into() };
    }
    let info = &data.levels[data.level_of(practice.word)];
    let level = format!("HSK {} · level {}/{}", info.hsk, info.part, info.parts);
    if with_word { format!("{level} · word {}/{}", practice.word + 1 - info.words.start, info.words.len()) } else { level }
}

/// The banner of a new word over the drawing, then it fades.
#[allow(clippy::too_many_arguments)]
fn update_banner(
    practice: Res<Practice>,
    progress: Res<Progress>,
    data: Res<HanziData>,
    theme: Res<UiTheme>,
    time: Res<Time>,
    mut banners: Query<(&mut Node, &Children), With<Banner>>,
    mut pills: Query<&mut BackgroundColor>,
    mut texts: Query<(&mut Text, &mut TextColor), With<BannerText>>,
    mut shown: Local<Option<(usize, bool, u64)>>,
    mut left: Local<f32>,
    mut next: Local<usize>,
) {
    // Every start of a word, also back from the touch typing.
    let key = (practice.word, practice.reviewing, practice.begun);
    if practice.started && *shown != Some(key) {
        *shown = Some(key);
        *left = BANNER_SECONDS;
        let encouragement = ENCOURAGEMENTS[*next % ENCOURAGEMENTS.len()];
        *next += 1 + fastrand::usize(..3);
        for (mut text, _) in &mut texts {
            text.0 = format!("{}\n{encouragement}", place(&progress, &data, &practice, true));
        }
    } else if *left <= 0.0 {
        return;
    }
    *left -= time.delta_secs();
    let alpha = (*left / BANNER_FADE).clamp(0.0, 1.0);
    for (mut node, children) in &mut banners {
        node.display = if *left > 0.0 { Display::Flex } else { Display::None };
        for child in children.iter() {
            if let Ok(mut background) = pills.get_mut(child) {
                background.0 = theme.color(&feathers_tokens::WINDOW_BG).with_alpha(0.92 * alpha);
            }
        }
    }
    for (_, mut color) in &mut texts {
        color.0 = theme.color(&feathers_tokens::TEXT_MAIN).with_alpha(alpha);
    }
}

/// The drawing as big as the window allows: beside the column of the word when there is room,
/// that column narrower if need be, else under it; over it in a window wider than high, the
/// drawing first.
#[allow(clippy::type_complexity)]
fn fit_canvas(
    window: Single<&Window>,
    scale: Res<UiScale>,
    bars: Query<
        (&ComputedNode, &Node),
        (Or<(With<crate::ui::Toolbar>, With<crate::ui::ActionsBar>)>, Without<CanvasNode>, Without<SideColumn>, Without<DrawingRow>),
    >,
    mut nodes: ParamSet<(
        Query<&mut Node, With<CanvasNode>>,
        Query<&mut Node, With<SideColumn>>,
        Query<&mut Node, With<DrawingRow>>,
    )>,
) {
    let size = window.size() / scale.0.max(0.1);
    // The toolbar over the window when it hides takes no room.
    let bars: f32 = bars
        .iter()
        .filter(|(_, node)| node.position_type == PositionType::Relative)
        .map(|(computed, _)| computed.size().y * computed.inverse_scale_factor)
        .sum();
    let free_height = size.y - bars - 40.0;
    let free_width = size.x - 32.0;
    let beside = free_width - SIDE_MIN - 16.0 >= CANVAS_MIN;
    let wrap = if !beside && size.x > size.y { FlexWrap::WrapReverse } else { FlexWrap::Wrap };
    let (canvas, side) = if beside {
        let canvas = free_height.min(free_width - SIDE_MIN - 16.0).clamp(CANVAS_MIN, CANVAS_MAX).floor();
        (canvas, (free_width - canvas - 16.0).clamp(SIDE_MIN, SIDE).floor())
    } else {
        (free_width.min((free_height - 180.0).max(CANVAS_MIN)).clamp(CANVAS_MIN, CANVAS_MAX).floor(), SIDE)
    };
    for mut node in &mut nodes.p0() {
        if node.width != px(canvas) {
            node.width = px(canvas);
            node.height = px(canvas);
        }
    }
    for mut node in &mut nodes.p1() {
        if node.width != px(side) {
            node.width = px(side);
        }
    }
    for mut node in &mut nodes.p2() {
        if node.flex_wrap != wrap {
            node.flex_wrap = wrap;
        }
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
        && (matches!(practice.stage, Stage::Trace | Stage::Faded) || practice.glimpse > 0.0);
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

    #[test]
    fn a_glimpse_makes_the_word_weak() {
        let data = HanziData::load();
        let mut progress = Progress::default();
        let mut practice = Practice::default();
        let mut speaker = Speaker::default();
        let word = data.index_of("人").expect("人");
        practice.begin(word);
        // Three scribbles on the first stroke: it is shown, once.
        for _ in 0..MISSES_BEFORE_GLIMPSE + 1 {
            practice.ink = vec![Vec2::new(30.0, 470.0), Vec2::new(60.0, 480.0), Vec2::new(40.0, 500.0)];
            practice.drawing = true;
            finish_stroke(&mut practice, &data, &mut speaker);
        }
        assert!(practice.glimpse > 0.0);
        assert_eq!(practice.glimpses, 1);
        assert!(matches!(practice.looks(2, true)[0], Look::Reveal(t, _) if t < 0.1));
        // The stroke on its median: accepted, its accuracy near 100%.
        let glyph = data.glyph('人').expect("人").clone();
        practice.ink = glyph.medians[0].iter().map(|p| render::to_pixels(*p)).collect();
        practice.drawing = true;
        finish_stroke(&mut practice, &data, &mut speaker);
        assert_eq!(practice.stroke, 1);
        assert!(practice.accuracies[0] > 95.0, "{:?}", practice.accuracies);
        assert!(practice.message.contains("excellent"), "{}", practice.message);
        // The word picked: it is weak, the session counts the glimpse.
        practice.stage = Stage::Pinyin;
        practice.candidates = vec![Candidate { word: "人".into(), pinyin: "rén".into() }];
        pick_candidate(&mut practice, &mut progress, &data, &mut speaker, 0);
        let stats = &progress.hanzi.words["人"];
        assert_eq!((stats.glimpses, stats.weak), (1, 1));
        let session = progress.current().sessions.last().cloned().expect("a session");
        assert!(session.accuracy < 50.0 && session.quality.is_some(), "{session:?}");
        assert_eq!(weak_words(&progress, &data), vec!["人".to_string()]);
        // The review: the word again, then back to the level; written without help, less weak.
        assert!(start_review(&mut progress, &data));
        assert_eq!(target_word(&progress, &data), (word, true));
        practice.begin(word);
        practice.stage = Stage::Pinyin;
        practice.candidates = vec![Candidate { word: "人".into(), pinyin: "rén".into() }];
        pick_candidate(&mut practice, &mut progress, &data, &mut speaker, 0);
        assert!(practice.level_done);
        assert_eq!(progress.hanzi.words["人"].weak, 0);
        after_level(&mut practice, &mut progress, &data);
        assert!(!progress.hanzi.review);
    }
}
