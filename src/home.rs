//! The main menu: a column in the middle of the window, a row per subject and two for the
//! settings and the stats, the rows share the height and a tap on a phone goes in. A subject
//! shows its craft (keys in the colors of their fingers, two hanzi, a word in Spencerian
//! script), its points (a lesson Good 1, Great 2, Legendary 3) and a bar in three parts, one per
//! grade, each filling like a vial with the lessons that reached it; the settings and the stats
//! their icon. It opens at the start and with the house of the toolbar, see
//! [`crate::ui::Panel::Home`]; the settings and the stats opened from it come back to it.
use bevy::asset::RenderAssetUsages;
use bevy::feathers::theme::{ThemeBackgroundColor, UiTheme};
use bevy::feathers::tokens as feathers_tokens;
use bevy::picking::events::{PointerClick, PointerOut, PointerOver};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use tiny_skia::{LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::game::{ActiveLayout, Grid, NewExercise};
use crate::lessons::{Grade, Points};
use crate::progress::{ExerciseMode, Progress, Subject};
use crate::theme::tokens;
use crate::ui::Panel;

#[derive(Component, Default, Clone)]
pub struct HomeOverlay;

/// The column of the rows.
#[derive(Component, Default, Clone)]
struct HomeColumn;

/// A row of the main menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    Subject(Subject),
    Settings,
    Stats,
}

/// A panel opened from the main menu: closed, the main menu comes back.
#[derive(Resource, Default)]
struct FromHome(Option<Panel>);

/// The word of the penmanship card, in the small letters of the course.
const SCRIPT_WORD: &str = "write";

/// The keys of the touch typing card: the left hand on the home row, a finger each.
const HOME_KEYS: [char; 4] = ['a', 's', 'd', 'f'];

/// The image of the word in script, drawn once.
#[derive(Resource, Default)]
struct ScriptImage(Option<Handle<Image>>);

pub struct HomePlugin;

impl Plugin for HomePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScriptImage>()
            .init_resource::<FromHome>()
            .add_systems(Startup, open_at_start)
            .add_systems(Update, (back_home, show_home, fill_home).chain().after(crate::game::GameSystems));
    }
}

/// The overlay of the main menu, filled when it opens.
pub fn overlay() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            display: Display::None,
            justify_content: JustifyContent::Center,
            padding: px(16),
            overflow: Overflow::clip(),
        }
        ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
        GlobalZIndex(45)
        HomeOverlay
        Children [
            Node {
                width: percent(100),
                max_width: px(760),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
            }
            HomeColumn
        ]
    }
}

/// The settings or the stats opened from the main menu, once closed: the main menu again.
fn back_home(mut panel: ResMut<Panel>, mut from: ResMut<FromHome>) {
    if !panel.is_changed() {
        return;
    }
    match from.0 {
        Some(_) if *panel == Panel::None => {
            from.0 = None;
            *panel = Panel::Home;
        }
        // Another panel took its place, F1 for one.
        Some(opened) if *panel != opened => from.0 = None,
        _ => {}
    }
}

fn open_at_start(mut panel: ResMut<Panel>) {
    *panel = Panel::Home;
}

fn show_home(panel: Res<Panel>, mut overlays: Query<&mut Node, With<HomeOverlay>>) {
    if !panel.is_changed() {
        return;
    }
    for mut node in &mut overlays {
        let display = if *panel == Panel::Home { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
}

/// The subject a card leads to: the last exercise of the touch typing, the hanzi, the penmanship.
fn enter(progress: &mut Progress, subject: Subject) {
    progress.words_of = None;
    progress.mode = match subject {
        Subject::Typing => progress.typing_mode,
        Subject::Hanzi => ExerciseMode::Hanzi,
        Subject::Penmanship => ExerciseMode::Penmanship,
    };
}

#[allow(clippy::too_many_arguments)]
fn fill_home(
    panel: Res<Panel>,
    progress: Res<Progress>,
    grid: Res<Grid>,
    layout: Res<ActiveLayout>,
    hanzi: Res<crate::hanzi::data::HanziData>,
    hanzi_font: Res<crate::hanzi::HanziFont>,
    theme: Res<UiTheme>,
    asset_server: Res<AssetServer>,
    mut script: ResMut<ScriptImage>,
    mut images: ResMut<Assets<Image>>,
    overlays: Query<Entity, With<HomeColumn>>,
    mut commands: Commands,
) {
    if *panel != Panel::Home || !(panel.is_changed() || theme.is_changed()) {
        return;
    }
    let script = script.0.get_or_insert_with(|| images.add(script_image(SCRIPT_WORD))).clone();
    let font = |bold: bool, size: f32| TextFont {
        font: bevy::text::FontSource::Handle(asset_server.load(if bold {
            bevy::feathers::constants::fonts::BOLD
        } else {
            bevy::feathers::constants::fonts::REGULAR
        })),
        font_size: FontSize::Px(size),
        ..default()
    };
    let entries = Subject::ALL.map(Entry::Subject).into_iter().chain([Entry::Settings, Entry::Stats]);
    for root in &overlays {
        commands.entity(root).despawn_children();
        for entry in entries.clone() {
            let last = entry == Entry::Subject(progress.mode.subject());
            let rest = theme.color(&if last { tokens::CURSOR } else { tokens::KEY_BORDER });
            // The subjects take more of the height than the settings and the stats.
            let grow = if matches!(entry, Entry::Subject(_)) { 1.0 } else { 0.6 };
            let row = commands
                .spawn((
                    Node {
                        flex_grow: grow,
                        flex_basis: px(0),
                        min_height: px(64),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(24),
                        padding: UiRect::axes(px(24), px(8)),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(14)),
                        ..default()
                    },
                    BackgroundColor(theme.color(&tokens::KEY_BG)),
                    BorderColor::all(rest),
                    ChildOf(root),
                ))
                .id();
            let hover = theme.color(&tokens::CURSOR);
            commands
                .entity(row)
                .observe(move |_: On<PointerOver>, mut borders: Query<&mut BorderColor>| {
                    if let Ok(mut border) = borders.get_mut(row) {
                        *border = BorderColor::all(hover);
                    }
                })
                .observe(move |_: On<PointerOut>, mut borders: Query<&mut BorderColor>| {
                    if let Ok(mut border) = borders.get_mut(row) {
                        *border = BorderColor::all(rest);
                    }
                })
                .observe(
                    move |_: On<PointerClick>,
                          mut progress: ResMut<Progress>,
                          mut panel: ResMut<Panel>,
                          mut from: ResMut<FromHome>,
                          mut tab: ResMut<crate::ui::SettingsTab>,
                          mut new_exercise: MessageWriter<NewExercise>| {
                        match entry {
                            Entry::Subject(subject) => {
                                enter(&mut progress, subject);
                                progress.save();
                                new_exercise.write_default();
                                *panel = Panel::None;
                            }
                            Entry::Settings => {
                                *tab = crate::ui::SettingsTab::General;
                                *panel = Panel::Settings;
                                from.0 = Some(Panel::Settings);
                            }
                            Entry::Stats => {
                                *panel = Panel::Stats;
                                from.0 = Some(Panel::Stats);
                            }
                        }
                    },
                );
            // The craft or the icon, in a box of its own at the left.
            let emblem = commands
                .spawn((
                    Node { width: px(EMBLEM_WIDTH), flex_shrink: 0.0, justify_content: JustifyContent::Center, align_items: AlignItems::Center, column_gap: px(5), ..default() },
                    Pickable::IGNORE,
                    ChildOf(row),
                ))
                .id();
            match entry {
                Entry::Subject(Subject::Typing) => {
                    for c in HOME_KEYS {
                        crate::ui::spawn_keycap(&mut commands, emblem, c, &layout.0, &theme, &asset_server, 1.7);
                    }
                }
                Entry::Subject(Subject::Hanzi) => {
                    commands.spawn((
                        Text::new("汉字"),
                        TextFont { font: bevy::text::FontSource::Handle(hanzi_font.0.clone()), font_size: FontSize::Px(56.0), ..default() },
                        TextColor(theme.color(&feathers_tokens::TEXT_MAIN)),
                        Pickable::IGNORE,
                        ChildOf(emblem),
                    ));
                }
                Entry::Subject(Subject::Penmanship) => {
                    commands.spawn((
                        Node { width: px(EMBLEM_WIDTH), height: px(EMBLEM_WIDTH * SCRIPT_SIZE.1 as f32 / SCRIPT_SIZE.0 as f32), ..default() },
                        ImageNode { image: script.clone(), color: theme.color(&feathers_tokens::TEXT_MAIN), ..default() },
                        Pickable::IGNORE,
                        ChildOf(emblem),
                    ));
                }
                Entry::Settings | Entry::Stats => {
                    let icon = if entry == Entry::Settings { crate::icons::Icon::Settings } else { crate::icons::Icon::Stats };
                    commands.spawn((
                        Node { width: px(40), height: px(40), ..default() },
                        ImageNode::default(),
                        crate::icons::IconNode(icon),
                        crate::icons::IconTint(feathers_tokens::TEXT_MAIN),
                        Pickable::IGNORE,
                        ChildOf(emblem),
                    ));
                }
            }
            let text = commands
                .spawn((Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, row_gap: px(8), ..default() }, Pickable::IGNORE, ChildOf(row)))
                .id();
            let name = match entry {
                Entry::Subject(subject) => subject.name(),
                Entry::Settings => "Settings",
                Entry::Stats => "Stats",
            };
            commands.spawn((Text::new(name), font(true, 26.0), TextColor(theme.color(&feathers_tokens::TEXT_MAIN)), Pickable::IGNORE, ChildOf(text)));
            if let Entry::Subject(subject) = entry {
                let sections = crate::lessons::subject_sections(&progress, subject, &grid.0, &layout.0, &hanzi);
                let points = crate::lessons::points(&sections);
                commands.spawn((
                    Text::new(format!("{}/{} points earned.", points.earned, points.total)),
                    font(false, 15.0),
                    TextColor(theme.color(&feathers_tokens::TEXT_DIM)),
                    Pickable::IGNORE,
                    ChildOf(text),
                ));
                spawn_vials(&mut commands, text, &points, &theme);
            }
        }
    }
}

/// The width of the box of the craft or the icon, at the left of a row.
const EMBLEM_WIDTH: f32 = 220.0;

/// The bar of the points: a part per grade, a faint vial each, filled with the share of the
/// lessons that reached the grade.
fn spawn_vials(commands: &mut Commands, parent: Entity, points: &Points, theme: &UiTheme) {
    let bar = commands.spawn((Node { flex_direction: FlexDirection::Row, column_gap: px(4), ..default() }, Pickable::IGNORE, ChildOf(parent))).id();
    for (tier, grade) in [Grade::Good, Grade::Great, Grade::Legendary].into_iter().enumerate() {
        let color = theme.color(&grade.token());
        let share = if points.lessons == 0 { 0.0 } else { points.tiers[tier] as f32 / points.lessons as f32 };
        let vial = commands
            .spawn((
                Node { flex_grow: 1.0, flex_basis: px(0), height: px(12), border_radius: BorderRadius::all(px(6)), overflow: Overflow::clip(), ..default() },
                BackgroundColor(color.with_alpha(0.065)),
                Pickable::IGNORE,
                ChildOf(bar),
            ))
            .id();
        commands.spawn((
            Node { width: percent(100.0 * share), height: percent(100), border_radius: BorderRadius::all(px(6)), ..default() },
            BackgroundColor(color),
            Pickable::IGNORE,
            ChildOf(vial),
        ));
    }
}

/// The pixels of the script image, twice its size on the screen.
const SCRIPT_SIZE: (u32, u32) = (600, 240);

/// A word in the Spencerian letters of the course, white on clear: the image node tints it.
fn script_image(word: &str) -> Image {
    use crate::penmanship::letters::{Style, letter};
    let style = Style::Spencerian;
    let figures: Vec<Vec<crate::penmanship::letters::Stroke>> = word.chars().map(|c| letter(c).on_sheet(style)).collect();
    // The width of the word in x-heights, the letters a little apart.
    let bounds = |strokes: &[crate::penmanship::letters::Stroke]| {
        strokes.iter().flat_map(|s| &s.points).fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.x), hi.max(p.x)))
    };
    // The letters join, as written.
    let gap = 0.0;
    let width: f32 = figures.iter().map(|f| bounds(f).1 - bounds(f).0 + gap).sum::<f32>() - gap;
    let (w, h) = (SCRIPT_SIZE.0 as f32, SCRIPT_SIZE.1 as f32);
    // The height of the word itself, from its lowest point to its highest.
    let (low, high) = figures.iter().flatten().flat_map(|s| &s.points).fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.y), hi.max(p.y)));
    let unit = ((w - 24.0) / width).min((h - 24.0) / (high - low));
    let base = (h + (high + low) * unit) / 2.0;
    let mut pixmap = Pixmap::new(SCRIPT_SIZE.0, SCRIPT_SIZE.1).expect("the script image");
    let mut paint = Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);
    paint.anti_alias = true;
    let stroke = Stroke { width: 0.2 * unit, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() };
    let mut x = (w - width * unit) / 2.0;
    for strokes in &figures {
        let (min_x, max_x) = bounds(strokes);
        for s in strokes {
            let point = |p: Vec2| (x + (p.x - min_x) * unit, base - p.y * unit);
            if s.dot {
                if let Some(p) = s.points.first()
                    && let Some(path) = PathBuilder::from_circle(point(*p).0, point(*p).1, 0.16 * unit)
                {
                    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
                }
                continue;
            }
            let mut pb = PathBuilder::new();
            for (i, p) in s.points.iter().enumerate() {
                let (px, py) = point(*p);
                if i == 0 { pb.move_to(px, py) } else { pb.line_to(px, py) }
            }
            if let Some(path) = pb.finish() {
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
        x += (max_x - min_x + gap) * unit;
    }
    Image::new(
        Extent3d { width: SCRIPT_SIZE.0, height: SCRIPT_SIZE.1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixmap.data().to_vec(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_word_has_ink_inside_the_image() {
        let image = script_image(SCRIPT_WORD);
        let data = image.data.expect("pixels");
        let inked = data.chunks(4).filter(|p| p[3] > 0).count();
        assert!(inked > 1000, "{inked}");
        // The corners stay clear.
        assert_eq!(data[3], 0);
    }

    #[test]
    fn a_card_leads_to_its_subject() {
        let mut progress = Progress { mode: ExerciseMode::Hanzi, typing_mode: ExerciseMode::Velocity, words_of: Some(3), ..Progress::default() };
        enter(&mut progress, Subject::Typing);
        assert_eq!((progress.mode, progress.words_of), (ExerciseMode::Velocity, None));
        enter(&mut progress, Subject::Penmanship);
        assert_eq!(progress.mode, ExerciseMode::Penmanship);
    }
}
