//! The window: the feathers toolbar, the live statistics, the exercise, the keyboard and the
//! rankings of the keys.
use bevy::feathers::containers::{flex_spacer, group, group_body, group_header};
use bevy::feathers::controls::*;
use bevy::feathers::display::{caption, label, label_dim};
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor};
use bevy::feathers::tokens as feathers_tokens;
use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::ui::{Checked, Selected};
use bevy::input::mouse::MouseScrollPixelsPerLine;
use bevy::input::touch::Touches;
use bevy::picking::events::{PointerClick, PointerOut, PointerOver, PointerScroll};
use bevy::ui_widgets::{Activate, ValueChange};

use crate::drills::FINGERS;
use crate::game::{ActiveLayout, Exercise, NewExercise, go_to_suggestion};
use crate::klavaro::N_BASIC_LESSONS;
use crate::klavaro::tutor::Query as TutorQuery;
use crate::layout::{LayoutId, finger_name};
use crate::progress::{BackgroundLevel, ExerciseMode, LayoutProgress, Progress, Subject};
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

/// The summary at the end of an exercise: the pane, its title, the accuracy (green when the
/// goal is reached, red otherwise), where Enter goes (green on, amber again), the record and
/// the tip.
#[derive(Component, Default, Clone)]
struct SummaryPane;
#[derive(Component, Default, Clone)]
struct SummaryTitle;
#[derive(Component, Default, Clone)]
struct AccuracyText;
#[derive(Component, Default, Clone)]
struct VerdictText;
#[derive(Component, Default, Clone)]
struct RecordText;

/// The footer tells the key to go on with after this many seconds without a key: whoever
/// knows it has pressed it by then.
const REST_SECONDS: f64 = 5.0;

/// The progress of a drawing exercise in steps: the writes of a hanzi word, the writes of the
/// figures of a penmanship lesson, a hundred writes at most. One bar filling with the steps,
/// as the one of the typing exercises, see [`steps_bar`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Steps {
    pub total: usize,
    /// The steps done, with the fraction of the one in progress.
    pub done: f32,
}

/// The fill of a [`Steps`] bar.
#[derive(Component, Default, Clone, Copy)]
struct StepFill;

/// The progress bar of the typing exercises for the steps of a drawing exercise; the owner sets
/// its [`Steps`] and its width.
pub fn steps_bar() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: px(6),
            border_radius: BorderRadius::all(px(3)),
        }
        ThemeBackgroundColor(tokens::PROGRESS_TRACK)
        Steps
        Children [
            Node {
                width: percent(0),
                height: percent(100),
                border_radius: BorderRadius::all(px(3)),
            }
            ThemeBackgroundColor(tokens::PROGRESS_BAR)
            StepFill
        ]
    }
}

/// The fills of the steps bars follow their steps.
fn fill_steps_bars(bars: Query<(&Steps, &Children), Changed<Steps>>, mut fills: Query<&mut Node, With<StepFill>>) {
    for (steps, children) in &bars {
        let width = percent((100.0 * steps.done / steps.total.max(1) as f32).clamp(0.0, 100.0));
        for child in children.iter() {
            if let Ok(mut node) = fills.get_mut(child)
                && node.width != width
            {
                node.width = width;
            }
        }
    }
}

/// The first menu: touch typing or hanzi.
#[derive(Component, Default, Clone)]
struct SubjectSelect;

/// The second menu: the exercises of the touch typing, or the HSK levels and the weak words.
#[derive(Component, Default, Clone)]
struct TypingSelect;
#[derive(Component, Default, Clone)]
struct HanziSelect;

/// The toolbar at the top, it hides with the setting and in the full screen.
#[derive(Component, Default, Clone)]
pub struct Toolbar;

/// The keys of the moment, at the bottom.
#[derive(Component, Default, Clone)]
pub struct ActionsBar;

/// ‹ and ›, disabled when they lead nowhere.
#[derive(Component, Default, Clone)]
struct PreviousButton;
#[derive(Component, Default, Clone)]
struct NextButton;

#[derive(Component, Default, Clone)]
struct LayoutSelect;

/// The toggles and checkboxes of the settings, they start unchecked.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Setting {
    #[default]
    FadeLegends,
    LayerTrainer,
    ShowHands,
    ReduceMotion,
    LargeText,
    FitWindow,
    PinyinKeypad,
    Sound,
    TypingSounds,
    Brush,
    LockLessons,
    Compact,
    FixWithBackspace,
    AutoHideToolbar,
}

#[derive(Component, Default, Clone)]
struct SettingControl(Setting);

#[derive(Component, Default, Clone)]
struct SuggestButton;

#[derive(Component, Default, Clone)]
struct ZoomLabel;

/// The writes of a new hanzi word, in the settings: auto, or a count up to a hundred.
#[derive(Component, Default, Clone)]
struct WritesLabel;

/// The counts the − and + buttons go through, 0 is the automatic count.
const WRITES_STEPS: [u32; 11] = [0, 2, 4, 6, 8, 10, 15, 20, 30, 50, 100];

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

/// A paste of a text to type waiting for the clipboard, see [`crate::pasted`].
#[derive(Resource, Default)]
struct PendingText(Option<bevy::clipboard::ClipboardRead>);

/// Shown in the exercise "Your text".
#[derive(Component, Default, Clone)]
struct PasteTextButton;

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
#[derive(Component, Default, Clone)]
struct LessonsTitle;

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
    /// The strokes of the hanzi word, see [`crate::hanzi::breakdown`].
    Strokes,
    /// The exercise on paper, see [`crate::print`].
    Print,
}

/// The tabs of the settings, one per subject and the general ones.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Typing,
    Hanzi,
    Penmanship,
}

impl SettingsTab {
    fn name(self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Typing => "Touch typing",
            SettingsTab::Hanzi => "Hanzi",
            SettingsTab::Penmanship => "Penmanship",
        }
    }

    fn of(subject: Subject) -> SettingsTab {
        match subject {
            Subject::Typing => SettingsTab::Typing,
            Subject::Hanzi => SettingsTab::Hanzi,
            Subject::Penmanship => SettingsTab::Penmanship,
        }
    }
}

#[derive(Component, Clone, Copy, Default)]
struct TabButton(SettingsTab);
#[derive(Component, Clone, Copy, Default)]
struct TabPanel(SettingsTab);

/// The sheet of the penmanship, in the settings.
#[derive(Component, Default, Clone)]
struct SheetSelect;
#[derive(Component, Default, Clone)]
struct PenWritesLabel;
#[derive(Component, Default, Clone)]
struct PenRowsLabel;

/// The writes of a penmanship figure the − and + buttons go through, 0 is the five of the
/// course.
const PEN_WRITES_STEPS: [u32; 12] = [0, 2, 3, 4, 6, 8, 10, 15, 20, 30, 50, 100];

impl Panel {
    pub fn toggle(&mut self, panel: Panel) {
        *self = if *self == panel { Panel::None } else { panel };
    }
}

/// The sections of the help, their headings in the color and the weight of [`tokens::HEADING`].
const HELP: [(&str, &str); 9] = [
    (
        "Menus",
        "\
The first menu is what you learn: Touch typing, Hanzi or Penmanship. The second one is its exercise or level,
‹ › go through its lessons; they are grayed out at the ends and before a locked lesson.
Lessons and Stats show those of the menu picked: the touch typing, the hanzi and the penmanship apart.",
    ),
    (
        "Touch typing",
        "\
Basic course, Adaptability, Velocity, Fluidness: the exercises of Klavaro, real words at the end
Layer course: the keys of Lower and Raise row by row    Warm-up: your weak keys, after a pause
Finger drill, Left and Right hand finger bounce: one finger, or middle and index in turns
Shortcuts & keys: Esc, arrows, Ctrl and Alt shortcuts, Alt+Tab",
    ),
    (
        "Hanzi (HSK 1-4)",
        "\
Write each word with a pen, a finger or the mouse, then type its pinyin; each stroke gets its accuracy
A new word is written before your eyes first, then ten times over fainter outlines, fewer as it is learned
(or the count of the settings). The speaker says the word (Space too), Sound in the settings turns it off
After three misses on a stroke it is written again: the word goes to the weak words, review them in the
second menu. Hint shows the stroke too    Stroke breakdown: the word stroke by stroke, how the brush writes each
The printer: the word in practice grids on an A4 page, as many times as you like, saved as a PDF
Brush (settings): rings where the brush presses, thin ends lift; slow is thick without pressure",
    ),
    (
        "Penmanship",
        "\
Spencerian and Copperplate on their guide sheets, the small letters after the drills
Write with a pen, a finger or the mouse, from the dot, in one movement; a figure is written before
your eyes when it starts (F5 shows it again), the counts 1, 2... mark where each part of a stroke begins
Hint (or Space) shows the stroke again when you pause or miss, that write is not a first try
75% of the strokes on the first try opens the next lesson    The printer: the figures on ruled rows, a PDF
Settings, Penmanship: the Spencerian or the Copperplate sheet, the writes of a figure, the rows of the pad",
    ),
    (
        "Toolbar",
        "\
The circling arrow (F5)  a new text, the word or the figure again    The notebook  your lessons and levels    The bars  calendar, progress, your keys or words
The gear  the settings in tabs: general (keyboard, theme, zoom), touch typing, hanzi (sound, T9, writes), penmanship
The toolbar hides in the full screen, it shows when the pointer goes to the top (or always, in the settings)",
    ),
    (
        "Shortcuts",
        "\
F1 or ?  this help    F5  the exercise, the word or the figure again    F11  full screen    Esc  closes a panel
Enter  the next exercise or level, once done    Backspace  the previous basic lesson, once done    Tab  the suggested drill
← →  the lesson, the hanzi level or the penmanship lesson: before the first key and at the end, the ‹ › buttons at any time
Ctrl+Backspace  all the errors    Space  hanzi: the word said; penmanship: the stroke shown, when Hint is there
Ctrl and the wheel, a pinch, Ctrl+= Ctrl+- Ctrl+0  zoom",
    ),
    (
        "Typing",
        "\
Backspace erases the errors, Ctrl+Backspace all of them
A line without errors clears in green, with one slip in orange
At the end the summary says whether Enter goes on to the next lesson (green) or repeats it (amber)",
    ),
    (
        "Colors",
        "\
green the next key, purple shift or hold the key longer (auto shift), amber hold, red backspace
The stripes of the keys and the hands show the finger of every key, a miss streams out of it",
    ),
    (
        "On-screen keyboard",
        "\
Click a key to type it, hold it past the bar under it for the capital (auto shift)
Lower and Raise: click to show and use their layer, click again to go back, or hold them
Ctrl, Alt, Win: click them, then the key: they are never sent to your system",
    ),
];

const HELP_FOOTER: &str = "Esc or a click closes this help. The progress is saved in ~/.config/claudralingo";

/// A section of the help.
#[derive(Component, Default, Clone)]
struct HelpSections;

#[derive(Component, Default, Clone)]
struct SuggestCaption;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Panel>()
            .init_resource::<SettingsTab>()
            .init_resource::<PendingKeymap>()
            .init_resource::<PendingText>().add_systems(Startup, (spawn_ui, first_help)).add_systems(
            Update,
            (
                (sync_settings, sync_mode_select, remember_typing_mode, enable_arrows, show_toolbar, full_screen),
                show_mode_views,
                update_lesson_label,
                update_live_stats,
                update_message,
                update_suggestion,
                show_panels.run_if(resource_changed::<Panel>),
                fill_lessons.run_if(resource_changed::<Panel>),
                show_hands,
                show_hanzi,
                fill_help,
                fill_steps_bars,
                update_trends,
                update_actions,
                arrow_lessons.after(crate::game::GameSystems),
                import_keymap,
                import_text,
                update_rankings.run_if(|p: Res<Progress>, panel: Res<Panel>| p.is_changed() || panel.is_changed()),
                (
                    update_zoom_label.run_if(resource_changed::<UiScale>),
                    update_writes_label,
                    update_pen_labels,
                    show_settings_tab.run_if(|tab: Res<SettingsTab>, panel: Res<Panel>| tab.is_changed() || panel.is_changed()),
                ),
            )
                .after(crate::game::GameSystems),
        );
    }
}

/// The options of the hanzi menu: the HSK levels, then the weak words.
const HANZI_OPTIONS: [&str; 5] = ["HSK 1", "HSK 2", "HSK 3", "HSK 4", "Weak words"];

/// The option of the hanzi menu of the progress.
fn hanzi_option(progress: &Progress, data: &crate::hanzi::data::HanziData) -> usize {
    if progress.hanzi.review {
        return HANZI_OPTIONS.len() - 1;
    }
    data.levels.get(progress.hanzi.level).map_or(0, |l| l.hsk.saturating_sub(1) as usize)
}

fn spawn_ui(mut commands: Commands, progress: Res<Progress>, data: Res<crate::hanzi::data::HanziData>) {
    commands.spawn(Camera2d);
    let subject = Subject::ALL.iter().position(|s| *s == progress.mode.subject());
    let subjects = list_rows_from_strings(Subject::ALL.map(Subject::name), subject);
    let typing = Subject::Typing.modes();
    let typing_mode = if progress.mode.subject() == Subject::Typing { progress.mode } else { progress.typing_mode };
    let exercise = typing.iter().position(|m| *m == typing_mode);
    let exercises = list_rows_from_strings(typing.iter().map(|m| m.name()), exercise);
    let books = list_rows_from_strings(HANZI_OPTIONS, Some(hanzi_option(&progress, &data)));
    let layout = LayoutId::ALL.iter().position(|l| *l == progress.layout);
    let layouts = list_rows_from_strings(LayoutId::ALL.map(LayoutId::name), layout);
    let background = BackgroundLevel::ALL.iter().position(|b| *b == progress.background);
    let backgrounds = list_rows_from_strings(BackgroundLevel::ALL.map(BackgroundLevel::name), background);
    let theme = ThemeMode::ALL.iter().position(|t| *t == progress.theme);
    let themes = list_rows_from_strings(ThemeMode::ALL.map(ThemeMode::name), theme);
    use crate::penmanship::letters::Style;
    let sheet = Style::ALL.iter().position(|s| *s == progress.penmanship.style);
    let sheets = list_rows_from_strings(Style::ALL.map(Style::name), sheet);
    commands.spawn_scene(root([subjects, exercises, books], layouts, backgrounds, themes, sheets));
}

fn root(
    menus: [Box<dyn SceneList>; 3],
    layouts: Box<dyn SceneList>,
    backgrounds: Box<dyn SceneList>,
    themes: Box<dyn SceneList>,
    sheets: Box<dyn SceneList>,
) -> impl Scene {
    bsn! {
        // The root is a flex item of the viewport: without a minimum height of 0 it grows with
        // its content, zoomed in the footer went out of the window.
        Node {
            width: percent(100),
            height: percent(100),
            min_height: px(0),
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
            @toolbar(menus)
            --
            // Everything but the toolbar and the overlays scrolls, zoomed in.
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
                    // The summary at the end: how it went, where Enter goes, the tip.
                    @group()
                    Node {
                        display: Display::None,
                        width: px(640),
                        max_width: percent(96),
                    }
                    SummaryPane
                    Children [
                        @group_header() Children [ @label("") SummaryTitle ]
                        --
                        @group_body()
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: px(8),
                            padding: UiRect::axes(px(16), px(14)),
                        }
                        Children [
                            @label("") AccuracyText
                            TextFont { font_size: FontSize::Px(24.0) }
                            Node { display: Display::None }
                            --
                            @label("") VerdictText
                            TextFont { font_size: FontSize::Px(18.0) }
                            Node { display: Display::None }
                            --
                            @label("") RecordText
                            TextFont { font_size: FontSize::Px(16.0) }
                            ThemeTextColor(tokens::CHAR_CORRECT)
                            Node { display: Display::None }
                            --
                            @label("") MessageText
                            TextFont { font_size: FontSize::Px(16.0) }
                            TextLayout { justify: Justify::Center }
                            Node { display: Display::None }
                            --
                            @FeathersButton {
                                @caption: bsn! { @caption("") SuggestCaption }
                            }
                            SuggestButton
                            Node { display: Display::None }
                            on(train_suggested_finger)
                        ]
                    ]
                ]
                --
                @crate::hanzi::view()
                --
                @crate::penmanship::view()
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
                max_width: percent(100),
            }
            ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
            ActionsBar
            Children [
                @label_dim("") ActionsText
                TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary }
            ]
            --
            @help_overlay()
            --
            @settings_overlay(layouts, backgrounds, themes, sheets)
            --
            @lessons_overlay()
            --
            @crate::print::overlay()
            --
            @crate::stats::overlay()
            --
            @crate::hanzi::breakdown::overlay()
        ]
    }
}

fn toolbar([subjects, exercises, books]: [Box<dyn SceneList>; 3]) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            column_gap: px(8),
            row_gap: px(6),
            padding: px(8),
            top: px(0),
            left: px(0),
        }
        ThemeBackgroundColor(feathers_tokens::PANE_HEADER_BG)
        GlobalZIndex(40)
        Toolbar
        Children [
            @FeathersSelect {
                @options: subjects,
            }
            SubjectSelect
            Node {
                min_width: px(130),
            }
            on(on_subject_change)
            --
            @FeathersSelect {
                @options: exercises,
            }
            TypingSelect
            Node {
                min_width: px(200),
            }
            on(on_exercise_change)
            --
            @FeathersSelect {
                @options: books,
            }
            HanziSelect
            Node {
                display: Display::None,
                min_width: px(110),
            }
            on(on_hanzi_change)
            --
            @FeathersButton {
                @caption: bsn! { @caption("‹") }
            }
            PreviousButton
            on(previous_lesson)
            --
            @label_dim("") LessonLabel
            --
            @FeathersButton {
                @caption: bsn! { @caption("›") }
            }
            NextButton
            on(next_lesson)
            --
            @FeathersButton {
                @caption: bsn! { @caption("Paste text") }
            }
            PasteTextButton
            Node {
                display: Display::None,
            }
            on(paste_text)
            --
            @FeathersButton {
                @caption: bsn! { @crate::icons::icon(crate::icons::Icon::Restart) }
            }
            on(restart)
            --
            @flex_spacer()
            --
            @FeathersButton {
                @caption: bsn! { @crate::icons::icon(crate::icons::Icon::Lessons) }
            }
            on(toggle_lessons)
            --
            @FeathersButton {
                @caption: bsn! { @crate::icons::icon(crate::icons::Icon::Stats) }
            }
            on(toggle_stats)
            --
            @FeathersButton {
                @caption: bsn! { @crate::icons::icon(crate::icons::Icon::Settings) }
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
pub fn setting_checkbox(setting: Setting, text: &'static str) -> impl Scene {
    bsn! {
        @FeathersCheckbox {
            @caption: bsn! { @caption(text) }
        }
        SettingControl({setting})
        on(on_setting_change)
    }
}

fn settings_overlay(layouts: Box<dyn SceneList>, backgrounds: Box<dyn SceneList>, themes: Box<dyn SceneList>, sheets: Box<dyn SceneList>) -> impl Scene {
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
            // At a high zoom it scrolls, the header stays.
            @group()
            Node {
                width: px(600),
                max_width: percent(96),
                max_height: percent(94),
            }
            Children [
                @group_header() Children [ @label("Settings") ]
                --
                @group_body()
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    padding: px(12),
                    min_height: px(0),
                    overflow: Overflow::scroll_y(),
                }
                bevy::ui_widgets::ScrollArea
                Children [
                    // The tabs: the general settings and one per subject.
                    Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: px(6),
                        row_gap: px(6),
                    }
                    Children [
                        @settings_tab(SettingsTab::General) -- @settings_tab(SettingsTab::Typing)
                        -- @settings_tab(SettingsTab::Hanzi) -- @settings_tab(SettingsTab::Penmanship)
                    ]
                    --
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12),
                    }
                    TabPanel({SettingsTab::General})
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
                                min_width: px(150),
                            }
                            on(on_theme_change)
                            --
                            @label("Background")
                            --
                            @FeathersSelect {
                                @options: backgrounds,
                            }
                            Node {
                                min_width: px(110),
                            }
                            on(on_background_change)
                        ]
                        --
                        @setting_checkbox(Setting::LockLessons, "Lock lessons: pass a lesson (95%) to open the next one")
                        --
                        @setting_checkbox(Setting::AutoHideToolbar, "Hide the toolbar: it shows when the pointer goes to the top (always in full screen, F11)")
                        --
                        @setting_checkbox(Setting::ReduceMotion, "Reduce motion: no drift, sparks, shakes nor breathing keys")
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
                    ]
                    --
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12),
                    }
                    TabPanel({SettingsTab::Typing})
                    Children [
                        @setting_checkbox(Setting::FixWithBackspace, "Fix mistakes with Backspace: off, the cursor waits on the missed key")
                        --
                        @setting_checkbox(Setting::FadeLegends, "Fade legends: the letters fade while you type well")
                        --
                        @setting_checkbox(Setting::LayerTrainer, "Layer trainer: show the layer and modifier keys to hold")
                        --
                        @setting_checkbox(Setting::ShowHands, "Show the hands under the keyboard")
                        --
                        @setting_checkbox(Setting::Compact, "Compact, for phones: no keyboard, the finger tips and the layer to hold")
                        --
                        @setting_checkbox(Setting::LargeText, "Large text to type")
                        --
                        @setting_checkbox(Setting::TypingSounds, "Typing sounds: a bell on a miss, a chime on a clean line, a falling tone for a key held too long")
                    ]
                    --
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12),
                    }
                    TabPanel({SettingsTab::Hanzi})
                    Children [
                        @setting_checkbox(Setting::Sound, "Sound: the words spoken (the speaker button, Space)")
                        --
                        @setting_checkbox(Setting::PinyinKeypad, "Type the pinyin on the nine keys of a phone (T9)")
                        --
                        @setting_checkbox(Setting::Brush, "Brush, where to press and lift; a pen's pressure or the speed (slow is thick)")
                        --
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(8),
                        }
                        Children [
                            @label("Writes") Node { min_width: px(110) }
                            --
                            @FeathersButton {
                                @caption: bsn! { @caption("−") }
                            }
                            on(writes_less)
                            --
                            @label("") WritesLabel Node { min_width: px(48), justify_content: JustifyContent::Center }
                            --
                            @FeathersButton {
                                @caption: bsn! { @caption("+") }
                            }
                            on(writes_more)
                            --
                            @label_dim("of a new word before its pinyin; auto: ten, fewer as you learn")
                        ]
                    ]
                    --
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12),
                    }
                    TabPanel({SettingsTab::Penmanship})
                    Children [
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(12),
                        }
                        Children [
                            @label("Sheet") Node { min_width: px(110) }
                            --
                            @FeathersSelect {
                                @options: sheets,
                            }
                            Node {
                                min_width: px(150),
                            }
                            SheetSelect
                            on(on_sheet_change)
                            --
                            @label_dim("Spencerian 52°, the 2:1:2 sheet; Copperplate 55°, the 3:2:3 sheet")
                        ]
                        --
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(8),
                        }
                        Children [
                            @label("Writes") Node { min_width: px(110) }
                            --
                            @FeathersButton {
                                @caption: bsn! { @caption("−") }
                            }
                            on(pen_writes_less)
                            --
                            @label("") PenWritesLabel Node { min_width: px(48), justify_content: JustifyContent::Center }
                            --
                            @FeathersButton {
                                @caption: bsn! { @caption("+") }
                            }
                            on(pen_writes_more)
                            --
                            @label_dim("of each figure: over the model, over the faded model, then on the empty sheet")
                        ]
                        --
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(8),
                        }
                        Children [
                            @label("Rows") Node { min_width: px(110) }
                            --
                            @FeathersButton {
                                @caption: bsn! { @caption("−") }
                            }
                            on(pen_rows_less)
                            --
                            @label("") PenRowsLabel Node { min_width: px(48), justify_content: JustifyContent::Center }
                            --
                            @FeathersButton {
                                @caption: bsn! { @caption("+") }
                            }
                            on(pen_rows_more)
                            --
                            @label_dim("of the pad: the writes go down the rows, the sheet clears once they are full")
                        ]
                    ]
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

/// A tab of the settings.
fn settings_tab(tab: SettingsTab) -> impl Scene {
    bsn! {
        @FeathersButton {
            @caption: bsn! { @caption(tab.name()) }
        }
        TabButton({tab})
        on(pick_tab)
    }
}

/// The lessons of the subject on a path: a node per course or HSK level, the lessons as tiles
/// with their grade, the ones ahead dim; a click practices one, a click beside the panel
/// closes it, see [`crate::lessons`].
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
                width: px(860),
                max_width: percent(96),
                max_height: percent(94),
            }
            Children [
                @group_header() Children [ @label("") LessonsTitle ]
                --
                @group_body()
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    padding: px(12),
                    min_height: px(0),
                    overflow: Overflow::scroll_y(),
                }
                bevy::ui_widgets::ScrollArea
                Children [
                    @label_dim("") LessonsHint
                    --
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                    }
                    LessonsGrid
                ]
            ]
            --
            // Over a lesson ahead: what opens it.
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                padding: UiRect::axes(px(10), px(6)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
            }
            ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
            ThemeBorderColor(feathers_tokens::GROUP_BORDER)
            GlobalZIndex(60)
            Pickable::IGNORE
            LessonTip
            Children [
                @label("") LessonTipText
                Pickable::IGNORE
            ]
        ]
    }
}

/// The tip over a lesson ahead, and its text.
#[derive(Component, Default, Clone)]
struct LessonTip;
#[derive(Component, Default, Clone)]
struct LessonTipText;

/// The tile of a lesson, 96 by 64, its border the grade.
const TILE: (f32, f32) = (96.0, 64.0);

#[allow(clippy::too_many_arguments)]
fn fill_lessons(
    panel: Res<Panel>,
    progress: Res<Progress>,
    grid: Res<crate::game::Grid>,
    layout: Res<ActiveLayout>,
    theme: Res<bevy::feathers::theme::UiTheme>,
    asset_server: Res<AssetServer>,
    grids: Query<Entity, With<LessonsGrid>>,
    hanzi: Res<crate::hanzi::data::HanziData>,
    hanzi_font: Res<crate::hanzi::HanziFont>,
    mut hints: Query<&mut Text, (With<LessonsHint>, Without<LessonsTitle>)>,
    mut titles: Query<&mut Text, With<LessonsTitle>>,
    mut commands: Commands,
) {
    if *panel != Panel::Lessons {
        return;
    }
    for mut title in &mut titles {
        title.0 = format!("Lessons · {}", progress.mode.subject().name());
    }
    let goal = match progress.mode.subject() {
        Subject::Penmanship => "Good from 75% of the strokes on the first try, Great from 90%, Legendary at 100%",
        Subject::Hanzi => "Good once the ten words are learned, Great with four misses at most, Legendary without one",
        Subject::Typing => "Good from the goal of the exercise, Great from 98%, Legendary without a single error",
    };
    for mut text in &mut hints {
        text.0 = format!("Click a lesson to practice it. {goal}. The dim ones come after the one in progress.");
    }
    let sections = crate::lessons::sections(&progress, &grid.0, &layout.0, &hanzi);
    let font = |size: f32| TextFont {
        font: bevy::text::FontSource::Handle(asset_server.load(bevy::feathers::constants::fonts::REGULAR)),
        font_size: FontSize::Px(size),
        ..default()
    };
    let cjk = TextFont { font: bevy::text::FontSource::Handle(hanzi_font.0.clone()), font_size: FontSize::Px(15.0), ..default() };
    for root in &grids {
        commands.entity(root).despawn_children();
        let last = sections.len().saturating_sub(1);
        for (k, section) in sections.iter().enumerate() {
            spawn_section(&mut commands, root, section, k == last, &theme, &font, &cjk);
        }
    }
}

/// A section: its node and its line on the path at the left, its name and its tiles.
fn spawn_section(
    commands: &mut Commands,
    root: Entity,
    section: &crate::lessons::Section,
    last: bool,
    theme: &bevy::feathers::theme::UiTheme,
    font: &dyn Fn(f32) -> TextFont,
    cjk: &TextFont,
) {
    use crate::lessons::Status;
    let row = commands
        .spawn((Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Stretch, column_gap: px(14), ..default() }, ChildOf(root)))
        .id();
    // The path: a node filled in the color of the grade once the section is done, a ring
    // around the one in progress, dim ahead; the line under it fills with the lessons passed.
    let (grade, total, passed) = (section.grade(), section.lessons.len().max(1), section.passed());
    let (fill, border) = match grade {
        Some(grade) => (theme.color(&grade.token()), theme.color(&grade.token())),
        None if section.current() || section.reached() => (Color::NONE, theme.color(&tokens::CURSOR)),
        None => (Color::NONE, theme.color(&tokens::KEY_BORDER)),
    };
    let path = commands
        .spawn((
            Node { width: px(22), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, flex_shrink: 0.0, ..default() },
            ChildOf(row),
        ))
        .id();
    commands.spawn((
        Node { width: px(22), height: px(22), border: UiRect::all(px(3)), border_radius: BorderRadius::MAX, flex_shrink: 0.0, ..default() },
        BackgroundColor(fill),
        BorderColor::all(border),
        ChildOf(path),
    ));
    if !last {
        let line = commands
            .spawn((
                Node { width: px(3), flex_grow: 1.0, margin: UiRect::vertical(px(4)), border_radius: BorderRadius::all(px(2)), ..default() },
                BackgroundColor(theme.color(&tokens::PROGRESS_TRACK)),
                ChildOf(path),
            ))
            .id();
        commands.spawn((
            Node { width: percent(100), height: percent(100.0 * passed as f32 / total as f32), border_radius: BorderRadius::all(px(2)), ..default() },
            BackgroundColor(theme.color(&tokens::PROGRESS_BAR)),
            ChildOf(line),
        ));
    }
    let content = commands
        .spawn((
            Node { flex_direction: FlexDirection::Column, row_gap: px(8), flex_grow: 1.0, min_width: px(0), padding: UiRect::bottom(px(14)), ..default() },
            ChildOf(row),
        ))
        .id();
    let heading = match grade {
        Some(grade) => format!("{} · {}", section.name, grade.name()),
        None => format!("{} · {passed} of {total}", section.name),
    };
    commands.spawn((
        Text::new(heading),
        font(16.0),
        ThemeTextColor(grade.map_or(feathers_tokens::TEXT_MAIN, |g| g.token())),
        Node { margin: UiRect::top(px(1)), ..default() },
        ChildOf(content),
    ));
    let tiles = commands
        .spawn((Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(6), row_gap: px(6), ..default() }, ChildOf(content)))
        .id();
    // The names of the penmanship lessons and of the layer lessons are long.
    let wide = section.lessons.iter().any(|l| l.cjk.is_none() && l.title.chars().count() > 11);
    for lesson in &section.lessons {
        let ahead = matches!(lesson.status, Status::Ahead { .. });
        let border = match (&lesson.status, lesson.current) {
            (_, true) => theme.color(&tokens::CURSOR),
            (Status::Passed(grade), _) => theme.color(&grade.token()),
            _ => Color::NONE,
        };
        let dim = |color: Color| if ahead { color.with_alpha(0.4) } else { color };
        let tile = commands
            .spawn((
                Node {
                    width: px(if wide { TILE.0 * 1.75 } else { TILE.0 }),
                    height: px(TILE.1),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::axes(px(6), px(4)),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(6)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(dim(theme.color(&tokens::KEY_BG))),
                BorderColor::all(border),
                ChildOf(tiles),
            ))
            .id();
        commands.spawn((
            Text::new(lesson.title.clone()),
            font(if lesson.cjk.is_some() { 13.0 } else { 15.0 }),
            TextLayout::no_wrap(),
            TextColor(dim(theme.color(&feathers_tokens::TEXT_MAIN))),
            Pickable::IGNORE,
            ChildOf(tile),
        ));
        if let Some(words) = &lesson.cjk {
            commands.spawn((Text::new(words.clone()), cjk.clone(), TextLayout::no_wrap(), TextColor(dim(theme.color(&feathers_tokens::TEXT_MAIN))), Pickable::IGNORE, ChildOf(tile)));
        }
        let (detail, color) = match &lesson.status {
            Status::Passed(grade) => (grade.name().to_string(), theme.color(&grade.token())),
            Status::Below(best) => (format!("best {best:.0}%"), theme.color(&tokens::NEAR_MISS)),
            Status::NotYet if lesson.current => ("in progress".to_string(), theme.color(&tokens::CURSOR)),
            _ => (String::new(), Color::NONE),
        };
        if !detail.is_empty() {
            commands.spawn((Text::new(detail), font(12.0), TextColor(color), Pickable::IGNORE, ChildOf(tile)));
        }
        let go = lesson.go;
        let locked = matches!(lesson.status, Status::Ahead { locked: true, .. });
        commands.entity(tile).observe(
            move |_: On<PointerClick>, mut progress: ResMut<Progress>, mut panel: ResMut<Panel>, mut new_exercise: MessageWriter<NewExercise>| {
                if locked {
                    return;
                }
                crate::lessons::go(&mut progress, go);
                progress.save();
                new_exercise.write_default();
                *panel = Panel::None;
            },
        );
        if let Status::Ahead { locked, next } = &lesson.status {
            let tip = if *locked { format!("Pass {next} first") } else { format!("{} is next: click to jump ahead", capitalize(next)) };
            commands.entity(tile).observe(
                move |_: On<PointerOver>,
                      tiles: Query<(&ComputedNode, &UiGlobalTransform)>,
                      mut tips: Query<&mut Node, With<LessonTip>>,
                      mut texts: Query<&mut Text, With<LessonTipText>>| {
                    let Ok((node, transform)) = tiles.get(tile) else {
                        return;
                    };
                    let scale = node.inverse_scale_factor;
                    let (size, center) = (node.size() * scale, transform.translation * scale);
                    for mut tip_node in &mut tips {
                        tip_node.display = Display::Flex;
                        tip_node.left = px(center.x - size.x / 2.0);
                        tip_node.top = px(center.y + size.y / 2.0 + 4.0);
                    }
                    for mut text in &mut texts {
                        text.0.clone_from(&tip);
                    }
                },
            );
            commands.entity(tile).observe(|_: On<PointerOut>, mut tips: Query<&mut Node, With<LessonTip>>| {
                for mut tip_node in &mut tips {
                    tip_node.display = Display::None;
                }
            });
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
                width: px(860),
                max_width: percent(96),
                max_height: percent(94),
            }
            Children [
                @group_header() Children [ @label("Help") ]
                --
                @group_body()
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    min_height: px(0),
                    overflow: Overflow::scroll_y(),
                }
                bevy::ui_widgets::ScrollArea
                Children [
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                    }
                    HelpSections
                    --
                    @label_dim(HELP_FOOTER)
                    Node { margin: UiRect::top(px(10)) }
                ]
            ]
        ]
    }
}

/// The sections of the help: a heading, bold and in the color of the headings, and its lines.
fn fill_help(sections: Query<Entity, Added<HelpSections>>, asset_server: Res<AssetServer>, mut commands: Commands) {
    for root in &sections {
        let font = |bold: bool, size: f32| TextFont {
            font: bevy::text::FontSource::Handle(asset_server.load(if bold {
                bevy::feathers::constants::fonts::BOLD
            } else {
                bevy::feathers::constants::fonts::REGULAR
            })),
            font_size: FontSize::Px(size),
            ..default()
        };
        for (i, (heading, body)) in HELP.iter().enumerate() {
            commands.spawn((
                Text::new(heading.to_uppercase()),
                font(true, 15.0),
                ThemeTextColor(tokens::HEADING),
                Node { margin: UiRect::top(px(if i == 0 { 0.0 } else { 10.0 })), ..default() },
                ChildOf(root),
            ));
            commands.spawn((
                Text::new(*body),
                font(false, 14.0),
                ThemeTextColor(feathers_tokens::TEXT_MAIN),
                Node { margin: UiRect::left(px(12)), ..default() },
                ChildOf(root),
            ));
        }
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
            Setting::TypingSounds => progress.typing_sounds,
            Setting::Brush => progress.brush,
            Setting::LockLessons => progress.lock_lessons,
            Setting::Compact => progress.compact,
            Setting::FixWithBackspace => progress.fix_with_backspace,
            Setting::AutoHideToolbar => progress.auto_hide_toolbar,
        }
    }
}

/// The toggles and the checkboxes have no initial value prop; a setting may have two of them,
/// the sound of the hanzi, they follow the progress.
fn sync_settings(
    settings: Query<(Entity, &SettingControl, Has<Checked>)>,
    added: Query<(), Added<SettingControl>>,
    progress: Res<Progress>,
    mut commands: Commands,
) {
    if !progress.is_changed() && added.is_empty() {
        return;
    }
    for (entity, SettingControl(setting), checked) in &settings {
        match (setting.get(&progress), checked) {
            (true, false) => {
                commands.entity(entity).insert(Checked);
            }
            (false, true) => {
                commands.entity(entity).remove::<Checked>();
            }
            _ => {}
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
        Setting::TypingSounds => progress.typing_sounds = change.value,
        Setting::Brush => progress.brush = change.value,
        Setting::LockLessons => progress.lock_lessons = change.value,
        Setting::Compact => progress.compact = change.value,
        Setting::FixWithBackspace => progress.fix_with_backspace = change.value,
        Setting::AutoHideToolbar => progress.auto_hide_toolbar = change.value,
    }
    progress.save();
    release_focus(&mut focus);
}

fn on_subject_change(
    change: On<ValueChange<Entity>>,
    options: Query<&OptionIndex>,
    mut progress: ResMut<Progress>,
    mut new_exercise: MessageWriter<NewExercise>,
    mut focus: ResMut<InputFocus>,
) {
    if let Ok(option) = options.get(change.value)
        && let Some(subject) = Subject::ALL.get(option.0)
        && *subject != progress.mode.subject()
    {
        progress.mode = match subject {
            Subject::Typing => progress.typing_mode,
            Subject::Hanzi => ExerciseMode::Hanzi,
            Subject::Penmanship => ExerciseMode::Penmanship,
        };
        progress.save();
        new_exercise.write_default();
    }
    release_focus(&mut focus);
}

fn on_exercise_change(
    change: On<ValueChange<Entity>>,
    options: Query<&OptionIndex>,
    mut progress: ResMut<Progress>,
    mut new_exercise: MessageWriter<NewExercise>,
    mut focus: ResMut<InputFocus>,
) {
    if let Ok(option) = options.get(change.value)
        && let Some(mode) = Subject::Typing.modes().get(option.0)
    {
        progress.mode = *mode;
        progress.save();
        new_exercise.write_default();
    }
    release_focus(&mut focus);
}

/// An HSK level: its first level not done yet, not past the locked ones; or the weak words.
#[allow(clippy::too_many_arguments)]
fn on_hanzi_change(
    change: On<ValueChange<Entity>>,
    options: Query<&OptionIndex>,
    mut progress: ResMut<Progress>,
    data: Res<crate::hanzi::data::HanziData>,
    mut practice: ResMut<crate::hanzi::Practice>,
    mut new_exercise: MessageWriter<NewExercise>,
    mut focus: ResMut<InputFocus>,
) {
    release_focus(&mut focus);
    let Ok(OptionIndex(option)) = options.get(change.value).copied() else {
        return;
    };
    if option == HANZI_OPTIONS.len() - 1 {
        if crate::hanzi::start_review(&mut progress, &data) {
            new_exercise.write_default();
        } else {
            practice.message =
                "No weak words yet: a word becomes weak when one of its strokes is shown, after three misses or with Hint.".into();
            // The menu goes back to the level.
            progress.set_changed();
        }
        return;
    }
    let levels = data.levels_of_hsk(option as u8 + 1);
    let open = if progress.lock_lessons { hanzi_unlocked(&progress) } else { usize::MAX };
    if levels.start > open {
        practice.message = format!("HSK {} opens when HSK {} is done, or without the locked lessons (settings).", option + 1, option);
        progress.set_changed();
        return;
    }
    let level = levels.clone().find(|l| !progress.hanzi.levels_done.contains_key(l)).unwrap_or(levels.start).min(open);
    crate::hanzi::go_to_level(&mut practice, &mut progress, &data, level);
    new_exercise.write_default();
}

/// The menus follow the progress: the mode also changes from the keyboard and the buttons.
#[allow(clippy::type_complexity)]
fn sync_mode_select(
    progress: Res<Progress>,
    data: Res<crate::hanzi::data::HanziData>,
    selects: Query<
        (Entity, Has<SubjectSelect>, Has<TypingSelect>, Has<HanziSelect>, Has<LayoutSelect>),
        Or<(With<SubjectSelect>, With<TypingSelect>, With<HanziSelect>, With<LayoutSelect>)>,
    >,
    mut nodes: Query<&mut Node, Or<(With<TypingSelect>, With<HanziSelect>)>>,
    children: Query<&Children>,
    rows: Query<(&OptionIndex, Has<Selected>)>,
    mut commands: Commands,
) {
    if !progress.is_changed() {
        return;
    }
    let subject = progress.mode.subject();
    let typing_mode = if subject == Subject::Typing { progress.mode } else { progress.typing_mode };
    for (select, is_subject, is_typing, is_hanzi, is_layout) in &selects {
        let index = if is_subject {
            Subject::ALL.iter().position(|s| *s == subject)
        } else if is_typing {
            Subject::Typing.modes().iter().position(|m| *m == typing_mode)
        } else if is_hanzi {
            Some(hanzi_option(&progress, &data))
        } else if is_layout {
            LayoutId::ALL.iter().position(|l| *l == progress.layout)
        } else {
            continue;
        };
        if let Ok(mut node) = nodes.get_mut(select) {
            let shown = if (is_typing && subject == Subject::Typing) || (is_hanzi && subject == Subject::Hanzi) {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != shown {
                node.display = shown;
            }
        }
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

/// The exercise of the touch typing to go back to from the hanzi.
fn remember_typing_mode(mut progress: ResMut<Progress>) {
    if progress.is_changed() && progress.mode.subject() == Subject::Typing && progress.typing_mode != progress.mode {
        progress.typing_mode = progress.mode;
    }
}

fn paste_text(
    _: On<Activate>,
    mut clipboard: ResMut<bevy::clipboard::Clipboard>,
    mut pending: ResMut<PendingText>,
    mut focus: ResMut<InputFocus>,
) {
    pending.0 = Some(clipboard.fetch_text());
    release_focus(&mut focus);
}

/// The text of the clipboard becomes the text of "Your text", from its first part.
fn import_text(
    mut pending: ResMut<PendingText>,
    mut progress: ResMut<Progress>,
    mut new_exercise: MessageWriter<NewExercise>,
    mut buttons: Query<&mut Node, With<PasteTextButton>>,
) {
    let show = if progress.mode == ExerciseMode::Custom { Display::Flex } else { Display::None };
    for mut node in &mut buttons {
        if node.display != show {
            node.display = show;
        }
    }
    let Some(read) = pending.0.as_mut() else {
        return;
    };
    let Some(result) = read.poll_result() else {
        return;
    };
    pending.0 = None;
    match result {
        Ok(text) if !text.trim().is_empty() => {
            // A book is typed part by part, a few pages are plenty.
            const MAX_CHARS: usize = 20_000;
            progress.pasted_text = text.chars().take(MAX_CHARS).collect();
            progress.pasted_part = 0;
            progress.save();
            new_exercise.write_default();
        }
        Ok(_) => warn!("the clipboard has no text"),
        Err(err) => warn!("could not read the clipboard: {err:?}"),
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

/// The last hanzi level open with the lessons locked: the ones done and the next one.
pub fn hanzi_unlocked(progress: &Progress) -> usize {
    progress.hanzi.levels_done.keys().map(|l| l + 1).max().unwrap_or(0).max(progress.hanzi.level)
}

/// Where ‹ and › lead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Basic(usize),
    Layer(usize),
    Finger(u8),
    /// The other hand of the bounces.
    Mode(ExerciseMode),
    HanziLevel(usize),
    PastedPart(usize),
    PenLesson(usize),
}

/// ‹ and › change the basic or the layer lesson, the finger of the finger drills, the hand of
/// the bounces, the level within the HSK level. Nowhere at the ends, before a locked lesson,
/// in the review of the weak words and in the exercises without lessons.
fn step(progress: &Progress, layout: &crate::layout::Layout, data: &crate::hanzi::data::HanziData, delta: isize) -> Option<Step> {
    let locked = progress.lock_lessons;
    let default = LayoutProgress::default();
    let current = progress.layouts.get(progress.layout.key()).unwrap_or(&default);
    let within = |value: usize, delta: isize, range: std::ops::RangeInclusive<usize>, open: usize| {
        let next = value.checked_add_signed(delta)?;
        (range.contains(&next) && (!locked || next <= open) && next != value).then_some(next)
    };
    match progress.mode {
        ExerciseMode::BounceLeft => Some(Step::Mode(ExerciseMode::BounceRight)),
        ExerciseMode::BounceRight => Some(Step::Mode(ExerciseMode::BounceLeft)),
        ExerciseMode::Fingers => {
            let i = FINGERS.iter().position(|f| *f == current.finger).unwrap_or(0);
            Some(Step::Finger(FINGERS[(i as isize + delta).rem_euclid(FINGERS.len() as isize) as usize]))
        }
        ExerciseMode::Basic => within(current.basic_lesson, delta, 1..=N_BASIC_LESSONS, current.basic_unlocked).map(Step::Basic),
        ExerciseMode::Layers => {
            let n = crate::course::layer_lessons(layout).len().max(1);
            within(current.layer_lesson, delta, 1..=n, current.layer_unlocked).map(Step::Layer)
        }
        ExerciseMode::Penmanship => {
            let pen = &progress.penmanship;
            within(pen.lesson, delta, 0..=crate::penmanship::n_lessons() - 1, pen.unlocked).map(Step::PenLesson)
        }
        ExerciseMode::Custom => {
            let lines = crate::pasted::lines(&progress.pasted_text, |c| layout.locate(c).is_some());
            let n = crate::pasted::parts(&lines);
            within(progress.pasted_part + 1, delta, 1..=n, n).map(|part| Step::PastedPart(part - 1))
        }
        ExerciseMode::Hanzi if !progress.hanzi.review => {
            let level = progress.hanzi.level.min(data.levels.len() - 1);
            let book = data.levels_of_hsk(data.levels[level].hsk);
            within(level, delta, book.start..=book.end - 1, hanzi_unlocked(progress)).map(Step::HanziLevel)
        }
        _ => None,
    }
}

fn change_lesson(progress: &mut Progress, layout: &crate::layout::Layout, data: &crate::hanzi::data::HanziData, delta: isize) -> bool {
    let Some(step) = step(progress, layout, data, delta) else {
        return false;
    };
    match step {
        Step::Basic(lesson) => progress.current().basic_lesson = lesson,
        Step::Layer(lesson) => progress.current().layer_lesson = lesson,
        Step::Finger(finger) => progress.current().finger = finger,
        Step::Mode(mode) => progress.mode = mode,
        Step::HanziLevel(level) => {
            progress.hanzi.level = level;
            progress.hanzi.word = 0;
        }
        Step::PastedPart(part) => progress.pasted_part = part,
        Step::PenLesson(lesson) => progress.penmanship.lesson = lesson,
    }
    progress.save();
    true
}

/// ‹ and › grayed out when they lead nowhere.
#[allow(clippy::type_complexity)]
fn enable_arrows(
    progress: Res<Progress>,
    layout: Res<ActiveLayout>,
    data: Res<crate::hanzi::data::HanziData>,
    buttons: Query<(Entity, Has<PreviousButton>, Has<bevy::ui::InteractionDisabled>), Or<(With<PreviousButton>, With<NextButton>)>>,
    mut commands: Commands,
) {
    if !progress.is_changed() && !layout.is_changed() {
        return;
    }
    for (entity, previous, disabled) in &buttons {
        let enabled = step(&progress, &layout.0, &data, if previous { -1 } else { 1 }).is_some();
        if enabled && disabled {
            commands.entity(entity).remove::<bevy::ui::InteractionDisabled>();
        } else if !enabled && !disabled {
            commands.entity(entity).insert(bevy::ui::InteractionDisabled);
        }
    }
}

fn previous_lesson(
    _: On<Activate>,
    mut progress: ResMut<Progress>,
    layout: Res<ActiveLayout>,
    data: Res<crate::hanzi::data::HanziData>,
    mut new_exercise: MessageWriter<NewExercise>,
    mut focus: ResMut<InputFocus>,
) {
    if change_lesson(&mut progress, &layout.0, &data, -1) {
        new_exercise.write_default();
    }
    release_focus(&mut focus);
}

fn next_lesson(
    _: On<Activate>,
    mut progress: ResMut<Progress>,
    layout: Res<ActiveLayout>,
    data: Res<crate::hanzi::data::HanziData>,
    mut new_exercise: MessageWriter<NewExercise>,
    mut focus: ResMut<InputFocus>,
) {
    if change_lesson(&mut progress, &layout.0, &data, 1) {
        new_exercise.write_default();
    }
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

fn step_writes(progress: &mut Progress, delta: isize) {
    let i = WRITES_STEPS.iter().position(|w| *w >= progress.hanzi.writes).unwrap_or(0) as isize;
    progress.hanzi.writes = WRITES_STEPS[(i + delta).clamp(0, WRITES_STEPS.len() as isize - 1) as usize];
    progress.save();
}

fn writes_less(_: On<Activate>, mut progress: ResMut<Progress>, mut focus: ResMut<InputFocus>) {
    step_writes(&mut progress, -1);
    release_focus(&mut focus);
}

fn writes_more(_: On<Activate>, mut progress: ResMut<Progress>, mut focus: ResMut<InputFocus>) {
    step_writes(&mut progress, 1);
    release_focus(&mut focus);
}

fn update_writes_label(progress: Res<Progress>, mut labels: Query<&mut Text, With<WritesLabel>>) {
    if !progress.is_changed() {
        return;
    }
    let text = if progress.hanzi.writes == 0 { "auto".to_string() } else { progress.hanzi.writes.to_string() };
    for mut label in &mut labels {
        if label.0 != text {
            label.0.clone_from(&text);
        }
    }
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

/// The help on the first start; after it the app opens on the exercise of the last session.
fn first_help(mut progress: ResMut<Progress>, mut panel: ResMut<Panel>) {
    if !progress.help_seen {
        *panel = Panel::Help;
        progress.help_seen = true;
        progress.save();
    }
}

/// "Basic course, lesson 10": an exercise with its lesson, finger, level or part, as the
/// sessions record them (the lessons from 1).
fn place_name(mode: ExerciseMode, lesson: Option<usize>) -> String {
    let name = mode.name();
    match (mode, lesson) {
        (ExerciseMode::Basic | ExerciseMode::Layers, Some(l)) => format!("{name}, lesson {l}"),
        (ExerciseMode::Fingers, Some(f)) => format!("{name}, {}", finger_name(f as u8)),
        (ExerciseMode::Hanzi, Some(l)) => format!("{name}, level {l}"),
        (ExerciseMode::Penmanship, Some(l)) => format!("{name}, {}", crate::penmanship::lesson_name(l.saturating_sub(1))),
        (ExerciseMode::Custom, Some(p)) => format!("{name}, part {p}"),
        _ => name.to_string(),
    }
}

/// Where the last session stopped, the first line of the statistics: the last exercise with its
/// accuracy against its goal and how long ago, then the exercise to go on with when it is
/// another one.
pub fn last_time(progress: &Progress, now: u64) -> String {
    let mode = progress.mode;
    let current = progress.layouts.get(progress.layout.key());
    let lesson = match mode {
        ExerciseMode::Basic => Some(current.map_or(1, |l| l.basic_lesson)),
        ExerciseMode::Layers => Some(current.map_or(1, |l| l.layer_lesson)),
        ExerciseMode::Fingers => Some(current.map_or(FINGERS[0], |l| l.finger) as usize),
        ExerciseMode::Hanzi => Some(progress.hanzi.level + 1),
        ExerciseMode::Penmanship => Some(progress.penmanship.lesson + 1),
        ExerciseMode::Custom => Some(progress.pasted_part + 1),
        _ => None,
    };
    let next = place_name(mode, lesson);
    let Some(last) = progress.layouts.values().flat_map(|l| l.sessions.iter()).max_by_key(|s| s.finished_at) else {
        return format!("Last time: {next}.");
    };
    let place = place_name(last.mode, last.lesson);
    let goal = crate::game::goal(last.mode);
    let verdict = if last.accuracy >= goal { "passed".to_string() } else { format!("below the goal of {goal:.0}%") };
    let when = crate::stats::ago(last.finished_at, now);
    let mut text = format!("Last time: {place} at {:.1}%, {verdict}, {when}.", last.accuracy);
    if next != place {
        text.push_str(&format!("\nUp next: {next}."));
    }
    text
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

/// The settings open on the tab of the subject of the moment.
fn toggle_settings(_: On<Activate>, progress: Res<Progress>, mut panel: ResMut<Panel>, mut tab: ResMut<SettingsTab>, mut focus: ResMut<InputFocus>) {
    panel.toggle(Panel::Settings);
    if *panel == Panel::Settings {
        *tab = SettingsTab::of(progress.mode.subject());
    }
    release_focus(&mut focus);
}

fn pick_tab(activate: On<Activate>, buttons: Query<&TabButton>, mut tab: ResMut<SettingsTab>, mut focus: ResMut<InputFocus>) {
    if let Ok(button) = buttons.get(activate.entity) {
        *tab = button.0;
    }
    release_focus(&mut focus);
}

/// The panel of the tab picked, its button in the primary color.
fn show_settings_tab(
    tab: Res<SettingsTab>,
    mut panels: Query<(&TabPanel, &mut Node)>,
    buttons: Query<(Entity, &TabButton)>,
    mut commands: Commands,
) {
    for (panel, mut node) in &mut panels {
        let display = if panel.0 == *tab { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
    for (entity, button) in &buttons {
        commands.entity(entity).insert(if button.0 == *tab { ButtonVariant::Primary } else { ButtonVariant::Normal });
    }
}

fn on_sheet_change(
    change: On<ValueChange<Entity>>,
    options: Query<&OptionIndex>,
    mut progress: ResMut<Progress>,
    mut focus: ResMut<InputFocus>,
) {
    use crate::penmanship::letters::Style;
    if let Ok(option) = options.get(change.value)
        && let Some(style) = Style::ALL.get(option.0)
    {
        progress.penmanship.style = *style;
        progress.save();
    }
    release_focus(&mut focus);
}

fn pen_writes_less(_: On<Activate>, mut progress: ResMut<Progress>, mut focus: ResMut<InputFocus>) {
    let i = PEN_WRITES_STEPS.iter().position(|w| *w >= progress.penmanship.writes).unwrap_or(0);
    progress.penmanship.writes = PEN_WRITES_STEPS[i.saturating_sub(1)];
    progress.save();
    release_focus(&mut focus);
}

fn pen_writes_more(_: On<Activate>, mut progress: ResMut<Progress>, mut focus: ResMut<InputFocus>) {
    let i = PEN_WRITES_STEPS.iter().position(|w| *w >= progress.penmanship.writes).unwrap_or(0);
    progress.penmanship.writes = PEN_WRITES_STEPS[(i + 1).min(PEN_WRITES_STEPS.len() - 1)];
    progress.save();
    release_focus(&mut focus);
}

fn pen_rows_less(_: On<Activate>, mut progress: ResMut<Progress>, mut focus: ResMut<InputFocus>) {
    progress.penmanship.rows = progress.penmanship.rows.max(1).saturating_sub(1).max(1);
    progress.save();
    release_focus(&mut focus);
}

fn pen_rows_more(_: On<Activate>, mut progress: ResMut<Progress>, mut focus: ResMut<InputFocus>) {
    progress.penmanship.rows = (progress.penmanship.rows.max(1) + 1).min(4);
    progress.save();
    release_focus(&mut focus);
}

#[allow(clippy::type_complexity)]
fn update_pen_labels(
    progress: Res<Progress>,
    mut writes: Query<&mut Text, (With<PenWritesLabel>, Without<PenRowsLabel>)>,
    mut rows: Query<&mut Text, With<PenRowsLabel>>,
) {
    if !progress.is_changed() {
        return;
    }
    let pen = &progress.penmanship;
    let text = if pen.writes == 0 { "5".to_string() } else { pen.writes.to_string() };
    for mut label in &mut writes {
        if label.0 != text {
            label.0.clone_from(&text);
        }
    }
    let text = pen.rows.max(1).to_string();
    for mut label in &mut rows {
        if label.0 != text {
            label.0.clone_from(&text);
        }
    }
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
    let drawing = !progress.mode.is_typing() && progress.mode != ExerciseMode::Shortcuts;
    let show = |visible: bool| if visible { Display::Flex } else { Display::None };
    for (mut node, is_keyboard) in &mut typing {
        // The exercise area hides its own parts; the compact view has no keyboard.
        let display = show(!drawing && !(is_keyboard && progress.compact));
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
        node.display = if progress.compact && progress.mode.subject() == Subject::Typing { Display::Flex } else { Display::None };
    }
    // No hands in the hanzi and penmanship modes, nor in the Matrix.
    let display = if progress.show_hands && progress.mode.subject() == Subject::Typing && progress.theme != ThemeMode::Matrix {
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
type PromptOnly = (With<PromptRoot>, Without<ShortcutsRoot>, Without<SummaryPane>);
type ShortcutsOnly = (With<ShortcutsRoot>, Without<SummaryPane>);

/// The text of the typing exercises or the tasks of the shortcuts, the summary at the end.
fn show_mode_views(
    progress: Res<Progress>,
    exercise: Res<Exercise>,
    drill: Res<Drill>,
    mut prompts: Query<&mut Node, PromptOnly>,
    mut shortcuts: Query<&mut Node, ShortcutsOnly>,
    mut panes: Query<&mut Node, With<SummaryPane>>,
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
    for mut node in &mut panes {
        set(&mut node, ended);
    }
}

fn update_lesson_label(
    exercise: Res<Exercise>,
    layout: Res<ActiveLayout>,
    progress: Res<Progress>,
    hanzi: Res<crate::hanzi::data::HanziData>,
    mut labels: Query<&mut Text, With<LessonLabel>>,
) {
    if !exercise.is_changed() && !layout.is_changed() && !progress.is_changed() {
        return;
    }
    let mode = if progress.mode.is_typing() { exercise.mode } else { progress.mode };
    let text = match mode {
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
        ExerciseMode::Custom if progress.pasted_text.trim().is_empty() => "Copy a text, then Paste text".into(),
        ExerciseMode::Custom => {
            let lines = crate::pasted::lines(&progress.pasted_text, |c| layout.0.locate(c).is_some());
            format!("Your text: part {}/{}", exercise.lesson, crate::pasted::parts(&lines))
        }
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
        ExerciseMode::Penmanship => {
            let lesson = progress.penmanship.lesson;
            format!("Lesson {}/{}: {}", lesson + 1, crate::penmanship::n_lessons(), crate::penmanship::lesson_name(lesson))
        }
        ExerciseMode::Hanzi if progress.hanzi.review => {
            let n = progress.hanzi.review_words.len();
            format!("word {}/{n}", (progress.hanzi.review_word + 1).min(n))
        }
        ExerciseMode::Hanzi => {
            let book = hanzi.levels.get(progress.hanzi.level).map(|l| (l.part, l.parts));
            book.map_or(String::new(), |(part, parts)| format!("Level {part}/{parts}"))
        }
    };
    for mut label in &mut labels {
        if label.0 != text {
            label.0.clone_from(&text);
        }
    }
}

/// The toolbar over the top of the window when it hides: with the setting or in the full
/// screen, it shows while the pointer is near the top or on it, a panel is open, or a moment
/// after a touch at the top.
#[allow(clippy::too_many_arguments)]
fn show_toolbar(
    progress: Res<Progress>,
    panel: Res<Panel>,
    window: Single<&Window>,
    scale: Res<UiScale>,
    touches: Res<Touches>,
    time: Res<Time<Real>>,
    hovers: Res<bevy::picking::hover::HoverMap>,
    parents: Query<&ChildOf>,
    mut toolbars: Query<(Entity, &mut Node, &ComputedNode), With<Toolbar>>,
    mut shown_until: Local<f64>,
) {
    let full_screen = !matches!(window.mode, bevy::window::WindowMode::Windowed);
    let hides = progress.auto_hide_toolbar || full_screen;
    let now = time.elapsed_secs_f64();
    for (toolbar, mut node, computed) in &mut toolbars {
        let position = if hides { PositionType::Absolute } else { PositionType::Relative };
        if node.position_type != position {
            node.position_type = position;
        }
        // The height of the toolbar in the pixels of the window, a strip of 12 when hidden.
        let height = (computed.size().y * computed.inverse_scale_factor * scale.0).max(12.0);
        let near_top = window.cursor_position().is_some_and(|p| p.y <= height + 8.0);
        let touched = touches.iter_just_pressed().any(|t| t.position().y <= 48.0 * scale.0);
        // On the toolbar or its open menus, they hang below it.
        let on_it = hovers.values().flat_map(|hovered| hovered.keys()).any(|e| {
            std::iter::successors(Some(*e), |e| parents.get(*e).ok().map(ChildOf::parent)).any(|e| e == toolbar)
        });
        if near_top || on_it || *panel != Panel::None {
            *shown_until = now + 0.6;
        }
        if touched {
            *shown_until = now + 4.0;
        }
        let display = if !hides || now < *shown_until { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
}

/// F11: the full screen, the toolbar hides.
fn full_screen(keys: Res<ButtonInput<KeyCode>>, mut window: Single<&mut Window>) {
    if keys.just_pressed(KeyCode::F11) {
        window.mode = match window.mode {
            bevy::window::WindowMode::Windowed => {
                bevy::window::WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Current)
            }
            _ => bevy::window::WindowMode::Windowed,
        };
    }
}

pub fn capitalize(s: &str) -> String {
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

#[allow(clippy::type_complexity)]
fn update_message(
    exercise: Res<Exercise>,
    drill: Res<Drill>,
    progress: Res<Progress>,
    mut texts: ParamSet<(
        Query<(Entity, &mut Text), With<SummaryTitle>>,
        Query<(Entity, &mut Text, &mut Node), With<AccuracyText>>,
        Query<(Entity, &mut Text, &mut Node), With<VerdictText>>,
        Query<(&mut Text, &mut Node), With<RecordText>>,
        Query<(&mut Text, &mut Node), With<MessageText>>,
    )>,
    mut commands: Commands,
) {
    if !exercise.is_changed() && !drill.is_changed() && !progress.is_changed() {
        return;
    }
    let summary = if progress.mode.is_typing() {
        summary(&exercise)
    } else {
        Summary { title: "Round done".into(), tone: feathers_tokens::TEXT_MAIN, accuracy: None, verdict: None, record: None, tip: drill.summary.clone() }
    };
    let set = |text: &mut Text, value: &str| {
        if text.0 != value {
            text.0 = value.to_string();
        }
    };
    let show = |node: &mut Node, visible: bool| {
        let display = if visible { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    };
    for (entity, mut text) in &mut texts.p0() {
        set(&mut text, &summary.title);
        commands.entity(entity).insert(ThemeTextColor(summary.tone.clone()));
    }
    for (entity, mut text, mut node) in &mut texts.p1() {
        let (value, token) = summary.accuracy.clone().unwrap_or_default();
        set(&mut text, &value);
        show(&mut node, summary.accuracy.is_some());
        commands.entity(entity).insert(ThemeTextColor(token));
    }
    for (entity, mut text, mut node) in &mut texts.p2() {
        let (value, token) = summary.verdict.clone().unwrap_or_default();
        set(&mut text, &value);
        show(&mut node, summary.verdict.is_some());
        commands.entity(entity).insert(ThemeTextColor(token));
    }
    for (mut text, mut node) in &mut texts.p3() {
        set(&mut text, summary.record.as_deref().unwrap_or_default());
        show(&mut node, summary.record.is_some());
    }
    for (mut text, mut node) in &mut texts.p4() {
        set(&mut text, &summary.tip);
        show(&mut node, !summary.tip.is_empty());
    }
}

/// The summary at the end of an exercise.
#[derive(Debug, Clone, PartialEq)]
struct Summary {
    /// "Lesson 7 complete", "Left pinky drill: below the goal".
    title: String,
    /// The color of the title: green passed, amber below the goal.
    tone: bevy::feathers::theme::ThemeToken,
    /// "Accuracy 97.3%, the goal is 95%", green or red.
    accuracy: Option<(String, bevy::feathers::theme::ThemeToken)>,
    /// Where Enter goes: on to the next lesson in green, the same one again in amber.
    verdict: Option<(String, bevy::feathers::theme::ThemeToken)>,
    record: Option<String>,
    /// Klavaro's comment as a tip, and the reason of the suggested drill.
    tip: String,
}

impl Default for Summary {
    fn default() -> Self {
        Summary { title: String::new(), tone: feathers_tokens::TEXT_MAIN, accuracy: None, verdict: None, record: None, tip: String::new() }
    }
}

/// The summary of a typing exercise, empty before its end.
fn summary(exercise: &Exercise) -> Summary {
    let Some(results) = exercise.tutor.results.filter(|_| exercise.tutor.query == TutorQuery::End) else {
        return Summary::default();
    };
    let mode = exercise.mode;
    let goal = crate::game::goal(mode);
    let passed = results.accuracy >= goal;
    let what = match mode {
        ExerciseMode::Basic => format!("Lesson {}", exercise.lesson),
        ExerciseMode::Layers => format!("Layer lesson {}", exercise.lesson),
        ExerciseMode::Fingers => format!("{} drill", capitalize(finger_name(exercise.finger))),
        ExerciseMode::Review => "Warm-up".into(),
        _ => "Exercise".into(),
    };
    let title = if passed { format!("{what} complete") } else { format!("{what}: below the goal") };
    let accuracy = if results.n_errors == 0 {
        ("100%, not a single error!".to_string(), tokens::CHAR_CORRECT)
    } else if passed {
        (format!("Accuracy {:.1}%, the goal is {goal:.0}%", results.accuracy), tokens::CHAR_CORRECT)
    } else {
        (format!("Accuracy {:.1}%, the goal is {goal:.0}%", results.accuracy), tokens::CHAR_WRONG)
    };
    let verdict = if passed {
        match mode {
            ExerciseMode::Basic if exercise.lesson < N_BASIC_LESSONS => format!("Enter goes on to lesson {}", exercise.lesson + 1),
            ExerciseMode::Basic => "The basic course is done: Enter for one more text, the menu for the next exercise".into(),
            ExerciseMode::Layers => "Enter goes on to the next layer lesson".into(),
            ExerciseMode::Fingers => match crate::drills::next_finger(exercise.finger) {
                Some(next) => format!("Enter goes on to the {} drill", finger_name(next)),
                None => "Every finger has had its drill: Enter for another one".into(),
            },
            ExerciseMode::Review => "Enter goes on with the session".into(),
            ExerciseMode::Custom => "Enter goes on to the next part".into(),
            _ => "Enter starts a new text".into(),
        }
    } else {
        match mode {
            ExerciseMode::Basic | ExerciseMode::Layers => "Enter repeats the lesson".into(),
            ExerciseMode::Fingers => "Enter repeats the drill".into(),
            ExerciseMode::Review => "Enter goes on with the session".into(),
            _ => "Enter: another text, the same exercise".into(),
        }
    };
    let tone = if passed { tokens::CHAR_CORRECT } else { tokens::NEAR_MISS };
    // The numbers are in the bar at the top, the keys in the verdict: the advice only.
    let mut lines: Vec<String> = exercise
        .message
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.contains("Press Enter") && !l.contains("Backspace") && *l != ":-(")
        .map(str::to_string)
        .collect();
    if let Some(first) = lines.first_mut() {
        *first = format!("Tip: {first}");
    }
    if let Some((_, reason)) = &exercise.suggestion {
        lines.push(capitalize(reason));
    }
    Summary { title, tone: tone.clone(), accuracy: Some(accuracy), verdict: Some((verdict, tone)), record: exercise.record.clone(), tip: lines.join("\n") }
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

/// The keys of the moment, at the bottom. The key to go on with once an exercise is done comes
/// after a pause only: whoever knows it has pressed it by then, the help lists them all.
#[allow(clippy::too_many_arguments)]
fn update_actions(
    exercise: Res<Exercise>,
    progress: Res<Progress>,
    drill: Res<Drill>,
    hanzi: Res<crate::hanzi::Practice>,
    pen: Res<crate::penmanship::Practice>,
    last: Res<crate::game::LastInput>,
    time: Res<Time<Real>>,
    mut labels: Query<&mut Text, With<ActionsText>>,
) {
    let rested = last.idle(&time) >= REST_SECONDS;
    let ended = exercise.tutor.query == TutorQuery::End;
    let mut actions: Vec<String> = Vec::new();
    match progress.mode {
        ExerciseMode::Hanzi => {
            if hanzi.level_done && rested {
                actions.push("Enter next level".into());
            }
        }
        ExerciseMode::Penmanship => {
            if pen.is_done() && rested {
                actions.push("Enter next lesson".into());
            }
        }
        ExerciseMode::Shortcuts => {
            if drill.finished.is_some() {
                if rested {
                    actions.push("Enter another round".into());
                }
            } else {
                actions.push("Esc, the arrows and the shortcuts are the drill".into());
            }
        }
        _ if ended => {
            if rested {
                let passed = exercise.tutor.results.is_some_and(|r| r.accuracy >= crate::game::goal(exercise.mode));
                let next_finger = (exercise.mode == ExerciseMode::Fingers && passed).then(|| crate::drills::next_finger(exercise.finger)).flatten();
                actions.push(match next_finger {
                    Some(finger) => format!("Enter {} drill", finger_name(finger)),
                    None => "Enter next exercise".into(),
                });
                if exercise.mode == ExerciseMode::Basic {
                    actions.push("Backspace previous lesson".into());
                }
                if let Some((finger, _)) = &exercise.suggestion {
                    actions.push(format!("Tab {} drill", finger_name(*finger)));
                }
            }
            actions.push("← → lessons".into());
        }
        _ if exercise.tutor.query == TutorQuery::Start => {
            actions.push("← → lessons".into());
            actions.push("Ctrl+Backspace clear the errors".into());
        }
        _ => {
            actions.push("‹ › lessons".into());
            actions.push("Ctrl+Backspace clear the errors".into());
        }
    }
    let text = actions.join("   ·   ");
    for mut label in &mut labels {
        if label.0 != text {
            label.0.clone_from(&text);
        }
    }
}

/// ← and → change the lesson, the finger, the hand or the hanzi level, not in the shortcuts:
/// the arrows are part of the drill. Not while an exercise is being typed either: an arrow hit
/// by mistake (they sit under the letters on the small keyboards) would throw the exercise
/// away, only the ‹ › buttons change the lesson then.
fn arrow_lessons(
    mut keys: MessageReader<crate::game::KeyPressed>,
    mut progress: ResMut<Progress>,
    exercise: Res<Exercise>,
    layout: Res<ActiveLayout>,
    data: Res<crate::hanzi::data::HanziData>,
    panel: Res<Panel>,
    mut new_exercise: MessageWriter<NewExercise>,
) {
    use crate::layout::{Chord, Special};
    for key in keys.read() {
        if progress.mode == ExerciseMode::Shortcuts || *panel != Panel::None {
            continue;
        }
        if progress.mode.is_typing() && exercise.tutor.query == TutorQuery::ProcessTouches {
            continue;
        }
        let delta = match key.chord {
            Some(c) if c == Chord::special(Special::Left) => -1,
            Some(c) if c == Chord::special(Special::Right) => 1,
            _ => continue,
        };
        if change_lesson(&mut progress, &layout.0, &data, delta) {
            new_exercise.write_default();
        }
    }
}

/// The words of the hanzi statistics: the weak ones, the most missed, the best written.
fn hanzi_rankings(progress: &Progress, data: &crate::hanzi::data::HanziData) -> String {
    let words = &progress.hanzi.words;
    if words.is_empty() {
        return "No word written yet.".into();
    }
    let weak: Vec<String> = crate::hanzi::weak_words(progress, data)
        .iter()
        .map(|w| format!("{w} ×{}", words.get(w).map_or(0, |s| s.weak)))
        .collect();
    let mut missed: Vec<(&String, u32)> = words.iter().filter(|(_, s)| s.misses > 0).map(|(w, s)| (w, s.misses)).collect();
    missed.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    let missed: Vec<String> = missed.iter().take(10).map(|(w, n)| format!("{w} ×{n}")).collect();
    let mut best: Vec<(&String, f64)> = words.iter().filter_map(|(w, s)| s.quality().map(|q| (w, q))).collect();
    best.sort_by(|a, b| b.1.total_cmp(&a.1));
    let best: Vec<String> = best.iter().take(10).map(|(w, q)| format!("{w} {q:.0}%")).collect();
    let (sum, n) = words.values().fold((0.0, 0), |(sum, n), s| (sum + s.quality_sum, n + s.quality_n));
    let glimpses: u32 = words.values().map(|s| s.glimpses).sum();
    let or_none = |v: Vec<String>, none: &str| if v.is_empty() { none.to_string() } else { v.join("   ") };
    format!(
        "Weak words, a stroke was shown: {} (review them in the second menu)\nMost missed: {}\nBest written: {}\n{} words written, {} levels done · the strokes {} accurate · {glimpses} strokes shown",
        or_none(weak, "none"),
        or_none(missed, "none"),
        or_none(best, "not yet"),
        words.len(),
        progress.hanzi.levels_done.len(),
        if n > 0 { format!("{:.0}%", sum / n as f64) } else { "not yet".into() },
    )
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
    hanzi: Res<crate::hanzi::data::HanziData>,
    mut labels: Query<&mut Text, With<RankingText>>,
) {
    if progress.mode.subject() == Subject::Hanzi {
        let text = hanzi_rankings(&progress, &hanzi);
        for mut label in &mut labels {
            label.0.clone_from(&text);
        }
        return;
    }
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::Session;

    fn session(mode: ExerciseMode, lesson: Option<usize>, accuracy: f64, finished_at: u64) -> Session {
        Session { elapsed: 60.0, mode, lesson, accuracy, velocity: 20.0, fluidness: 80.0, finished_at, quality: None }
    }

    #[test]
    fn the_last_time_tells_the_last_exercise_and_the_next_one() {
        let now = 1_793_000_000;
        let mut progress = Progress { mode: ExerciseMode::Basic, ..Progress::default() };
        assert_eq!(last_time(&progress, now), "Last time: Basic course, lesson 1.");
        // Lesson 10 failed: the same lesson again.
        progress.current().basic_lesson = 10;
        progress.current().sessions.push(session(ExerciseMode::Basic, Some(10), 94.4, now - 2 * 3600));
        assert_eq!(
            last_time(&progress, now),
            "Last time: Basic course, lesson 10 at 94.4%, below the goal of 95%, 2 hours ago."
        );
        // Then passed: the next lesson is up.
        progress.current().basic_lesson = 11;
        progress.current().sessions.push(session(ExerciseMode::Basic, Some(10), 96.3, now - 20 * 60));
        assert_eq!(
            last_time(&progress, now),
            "Last time: Basic course, lesson 10 at 96.3%, passed, 20 minutes ago.\nUp next: Basic course, lesson 11."
        );
        // A finger drill passed, the next finger up.
        progress.mode = ExerciseMode::Fingers;
        progress.current().finger = 9;
        progress.current().sessions.push(session(ExerciseMode::Fingers, Some(8), 97.5, now - 5 * 60));
        assert_eq!(
            last_time(&progress, now),
            "Last time: Finger drill, right ring finger at 97.5%, passed, 5 minutes ago.\nUp next: Finger drill, right pinky."
        );
    }
}
