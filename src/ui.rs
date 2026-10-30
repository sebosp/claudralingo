//! The window: the feathers toolbar, the live statistics, the exercise, the keyboard and the
//! rankings of the keys.
use bevy::feathers::containers::{flex_spacer, group, group_body, group_header};
use bevy::feathers::controls::*;
use bevy::feathers::display::{caption, label, label_dim};
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeTextColor};
use bevy::feathers::tokens as feathers_tokens;
use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::ui::{Checked, Selected};
use bevy::input::mouse::MouseScrollPixelsPerLine;
use bevy::picking::events::{PointerClick, PointerScroll};
use bevy::ui_widgets::{Activate, ValueChange};

use crate::drills::FINGERS;
use crate::game::{ActiveLayout, Exercise, NewExercise, go_to_suggestion};
use crate::klavaro::N_BASIC_LESSONS;
use crate::klavaro::tutor::Query as TutorQuery;
use crate::layout::{LayoutId, finger_name};
use crate::progress::{BackgroundLevel, ExerciseMode, Progress};
use crate::shortcuts::Drill;
use crate::theme::{ThemeMode, tokens};
use crate::zoom::Zoom;

#[derive(Component, Default, Clone)]
pub struct PromptRoot;

/// The text to type, the shortcuts or the results, the background blurs under it.
#[derive(Component, Default, Clone)]
pub struct ExerciseArea;

/// The animated background, see [`crate::background`].
#[derive(Component, Default, Clone)]
pub struct BackgroundRoot;

#[derive(Component, Default, Clone)]
pub struct ShortcutsRoot;

#[derive(Component, Default, Clone)]
pub struct KeyboardRoot;

/// The hands under the keyboard, see [`crate::hands`].
#[derive(Component, Default, Clone)]
pub struct HandsRoot;

/// The instructions of the layer trainer, above the keyboard.
#[derive(Component, Default, Clone)]
pub struct TrainerHint;

#[derive(Component, Default, Clone)]
struct LessonLabel;

#[derive(Component, Default, Clone)]
struct LiveStats;

#[derive(Component, Default, Clone)]
struct ProgressBar;

#[derive(Component, Default, Clone)]
pub struct MessageText;

#[derive(Component, Default, Clone)]
pub struct RankingText;

/// The accuracy at the end, green when the goal is reached, red otherwise.
#[derive(Component, Default, Clone)]
struct AccuracyText;

#[derive(Component, Default, Clone)]
struct ModeSelect;

#[derive(Component, Default, Clone)]
struct LayoutSelect;

/// The toggles and checkboxes of the settings, they start unchecked.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Setting {
    #[default]
    FadeLegends,
    LayerTrainer,
    ShowHands,
    ReduceMotion,
    LargeText,
    FitWindow,
    PinyinKeypad,
    Sound,
    LockLessons,
    Compact,
    FixWithBackspace,
}

#[derive(Component, Default, Clone)]
struct SettingControl(Setting);

#[derive(Component, Default, Clone)]
struct SuggestButton;

#[derive(Component, Default, Clone)]
struct ZoomLabel;

/// Under the live statistics at the end: the trends against the last time of the lesson.
#[derive(Component, Default, Clone)]
struct TrendRow;
#[derive(Component, Default, Clone)]
struct TrendText(usize);

/// The keys of the moment, at the bottom of the window.
#[derive(Component, Default, Clone)]
struct ActionsText;

/// The result of the last keymap import.
#[derive(Component, Default, Clone)]
struct KeymapStatus;

/// A paste of a keymap waiting for the clipboard.
#[derive(Resource, Default)]
struct PendingKeymap(Option<bevy::clipboard::ClipboardRead>);

/// Pushes the hands to the bottom in the compact view.
#[derive(Component, Default, Clone)]
struct CompactSpacer;

/// The views of the keyboard modes, hidden in the hanzi mode.
#[derive(Component, Default, Clone)]
pub struct TypingOnly;

/// The column of the window that scrolls when zoomed in.
#[derive(Component, Default, Clone)]
struct WindowScroll;

#[derive(Component, Default, Clone)]
struct HelpOverlay;

#[derive(Component, Default, Clone)]
struct SettingsOverlay;

#[derive(Component, Default, Clone)]
struct LessonsOverlay;

#[derive(Component, Default, Clone)]
struct LessonsHint;

/// The tiles of the lessons, filled when the panel opens.
#[derive(Component, Default, Clone)]
struct LessonsGrid;

/// The panel over the window, it takes the keys.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    #[default]
    None,
    Help,
    Settings,
    Lessons,
    Stats,
    /// At the start: continue where the last session stopped, or warm up first.
    Welcome,
}

impl Panel {
    pub fn toggle(&mut self, panel: Panel) {
        *self = if *self == panel { Panel::None } else { panel };
    }
}

const HELP: &str = "\
MODES, the first menu
  Basic course, Adaptability, Velocity, Fluidness: the exercises of Klavaro, real words at the end
  Layer course: the keys of Lower and Raise row by row    Warm-up: your weak keys, after a pause
  Finger drill, Left and Right hand finger bounce: one finger, or middle and index in turns
  Shortcuts & keys: Esc, arrows, Ctrl and Alt shortcuts, Alt+Tab
  Hanzi (HSK 1-4): write each word with a pen, mouse or finger, then type its pinyin

TOOLBAR
  ‹ ›  the lesson, the finger, the hand or the hanzi level    Restart  a new text
  Lessons  your lessons and levels    Stats  calendar, words per minute, accuracy, your keys
  Settings  the keyboard (paste or drop a ZMK keymap), dark mode, locked lessons, zoom, T9, sound

TYPING
  Backspace erases the errors, Ctrl+Backspace all of them
  A line without errors clears in green, with one slip in orange
  Zoom: Ctrl and the mouse wheel, a pinch, Ctrl+= Ctrl+- Ctrl+0
  At the end: Enter for the next exercise, Backspace the previous lesson, Tab the suggested drill

COLORS
  green the next key, purple shift or hold the key longer (auto shift), amber hold, red backspace
  The stripes of the keys and the hands show the finger of every key, a miss streams out of it

ON-SCREEN KEYBOARD
  Click a key to type it, hold it past the bar under it for the capital (auto shift)
  Lower and Raise: click to show and use their layer, click again to go back, or hold them
  Ctrl, Alt, Win: click them, then the key: they are never sent to your system

F1 or ? opens this help, Esc or a click closes it. The progress is saved in ~/.config/claudralingo";

#[derive(Component, Default, Clone)]
struct SuggestCaption;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Panel>()
            .init_resource::<PendingKeymap>().add_systems(Startup, (spawn_ui, first_help)).add_systems(
            Update,
            (
                init_settings,
                sync_mode_select,
                show_mode_views,
                update_lesson_label,
                update_live_stats,
                update_message,
                update_suggestion,
                show_panels.run_if(resource_changed::<Panel>),
                fill_lessons.run_if(resource_changed::<Panel>),
                show_hands,
                show_hanzi,
                fill_welcome,
                update_trends,
                update_actions,
                arrow_lessons.after(crate::game::GameSystems),
                import_keymap,
                update_rankings.run_if(|p: Res<Progress>, panel: Res<Panel>| p.is_changed() || panel.is_changed()),
                update_zoom_label.run_if(resource_changed::<UiScale>),
            )
                .after(crate::game::GameSystems),
        );
    }
}

fn spawn_ui(mut commands: Commands, progress: Res<Progress>) {
    commands.spawn(Camera2d);
    let mode = ExerciseMode::ALL.iter().position(|m| *m == progress.mode);
    let layout = LayoutId::ALL.iter().position(|l| *l == progress.layout);
    let modes = list_rows_from_strings(ExerciseMode::ALL.map(ExerciseMode::name), mode);
    let layouts = list_rows_from_strings(LayoutId::ALL.map(LayoutId::name), layout);
    let background = BackgroundLevel::ALL.iter().position(|b| *b == progress.background);
    let backgrounds = list_rows_from_strings(BackgroundLevel::ALL.map(BackgroundLevel::name), background);
    let theme = ThemeMode::ALL.iter().position(|t| *t == progress.theme);
    let themes = list_rows_from_strings(ThemeMode::ALL.map(ThemeMode::name), theme);
    commands.spawn_scene(root(modes, layouts, backgrounds, themes));
}

fn root(
    modes: Box<dyn SceneList>,
    layouts: Box<dyn SceneList>,
    backgrounds: Box<dyn SceneList>,
    themes: Box<dyn SceneList>,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(10),
        }
        TabGroup
        ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
        Children [
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
            }
            BackgroundRoot
            --
            // Everything but the overlays scrolls, zoomed in.
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(10),
                overflow: Overflow::scroll_y(),
            }
            ScrollPosition::default()
            WindowScroll
            on(scroll_window)
            Children [
                @toolbar(modes)
                --
                @stats_bar()
                --
                // The text, the shortcuts or the results, in the same place.
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: px(10),
                    min_height: px(4.0 * crate::prompt::LINE_HEIGHT),
                    margin: UiRect::top(px(6)),
                }
                ExerciseArea
                TypingOnly
                Children [
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Start,
                    }
                    PromptRoot
                    --
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(6),
                    }
                    ShortcutsRoot
                    --
                    @label("") AccuracyText
                    Node {
                        display: Display::None,
                    }
                    --
                    @label("") MessageText
                    Node {
                        display: Display::None,
                    }
                    TextLayout {
                        justify: Justify::Center,
                    }
                    --
                    @FeathersButton {
                        @caption: bsn! { @caption("") SuggestCaption }
                    }
                    SuggestButton
                    Node {
                        display: Display::None,
                    }
                    on(train_suggested_finger)
                ]
                --
                @crate::hanzi::view()
                --
                @label("") TrainerHint
                TypingOnly
                Node {
                    min_height: px(20),
                }
                --
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Start,
                    column_gap: px(48),
                    padding: px(8),
                }
                KeyboardRoot
                TypingOnly
                --
                Node {
                    display: Display::None,
                    // Takes the space before the spacer at the bottom.
                    flex_grow: 100.0,
                }
                CompactSpacer
                --
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::End,
                }
                HandsRoot
                --
                @flex_spacer()
            ]
            --
            Node {
                padding: UiRect::axes(px(14), px(5)),
                margin: UiRect::bottom(px(6)),
                border_radius: BorderRadius::all(px(8)),
            }
            ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
            Children [
                @label_dim("") ActionsText
            ]
            --
            @help_overlay()
            --
            @settings_overlay(layouts, backgrounds, themes)
            --
            @lessons_overlay()
            --
            @welcome_overlay()
            --
            @crate::stats::overlay()
        ]
    }
}

fn toolbar(modes: Box<dyn SceneList>) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            padding: px(8),
        }
        ThemeBackgroundColor(feathers_tokens::PANE_HEADER_BG)
        Children [
            @label("Claudralingo")
            --
            @FeathersSelect {
                @options: modes,
            }
            ModeSelect
            Node {
                min_width: px(160),
            }
            on(on_mode_change)
            --
            @FeathersButton {
                @caption: bsn! { @caption("‹") }
            }
            on(previous_lesson)
            --
            @label_dim("") LessonLabel
            Node {
                min_width: px(200),
            }
            --
            @FeathersButton {
                @caption: bsn! { @caption("›") }
            }
            on(next_lesson)
            --
            @FeathersButton {
                @caption: bsn! { @caption("Restart") }
                @variant: ButtonVariant::Primary,
            }
            on(restart)
            --
            @flex_spacer()
            --
            @FeathersButton {
                @caption: bsn! { @caption("Lessons") }
            }
            on(toggle_lessons)
            --
            @FeathersButton {
                @caption: bsn! { @caption("Stats") }
            }
            on(toggle_stats)
            --
            @FeathersButton {
                @caption: bsn! { @caption("Settings") }
            }
            on(toggle_settings)
            --
            @FeathersButton {
                @caption: bsn! { @caption("?") }
            }
            on(toggle_help)
        ]
    }
}

/// A checkbox of the settings, with what it does.
fn setting_checkbox(setting: Setting, text: &'static str) -> impl Scene {
    bsn! {
        @FeathersCheckbox {
            @caption: bsn! { @caption(text) }
        }
        SettingControl({setting})
        on(on_setting_change)
    }
}

fn settings_overlay(layouts: Box<dyn SceneList>, backgrounds: Box<dyn SceneList>, themes: Box<dyn SceneList>) -> impl Scene {
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
        SettingsOverlay
        Children [
            // A click beside the panel closes it, the clicks in the panel do not reach it.
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
            }
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6))
            on(close_panel)
            --
            @group()
            Node {
                width: px(520),
            }
            Children [
                @group_header() Children [ @label("Settings") ]
                --
                @group_body()
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    padding: px(12),
                }
                Children [
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(12),
                    }
                    Children [
                        @label("Keyboard") Node { min_width: px(110) }
                        --
                        @FeathersSelect {
                            @options: layouts,
                        }
                        Node {
                            min_width: px(180),
                        }
                        LayoutSelect
                        on(on_layout_change)
                    ]
                    --
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(12),
                    }
                    Children [
                        @FeathersButton {
                            @caption: bsn! { @caption("Paste ZMK keymap") }
                        }
                        on(paste_keymap)
                        --
                        @label_dim("Copy your .keymap file, or drop it on the window") KeymapStatus
                    ]
                    --
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(12),
                    }
                    Children [
                        @label("Theme") Node { min_width: px(110) }
                        --
                        @FeathersSelect {
                            @options: themes,
                        }
                        Node {
                            min_width: px(180),
                        }
                        on(on_theme_change)
                    ]
                    --
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(12),
                    }
                    Children [
                        @label("Background") Node { min_width: px(110) }
                        --
                        @FeathersSelect {
                            @options: backgrounds,
                        }
                        Node {
                            min_width: px(180),
                        }
                        on(on_background_change)
                    ]
                    --
                    @setting_checkbox(Setting::FixWithBackspace, "Fix mistakes with Backspace: off, the cursor waits on the missed key")
                    --
                    @setting_checkbox(Setting::LockLessons, "Lock lessons: pass a lesson (95%) to open the next one")
                    --
                    @setting_checkbox(Setting::FadeLegends, "Fade legends: the letters fade while you type well")
                    --
                    @setting_checkbox(Setting::LayerTrainer, "Layer trainer: show the layer and modifier keys to hold")
                    --
                    @setting_checkbox(Setting::ShowHands, "Show the hands under the keyboard")
                    --
                    @setting_checkbox(Setting::Compact, "Compact, for phones: no keyboard, the finger tips and the layer to hold")
                    --
                    @setting_checkbox(Setting::ReduceMotion, "Reduce motion: no drift, sparks, shakes nor breathing keys")
                    --
                    @setting_checkbox(Setting::LargeText, "Large text to type")
                    --
                    @setting_checkbox(Setting::PinyinKeypad, "Hanzi: type the pinyin on the nine keys of a phone (T9)")
                    --
                    @setting_checkbox(Setting::Sound, "Hanzi: speak the words")
                    --
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(8),
                    }
                    Children [
                        @label("Zoom") Node { min_width: px(110) }
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("−") }
                        }
                        on(zoom_out)
                        --
                        @label("100%") ZoomLabel Node { min_width: px(48), justify_content: JustifyContent::Center }
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("+") }
                        }
                        on(zoom_in)
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("Reset") }
                        }
                        on(zoom_reset)
                    ]
                    --
                    @setting_checkbox(Setting::FitWindow, "Fit the window: a bigger window shows everything bigger")
                    --
                    @label_dim("Or Ctrl and the mouse wheel, a pinch of the trackpad, Ctrl+= Ctrl+- Ctrl+0, the wheel scrolls")
                    --
                    @FeathersButton {
                        @caption: bsn! { @caption("Close") }
                        @variant: ButtonVariant::Primary,
                    }
                    on(close_settings)
                ]
            ]
        ]
    }
}

/// The lessons of the basic course with their accuracy, a click practices one again.
fn lessons_overlay() -> impl Scene {
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
        LessonsOverlay
        Children [
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
            }
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6))
            on(close_panel)
            --
            @group()
            Node {
                width: px(820),
            }
            Children [
                @group_header() Children [ @label("Lessons") ]
                --
                @group_body()
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                }
                Children [
                    @label_dim("") LessonsHint
                    --
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: px(6),
                        row_gap: px(6),
                        max_height: vh(62),
                        overflow: Overflow::scroll_y(),
                    }
                    bevy::ui_widgets::ScrollArea
                    LessonsGrid
                    --
                    @FeathersButton {
                        @caption: bsn! { @caption("Close") }
                        @variant: ButtonVariant::Primary,
                    }
                    on(close_settings)
                ]
            ]
        ]
    }
}

/// The characters on a tile of the lessons.
const MAX_TILE_CHARS: usize = 7;

/// The best and the last accuracy of a lesson of the basic course, and the times typed.
fn lesson_stats(progress: &Progress, lesson: usize) -> Option<(f64, f64, usize)> {
    let layout = progress.layouts.get(progress.layout.key())?;
    let accuracies: Vec<f64> = layout
        .sessions
        .iter()
        .filter(|s| s.mode == ExerciseMode::Basic && s.lesson == Some(lesson))
        .map(|s| s.accuracy)
        .collect();
    let last = *accuracies.last()?;
    Some((accuracies.iter().copied().fold(0.0, f64::max), last, accuracies.len()))
}

#[allow(clippy::too_many_arguments)]
fn fill_lessons(
    panel: Res<Panel>,
    progress: Res<Progress>,
    grid: Res<crate::game::Grid>,
    theme: Res<bevy::feathers::theme::UiTheme>,
    asset_server: Res<AssetServer>,
    grids: Query<Entity, With<LessonsGrid>>,
    hanzi: Res<crate::hanzi::data::HanziData>,
    hanzi_font: Res<crate::hanzi::HanziFont>,
    mut hints: Query<&mut Text, With<LessonsHint>>,
    mut commands: Commands,
) {
    if *panel != Panel::Lessons {
        return;
    }
    let hint = if progress.mode == ExerciseMode::Hanzi {
        "The levels of the hanzi mode, ten words each, green when done. Click a level to practice it."
    } else {
        "The best and the last accuracy, the times typed: green when the best reached 95%, red below. Click a lesson to practice it again."
    };
    for mut text in &mut hints {
        text.0 = hint.into();
    }
    if progress.mode == ExerciseMode::Hanzi {
        fill_hanzi_levels(&progress, &hanzi, &hanzi_font, &theme, &asset_server, &grids, &mut commands);
        return;
    }
    let font = |size: f32| TextFont {
        font: bevy::text::FontSource::Handle(asset_server.load(bevy::feathers::constants::fonts::REGULAR)),
        font_size: FontSize::Px(size),
        ..default()
    };
    let current = progress.layouts.get(progress.layout.key()).map(|l| l.basic_lesson).unwrap_or(1);
    let unlocked = progress.layouts.get(progress.layout.key()).map(|l| l.basic_unlocked).unwrap_or(1);
    let goal = crate::klavaro::basic::GOAL_ACCURACY;
    for root in &grids {
        commands.entity(root).despawn_children();
        for lesson in 1..=N_BASIC_LESSONS {
            // The characters of the lesson, the long sets shortened.
            let chars = crate::klavaro::basic::lesson_char_set(lesson, &grid.0);
            let mut shown: String = chars.iter().take(MAX_TILE_CHARS).collect();
            if chars.len() > MAX_TILE_CHARS {
                shown.push('…');
            }
            let stats = lesson_stats(&progress, lesson);
            // The status in words and in color: green reached, red below the goal.
            let (status, detail) = match stats {
                Some((best, last, n)) => (
                    Some(if best >= goal { tokens::CHAR_CORRECT } else { tokens::CHAR_WRONG }),
                    format!("best {best:.0}%\nlast {last:.0}% · {n}×"),
                ),
                None => (None, "not yet".to_string()),
            };
            let border = if lesson == current { theme.color(&tokens::CURSOR) } else { Color::NONE };
            let locked = progress.lock_lessons && lesson > unlocked;
            let detail = if locked { "locked".to_string() } else { detail };
            let tile = commands
                .spawn((
                    Node {
                        width: px(92),
                        height: px(68),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::axes(px(6), px(4)),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(6)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(theme.color(&tokens::KEY_BG)),
                    BorderColor::all(border),
                    ChildOf(root),
                ))
                .observe(move |_: On<PointerClick>, mut progress: ResMut<Progress>, mut panel: ResMut<Panel>, mut new_exercise: MessageWriter<NewExercise>| {
                    if locked {
                        return;
                    }
                    progress.mode = ExerciseMode::Basic;
                    progress.current().basic_lesson = lesson;
                    progress.save();
                    new_exercise.write_default();
                    *panel = Panel::None;
                })
                .id();
            commands.spawn((
                Text::new(format!("{lesson}  {shown}")),
                font(15.0),
                TextLayout::no_wrap(),
                ThemeTextColor(feathers_tokens::TEXT_MAIN),
                Pickable::IGNORE,
                ChildOf(tile),
            ));
            commands.spawn((
                Text::new(detail),
                font(12.0),
                ThemeTextColor(status.unwrap_or(feathers_tokens::TEXT_DIM)),
                Pickable::IGNORE,
                ChildOf(tile),
            ));
        }
    }
}

/// The levels of the hanzi mode: their words, done or not, a click practices one.
fn fill_hanzi_levels(
    progress: &Progress,
    data: &crate::hanzi::data::HanziData,
    font: &crate::hanzi::HanziFont,
    theme: &bevy::feathers::theme::UiTheme,
    asset_server: &AssetServer,
    grids: &Query<Entity, With<LessonsGrid>>,
    commands: &mut Commands,
) {
    let latin = |size: f32| TextFont {
        font: bevy::text::FontSource::Handle(asset_server.load(bevy::feathers::constants::fonts::REGULAR)),
        font_size: FontSize::Px(size),
        ..default()
    };
    let cjk = TextFont {
        font: bevy::text::FontSource::Handle(font.0.clone()),
        font_size: FontSize::Px(15.0),
        ..default()
    };
    for root in grids {
        commands.entity(root).despawn_children();
        for (i, level) in data.levels.iter().enumerate() {
            let words: String = data.words[level.words.clone()].iter().take(4).map(|w| w.word.as_str()).collect::<Vec<_>>().join(" ");
            let (status, detail) = match progress.hanzi.levels_done.get(&i) {
                Some(misses) => (Some(tokens::CHAR_CORRECT), format!("done · {misses} misses")),
                None if i == progress.hanzi.level => (None, format!("word {}/{}", progress.hanzi.word + 1, level.words.len())),
                None => (None, "not yet".into()),
            };
            let border = if i == progress.hanzi.level { theme.color(&tokens::CURSOR) } else { Color::NONE };
            let locked = progress.lock_lessons && i > hanzi_unlocked(progress);
            let detail = if locked { "locked".to_string() } else { detail };
            let tile = commands
                .spawn((
                    Node {
                        width: px(92),
                        height: px(68),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::axes(px(6), px(4)),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(6)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(theme.color(&tokens::KEY_BG)),
                    BorderColor::all(border),
                    ChildOf(root),
                ))
                .observe(move |_: On<PointerClick>, mut progress: ResMut<Progress>, mut panel: ResMut<Panel>| {
                    if locked {
                        return;
                    }
                    progress.mode = ExerciseMode::Hanzi;
                    progress.hanzi.level = i;
                    progress.hanzi.word = 0;
                    progress.save();
                    *panel = Panel::None;
                })
                .id();
            commands.spawn((
                Text::new(format!("HSK{} · {}/{}", level.hsk, level.part, level.parts)),
                latin(13.0),
                TextLayout::no_wrap(),
                ThemeTextColor(feathers_tokens::TEXT_MAIN),
                Pickable::IGNORE,
                ChildOf(tile),
            ));
            commands.spawn((Text::new(words), cjk.clone(), TextLayout::no_wrap(), ThemeTextColor(feathers_tokens::TEXT_MAIN), Pickable::IGNORE, ChildOf(tile)));
            commands.spawn((
                Text::new(detail),
                latin(11.0),
                ThemeTextColor(status.unwrap_or(feathers_tokens::TEXT_DIM)),
                Pickable::IGNORE,
                ChildOf(tile),
            ));
        }
    }
}

fn help_overlay() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            display: Display::None,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6))
        GlobalZIndex(50)
        HelpOverlay
        on(close_help)
        Children [
            @group()
            Node {
                width: px(820),
            }
            Children [
                @group_header() Children [ @label("Claudralingo") ]
                --
                @group_body() Children [ @label(HELP) ]
            ]
        ]
    }
}

/// The live statistics and the progress of the exercise, on a panel of their own: the
/// background does not run under them.
fn stats_bar() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(6),
            width: px(640),
            padding: UiRect::axes(px(16), px(8)),
            border_radius: BorderRadius::all(px(10)),
            margin: UiRect::top(px(6)),
        }
        ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
        TypingOnly
        Children [
            @label("") LiveStats
            --
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Row,
                column_gap: px(18),
            }
            TrendRow
            Children [
                Text("") TrendText(0) --
                Text("") TrendText(1) --
                Text("") TrendText(2)
            ]
            --
            Node {
                width: percent(100),
                height: px(6),
                border_radius: BorderRadius::all(px(3)),
            }
            ThemeBackgroundColor(tokens::PROGRESS_TRACK)
            Children [
                Node {
                    width: percent(0),
                    height: percent(100),
                    border_radius: BorderRadius::all(px(3)),
                }
                ThemeBackgroundColor(tokens::PROGRESS_BAR)
                ProgressBar
            ]
        ]
    }
}


impl Setting {
    fn get(self, progress: &Progress) -> bool {
        match self {
            Setting::FadeLegends => progress.fade_legends,
            Setting::LayerTrainer => progress.layer_trainer,
            Setting::ShowHands => progress.show_hands,
            Setting::ReduceMotion => progress.reduce_motion,
            Setting::LargeText => progress.large_text,
            Setting::FitWindow => progress.fit_window,
            Setting::PinyinKeypad => progress.pinyin_keypad,
            Setting::Sound => progress.sound,
            Setting::LockLessons => progress.lock_lessons,
            Setting::Compact => progress.compact,
            Setting::FixWithBackspace => progress.fix_with_backspace,
        }
    }
}

/// The toggles and the checkboxes have no initial value prop.
fn init_settings(settings: Query<(Entity, &SettingControl), Added<SettingControl>>, progress: Res<Progress>, mut commands: Commands) {
    for (entity, SettingControl(setting)) in &settings {
        if setting.get(&progress) {
            commands.entity(entity).insert(Checked);
        }
    }
}

/// The typing goes to the exercise, not to the last clicked control: Space and Enter would
/// activate it again.
fn release_focus(focus: &mut InputFocus) {
    focus.clear();
}

fn on_setting_change(
    change: On<ValueChange<bool>>,
    settings: Query<&SettingControl>,
    mut progress: ResMut<Progress>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    let Ok(SettingControl(setting)) = settings.get(change.source) else {
        return;
    };
    if change.value {
        commands.entity(change.source).insert(Checked);
    } else {
        commands.entity(change.source).remove::<Checked>();
    }
    match setting {
        Setting::FadeLegends => progress.fade_legends = change.value,
        Setting::LayerTrainer => progress.layer_trainer = change.value,
        Setting::ShowHands => progress.show_hands = change.value,
        Setting::ReduceMotion => progress.reduce_motion = change.value,
        Setting::LargeText => progress.large_text = change.value,
        Setting::FitWindow => progress.fit_window = change.value,
        Setting::PinyinKeypad => progress.pinyin_keypad = change.value,
        Setting::Sound => progress.sound = change.value,
        Setting::LockLessons => progress.lock_lessons = change.value,
        Setting::Compact => progress.compact = change.value,
        Setting::FixWithBackspace => progress.fix_with_backspace = change.value,
    }
    progress.save();
    release_focus(&mut focus);
}

fn on_mode_change(
    change: On<ValueChange<Entity>>,
    options: Query<&OptionIndex>,
    mut progress: ResMut<Progress>,
    mut new_exercise: MessageWriter<NewExercise>,
    mut focus: ResMut<InputFocus>,
) {
    if let Ok(option) = options.get(change.value)
        && let Some(mode) = ExerciseMode::ALL.get(option.0)
    {
        progress.mode = *mode;
        progress.save();
        new_exercise.write_default();
    }
    release_focus(&mut focus);
}

/// The mode also changes from the keyboard and the buttons, the select follows.
fn sync_mode_select(
    progress: Res<Progress>,
    selects: Query<Entity, With<ModeSelect>>,
    layout_selects: Query<Entity, With<LayoutSelect>>,
    children: Query<&Children>,
    rows: Query<(&OptionIndex, Has<Selected>)>,
    mut commands: Commands,
) {
    if !progress.is_changed() {
        return;
    }
    let mode = ExerciseMode::ALL.iter().position(|m| *m == progress.mode);
    let layout = LayoutId::ALL.iter().position(|l| *l == progress.layout);
    let selects = selects.iter().map(|s| (s, mode)).chain(layout_selects.iter().map(|s| (s, layout)));
    for (select, index) in selects {
        for row in children.iter_descendants(select) {
            if let Ok((option, selected)) = rows.get(row) {
                let wanted = Some(option.0) == index;
                if wanted && !selected {
                    commands.entity(row).insert(Selected);
                } else if !wanted && selected {
                    commands.entity(row).remove::<Selected>();
                }
            }
        }
    }
}

fn paste_keymap(
    _: On<Activate>,
    mut clipboard: ResMut<bevy::clipboard::Clipboard>,
    mut pending: ResMut<PendingKeymap>,
    mut focus: ResMut<InputFocus>,
) {
    pending.0 = Some(clipboard.fetch_text());
    release_focus(&mut focus);
}

/// A keymap of ZMK pasted or dropped on the window becomes the layout "My ZMK keymap".
fn import_keymap(
    mut pending: ResMut<PendingKeymap>,
    mut drops: MessageReader<FileDragAndDrop>,
    mut progress: ResMut<Progress>,
    mut layout: ResMut<ActiveLayout>,
    mut status: Query<&mut Text, With<KeymapStatus>>,
    mut new_exercise: MessageWriter<NewExercise>,
) {
    let mut text: Option<Result<String, String>> = None;
    if let Some(read) = pending.0.as_mut()
        && let Some(result) = read.poll_result()
    {
        pending.0 = None;
        text = Some(result.map_err(|err| format!("could not read the clipboard: {err:?}")));
    }
    for drop in drops.read() {
        if let FileDragAndDrop::DroppedFile { path_buf, .. } = drop {
            text = Some(std::fs::read_to_string(path_buf).map_err(|err| format!("{}: {err}", path_buf.display())));
        }
    }
    let Some(text) = text else {
        return;
    };
    let message = match text.and_then(|t| crate::layout::custom_from_zmk(&t).map(|l| (t, l))) {
        Ok((keymap, custom)) => {
            let message = format!(
                "Imported: {} keys, layers {}",
                custom.keys.len(),
                custom.layer_names.join(", ")
            );
            crate::layout::set_custom_keymap(Some(keymap.clone()));
            progress.custom_keymap = Some(keymap);
            progress.layout = LayoutId::Custom;
            progress.save();
            layout.0 = custom;
            new_exercise.write_default();
            message
        }
        Err(err) => format!("Not imported: {err}"),
    };
    for mut label in &mut status {
        label.0 = message.clone();
    }
}

fn on_layout_change(
    change: On<ValueChange<Entity>>,
    options: Query<&OptionIndex>,
    mut progress: ResMut<Progress>,
    mut layout: ResMut<ActiveLayout>,
    mut new_exercise: MessageWriter<NewExercise>,
    mut focus: ResMut<InputFocus>,
) {
    if let Ok(option) = options.get(change.value)
        && let Some(id) = LayoutId::ALL.get(option.0)
    {
        progress.layout = *id;
        progress.save();
        layout.0 = id.build();
        // The lessons and the statistics are per layout.
        new_exercise.write_default();
    }
    release_focus(&mut focus);
}

/// ‹ and › change the finger in the finger drills, the hand in the bounces, the basic lesson
/// otherwise.
/// The last hanzi level open with the lessons locked: the ones done and the next one.
pub fn hanzi_unlocked(progress: &Progress) -> usize {
    progress.hanzi.levels_done.keys().map(|l| l + 1).max().unwrap_or(0).max(progress.hanzi.level)
}

fn change_lesson(progress: &mut Progress, layout: &crate::layout::Layout, delta: isize) {
    let locked = progress.lock_lessons;
    if matches!(progress.mode, ExerciseMode::BounceLeft | ExerciseMode::BounceRight) {
        // The other hand.
        progress.mode = if progress.mode == ExerciseMode::BounceLeft {
            ExerciseMode::BounceRight
        } else {
            ExerciseMode::BounceLeft
        };
    } else if progress.mode == ExerciseMode::Hanzi {
        let mut level = progress.hanzi.level.saturating_add_signed(delta);
        if locked {
            level = level.min(hanzi_unlocked(progress));
        }
        if level != progress.hanzi.level {
            progress.hanzi.level = level;
            progress.hanzi.word = 0;
        }
    } else if progress.mode == ExerciseMode::Layers {
        let n = crate::course::layer_lessons(layout).len().max(1);
        let current = progress.current();
        let mut lesson = current.layer_lesson.saturating_add_signed(delta).clamp(1, n);
        if locked {
            lesson = lesson.min(current.layer_unlocked);
        }
        current.layer_lesson = lesson;
    } else if progress.mode == ExerciseMode::Fingers {
        let current = progress.current();
        let i = FINGERS.iter().position(|f| *f == current.finger).unwrap_or(0);
        current.finger = FINGERS[(i as isize + delta).rem_euclid(FINGERS.len() as isize) as usize];
    } else {
        let current = progress.current();
        let mut lesson = current.basic_lesson.saturating_add_signed(delta).clamp(1, N_BASIC_LESSONS);
        if locked {
            lesson = lesson.min(current.basic_unlocked);
        }
        current.basic_lesson = lesson;
        progress.mode = ExerciseMode::Basic;
    }
    progress.save();
}

fn previous_lesson(
    _: On<Activate>,
    mut progress: ResMut<Progress>,
    layout: Res<ActiveLayout>,
    mut new_exercise: MessageWriter<NewExercise>,
    mut focus: ResMut<InputFocus>,
) {
    change_lesson(&mut progress, &layout.0, -1);
    new_exercise.write_default();
    release_focus(&mut focus);
}

fn next_lesson(
    _: On<Activate>,
    mut progress: ResMut<Progress>,
    layout: Res<ActiveLayout>,
    mut new_exercise: MessageWriter<NewExercise>,
    mut focus: ResMut<InputFocus>,
) {
    change_lesson(&mut progress, &layout.0, 1);
    new_exercise.write_default();
    release_focus(&mut focus);
}

/// The wheel and the trackpad scroll the window, not with Ctrl: that zooms.
fn scroll_window(
    mut scroll: On<PointerScroll>,
    keys: Res<ButtonInput<KeyCode>>,
    ratio: Res<MouseScrollPixelsPerLine>,
    mut areas: Query<(&ComputedNode, &mut ScrollPosition), With<WindowScroll>>,
) {
    scroll.propagate(false);
    if keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        return;
    }
    let delta = scroll.to_pixels(&ratio);
    for (node, mut position) in &mut areas {
        let visible = node.size().y * node.inverse_scale_factor;
        let content = node.content_size().y * node.inverse_scale_factor;
        let y = (position.y - delta.y).clamp(0.0, (content - visible).max(0.0));
        if position.y != y {
            position.y = y;
        }
    }
}

fn zoom_in(_: On<Activate>, mut zoom: MessageWriter<Zoom>, mut focus: ResMut<InputFocus>) {
    zoom.write(Zoom::IN);
    release_focus(&mut focus);
}

fn zoom_out(_: On<Activate>, mut zoom: MessageWriter<Zoom>, mut focus: ResMut<InputFocus>) {
    zoom.write(Zoom::OUT);
    release_focus(&mut focus);
}

fn zoom_reset(_: On<Activate>, mut zoom: MessageWriter<Zoom>, mut focus: ResMut<InputFocus>) {
    zoom.write(Zoom::Reset);
    release_focus(&mut focus);
}

fn update_zoom_label(scale: Res<UiScale>, mut labels: Query<&mut Text, With<ZoomLabel>>) {
    for mut text in &mut labels {
        let percent = crate::zoom::percent(scale.0);
        if text.0 != percent {
            text.0 = percent;
        }
    }
}

fn restart(_: On<Activate>, mut new_exercise: MessageWriter<NewExercise>, mut focus: ResMut<InputFocus>) {
    new_exercise.write_default();
    release_focus(&mut focus);
}

fn train_suggested_finger(
    _: On<Activate>,
    exercise: Res<Exercise>,
    mut progress: ResMut<Progress>,
    mut new_exercise: MessageWriter<NewExercise>,
    mut focus: ResMut<InputFocus>,
) {
    go_to_suggestion(&exercise, &mut progress, &mut new_exercise);
    release_focus(&mut focus);
}

/// The help on the first start.
fn first_help(mut progress: ResMut<Progress>, mut panel: ResMut<Panel>) {
    if !progress.help_seen {
        *panel = Panel::Help;
        progress.help_seen = true;
        progress.save();
    } else if progress.layouts.values().any(|l| !l.sessions.is_empty()) {
        *panel = Panel::Welcome;
    }
}

#[derive(Component, Default, Clone)]
struct WelcomeOverlay;
#[derive(Component, Default, Clone)]
struct WelcomeText;
#[derive(Component, Default, Clone)]
struct WarmupButton;
#[derive(Component, Default, Clone)]
struct WarmupCaption;

fn welcome_overlay() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            display: Display::None,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6))
        GlobalZIndex(50)
        WelcomeOverlay
        Children [
            @group()
            Node {
                width: px(520),
            }
            Children [
                @group_header() Children [ @label("Welcome back") ]
                --
                @group_body()
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                }
                Children [
                    @label("") WelcomeText
                    --
                    @FeathersButton {
                        @caption: bsn! { @caption("Continue where you left off (Enter)") }
                        @variant: ButtonVariant::Primary,
                    }
                    on(welcome_continue)
                    --
                    @FeathersButton {
                        @caption: bsn! { @caption("") WarmupCaption }
                    }
                    WarmupButton
                    on(welcome_warmup)
                ]
            ]
        ]
    }
}

fn welcome_continue(_: On<Activate>, mut panel: ResMut<Panel>, mut focus: ResMut<InputFocus>) {
    *panel = Panel::None;
    release_focus(&mut focus);
}

fn welcome_warmup(
    _: On<Activate>,
    mut panel: ResMut<Panel>,
    mut progress: ResMut<Progress>,
    mut new_exercise: MessageWriter<NewExercise>,
    mut focus: ResMut<InputFocus>,
) {
    crate::game::start_warmup(&mut progress);
    progress.save();
    new_exercise.write_default();
    *panel = Panel::None;
    release_focus(&mut focus);
}

/// What the last session was, and the keys of the warm-up.
fn fill_welcome(
    panel: Res<Panel>,
    mut progress: ResMut<Progress>,
    mut texts: Query<&mut Text, (With<WelcomeText>, Without<WarmupCaption>)>,
    mut captions: Query<&mut Text, With<WarmupCaption>>,
    mut buttons: Query<&mut Node, (With<WarmupButton>, Without<WelcomeOverlay>)>,
    mut overlays: Query<&mut Node, With<WelcomeOverlay>>,
) {
    if !panel.is_changed() {
        return;
    }
    for mut node in &mut overlays {
        node.display = if *panel == Panel::Welcome { Display::Flex } else { Display::None };
    }
    if *panel != Panel::Welcome {
        return;
    }
    let mode = progress.mode;
    let current = progress.current();
    let place = match mode {
        ExerciseMode::Basic => format!("{}, lesson {}", mode.name(), current.basic_lesson),
        ExerciseMode::Layers => format!("{}, lesson {}", mode.name(), current.layer_lesson),
        ExerciseMode::Hanzi => format!("{}, level {}", mode.name(), progress.hanzi.level + 1),
        _ => mode.name().to_string(),
    };
    let last = progress.layouts.values().flat_map(|l| l.sessions.last()).map(|s| s.finished_at).max();
    let when = last.map(|t| format!(", on {}", crate::stats::short_date(t))).unwrap_or_default();
    for mut text in &mut texts {
        text.0 = format!("Last time: {place}{when}.");
    }
    let keys = crate::game::warmup_keys(&mut progress);
    let shown: Vec<String> = keys.iter().copied().map(display_char).collect();
    for mut caption in &mut captions {
        caption.0 = format!("Warm up first on your weak keys: {}", shown.join(" "));
    }
    for mut node in &mut buttons {
        node.display = if keys.is_empty() { Display::None } else { Display::Flex };
    }
}

fn toggle_help(_: On<Activate>, mut panel: ResMut<Panel>, mut focus: ResMut<InputFocus>) {
    panel.toggle(Panel::Help);
    release_focus(&mut focus);
}

fn toggle_stats(_: On<Activate>, mut panel: ResMut<Panel>, mut focus: ResMut<InputFocus>) {
    panel.toggle(Panel::Stats);
    release_focus(&mut focus);
}

fn toggle_lessons(_: On<Activate>, mut panel: ResMut<Panel>, mut focus: ResMut<InputFocus>) {
    panel.toggle(Panel::Lessons);
    release_focus(&mut focus);
}

fn toggle_settings(_: On<Activate>, mut panel: ResMut<Panel>, mut focus: ResMut<InputFocus>) {
    panel.toggle(Panel::Settings);
    release_focus(&mut focus);
}

fn close_settings(_: On<Activate>, mut panel: ResMut<Panel>, mut focus: ResMut<InputFocus>) {
    *panel = Panel::None;
    release_focus(&mut focus);
}

fn close_help(_: On<PointerClick>, mut panel: ResMut<Panel>) {
    if *panel == Panel::Help {
        *panel = Panel::None;
    }
}

fn close_panel(_: On<PointerClick>, mut panel: ResMut<Panel>) {
    *panel = Panel::None;
}

fn on_theme_change(
    change: On<ValueChange<Entity>>,
    options: Query<&OptionIndex>,
    mut mode: ResMut<ThemeMode>,
    mut progress: ResMut<Progress>,
    mut focus: ResMut<InputFocus>,
) {
    if let Ok(option) = options.get(change.value)
        && let Some(theme) = ThemeMode::ALL.get(option.0)
    {
        *mode = *theme;
        progress.theme = *theme;
        progress.save();
    }
    release_focus(&mut focus);
}

fn on_background_change(
    change: On<ValueChange<Entity>>,
    options: Query<&OptionIndex>,
    mut progress: ResMut<Progress>,
    mut focus: ResMut<InputFocus>,
) {
    if let Ok(option) = options.get(change.value)
        && let Some(level) = BackgroundLevel::ALL.get(option.0)
    {
        progress.background = *level;
        progress.save();
    }
    release_focus(&mut focus);
}

fn show_panels(
    panel: Res<Panel>,
    mut helps: Query<&mut Node, HelpOnly>,
    mut settings: Query<&mut Node, (With<SettingsOverlay>, Without<LessonsOverlay>)>,
    mut lessons: Query<&mut Node, With<LessonsOverlay>>,
) {
    let show = |visible: bool| if visible { Display::Flex } else { Display::None };
    for mut node in &mut helps {
        node.display = show(*panel == Panel::Help);
    }
    for mut node in &mut settings {
        node.display = show(*panel == Panel::Settings);
    }
    for mut node in &mut lessons {
        node.display = show(*panel == Panel::Lessons);
    }
}

type TypingViews = (With<TypingOnly>, Without<crate::hanzi::HanziRoot>);

/// The keyboard views or the hanzi view.
fn show_hanzi(
    progress: Res<Progress>,
    mut typing: Query<(&mut Node, Has<KeyboardRoot>), TypingViews>,
    mut hanzi: Query<&mut Node, (With<crate::hanzi::HanziRoot>, Without<TypingOnly>)>,
) {
    if !progress.is_changed() {
        return;
    }
    let is_hanzi = progress.mode == ExerciseMode::Hanzi;
    let show = |visible: bool| if visible { Display::Flex } else { Display::None };
    for (mut node, is_keyboard) in &mut typing {
        // The exercise area hides its own parts; the compact view has no keyboard.
        let display = show(!is_hanzi && !(is_keyboard && progress.compact));
        if node.display != display {
            node.display = display;
        }
    }
    for mut node in &mut hanzi {
        let display = show(is_hanzi);
        if node.display != display {
            node.display = display;
        }
    }
}

fn show_hands(
    progress: Res<Progress>,
    mut hands: Query<&mut Node, (With<HandsRoot>, Without<CompactSpacer>)>,
    mut spacers: Query<&mut Node, With<CompactSpacer>>,
) {
    if !progress.is_changed() {
        return;
    }
    for mut node in &mut spacers {
        node.display = if progress.compact && progress.mode != ExerciseMode::Hanzi { Display::Flex } else { Display::None };
    }
    // No hands in the hanzi mode, nor in the Matrix.
    let display = if progress.show_hands && progress.mode != ExerciseMode::Hanzi && progress.theme != ThemeMode::Matrix {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut hands {
        if node.display != display {
            node.display = display;
        }
    }
}

fn display_char(c: char) -> String {
    match c {
        ' ' => "Space".into(),
        c => c.to_string(),
    }
}

type HelpOnly = (With<HelpOverlay>, Without<SettingsOverlay>, Without<LessonsOverlay>);
type PromptOnly = (With<PromptRoot>, Without<ShortcutsRoot>, Without<MessageText>, Without<AccuracyText>);
type ShortcutsOnly = (With<ShortcutsRoot>, Without<MessageText>, Without<AccuracyText>);
type AccuracyOnly = (With<AccuracyText>, Without<MessageText>);

/// The text of the typing exercises or the tasks of the shortcuts, the results at the end.
fn show_mode_views(
    progress: Res<Progress>,
    exercise: Res<Exercise>,
    drill: Res<Drill>,
    mut prompts: Query<&mut Node, PromptOnly>,
    mut shortcuts: Query<&mut Node, ShortcutsOnly>,
    mut messages: Query<&mut Node, With<MessageText>>,
    mut accuracies: Query<&mut Node, AccuracyOnly>,
) {
    let typing = progress.mode.is_typing();
    let ended = if typing { exercise.tutor.query == TutorQuery::End } else { drill.finished.is_some() };
    let show = |visible: bool| if visible { Display::Flex } else { Display::None };
    let set = |node: &mut Node, visible: bool| {
        if node.display != show(visible) {
            node.display = show(visible);
        }
    };
    for mut node in &mut prompts {
        set(&mut node, typing && !ended);
    }
    for mut node in &mut shortcuts {
        set(&mut node, !typing && !ended);
    }
    for mut node in &mut messages {
        set(&mut node, ended);
    }
    for mut node in &mut accuracies {
        set(&mut node, typing && ended);
    }
}

fn update_lesson_label(
    exercise: Res<Exercise>,
    layout: Res<ActiveLayout>,
    progress: Res<Progress>,
    mut labels: Query<&mut Text, With<LessonLabel>>,
) {
    if !exercise.is_changed() && !layout.is_changed() && !progress.is_changed() {
        return;
    }
    let text = match exercise.mode {
        ExerciseMode::Basic => {
            let chars: Vec<String> = exercise.char_set.iter().copied().map(display_char).collect();
            format!("Lesson {}/{N_BASIC_LESSONS}: {}", exercise.lesson, chars.join(" "))
        }
        ExerciseMode::Layers => {
            let lessons = crate::course::layer_lessons(&layout.0);
            let name = lessons.get(exercise.lesson.saturating_sub(1)).map(|l| l.name.clone()).unwrap_or_default();
            format!("Layer lesson {}/{}: {name}", exercise.lesson, lessons.len())
        }
        ExerciseMode::Review => {
            let chars: Vec<String> = exercise.char_set.iter().copied().map(display_char).collect();
            format!("Warm-up: {}", chars.join(" "))
        }
        ExerciseMode::Adaptability => "Random words, your errors first".into(),
        ExerciseMode::Velocity => "Random words of the dictionary".into(),
        ExerciseMode::Fluidness => "Paragraphs, backspace fixes errors".into(),
        ExerciseMode::Fingers if exercise.finger == 5 => "Thumbs: Space and Enter".into(),
        ExerciseMode::Fingers => {
            let chars: Vec<String> = layout.0.finger_chars(exercise.finger).iter().map(|(c, _)| c.to_string()).collect();
            format!("{}: {}", capitalize(finger_name(exercise.finger)), chars.join(" "))
        }
        ExerciseMode::BounceLeft | ExerciseMode::BounceRight => {
            let (middle, columns) = crate::drills::bounce_keys(&layout.0, exercise.mode == ExerciseMode::BounceLeft);
            let columns: Vec<String> = columns.iter().map(|c| c.iter().collect()).collect();
            format!("{} + {} · aLtErNaTe", middle.iter().collect::<String>(), columns.join(" "))
        }
        ExerciseMode::Shortcuts => "Esc, arrows, Ctrl, Alt".into(),
        ExerciseMode::Hanzi => format!("Hanzi, level {}", progress.hanzi.level + 1),
    };
    for mut label in &mut labels {
        if label.0 != text {
            label.0.clone_from(&text);
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

fn update_live_stats(
    exercise: Res<Exercise>,
    drill: Res<Drill>,
    progress: Res<Progress>,
    time: Res<Time<Real>>,
    mut labels: Query<&mut Text, With<LiveStats>>,
    mut bars: Query<&mut Node, With<ProgressBar>>,
) {
    let now = time.elapsed_secs_f64();
    let (text, done) = if progress.mode.is_typing() {
        let tutor = &exercise.tutor;
        let elapsed = tutor.elapsed(now);
        let correct = tutor.n_touchs.saturating_sub(tutor.n_errors);
        let wpm = if elapsed > 0.5 { 12.0 * correct as f64 / elapsed } else { 0.0 };
        let accuracy = if tutor.n_touchs > 0 {
            100.0 * (1.0 - tutor.n_errors as f64 / tutor.n_touchs as f64)
        } else {
            100.0
        };
        let mut text = format!("{wpm:.0} wpm  ·  {accuracy:.0}%  ·  {elapsed:.0}s");
        if let Some(results) = tutor.results {
            text.push_str(&format!("  ·  fluidness {:.0}%", results.fluidness));
        } else if tutor.query == TutorQuery::Start {
            text = "Type the first character to start".into();
        } else if tutor.retro_pos > 0 {
            text.push_str("  ·  ⌫ fix the errors");
        }
        let progress = if tutor.text.is_empty() { 0.0 } else { 100.0 * tutor.cursor as f32 / tutor.text.len() as f32 };
        (text, if tutor.query == TutorQuery::End { 100.0 } else { progress })
    } else {
        let text = if drill.is_away() {
            "Away, Alt+Tab back here".to_string()
        } else {
            format!(
                "{}/{} keys  ·  {} missed  ·  {:.0}s",
                drill.done_steps,
                drill.n_steps,
                drill.missed_steps,
                drill.elapsed(now)
            )
        };
        (text, 100.0 * drill.done_steps as f32 / drill.n_steps.max(1) as f32)
    };
    for mut label in &mut labels {
        if label.0 != text {
            label.0.clone_from(&text);
        }
    }
    for mut node in &mut bars {
        node.width = percent(done);
    }
}

fn update_message(
    exercise: Res<Exercise>,
    drill: Res<Drill>,
    progress: Res<Progress>,
    mut labels: Query<&mut Text, With<MessageText>>,
    mut accuracies: Query<(Entity, &mut Text), AccuracyOnly>,
    mut commands: Commands,
) {
    if !exercise.is_changed() && !drill.is_changed() && !progress.is_changed() {
        return;
    }
    if let Some(results) = exercise.tutor.results.filter(|_| exercise.tutor.query == TutorQuery::End) {
        let goal = crate::game::goal(exercise.mode);
        let (text, token) = if results.n_errors == 0 {
            (format!("Accuracy {:.0}%, perfect!", results.accuracy), tokens::CHAR_CORRECT)
        } else if results.accuracy >= goal {
            (format!("Accuracy {:.1}%, goal of {goal:.0}% reached", results.accuracy), tokens::CHAR_CORRECT)
        } else {
            (format!("Accuracy {:.1}%, below the goal of {goal:.0}%", results.accuracy), tokens::CHAR_WRONG)
        };
        for (entity, mut label) in &mut accuracies {
            if label.0 != text {
                label.0.clone_from(&text);
                commands.entity(entity).insert(ThemeTextColor(token.clone()));
            }
        }
    }
    let text = if !progress.mode.is_typing() {
        drill.summary.clone()
    } else if exercise.tutor.query == TutorQuery::End {
        // The numbers are in the bar at the top, the keys in the bar at the bottom: the comment
        // only, without its instructions.
        let mut lines: Vec<String> = exercise
            .message
            .lines()
            .filter(|l| !l.contains("Press Enter") && !l.contains("Backspace"))
            .map(str::to_string)
            .collect();
        if let Some((_, reason)) = &exercise.suggestion {
            lines.push(capitalize(reason));
        }
        lines.join("\n")
    } else {
        String::new()
    };
    for mut label in &mut labels {
        if label.0 != text {
            label.0.clone_from(&text);
        }
    }
}

/// ▲ better than the last time of the lesson, ▼ worse, ● the same, in words too.
fn update_trends(
    exercise: Res<Exercise>,
    theme: Res<bevy::feathers::theme::UiTheme>,
    asset_server: Res<AssetServer>,
    mut rows: Query<&mut Node, With<TrendRow>>,
    mut texts: Query<(&TrendText, &mut Text, &mut TextColor, &mut TextFont)>,
) {
    if !exercise.is_changed() {
        return;
    }
    let trend = exercise.trend.filter(|_| exercise.tutor.query == TutorQuery::End && exercise.mode.is_typing());
    for mut row in &mut rows {
        row.display = if trend.is_some() { Display::Flex } else { Display::None };
    }
    let Some(trend) = trend else {
        return;
    };
    for (TrendText(i), mut text, mut color, mut font) in &mut texts {
        let diff = trend[*i];
        let (name, unit, same) = [("WPM", "", 0.5), ("accuracy", "%", 0.5), ("fluidness", "%", 0.5)][*i];
        let (mark, token) = if diff > same {
            ("▲", tokens::CHAR_CORRECT)
        } else if diff < -same {
            ("▼", tokens::CHAR_WRONG)
        } else {
            ("●", tokens::NEAR_MISS)
        };
        text.0 = format!("{mark} {name} {diff:+.1}{unit}");
        color.0 = theme.color(&token);
        font.font = bevy::text::FontSource::Handle(asset_server.load(bevy::feathers::constants::fonts::MONO));
        font.font_size = FontSize::Px(13.0);
    }
}

/// The keys of the moment, at the bottom.
fn update_actions(exercise: Res<Exercise>, progress: Res<Progress>, drill: Res<Drill>, mut labels: Query<&mut Text, With<ActionsText>>) {
    if !exercise.is_changed() && !progress.is_changed() && !drill.is_changed() {
        return;
    }
    let ended = exercise.tutor.query == TutorQuery::End;
    let mut actions: Vec<String> = Vec::new();
    match progress.mode {
        ExerciseMode::Hanzi => {
            actions.push("Space listen".into());
            actions.push("Enter next level, once done".into());
            actions.push("← → levels".into());
        }
        ExerciseMode::Shortcuts => {
            actions.push(if drill.finished.is_some() { "Enter another round".into() } else { "Esc, the arrows and the shortcuts are the drill".into() });
        }
        _ if ended => {
            actions.push("Enter next exercise".into());
            if exercise.mode == ExerciseMode::Basic {
                actions.push("Backspace previous lesson".into());
            }
            if let Some((finger, _)) = &exercise.suggestion {
                actions.push(format!("Tab {} drill", finger_name(*finger)));
            }
            actions.push("← → lessons".into());
        }
        _ => {
            actions.push("← → lessons".into());
            actions.push("Ctrl+Backspace clear the errors".into());
        }
    }
    actions.push("F1 help".into());
    let text = actions.join("   ·   ");
    for mut label in &mut labels {
        if label.0 != text {
            label.0.clone_from(&text);
        }
    }
}

/// ← and → change the lesson, the finger, the hand or the hanzi level, not in the shortcuts:
/// the arrows are part of the drill.
fn arrow_lessons(
    mut keys: MessageReader<crate::game::KeyPressed>,
    mut progress: ResMut<Progress>,
    layout: Res<ActiveLayout>,
    panel: Res<Panel>,
    mut new_exercise: MessageWriter<NewExercise>,
) {
    use crate::layout::{Chord, Special};
    for key in keys.read() {
        if progress.mode == ExerciseMode::Shortcuts || *panel != Panel::None {
            continue;
        }
        let delta = match key.chord {
            Some(c) if c == Chord::special(Special::Left) => -1,
            Some(c) if c == Chord::special(Special::Right) => 1,
            _ => continue,
        };
        change_lesson(&mut progress, &layout.0, delta);
        new_exercise.write_default();
    }
}

fn update_suggestion(
    exercise: Res<Exercise>,
    mut buttons: Query<&mut Node, With<SuggestButton>>,
    mut captions: Query<&mut Text, With<SuggestCaption>>,
) {
    if !exercise.is_changed() {
        return;
    }
    let suggestion = exercise.suggestion.as_ref().filter(|_| exercise.tutor.query == TutorQuery::End);
    for mut node in &mut buttons {
        node.display = if suggestion.is_some() { Display::Flex } else { Display::None };
    }
    if let Some((finger, _)) = suggestion {
        for mut caption in &mut captions {
            caption.0 = format!("Train the {}", finger_name(*finger));
        }
    }
}

fn update_rankings(
    mut progress: ResMut<Progress>,
    layout: Res<ActiveLayout>,
    mut labels: Query<&mut Text, With<RankingText>>,
) {
    let layout_id = progress.layout;
    let current = progress.bypass_change_detection().current();
    current.update_rankings();
    let list = |keys: &[String]| {
        if keys.is_empty() {
            "not enough touches yet".to_string()
        } else {
            keys.iter()
                .map(|k| {
                    let stats = &current.keys[k];
                    format!("{} {:.0}%", display_char(k.chars().next().unwrap_or(' ')), stats.accuracy())
                })
                .collect::<Vec<_>>()
                .join("   ")
        }
    };
    let misses: Vec<String> = current
        .most_missed
        .iter()
        .map(|k| format!("{} ×{}", display_char(k.chars().next().unwrap_or(' ')), current.keys[k].misses))
        .collect();
    let mut fingers = current.finger_accuracy(|c| layout.0.finger_of(c));
    fingers.retain(|(f, _)| *f != 0);
    fingers.sort_by(|a, b| a.1.total_cmp(&b.1));
    let fingers: Vec<String> =
        fingers.iter().take(3).map(|(f, accuracy)| format!("{} {accuracy:.0}%", finger_name(*f))).collect();
    let mut text = format!(
        "Best keys: {}\nMost missed: {}\nWeakest fingers: {}\n{} · {} exercises done",
        list(&current.best_keys),
        if misses.is_empty() { "none".to_string() } else { misses.join("   ") },
        if fingers.is_empty() { "not enough touches yet".to_string() } else { fingers.join("   ") },
        layout_id.name(),
        current.sessions.len(),
    );
    if current.auto_shift_long + current.auto_shift_short > 0 {
        text.push_str(&format!(
            "\nAuto shift: {} capitals by a key held too long, {} capitals not held long enough",
            current.auto_shift_long, current.auto_shift_short
        ));
    }
    if let Some(best) = current.alt_tab_best {
        text.push_str(&format!(" · best Alt+Tab round trip {best:.1} s"));
    }
    for mut label in &mut labels {
        label.0.clone_from(&text);
    }
}
