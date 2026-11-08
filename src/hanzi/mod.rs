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
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, UiTheme};
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
/// A new word is written before the eyes first: the seconds per stroke, and after the last.
const INTRO_STROKE: f32 = 0.55;
const INTRO_END: f32 = 0.5;
/// The word glows once a write is done, to sink in.
const GLOW_SECONDS: f32 = 1.5;
/// A write done, the word is written again before the eyes in green, as slowly as the intro,
/// then flashes this many times, once a second, and fades out: the drawing is clean for the
/// next write. See [`finale_looks`].
const FINALE_FLASHES: f32 = 3.0;
const FINALE_FADE: f32 = 2.0;
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
/// The word at the top of the column, smaller for the words of three characters or more.
const WORD_FONT: f32 = 88.0;
const LONG_WORD_FONT: f32 = 60.0;
/// The feedback of the strokes, at the bottom of the column.
const MESSAGE_FONT: f32 = 17.0;

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

/// The writes of a word before its pinyin: ten the first time, two fewer each time it comes
/// back, never under two; a weak word (a stroke had to be shown) gets two more. A learner with
/// many words behind starts the new ones lower: eight from ten words learned, six from thirty,
/// five from a hundred; `wanted` (the settings) sets the first time instead, up to a hundred.
pub fn n_writes(stats: Option<&HanziWordStats>, words_learned: usize, wanted: u32) -> usize {
    let first: usize = match (wanted, words_learned) {
        (1.., _) => wanted as usize,
        (0, 0..=9) => 10,
        (0, 10..=29) => 8,
        (0, 30..=99) => 6,
        _ => 5,
    };
    let practiced = stats.map_or(0, |s| s.practiced) as usize;
    let weak = stats.is_some_and(|s| s.weak > 0);
    let n = first.saturating_sub(2 * practiced).max(2);
    if weak { (n + 2).min(first) } else { n }
}

/// The steps of `n` writes: over the outline first, from memory last. Four writes are one of
/// each step, the extra ones go to the tracing first, then to the memory, then to the
/// filling in: ten writes are 3 traced, 3 faded, 2 filled in, 2 from memory. Three writes
/// skip the faded outline, two trace nothing.
pub fn plan(n: usize) -> Vec<Stage> {
    let counts: [usize; 4] = match n {
        0..=2 => [0, 0, 1, 1],
        3 => [1, 0, 1, 1],
        _ => {
            let mut counts = [1, 1, 1, 1];
            for extra in 0..n - 4 {
                counts[[0, 1, 3, 2][extra % 4]] += 1;
            }
            counts
        }
    };
    [Stage::Trace, Stage::Faded, Stage::Partial, Stage::Memory]
        .iter()
        .zip(counts)
        .flat_map(|(stage, count)| std::iter::repeat_n(*stage, count))
        .collect()
}

impl Stage {
    fn instructions(self) -> &'static str {
        match self {
            Stage::Trace => "Trace the strokes in order, over the outline",
            Stage::Faded => "Trace the strokes over the faded outline",
            Stage::Partial => "Fill in the missing strokes",
            Stage::Memory => "Write the word from memory",
            Stage::Pinyin => "Type the pinyin, then pick the word",
        }
    }
}

/// The color of the message: the last stroke accepted, missed, or neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Tone {
    #[default]
    Plain,
    Good,
    Bad,
}

impl Tone {
    fn token(self) -> bevy::feathers::theme::ThemeToken {
        match self {
            Tone::Plain => feathers_tokens::TEXT_MAIN,
            Tone::Good => tokens::CHAR_CORRECT,
            Tone::Bad => tokens::CHAR_WRONG,
        }
    }
}

/// The word being practiced.
#[derive(Resource, Default)]
pub struct Practice {
    /// The index of the word in [`HanziData::words`].
    pub word: usize,
    pub stage: Stage,
    /// The writes of the word, see [`plan`], and the one being written; after them the pinyin.
    plan: Vec<Stage>,
    step: usize,
    /// Which strokes are taken out in the partial writes, the other half on each next one.
    parity: usize,
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
    /// The character written before the eyes, the brush following its strokes: the seconds
    /// since it began, on the first write of a word.
    intro: Option<f32>,
    /// The seconds left of the glow of the word after a write.
    glow: f32,
    /// The write just done, played again in green: the word and the seconds since it began.
    finale: Option<(usize, f32)>,
    pub message: String,
    tone: Tone,
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
/// The box of the pinyin typed, drawn as a focused text field, with its blinking caret.
#[derive(Component, Default, Clone)]
struct Caret;
/// The bar of the writes of the word, as wide as the drawing.
#[derive(Component, Default, Clone)]
struct HanziBar;
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
/// The strokes of the character: a line each, the name bold and its meaning plain, the next one
/// with how to write it.
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
                    blink_caret,
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
                column_gap: px(20),
                row_gap: px(10),
                width: percent(100),
                padding: UiRect::horizontal(px(8)),
            }
            DrawingRow
            Children [
                // As high as the drawing, its top level with the drawing's: the word in the
                // top quarter, the strokes in the middle half, the feedback and the buttons in
                // the bottom quarter. A section with more grows, the others keep their share.
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    width: px(SIDE),
                    max_width: percent(100),
                    // Under the bar of the writes, level with the drawing.
                    padding: UiRect::top(px(14)),
                }
                SideColumn
                Children [
                    // The word, its pinyin and its meaning.
                    Node {
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Start,
                    }
                    Children [
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(18),
                            padding: UiRect::axes(px(16), px(10)),
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
                                TextFont { font_size: FontSize::Px(26.0) }
                                --
                                @label_dim("") MeaningText
                                TextFont { font_size: FontSize::Px(17.0) }
                            ]
                        ]
                    ]
                    --
                    // The strokes of the character, or the pinyin to type.
                    Node {
                        flex_grow: 2.0,
                        flex_basis: px(0),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                    }
                    Children [
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(5),
                        }
                        KindText
                        --
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Start,
                        row_gap: px(10),
                    }
                    PinyinPanel
                    Children [
                        // A text field with the focus: the keys go here now.
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            min_width: px(240),
                            min_height: px(42),
                            padding: UiRect::axes(px(12), px(6)),
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(px(6)),
                        }
                        ThemeBackgroundColor(feathers_tokens::TEXT_INPUT_BG)
                        ThemeBorderColor(feathers_tokens::FOCUS_RING)
                        Children [
                            @label("") InputText
                            TextFont { font_size: FontSize::Px(20.0) }
                            --
                            Node {
                                width: px(2),
                                height: px(24),
                                margin: UiRect::left(px(2)),
                            }
                            ThemeBackgroundColor(tokens::CURSOR)
                            Caret
                        ]
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
                    --
                    // What to do now and how the last stroke went, the buttons under them.
                    Node {
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::End,
                        row_gap: px(10),
                    }
                    Children [
                    @label("") StageText
                    TextFont { font_size: FontSize::Px(17.0) }
                    --
                    @label("") MessageText
                    TextFont { font_size: FontSize::Px(MESSAGE_FONT) }
                    --
                    Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        align_items: AlignItems::Center,
                        column_gap: px(8),
                        row_gap: px(8),
                    }
                    Children [
                        @FeathersButton {
                            @caption: bsn! { @crate::icons::icon(crate::icons::Icon::Listen) }
                        }
                        on(listen)
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("Hint") }
                        }
                        on(show_hint)
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("Stroke breakdown") }
                        }
                        on(breakdown::open)
                        --
                        @FeathersButton {
                            @caption: bsn! { @crate::icons::icon(crate::icons::Icon::Print) }
                        }
                        on(crate::print::open)
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("Next level") NextLevelCaption }
                            @variant: ButtonVariant::Primary,
                        }
                        NextLevelButton
                        Node { display: Display::None }
                        on(next_level)
                    ]
                    ]
                ]
                --
                // The bar of the writes over the drawing, as wide as it.
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                }
                Children [
                    @crate::ui::steps_bar()
                    Node {
                        width: px(CANVAS),
                    }
                    HanziBar
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
    fn begin(&mut self, word: usize, plan: Vec<Stage>, data: &HanziData) {
        let reviewing = self.reviewing;
        let stage = plan.first().copied().unwrap_or(Stage::Pinyin);
        *self = Practice { word, reviewing, plan, dirty: true, started: true, begun: crate::progress::now_unix_seconds(), ..default() };
        self.start_stage(stage, data);
        // Traced first: the brush writes it before the eyes.
        self.intro = (stage == Stage::Trace).then_some(0.0);
        self.message = match self.plan.len() {
            n if n >= 8 => format!("A new word: listen, then trace its strokes. {n} writes, then its pinyin."),
            n => format!("Listen, then write it: {n} writes, then its pinyin."),
        };
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

    pub fn word<'a>(&self, data: &'a HanziData) -> &'a data::Word {
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
        while self.stage == Stage::Partial && self.stroke < n && !taken_out(self.stroke, n, self.word + self.parity) {
            self.stroke += 1;
        }
    }

    fn start_stage(&mut self, stage: Stage, data: &HanziData) {
        if stage == Stage::Partial {
            self.parity += 1;
        }
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
        if let Some(t) = self.intro {
            // The brush writes the strokes in order, in ink; the rest wait as outlines.
            let at = (t / INTRO_STROKE) as usize;
            return (0..n)
                .map(|i| match i.cmp(&at) {
                    std::cmp::Ordering::Less => Look::Done,
                    std::cmp::Ordering::Equal => Look::Reveal((t / INTRO_STROKE).fract(), 1.0),
                    std::cmp::Ordering::Greater => Look::Outline(0.3),
                })
                .collect();
        }
        (0..n)
            .map(|i| {
                let given = self.stage == Stage::Partial && !taken_out(i, n, self.word + self.parity);
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

    /// The brush of the intro, in pixels.
    fn tip(&self, glyph: &data::Glyph) -> Option<Vec2> {
        let t = self.intro?;
        let at = (t / INTRO_STROKE) as usize;
        render::point_along(glyph.medians.get(at)?, (t / INTRO_STROKE).fract())
    }

    /// The seconds the intro of the character takes.
    fn intro_length(&self, data: &HanziData) -> f32 {
        let n = self.current_char(data).and_then(|c| data.glyph(c)).map_or(0, |g| g.medians.len());
        n as f32 * INTRO_STROKE + INTRO_END
    }

    /// The writes of the word as steps, the pinyin the last one.
    fn steps(&self, data: &HanziData) -> crate::ui::Steps {
        let total = self.plan.len() + 1;
        let done = if self.level_done {
            total as f32
        } else if self.stage == Stage::Pinyin {
            self.plan.len() as f32
        } else {
            let n_chars = self.word(data).word.chars().count().max(1) as f32;
            let strokes = self.current_char(data).and_then(|c| data.glyph(c)).map_or(1, |g| g.medians.len()).max(1) as f32;
            self.step as f32 + (self.char_index as f32 + self.stroke as f32 / strokes) / n_chars
        };
        crate::ui::Steps { total, done }
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
    let plan = plan_for(&progress, &data, word);
    practice.begin(word, plan, &data);
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

/// What becomes of a word to say: said in the hanzi lesson, kept while the main menu is over it
/// (said if the hanzi is picked there), dropped in the other subjects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Say {
    Now,
    Later,
    Never,
}

fn when_to_say(mode: ExerciseMode, panel: crate::ui::Panel) -> Say {
    match (mode, panel) {
        (ExerciseMode::Hanzi, crate::ui::Panel::Home) => Say::Later,
        (ExerciseMode::Hanzi, _) => Say::Now,
        _ => Say::Never,
    }
}

/// Plays the recordings one after the other, only in the hanzi lesson: the word of the last
/// session is not said over the main menu at the start.
#[allow(clippy::too_many_arguments)]
fn speak(
    mut speaker: ResMut<Speaker>,
    data: Res<HanziData>,
    progress: Res<Progress>,
    panel: Res<crate::ui::Panel>,
    mut sources: ResMut<Assets<AudioSource>>,
    players: Query<(), With<AudioPlayer>>,
    mut commands: Commands,
) {
    match when_to_say(progress.mode, *panel) {
        Say::Now => {}
        Say::Later => return,
        Say::Never => {
            speaker.pending = None;
            return;
        }
    }
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
        // The eager hand: the brush stops showing, the green word too.
        practice.intro = None;
        practice.finale = None;
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
            practice.tone = Tone::Good;
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
            practice.tone = Tone::Bad;
            practice.message = match outcome {
                Outcome::Backwards => "The right stroke, drawn backwards: start at its other end.".to_string(),
                Outcome::OtherStroke(other)
                    if !(practice.stage == Stage::Partial && !taken_out(other, n, practice.word + practice.parity)) =>
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
        if practice.step == 0 && practice.stage == Stage::Trace {
            practice.intro = Some(0.0);
        }
        return;
    }
    // The word written: it glows, to sink in, the message green with it, and is written again
    // in green on the drawing.
    practice.glow = GLOW_SECONDS;
    practice.finale = Some((practice.word, 0.0));
    practice.tone = Tone::Good;
    // The step is done: again with many misses, else the next one.
    let stage = practice.stage;
    if practice.stage_misses >= MISSES_TO_REPEAT && stage != Stage::Trace {
        practice.start_stage(stage, data);
        practice.message = format!("{} misses: once more, slowly.", MISSES_TO_REPEAT);
        return;
    }
    practice.step += 1;
    let next = practice.plan.get(practice.step).copied().unwrap_or(Stage::Pinyin);
    practice.start_stage(next, data);
    practice.message = match next {
        Stage::Pinyin => "All the writes done!".into(),
        _ => "Well done.".into(),
    };
    // The word is said again when the writes from memory start.
    if next == Stage::Memory && stage != Stage::Memory {
        speaker.say(&practice.word(data).word);
    }
}

/// The writes of a word for a learner: its own history and the words learned so far.
fn plan_for(progress: &Progress, data: &HanziData, word: usize) -> Vec<Stage> {
    let learned = progress.hanzi.words.values().filter(|s| s.practiced > 0).count();
    plan(n_writes(progress.hanzi.words.get(&data.words[word].word), learned, progress.hanzi.writes))
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
        practice.tone = Tone::Bad;
        let stats = progress.hanzi.words.entry(word.word.clone()).or_default();
        stats.pinyin_misses += 1;
        return;
    }
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
            let plan = plan_for(progress, data, next);
            practice.begin(next, plan, data);
            practice.message = format!("{learned}. The next weak word.");
            // The word on the screen is the one heard: the new one, not the one just finished.
            speaker.say(&data.words[next].word);
        } else {
            let n = hanzi.review_words.len();
            progress.save();
            speaker.say(&word.word);
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
        let plan = plan_for(progress, data, next);
        practice.begin(next, plan, data);
        practice.message = format!("{learned}. The next word.");
        // The word on the screen is the one heard: the new one, not the one just finished.
        speaker.say(&data.words[next].word);
    } else {
        let misses: u32 = range.clone().filter_map(|w| progress.hanzi.words.get(&data.words[w].word)).map(|s| s.misses).sum();
        progress.hanzi.levels_done.insert(level, misses);
        progress.hanzi.word = range.len() - 1;
        progress.save();
        speaker.say(&word.word);
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
        practice.intro = None;
        practice.finale = None;
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

/// The number of strokes of each character of a word.
fn strokes_of(data: &HanziData, word: usize) -> Vec<usize> {
    data.words.get(word).map_or(Vec::new(), |w| w.word.chars().map(|c| data.glyph(c).map_or(0, |g| g.medians.len())).collect())
}

/// The finale at `t` seconds: the character on the drawing and the looks of its strokes, `None`
/// once it is over. Each character is written in turn as in the intro, then the last one
/// flashes, its opacity from 1 to 0 and back once a second, and fades out.
fn finale_looks(t: f32, strokes: &[usize]) -> Option<(usize, Vec<Look>)> {
    let mut t = t.max(0.0);
    for (i, n) in strokes.iter().enumerate() {
        let length = *n as f32 * INTRO_STROKE + INTRO_END;
        if t < length {
            let at = (t / INTRO_STROKE) as usize;
            let looks = (0..*n)
                .map(|k| match k.cmp(&at) {
                    std::cmp::Ordering::Less => Look::Reveal(1.0, 1.0),
                    std::cmp::Ordering::Equal => Look::Reveal((t / INTRO_STROKE).fract(), 1.0),
                    std::cmp::Ordering::Greater => Look::Hidden,
                })
                .collect();
            return Some((i, looks));
        }
        t -= length;
    }
    let last = strokes.len().checked_sub(1)?;
    let alpha = if t < FINALE_FLASHES {
        0.5 + 0.5 * (std::f32::consts::TAU * t).cos()
    } else if t < FINALE_FLASHES + FINALE_FADE {
        1.0 - (t - FINALE_FLASHES) / FINALE_FADE
    } else {
        return None;
    };
    Some((last, vec![Look::Reveal(1.0, alpha); strokes[last]]))
}

fn fade(time: Res<Time>, progress: Res<Progress>, data: Res<HanziData>, mut practice: ResMut<Practice>) {
    let dt = time.delta_secs();
    if let Some(t) = practice.intro {
        let length = practice.intro_length(&data);
        practice.intro = (!progress.reduce_motion && t + dt < length).then_some(t + dt);
        practice.dirty = true;
    }
    if let Some((word, t)) = practice.finale {
        let strokes = strokes_of(&data, word);
        practice.finale = (!progress.reduce_motion && finale_looks(t + dt, &strokes).is_some()).then_some((word, t + dt));
        practice.dirty = true;
    }
    if practice.glow > 0.0 {
        practice.glow = (practice.glow - dt).max(0.0);
        if practice.glow == 0.0 && practice.tone == Tone::Good {
            practice.tone = Tone::Plain;
        }
    }
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
        && practice.intro.is_none()
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
    let mut colors = breakdown::colors(&theme);
    // The write just done, again in green, over a clean drawing.
    if let Some((word, t)) = practice.finale
        && let Some((char_index, looks)) = finale_looks(t, &strokes_of(&data, word))
        && let Some(glyph) = data.words.get(word).and_then(|w| w.word.chars().nth(char_index)).and_then(|c| data.glyph(c))
    {
        colors.next = theme.color(&tokens::CHAR_CORRECT);
        let frame = render::Frame { looks: &looks, ..Default::default() };
        render::draw(&mut canvas.pixmap, glyph, &frame, &colors);
        let Canvas { image, pixmap } = &mut *canvas;
        if let Some(mut image) = images.get_mut(image) {
            image.data = Some(pixmap.data().to_vec());
        }
        return;
    }
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
                tip: practice.tip(glyph),
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
        Query<(Entity, &mut Text, &mut TextFont), With<WordText>>,
        Query<&mut Text, With<PinyinText>>,
        Query<&mut Text, With<MeaningText>>,
        Query<&mut Text, With<StageText>>,
        Query<(Entity, &mut Text, &mut TextFont), With<MessageText>>,
        Query<(Entity, &mut Text), With<InputText>>,
        Query<&mut Text, With<NextLevelCaption>>,
    )>,
    mut panels: ParamSet<(
        Query<&mut Node, With<PinyinPanel>>,
        Query<&mut Node, With<Keypad>>,
        Query<&mut Node, With<NextLevelButton>>,
        Query<&mut crate::ui::Steps, With<HanziBar>>,
    )>,
    mut colored: Local<(bool, Tone, bool)>,
    mut commands: Commands,
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
    // The word is hidden while it is written from memory, the pinyin while it is typed; after
    // a write the word glows green, at the same size: a bigger word pushed the column down.
    let glowing = practice.glow > 0.0;
    for (entity, mut t, mut font) in &mut texts.p0() {
        set(&mut t, if memory { "？".repeat(word.word.chars().count()) } else { word.word.clone() });
        let size = FontSize::Px(if word.word.chars().count() > 2 { LONG_WORD_FONT } else { WORD_FONT });
        if font.font_size != size {
            font.font_size = size;
        }
        if colored.0 != glowing {
            commands.entity(entity).insert(ThemeTextColor(if glowing { tokens::CHAR_CORRECT } else { feathers_tokens::TEXT_MAIN }));
        }
    }
    colored.0 = glowing;
    for mut t in &mut texts.p1() {
        set(&mut t, if pinyin_step { "…".into() } else { word.pinyin.clone() });
    }
    for mut t in &mut texts.p2() {
        set(&mut t, word.meaning.clone());
    }
    // Where the word is in the level shows in the banner when it starts, see `update_banner`;
    // the writes in the bar over the drawing.
    let stage = if practice.level_done {
        format!("{}: done", place(&progress, &data, &practice, false))
    } else if practice.intro.is_some() {
        "Watch the brush write it, then trace it".to_string()
    } else {
        practice.stage.instructions().to_string()
    };
    for mut t in &mut texts.p3() {
        set(&mut t, stage.clone());
    }
    for (entity, mut t, mut font) in &mut texts.p4() {
        set(&mut t, practice.message.clone());
        if colored.1 != practice.tone {
            commands.entity(entity).insert(ThemeTextColor(practice.tone.token()));
        }
        // One size: growing with the word it moved the lines under it.
        let size = FontSize::Px(MESSAGE_FONT);
        if font.font_size != size {
            font.font_size = size;
        }
    }
    colored.1 = practice.tone;
    // The field: the letters, the digits of the nine keys with their spelling, or what to do.
    let empty = practice.input.is_empty();
    let input = if empty {
        if progress.pinyin_keypad { "the keys of the pinyin".to_string() } else { "the pinyin".to_string() }
    } else if progress.pinyin_keypad {
        let spelled = pinyin::t9_spellings(&practice.input).first().map(|s| s.letters.clone()).unwrap_or_else(|| "?".into());
        format!("{}  {spelled}", practice.input)
    } else {
        practice.input.clone()
    };
    for (entity, mut t) in &mut texts.p5() {
        set(&mut t, input.clone());
        if colored.2 != empty {
            commands.entity(entity).insert(ThemeTextColor(if empty { feathers_tokens::TEXT_DIM } else { feathers_tokens::TEXT_MAIN }));
        }
    }
    colored.2 = empty;
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
    let steps = practice.steps(&data);
    for mut bar in &mut panels.p3() {
        if *bar != steps {
            *bar = steps;
        }
    }
}

/// The caret of the pinyin field blinks.
fn blink_caret(time: Res<Time<Real>>, mut carets: Query<&mut Node, With<Caret>>) {
    let on = time.elapsed_secs() % 1.0 < 0.6;
    for mut node in &mut carets {
        let display = if on { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
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
        (Or<(With<crate::ui::Toolbar>, With<crate::ui::ActionsBar>)>, Without<CanvasNode>, Without<SideColumn>, Without<DrawingRow>, Without<HanziBar>),
    >,
    mut nodes: ParamSet<(
        Query<&mut Node, With<CanvasNode>>,
        Query<&mut Node, With<SideColumn>>,
        Query<&mut Node, With<DrawingRow>>,
        Query<&mut Node, With<HanziBar>>,
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
    // Beside the drawing the column is as high as it, the bar of the writes included; under
    // it, as high as its content.
    let side_height = if beside { px(canvas + 14.0) } else { Val::Auto };
    for mut node in &mut nodes.p1() {
        if node.width != px(side) {
            node.width = px(side);
        }
        if node.min_height != side_height {
            node.min_height = side_height;
        }
    }
    for mut node in &mut nodes.p2() {
        if node.flex_wrap != wrap {
            node.flex_wrap = wrap;
        }
    }
    for mut node in &mut nodes.p3() {
        if node.width != px(canvas) {
            node.width = px(canvas);
        }
    }
}

/// A line of the strokes: the name bold (its hanzi and pinyin), what it is plain, how to write
/// it under the next stroke; dim once written and for the strokes ahead.
#[derive(Debug, Clone, PartialEq)]
struct StrokeLine {
    name: String,
    what: String,
    how: Option<String>,
    next: bool,
}

/// The lines of the strokes of the character: with the outline to follow (or the stroke shown)
/// all of them, else only the ones written, the others are for the memory.
fn stroke_lines(kinds: &[&strokes::Kind], stroke: usize, shown: bool, brush: bool) -> Vec<StrokeLine> {
    kinds
        .iter()
        .enumerate()
        .take(if shown { kinds.len() } else { stroke })
        .map(|(i, kind)| StrokeLine {
            name: format!("{} {}", kind.hanzi, kind.pinyin),
            what: kind.english.to_string(),
            how: (i == stroke && brush).then(|| kind.brush.to_string()),
            next: i == stroke,
        })
        .collect()
}

/// The strokes of the character as a list while it is written: where it is in the word, then a
/// line per stroke. Rebuilt when they change.
#[allow(clippy::too_many_arguments)]
fn update_kind_text(
    practice: Res<Practice>,
    progress: Res<Progress>,
    data: Res<HanziData>,
    font: Res<HanziFont>,
    asset_server: Res<AssetServer>,
    lists: Query<Entity, With<KindText>>,
    mut shown: Local<Option<(String, Vec<StrokeLine>)>>,
    mut commands: Commands,
) {
    if !practice.is_changed() && !progress.is_changed() {
        return;
    }
    let glyph = practice.current_char(&data).and_then(|c| data.glyph(c));
    // The kind of the stroke is a hint: with the outline to follow, or the stroke shown.
    let kind_shown = matches!(practice.stage, Stage::Trace | Stage::Faded) || practice.glimpse > 0.0;
    let n_chars = practice.word(&data).word.chars().count();
    let writing = !practice.level_done && practice.stage != Stage::Pinyin && practice.intro.is_none();
    let (head, lines) = match glyph.filter(|_| writing) {
        Some(glyph) => {
            let n = glyph.kinds.len();
            let mut head = String::new();
            if n_chars > 1 {
                head.push_str(&format!("Character {} of {n_chars}, ", practice.char_index + 1));
            }
            head.push_str(&format!("stroke {} of {n}", (practice.stroke + 1).min(n)));
            (crate::ui::capitalize(&head), stroke_lines(&glyph.kinds, practice.stroke, kind_shown, progress.brush))
        }
        None => (String::new(), Vec::new()),
    };
    if shown.as_ref().is_some_and(|(h, l)| *h == head && *l == lines) {
        return;
    }
    let regular = breakdown::Fonts { latin: asset_server.load(bevy::feathers::constants::fonts::REGULAR), cjk: font.0.clone() };
    let bold = breakdown::Fonts { latin: asset_server.load(bevy::feathers::constants::fonts::BOLD), cjk: font.0.clone() };
    for list in &lists {
        commands.entity(list).despawn_children();
        if head.is_empty() {
            continue;
        }
        regular.spawn(&mut commands, list, &head, 14.0, feathers_tokens::TEXT_DIM);
        for line in &lines {
            let ink = if line.next { feathers_tokens::TEXT_MAIN } else { feathers_tokens::TEXT_DIM };
            let row = commands
                .spawn((Node { flex_direction: FlexDirection::Row, column_gap: px(8), margin: UiRect::left(px(4)), ..default() }, Pickable::IGNORE, ChildOf(list)))
                .id();
            regular.spawn(&mut commands, row, "\u{2022}", 15.0, ink.clone());
            let column = commands
                .spawn((Node { flex_direction: FlexDirection::Column, row_gap: px(2), flex_shrink: 1.0, min_width: px(0), ..default() }, Pickable::IGNORE, ChildOf(row)))
                .id();
            let text = bold.spawn(&mut commands, column, &line.name, 15.0, ink.clone());
            regular.append(&mut commands, text, &format!("  {}", line.what), 15.0, ink.clone());
            if let Some(how) = &line.how {
                regular.spawn(&mut commands, column, how, 14.0, feathers_tokens::TEXT_DIM);
            }
        }
    }
    *shown = Some((head, lines));
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
    fn the_word_is_said_only_in_the_hanzi_lesson() {
        use crate::ui::Panel;
        assert_eq!(when_to_say(ExerciseMode::Hanzi, Panel::None), Say::Now);
        assert_eq!(when_to_say(ExerciseMode::Hanzi, Panel::Strokes), Say::Now);
        // The main menu at the start, over the hanzi of the last session: it waits.
        assert_eq!(when_to_say(ExerciseMode::Hanzi, Panel::Home), Say::Later);
        assert_eq!(when_to_say(ExerciseMode::Basic, Panel::None), Say::Never);
        assert_eq!(when_to_say(ExerciseMode::Penmanship, Panel::Home), Say::Never);
    }

    #[test]
    fn the_finale_writes_the_word_flashes_three_times_and_fades() {
        let strokes = [2, 3];
        let first = 2.0 * INTRO_STROKE + INTRO_END;
        let second = 3.0 * INTRO_STROKE + INTRO_END;
        // The first character, its first stroke half written.
        assert_eq!(finale_looks(0.5 * INTRO_STROKE, &strokes), Some((0, vec![Look::Reveal(0.5, 1.0), Look::Hidden])));
        // The second one, its first stroke written.
        let (char_index, looks) = finale_looks(first + 1.5 * INTRO_STROKE, &strokes).unwrap();
        assert_eq!(char_index, 1);
        assert_eq!(looks[0], Look::Reveal(1.0, 1.0));
        assert_eq!(looks[2], Look::Hidden);
        // Then the flashes: whole at each second, gone in between.
        let alpha = |t: f32| match finale_looks(first + second + t, &strokes) {
            Some((1, looks)) => match looks[0] {
                Look::Reveal(1.0, a) => a,
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        };
        assert!((alpha(0.0) - 1.0).abs() < 1e-4);
        assert!(alpha(0.5) < 1e-4);
        assert!((alpha(2.0) - 1.0).abs() < 1e-3);
        // The fade, then a clean drawing.
        assert!((alpha(FINALE_FLASHES + 1.0) - 0.5).abs() < 1e-3);
        assert_eq!(finale_looks(first + second + FINALE_FLASHES + FINALE_FADE + 0.01, &strokes), None);
        assert_eq!(finale_looks(0.0, &[]), None);
    }

    #[test]
    fn the_strokes_ahead_are_listed_only_with_the_outline() {
        let kinds: Vec<&strokes::Kind> = strokes::KINDS.iter().take(3).collect();
        let shown = stroke_lines(&kinds, 1, true, true);
        assert_eq!(shown.len(), 3);
        assert!(shown[1].next && shown[1].how.is_some());
        assert!(!shown[0].next && shown[0].how.is_none());
        assert!(shown[0].name.starts_with(kinds[0].hanzi));
        // From memory: the strokes written only, no hint of the next one.
        let memory = stroke_lines(&kinds, 1, false, true);
        assert_eq!(memory.len(), 1);
        assert!(!memory[0].next);
        assert!(stroke_lines(&kinds, 1, true, false).iter().all(|l| l.how.is_none()));
    }

    #[test]
    fn a_new_word_is_written_before_the_eyes_and_the_writes_fill_the_bar() {
        let data = HanziData::load();
        let word = data.index_of("人").expect("人");
        let mut practice = Practice::default();
        practice.begin(word, plan(4), &data);
        assert_eq!(practice.intro, Some(0.0));
        assert_eq!(practice.steps(&data), crate::ui::Steps { total: 5, done: 0.0 });
        let glyph = data.glyph('人').expect("the glyph");
        assert!(practice.tip(glyph).is_some());
        assert!(practice.intro_length(&data) > INTRO_STROKE);
        assert!(matches!(practice.looks(2, true)[0], Look::Reveal(_, _)));
        // Not when the word is filled in or written from memory.
        practice.begin(word, vec![Stage::Partial, Stage::Memory], &data);
        assert_eq!(practice.intro, None);
    }

    #[test]
    fn the_steps_of_a_word() {
        let data = HanziData::load();
        let mut practice = Practice::default();
        let mut speaker = Speaker::default();
        // 人: two strokes, drawn on their medians.
        let word = data.words.iter().position(|w| w.word == "人").expect("人");
        practice.begin(word, plan(4), &data);
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
    fn the_writes_taper_as_the_word_and_the_learner_progress() {
        let stats = |practiced, weak| HanziWordStats { practiced, weak, ..default() };
        assert_eq!(n_writes(None, 0, 0), 10);
        assert_eq!(n_writes(Some(&stats(1, 0)), 3, 0), 8);
        assert_eq!(n_writes(Some(&stats(3, 0)), 3, 0), 4);
        assert_eq!(n_writes(Some(&stats(6, 0)), 3, 0), 2);
        // A weak word gets two more, never above the first time.
        assert_eq!(n_writes(Some(&stats(3, 2)), 3, 0), 6);
        assert_eq!(n_writes(Some(&stats(0, 2)), 3, 0), 10);
        // The learner with words behind.
        assert_eq!(n_writes(None, 10, 0), 8);
        assert_eq!(n_writes(None, 30, 0), 6);
        assert_eq!(n_writes(None, 200, 0), 5);
        assert_eq!(n_writes(Some(&stats(2, 0)), 200, 0), 2);
        // The writes wanted in the settings, a hundred for the keen.
        assert_eq!(n_writes(None, 200, 100), 100);
        assert_eq!(n_writes(Some(&stats(1, 0)), 0, 20), 18);
        assert_eq!(plan(100).len(), 100);
        assert_eq!(plan(100).iter().filter(|s| **s == Memory).count(), 25);
        // The plans: tracing first, memory last.
        use Stage::*;
        assert_eq!(plan(10), vec![Trace, Trace, Trace, Faded, Faded, Faded, Partial, Partial, Memory, Memory]);
        assert_eq!(plan(5), vec![Trace, Trace, Faded, Partial, Memory]);
        assert_eq!(plan(4), vec![Trace, Faded, Partial, Memory]);
        assert_eq!(plan(3), vec![Trace, Partial, Memory]);
        assert_eq!(plan(2), vec![Partial, Memory]);
        for n in 2..=12 {
            assert_eq!(plan(n).len(), n.max(2));
        }
    }

    #[test]
    fn two_partial_writes_take_out_the_other_strokes() {
        let data = HanziData::load();
        let mut practice = Practice::default();
        let word = data.index_of("人").expect("人");
        practice.begin(word, vec![Stage::Partial, Stage::Partial], &data);
        let first = practice.stroke;
        practice.start_stage(Stage::Partial, &data);
        assert_ne!(practice.stroke, first);
    }

    #[test]
    fn the_next_word_is_the_one_said() {
        let data = HanziData::load();
        let mut progress = Progress::default();
        let mut practice = Practice::default();
        let mut speaker = Speaker::default();
        let mut ime = InputMethod(None);
        let word = data.levels[0].words.start;
        practice.begin(word, plan(2), &data);
        practice.start_stage(Stage::Pinyin, &data);
        practice.input = data::toneless(&data.words[word].pinyin).replace(' ', "");
        refresh_candidates(&mut practice, &data, &mut ime, false);
        let i = practice.candidates.iter().position(|c| c.word == data.words[word].word).expect("the word among the candidates");
        pick_candidate(&mut practice, &mut progress, &data, &mut speaker, i);
        assert_eq!(practice.word, word + 1);
        assert_eq!(speaker.pending.as_deref(), Some(data.words[word + 1].word.as_str()));
    }

    #[test]
    fn a_glimpse_makes_the_word_weak() {
        let data = HanziData::load();
        let mut progress = Progress::default();
        let mut practice = Practice::default();
        let mut speaker = Speaker::default();
        let word = data.index_of("人").expect("人");
        practice.begin(word, plan(4), &data);
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
        practice.begin(word, plan(4), &data);
        practice.stage = Stage::Pinyin;
        practice.candidates = vec![Candidate { word: "人".into(), pinyin: "rén".into() }];
        pick_candidate(&mut practice, &mut progress, &data, &mut speaker, 0);
        assert!(practice.level_done);
        assert_eq!(progress.hanzi.words["人"].weak, 0);
        after_level(&mut practice, &mut progress, &data);
        assert!(!progress.hanzi.review);
    }
}
