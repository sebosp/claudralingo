//! The strokes of a word, broken down: every character stroke by stroke with the names of the
//! strokes, a click on a stroke shows how the brush writes it and the basic strokes it is made
//! of, a click on a basic stroke describes it, and a character that has the stroke too breaks
//! down in turn.
use bevy::asset::RenderAssetUsages;
use bevy::feathers::containers::{group, group_body, group_header};
use bevy::feathers::controls::*;
use bevy::feathers::display::{caption, label};
use bevy::feathers::theme::{ThemeTextColor, UiTheme};
use bevy::feathers::tokens as feathers_tokens;
use bevy::input_focus::InputFocus;
use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::text::FontSource;
use bevy::ui_widgets::Activate;

use super::data::HanziData;
use super::strokes::Basic;
use super::{HanziFont, Practice, brush, render};
use crate::theme::tokens;
use crate::ui::Panel;

/// The side of the images of the strokes, in pixels, shown at half.
const TILE: u32 = 160;
const DETAIL: u32 = 512;
/// The characters that have the stroke too.
const EXAMPLES: usize = 12;

/// What the breakdown shows.
#[derive(Resource, Default)]
pub struct Breakdown {
    /// The characters broken down: the word, or a character dived into.
    pub text: String,
    /// The word practiced, to go back to.
    pub word: String,
    /// The character and the stroke described.
    pub selected: (usize, usize),
    pub basic: Option<Basic>,
    dirty: bool,
}

#[derive(Component, Default, Clone)]
struct BreakdownOverlay;
#[derive(Component, Default, Clone)]
struct BreakdownBody;

pub struct BreakdownPlugin;

impl Plugin for BreakdownPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Breakdown>().add_systems(Update, (open_by_env, show, fill).chain());
    }
}

/// The overlay, with the other overlays of the window.
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
        BreakdownOverlay
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
                @group_header() Children [ @label("Strokes") ]
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
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12),
                    }
                    BreakdownBody
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

/// The Strokes button: the word being practiced, the stroke being written selected.
pub fn open(_: On<Activate>, practice: Res<Practice>, data: Res<HanziData>, mut breakdown: ResMut<Breakdown>, mut panel: ResMut<Panel>, mut focus: ResMut<InputFocus>) {
    breakdown.start(&practice, &data);
    *panel = Panel::Strokes;
    focus.clear();
}

impl Breakdown {
    fn start(&mut self, practice: &Practice, data: &HanziData) {
        let word = practice.word(data).word.clone();
        *self = Breakdown { text: word.clone(), word, selected: (practice.char_index, practice.stroke), basic: None, dirty: true };
    }

    fn select(&mut self, selected: (usize, usize)) {
        self.selected = selected;
        self.basic = None;
        self.dirty = true;
    }
}

/// `CLAUDRALINGO_STROKES=n` opens the breakdown, its stroke n selected, for the screenshots.
fn open_by_env(
    mut frame: Local<u32>,
    practice: Res<Practice>,
    data: Res<HanziData>,
    mut breakdown: ResMut<Breakdown>,
    mut panel: ResMut<Panel>,
) {
    *frame += 1;
    if *frame != 50 {
        return;
    }
    if let Some(stroke) = std::env::var("CLAUDRALINGO_STROKES").ok().and_then(|n| n.parse::<usize>().ok()) {
        breakdown.start(&practice, &data);
        breakdown.selected = (0, stroke);
        if let Some(basic) = std::env::var("CLAUDRALINGO_STROKES_BASIC").ok().and_then(|b| b.parse::<usize>().ok()) {
            breakdown.basic = Basic::ALL.get(basic).copied();
        }
        *panel = Panel::Strokes;
    }
}

fn close(_: On<PointerClick>, mut panel: ResMut<Panel>) {
    *panel = Panel::None;
}

fn close_button(_: On<Activate>, mut panel: ResMut<Panel>, mut focus: ResMut<InputFocus>) {
    *panel = Panel::None;
    focus.clear();
}

fn show(panel: Res<Panel>, mut breakdown: ResMut<Breakdown>, mut overlays: Query<&mut Node, With<BreakdownOverlay>>) {
    if !panel.is_changed() {
        return;
    }
    for mut node in &mut overlays {
        node.display = if *panel == Panel::Strokes { Display::Flex } else { Display::None };
    }
    if *panel == Panel::Strokes {
        breakdown.dirty = true;
    }
}

fn image_of(pixmap: &tiny_skia::Pixmap) -> Image {
    Image::new(
        Extent3d { width: pixmap.width(), height: pixmap.height(), depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixmap.data().to_vec(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

pub(super) fn colors(theme: &UiTheme) -> render::Colors {
    render::Colors {
        paper: theme.color(&tokens::KEY_BG),
        grid: theme.color(&tokens::KEY_BORDER),
        outline: theme.color(&feathers_tokens::TEXT_DIM),
        next: theme.color(&tokens::CURSOR),
        ink: theme.color(&feathers_tokens::TEXT_MAIN),
        user: theme.color(&tokens::SPARK_OK),
        wrong: theme.color(&tokens::CHAR_WRONG),
        guide: theme.color(&tokens::CHAR_WRONG),
    }
}

/// The fonts of the texts: the feathers font for the latin letters, the CJK font for the hanzi.
pub(super) struct Fonts {
    pub latin: Handle<Font>,
    pub cjk: Handle<Font>,
}

fn is_cjk(c: char) -> bool {
    matches!(c, '\u{3000}'..='\u{9fff}' | '\u{ff00}'..='\u{ffef}')
}

impl Fonts {
    /// A text in spans, each in the font of its characters, under `parent`.
    pub fn spawn(&self, commands: &mut Commands, parent: Entity, text: &str, size: f32, color: bevy::feathers::theme::ThemeToken) -> Entity {
        let root = commands
            .spawn((
                Text::new(""),
                TextFont { font: FontSource::Handle(self.latin.clone()), font_size: FontSize::Px(size), ..default() },
                ThemeTextColor(color.clone()),
                Pickable::IGNORE,
                ChildOf(parent),
            ))
            .id();
        self.fill(commands, root, text, size, color);
        root
    }

    /// The spans of a text, under its root.
    pub fn fill(&self, commands: &mut Commands, root: Entity, text: &str, size: f32, color: bevy::feathers::theme::ThemeToken) {
        commands.entity(root).despawn_children();
        let mut runs: Vec<(String, bool)> = Vec::new();
        for c in text.chars() {
            // The spaces and the punctuation stay in the run they are in.
            let cjk = is_cjk(c);
            match runs.last_mut() {
                Some((run, run_cjk)) if *run_cjk == cjk || c == ' ' => run.push(c),
                _ => runs.push((c.to_string(), cjk)),
            }
        }
        for (run, cjk) in runs {
            let font = if cjk { self.cjk.clone() } else { self.latin.clone() };
            commands.spawn((
                TextSpan::new(run),
                TextFont { font: FontSource::Handle(font), font_size: FontSize::Px(size), ..default() },
                ThemeTextColor(color.clone()),
                ChildOf(root),
            ));
        }
    }
}

/// A clickable chip with a text.
fn chip(commands: &mut Commands, parent: Entity, theme: &UiTheme, border: Option<Color>) -> Entity {
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(8), px(4)),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(theme.color(&tokens::KEY_BG)),
            BorderColor::all(border.unwrap_or(Color::NONE)),
            ChildOf(parent),
        ))
        .id()
}

#[allow(clippy::too_many_arguments)]
fn fill(
    mut breakdown: ResMut<Breakdown>,
    panel: Res<Panel>,
    data: Res<HanziData>,
    font: Res<HanziFont>,
    theme: Res<UiTheme>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    bodies: Query<Entity, With<BreakdownBody>>,
    mut commands: Commands,
) {
    if !breakdown.dirty || *panel != Panel::Strokes {
        return;
    }
    breakdown.dirty = false;
    let fonts = Fonts { latin: asset_server.load(bevy::feathers::constants::fonts::REGULAR), cjk: font.0.clone() };
    let colors = colors(&theme);
    let chars: Vec<char> = breakdown.text.chars().filter(|c| data.glyph(*c).is_some()).collect();
    if chars.is_empty() {
        return;
    }
    let (sel_char, sel_stroke) = breakdown.selected;
    let sel_char = sel_char.min(chars.len() - 1);
    let Some(sel_glyph) = data.glyph(chars[sel_char]) else {
        return;
    };
    let sel_stroke = sel_stroke.min(sel_glyph.kinds.len().saturating_sub(1));
    let main = feathers_tokens::TEXT_MAIN;
    let dim = feathers_tokens::TEXT_DIM;
    for body in &bodies {
        commands.entity(body).despawn_children();
        let total: usize = chars.iter().filter_map(|c| data.glyph(*c)).map(|g| g.kinds.len()).sum();
        let pinyin = data.words.iter().find(|w| w.word == breakdown.text).map(|w| format!(" {}", w.pinyin)).unwrap_or_default();
        fonts.spawn(
            &mut commands,
            body,
            &format!("{}{pinyin} · {total} strokes. Click a stroke to see how the brush writes it.", breakdown.text),
            16.0,
            main.clone(),
        );
        if breakdown.text != breakdown.word {
            let back = chip(&mut commands, body, &theme, None);
            commands.entity(back).insert(Node {
                align_self: AlignSelf::Start,
                padding: UiRect::axes(px(8), px(4)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            });
            fonts.spawn(&mut commands, back, &format!("‹ Back to {}", breakdown.word), 14.0, main.clone());
            commands.entity(back).observe(|_: On<PointerClick>, mut breakdown: ResMut<Breakdown>| {
                breakdown.text = breakdown.word.clone();
                breakdown.select((0, 0));
            });
        }
        // Every character, stroke by stroke.
        for (ci, c) in chars.iter().enumerate() {
            let Some(glyph) = data.glyph(*c) else {
                continue;
            };
            let row = commands
                .spawn((
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(12),
                        ..default()
                    },
                    ChildOf(body),
                ))
                .id();
            fonts.spawn(&mut commands, row, &c.to_string(), 44.0, main.clone());
            let tiles = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: px(6),
                        row_gap: px(6),
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        ..default()
                    },
                    ChildOf(row),
                ))
                .id();
            for (si, kind) in glyph.kinds.iter().enumerate() {
                let selected = (ci, si) == (sel_char, sel_stroke);
                let tile = chip(&mut commands, tiles, &theme, selected.then(|| theme.color(&tokens::CURSOR)));
                commands.entity(tile).insert(Node {
                    width: px(92),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(px(4)),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                });
                let mut pixmap = tiny_skia::Pixmap::new(TILE, TILE).expect("a tile");
                render::draw_stroke(&mut pixmap, glyph, si, None, &colors);
                commands.spawn((
                    Node { width: px(TILE as f32 / 2.0), height: px(TILE as f32 / 2.0), ..default() },
                    ImageNode::new(images.add(image_of(&pixmap))),
                    Pickable::IGNORE,
                    ChildOf(tile),
                ));
                fonts.spawn(&mut commands, tile, &format!("{} {}", si + 1, kind.hanzi), 14.0, main.clone());
                fonts.spawn(&mut commands, tile, kind.pinyin, 11.0, dim.clone());
                commands.entity(tile).observe(move |_: On<PointerClick>, mut breakdown: ResMut<Breakdown>| {
                    breakdown.select((ci, si));
                });
            }
        }
        // The stroke selected: its image with the brush guide, its name, how the brush writes it.
        let kind = sel_glyph.kinds[sel_stroke];
        let detail = commands
            .spawn((
                Node { flex_direction: FlexDirection::Row, column_gap: px(16), margin: UiRect::top(px(6)), ..default() },
                ChildOf(body),
            ))
            .id();
        let profile = brush::profile(&sel_glyph.outlines[sel_stroke], &sel_glyph.medians[sel_stroke]);
        let guide = brush::guide(&profile, kind);
        let mut pixmap = tiny_skia::Pixmap::new(DETAIL, DETAIL).expect("the detail");
        render::draw_stroke(&mut pixmap, sel_glyph, sel_stroke, Some(&guide), &colors);
        commands.spawn((
            Node { width: px(DETAIL as f32 / 2.0), height: px(DETAIL as f32 / 2.0), flex_shrink: 0.0, ..default() },
            ImageNode::new(images.add(image_of(&pixmap))),
            ChildOf(detail),
        ));
        let column = commands
            .spawn((Node { flex_direction: FlexDirection::Column, row_gap: px(8), flex_shrink: 1.0, ..default() }, ChildOf(detail)))
            .id();
        fonts.spawn(&mut commands, column, kind.hanzi, 34.0, main.clone());
        fonts.spawn(
            &mut commands,
            column,
            &format!("{}, {} · stroke {} of {} of {}", kind.pinyin, kind.english, sel_stroke + 1, sel_glyph.kinds.len(), chars[sel_char]),
            15.0,
            dim.clone(),
        );
        fonts.spawn(&mut commands, column, &format!("Brush: {}", kind.brush), 15.0, main.clone());
        let mut marks = String::from("On the image: the path and its arrow");
        if !guide.presses.is_empty() {
            marks.push_str(", a ring where the brush presses, as wide as the ring");
        }
        if guide.hidden_entry {
            marks.push_str(", dashed: the tip goes in against the direction first (藏锋)");
        }
        match (guide.thin_start, guide.thin_end) {
            (true, true) => marks.push_str("; it starts and ends thin"),
            (true, false) => marks.push_str("; it starts thin, touch down lightly"),
            (false, true) => marks.push_str("; it ends in a point, lift while moving"),
            (false, false) => {}
        }
        marks.push('.');
        fonts.spawn(&mut commands, column, &marks, 13.0, dim.clone());
        // Its basic strokes.
        let parts_of = kind.parts();
        let formula = parts_of.iter().map(|b| b.hanzi()).collect::<Vec<_>>().join(" + ");
        fonts.spawn(&mut commands, column, &format!("Made of: {formula}. Click one to see it alone."), 15.0, main.clone());
        let parts = commands
            .spawn((Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(6), row_gap: px(6), ..default() }, ChildOf(column)))
            .id();
        let mut seen = Vec::new();
        for basic in parts_of {
            if seen.contains(&basic) {
                continue;
            }
            seen.push(basic);
            let selected = breakdown.basic == Some(basic);
            let part = chip(&mut commands, parts, &theme, selected.then(|| theme.color(&tokens::CURSOR)));
            fonts.spawn(&mut commands, part, basic.hanzi(), 22.0, main.clone());
            fonts.spawn(&mut commands, part, &format!("{} · {}", basic.pinyin(), basic.english()), 11.0, dim.clone());
            commands.entity(part).observe(move |_: On<PointerClick>, mut breakdown: ResMut<Breakdown>| {
                breakdown.basic = if breakdown.basic == Some(basic) { None } else { Some(basic) };
                breakdown.dirty = true;
            });
        }
        if let Some(basic) = breakdown.basic {
            fonts.spawn(
                &mut commands,
                column,
                &format!("{} {}, {}: {}", basic.hanzi(), basic.pinyin(), basic.english(), basic.shape()),
                15.0,
                main.clone(),
            );
            fonts.spawn(&mut commands, column, &format!("Brush: {}", basic.brush()), 15.0, main.clone());
        }
        // The characters of the words that have this stroke too, to break them down in turn.
        let mut examples: Vec<(char, usize)> = Vec::new();
        for w in &data.words {
            for c in w.word.chars() {
                if examples.len() >= EXAMPLES || c == chars[sel_char] || examples.iter().any(|(e, _)| *e == c) {
                    continue;
                }
                if let Some(si) = data.glyph(c).and_then(|g| g.kinds.iter().position(|k| k.code == kind.code)) {
                    examples.push((c, si));
                }
            }
        }
        if !examples.is_empty() {
            fonts.spawn(&mut commands, column, &format!("Also a {} in these characters, click one to break it down:", kind.hanzi), 15.0, main.clone());
            let row = commands
                .spawn((Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(6), row_gap: px(6), ..default() }, ChildOf(column)))
                .id();
            for (c, si) in examples {
                let example = chip(&mut commands, row, &theme, None);
                fonts.spawn(&mut commands, example, &c.to_string(), 24.0, main.clone());
                commands.entity(example).observe(move |_: On<PointerClick>, mut breakdown: ResMut<Breakdown>| {
                    breakdown.text = c.to_string();
                    breakdown.select((0, si));
                });
            }
        }
    }
}
