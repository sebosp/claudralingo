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

/// The keys of the exercise beside its label, as on the keyboard, in the colors of their fingers.
#[derive(Component, Default, Clone)]
struct LessonKeys;

/// The grade at the end of an exercise: the stars and its name.
#[derive(Component, Default, Clone)]
struct GradeRow;
#[derive(Component, Default, Clone)]
struct GradeStar(usize);
#[derive(Component, Default, Clone)]
struct GradeName;

#[derive(Component, Default, Clone)]
struct LiveStats;

#[derive(Component, Default, Clone)]
struct ProgressBar;

#[derive(Component, Default, Clone)]
pub struct MessageText;

#[derive(Component, Default, Clone)]
pub struct RankingText;

/// The time of the exercise, at the right of the window.
#[derive(Component, Default, Clone)]
struct Timer;
#[derive(Component, Default, Clone)]
struct TimerText;

/// The accuracy at the end of a typing exercise, a line under the progress bar with the mark of
/// the goal: the line, its fill and the mark.
#[derive(Component, Default, Clone)]
struct AccuracyLine;
#[derive(Component, Default, Clone)]
struct AccuracyFill;
#[derive(Component, Default, Clone)]
struct AccuracyMark;

/// The summary at the end of an exercise: the pane, its header with the title (none for an
/// exercise failed, the rest of the screen tells), the stars and the grade, the record and the
/// tip. Any key goes on, see [`crate::game::continues`].
#[derive(Component, Default, Clone)]
pub struct SummaryPane;
#[derive(Component, Default, Clone)]
struct SummaryHeader;
#[derive(Component, Default, Clone)]
struct SummaryTitle;
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

/// The word under a button when the pointer rests on it, see [`show_tooltips`].
#[derive(Component, Default, Clone)]
pub struct Tooltip(pub &'static str);
#[derive(Component, Default, Clone)]
struct TooltipBox;
#[derive(Component, Default, Clone)]
struct TooltipText;

/// The pointer rests this long on a button before its tooltip shows.
const TOOLTIP_DELAY: f64 = 0.4;

/// The arrow beside the real words of a basic lesson: they are not needed to go on.
#[derive(Component, Default, Clone)]
struct SkipWordsButton;

/// The frosted glass over a paused exercise, Enter goes on.
#[derive(Component, Default, Clone)]
struct PauseOverlay;

/// The line of the suggested drill: the key, its misses and the button of the drill.
#[derive(Component, Default, Clone)]
struct SuggestRow;
#[derive(Component, Default, Clone)]
struct SuggestKey;
#[derive(Component, Default, Clone)]
struct SuggestText;
#[derive(Component, Default, Clone)]
struct SuggestCount;
#[derive(Component, Default, Clone)]
struct SuggestTail;
#[derive(Component, Default, Clone)]
struct SuggestFinger;

/// The switch of the key clicks, in the settings.
#[derive(Component, Default, Clone)]
struct SwitchSelect;

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
    ShowKeyboard,
    KeyClicks,
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
    DiscreteGpu,
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
/// The results at the end of a typing exercise, a column each for the words per minute, the
/// accuracy and the fluidness: its value and, under it, the change since the last time.
#[derive(Component, Default, Clone)]
struct EndStats;
#[derive(Component, Default, Clone)]
struct EndColumn(usize);
#[derive(Component, Default, Clone)]
struct EndValue(usize);
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
    /// The main menu, the three subjects, see [`crate::home`].
    Home,
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
/// A body whose lines start with `- ` is a bulleted list.
const HELP: [(&str, &str); 4] = [
    ("Hanzi", "HSK 1 through 4"),
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
        "Shortcuts",
        "\
- F1 or ?  this help
- F5  the exercise, the word or the figure again
- F11  full screen
- Esc  closes a panel
- Any key  the next exercise, once done
- Enter  the next hanzi level or penmanship lesson, once done
- Backspace  the previous basic lesson, once done
- Tab  the suggested drill
- ← →  the lesson, the hanzi level or the penmanship lesson: before the first key and at the end, the ‹ › buttons at any time
- Space  hanzi: the word said; penmanship: the stroke shown, when Hint is there
- Enter  goes on after a pause
- Ctrl and the wheel, a pinch, Ctrl+= Ctrl+- Ctrl+0  zoom",
    ),
    (
        "On-screen keyboard",
        "\
Click a key to type it, hold it past the bar under it for the capital (auto shift)
Lower and Raise: click to show and use their layer, click again to go back, or hold them
Ctrl, Alt, Win: click them, then the key: they are never sent to your system",
    ),
];

const HELP_TITLE: &str = concat!("Help - Claudralingo ", env!("CARGO_PKG_VERSION"));

const HELP_FOOTER: &str = "The progress is saved in ~/.config/claudralingo";

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
            .init_resource::<PendingText>().add_systems(Startup, spawn_ui).add_systems(
            Update,
            (
                (sync_settings, sync_selects, remember_typing_mode, enable_arrows, show_toolbar, keep_keys_off_the_menus, full_screen, show_tooltips),
                show_mode_views,
                update_lesson_label,
                (update_live_stats, show_pause, show_skip_words),
                update_message,
                update_suggestion,
                show_panels.run_if(resource_changed::<Panel>),
                fill_lessons.run_if(resource_changed::<Panel>),
                show_hands,
                show_hanzi,
                fill_help,
                fill_steps_bars,
                update_end_stats,
                update_actions,
                arrow_lessons.after(crate::game::GameSystems),
                import_keymap,
                import_text,
                update_rankings.run_if(|p: Res<Progress>, panel: Res<Panel>| p.is_changed() || panel.is_changed()),
                (
                    update_zoom_label.run_if(resource_changed::<UiScale>),
                    update_writes_label,
                    update_pen_labels,
                    update_accuracy_line,
                    show_settings_tab.run_if(|tab: Res<SettingsTab>, panel: Res<Panel>| tab.is_changed() || panel.is_changed()),
                ),
            )
                .after(crate::game::GameSystems),
        );
    }
}

fn spawn_ui(mut commands: Commands, progress: Res<Progress>) {
    commands.spawn(Camera2d);
    let layout = LayoutId::ALL.iter().position(|l| *l == progress.layout);
    let layouts = list_rows_from_strings(LayoutId::ALL.map(LayoutId::name), layout);
    let background = BackgroundLevel::ALL.iter().position(|b| *b == progress.background);
    let backgrounds = list_rows_from_strings(BackgroundLevel::ALL.map(BackgroundLevel::name), background);
    let theme = ThemeMode::ALL.iter().position(|t| *t == progress.theme);
    let themes = list_rows_from_strings(ThemeMode::ALL.map(ThemeMode::name), theme);
    use crate::penmanship::letters::Style;
    let sheet = Style::ALL.iter().position(|s| *s == progress.penmanship.style);
    let sheets = list_rows_from_strings(Style::ALL.map(Style::name), sheet);
    let switch = crate::sounds::Switch::ALL.iter().position(|s| *s == progress.switch);
    let switches = list_rows_from_strings(crate::sounds::Switch::ALL.map(crate::sounds::Switch::name), switch);
    commands.spawn_scene(root(layouts, backgrounds, themes, sheets, switches));
}

fn root(
    layouts: Box<dyn SceneList>,
    backgrounds: Box<dyn SceneList>,
    themes: Box<dyn SceneList>,
    sheets: Box<dyn SceneList>,
    switches: Box<dyn SceneList>,
) -> impl Scene {
    bsn! {
        // The root is a flex item of the viewport: without a minimum height of 0 it grows with
        // its content, zoomed in the footer went out of the window.
        // Nor of a minimum width of 0: zoomed in, the keyboard widened it past the window and the
        // buttons at the right of the toolbar went out of it.
        Node {
            width: percent(100),
            height: percent(100),
            min_height: px(0),
            min_width: px(0),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(2),
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
            @toolbar()
            --
            // Everything but the toolbar and the overlays scrolls, zoomed in.
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(4),
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
                    // The text right over the keyboard. The room of a fourth line is above it,
                    // the line just typed takes it and the keyboard does not move.
                    justify_content: JustifyContent::End,
                    row_gap: px(10),
                    min_height: px(4.0 * crate::prompt::LINE_HEIGHT),
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
                    // At the right of the words, on to the next lesson without them.
                    @FeathersButton {
                        @caption: bsn! { @crate::icons::icon(crate::icons::Icon::Forward) }
                    }
                    Node {
                        display: Display::None,
                        position_type: PositionType::Absolute,
                        right: px(-56),
                        bottom: px(crate::prompt::LINE_HEIGHT),
                    }
                    SkipWordsButton
                    Tooltip({"Next exercise"})
                    on(skip_words)
                    --
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(6),
                    }
                    ShortcutsRoot
                    --
                    @crate::history::view()
                    --
                    // The summary at the end: how it went, the tip.
                    @group()
                    Node {
                        display: Display::None,
                        width: px(640),
                        max_width: percent(96),
                    }
                    SummaryPane
                    Children [
                        @group_header()
                        Node { justify_content: JustifyContent::Center }
                        SummaryHeader
                        Children [ @label("") SummaryTitle ]
                        --
                        @group_body()
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: px(8),
                            padding: UiRect::axes(px(16), px(14)),
                        }
                        Children [
                            Node {
                                display: Display::None,
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: px(4),
                            }
                            GradeRow
                            Children [
                                @grade_star(0) --
                                @grade_star(1) --
                                @grade_star(2) --
                                @label("") GradeName
                                TextFont { font_size: FontSize::Px(24.0) }
                                Node { margin: UiRect::left(px(10)) }
                            ]
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
                            // "[S] was missed 6 times.  [Train left ring finger]"
                            Node {
                                display: Display::None,
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                column_gap: px(8),
                                row_gap: px(6),
                            }
                            SuggestRow
                            Children [
                                Node { flex_direction: FlexDirection::Row } SuggestKey
                                --
                                @label("") SuggestText
                                TextFont { font_size: FontSize::Px(16.0) }
                                --
                                // The finger underlined in its color, as the stripes of the keys. In
                                // the primary color: a gray button on the gray pane did not look
                                // like one.
                                @FeathersButton {
                                    @variant: ButtonVariant::Primary,
                                    @caption: bsn! {
                                        Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(5) }
                                        Children [
                                            @label("Train")
                                            ThemeTextColor(feathers_tokens::BUTTON_PRIMARY_TEXT)
                                            --
                                            Node { flex_direction: FlexDirection::Column, row_gap: px(1) }
                                            Children [
                                                @label("") SuggestCaption
                                                ThemeTextColor(feathers_tokens::BUTTON_PRIMARY_TEXT)
                                                --
                                                Node { height: px(2), border_radius: BorderRadius::all(px(1)) }
                                                BackgroundColor(Color::NONE)
                                                SuggestFinger
                                            ]
                                        ]
                                    }
                                }
                                SuggestButton
                                on(train_suggested_finger)
                            ]
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
                    min_height: px(18),
                }
                --
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Start,
                    column_gap: px(48),
                    padding: UiRect { left: px(8), right: px(8), top: px(0), bottom: px(8) },
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
            // Over the text and the keyboard of a paused exercise, under the toolbar.
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: px(18),
            }
            BackgroundColor(Color::NONE)
            GlobalZIndex(30)
            PauseOverlay
            Children [
                @crate::icons::icon(crate::icons::Icon::Pause)
                Node { width: px(56), height: px(56) }
                --
                Node { flex_direction: FlexDirection::Row } PauseKey
            ]
            --
            @crate::home::overlay()
            --
            @help_overlay()
            --
            @settings_overlay(layouts, backgrounds, themes, sheets, switches)
            --
            @lessons_overlay()
            --
            @crate::print::overlay()
            --
            @crate::stats::overlay()
            --
            @crate::hanzi::breakdown::overlay()
            --
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                padding: UiRect::axes(px(8), px(4)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
            }
            ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
            ThemeBorderColor(feathers_tokens::GROUP_BORDER)
            GlobalZIndex(80)
            Pickable::IGNORE
            TooltipBox
            Children [ @label("") TooltipText Pickable::IGNORE ]
        ]
    }
}

fn toolbar() -> impl Scene {
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
            @FeathersButton {
                @caption: bsn! { @crate::icons::icon(crate::icons::Icon::Home) }
            }
            Tooltip({"Main menu"})
            on(go_home)
            --
            @flex_spacer()
            --
            // The lesson in the middle, on one line: its name and its keys between ‹ and ›. A
            // narrow window moves it to a row of its own rather than breaking it.
            Node {
                flex_direction: FlexDirection::Row,
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(8),
            }
            Children [
                @FeathersButton {
                    @caption: bsn! { @caption("‹") }
                }
                PreviousButton
                Tooltip({"Previous"})
                on(previous_lesson)
                --
                @label_dim("") LessonLabel
                Node { flex_shrink: 0.0 }
                --
                Node {
                    flex_direction: FlexDirection::Row,
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    column_gap: px(3),
                }
                LessonKeys
                --
                @FeathersButton {
                    @caption: bsn! { @caption("›") }
                }
                NextButton
                Tooltip({"Next"})
                on(next_lesson)
            ]
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
            Tooltip({"Refresh"})
            on(restart)
            --
            @flex_spacer()
            --
            @FeathersButton {
                @caption: bsn! { @crate::icons::icon(crate::icons::Icon::Lessons) }
            }
            Tooltip({"Lessons"})
            on(toggle_lessons)
            --
            @FeathersButton {
                @caption: bsn! { @crate::icons::icon(crate::icons::Icon::Stats) }
            }
            Tooltip({"Stats"})
            on(toggle_stats)
            --
            @FeathersButton {
                @caption: bsn! { @crate::icons::icon(crate::icons::Icon::Settings) }
            }
            Tooltip({"Settings"})
            on(toggle_settings)
            --
            @FeathersButton {
                @caption: bsn! { @caption("?") }
            }
            Tooltip({"Help"})
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

fn settings_overlay(
    layouts: Box<dyn SceneList>,
    backgrounds: Box<dyn SceneList>,
    themes: Box<dyn SceneList>,
    sheets: Box<dyn SceneList>,
    switches: Box<dyn SceneList>,
) -> impl Scene {
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
                        @setting_checkbox(Setting::LockLessons, "Lock lessons: pass a lesson to open the next one")
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
                        --
                        // Chosen in main.rs, the browsers and the phones pick their own.
                        Node {
                            display: {if cfg!(any(target_arch = "wasm32", target_os = "android", target_os = "ios")) { Display::None } else { Display::Flex }},
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(8),
                        }
                        Children [
                            @setting_checkbox(Setting::DiscreteGpu, "Use the powerful graphics chip: the battery runs out sooner")
                            --
                            Node {
                                width: px(20),
                                height: px(20),
                                flex_shrink: 0.0,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border_radius: BorderRadius::MAX,
                            }
                            ThemeBackgroundColor(feathers_tokens::BUTTON_BG)
                            GpuHelpButton
                            on(show_gpu_help)
                            on(hide_gpu_help)
                            Children [
                                @label("?") Pickable::IGNORE
                            ]
                        ]
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
                        @setting_checkbox(Setting::ShowKeyboard, "Show the keyboard")
                        --
                        @setting_checkbox(Setting::ShowHands, "Show the fingers under the keyboard")
                        --
                        @setting_checkbox(Setting::Compact, "Compact, for phones: no keyboard, the finger tips and the layer to hold")
                        --
                        @setting_checkbox(Setting::LargeText, "Large text to type")
                        --
                        @setting_checkbox(Setting::TypingSounds, "Typing sounds: a bell on a miss, a chime on a clean line, a falling tone for a key held too long")
                        --
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(12),
                        }
                        Children [
                            @setting_checkbox(Setting::KeyClicks, "Key clicks, in the sound of a switch")
                            --
                            @FeathersSelect {
                                @options: switches,
                            }
                            Node {
                                min_width: px(150),
                            }
                            SwitchSelect
                            on(on_switch_change)
                        ]
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
                ]
            ]
            --
            // Over the question mark of the graphics chip, out of the panel that scrolls.
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                width: px(GPU_HELP_WIDTH),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::axes(px(10), px(8)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
            }
            ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
            ThemeBorderColor(feathers_tokens::GROUP_BORDER)
            GlobalZIndex(60)
            Pickable::IGNORE
            GpuHelp
            Children [
                @label("Only matters on computers with two graphics chips, such as many gaming laptops.") Pickable::IGNORE
                --
                @label("Off, the best for most people: uses less power and the battery lasts longer. Claudralingo does not need more.") Pickable::IGNORE
                --
                @label("On: uses more power and the battery runs out sooner. Try it only if the animations stutter.") Pickable::IGNORE
                --
                @label_dim("A change applies the next time you start Claudralingo.") Pickable::IGNORE
            ]
        ]
    }
}

/// The question mark beside the graphics chip setting, and the explanation it shows.
#[derive(Component, Default, Clone)]
struct GpuHelpButton;
#[derive(Component, Default, Clone)]
struct GpuHelp;

const GPU_HELP_WIDTH: f32 = 420.0;

/// Above the question mark, the setting is at the bottom of the panel, inside the window.
fn show_gpu_help(
    _: On<PointerOver>,
    buttons: Query<(&ComputedNode, &UiGlobalTransform), With<GpuHelpButton>>,
    overlays: Query<&ComputedNode, With<SettingsOverlay>>,
    mut helps: Query<&mut Node, With<GpuHelp>>,
) {
    let (Ok((button, transform)), Ok(overlay)) = (buttons.single(), overlays.single()) else {
        return;
    };
    let scale = button.inverse_scale_factor;
    let (size, center, window) = (button.size() * scale, transform.translation * scale, overlay.size() * scale);
    for mut help in &mut helps {
        help.display = Display::Flex;
        help.left = px((center.x - GPU_HELP_WIDTH / 2.0).min(window.x - GPU_HELP_WIDTH - 8.0).max(8.0));
        help.bottom = px(window.y - center.y + size.y / 2.0 + 6.0);
    }
}

fn hide_gpu_help(_: On<PointerOut>, mut helps: Query<&mut Node, With<GpuHelp>>) {
    for mut help in &mut helps {
        help.display = Display::None;
    }
}

/// A star of the grade at the end of an exercise.
fn grade_star(i: usize) -> impl Scene {
    bsn! {
        @crate::icons::icon(crate::icons::Icon::StarEmpty)
        Node { width: px(32), height: px(32) }
        GradeStar({i})
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
            move |_: On<PointerClick>,
                  mut progress: ResMut<Progress>,
                  mut panel: ResMut<Panel>,
                  data: Res<crate::hanzi::data::HanziData>,
                  mut practice: ResMut<crate::hanzi::Practice>,
                  mut new_exercise: MessageWriter<NewExercise>| {
                if locked {
                    return;
                }
                if go == crate::lessons::Go::HanziReview && !crate::hanzi::start_review(&mut progress, &data) {
                    practice.message =
                        "No weak words yet: a word becomes weak when one of its strokes is shown, after three misses or with Hint.".into();
                    progress.mode = ExerciseMode::Hanzi;
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
                @group_header() Children [ @label(HELP_TITLE) ]
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
            if body.starts_with("- ") {
                for item in body.lines().filter_map(|line| line.strip_prefix("- ")) {
                    // The bullet in a column of its own: a long item wraps under its text.
                    let row = commands
                        .spawn((
                            Node { margin: UiRect::left(px(12)), column_gap: px(8), ..default() },
                            ChildOf(root),
                        ))
                        .id();
                    for text in ["\u{2022}", item] {
                        commands.spawn((
                            Text::new(text),
                            font(false, 14.0),
                            ThemeTextColor(feathers_tokens::TEXT_MAIN),
                            ChildOf(row),
                        ));
                    }
                }
            } else {
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
}

/// The live statistics and the progress of the exercise, on a panel of their own: the
/// background does not run under them.
fn stats_bar() -> impl Scene {
    bsn! {
        // The whole width of the window, for the time at its right.
        Node {
            display: Display::Flex,
            width: percent(100),
            justify_content: JustifyContent::Center,
        }
        TypingOnly
        Children [
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(6),
                width: px(640),
                max_width: percent(100),
                padding: UiRect::axes(px(16), px(4)),
                border_radius: BorderRadius::all(px(10)),
            }
            ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
            Children [
                @label("") LiveStats
                --
                Node {
                    display: Display::None,
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                }
                EndStats
                Children [
                    @end_column(0) --
                    @end_column(1) --
                    @end_column(2)
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
                --
                // The accuracy once it is over, the mark taller than the line.
                Node {
                    display: Display::None,
                    width: percent(100),
                    height: px(14),
                    align_items: AlignItems::Center,
                }
                AccuracyLine
                Children [
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
                        ThemeBackgroundColor(tokens::CHAR_CORRECT)
                        AccuracyFill
                    ]
                    --
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(2),
                        height: percent(100),
                        margin: UiRect::left(px(-1)),
                    }
                    ThemeBackgroundColor(feathers_tokens::TEXT_MAIN)
                    AccuracyMark
                ]
            ]
            --
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                right: px(12),
                top: px(0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                padding: UiRect::axes(px(10), px(4)),
                border_radius: BorderRadius::all(px(8)),
            }
            ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
            Timer
            Children [
                @crate::icons::icon(crate::icons::Icon::Stopwatch)
                --
                @label("") TimerText
            ]
        ]
    }
}

/// A column of the results, the same width as the others.
fn end_column(i: usize) -> impl Scene {
    bsn! {
        Node {
            flex_basis: px(0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(2),
        }
        EndColumn({i})
        Children [
            @label("") EndValue({i})
            --
            Text("") TrendText({i})
            Node { display: Display::None }
        ]
    }
}

/// The accuracy in whole percents, rounded down: 94.8% shows 94%, never the 95% of a pass it
/// is not, nor 100% for a single error.
fn accuracy_shown(accuracy: f64) -> u32 {
    accuracy.max(0.0).floor() as u32
}

/// The time on the stopwatch, minutes and seconds.
fn clock(seconds: f64) -> String {
    let seconds = seconds.max(0.0) as u64;
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

/// How far the fill and the mark of the goal go on the accuracy line, in percents: it starts 20
/// points under the goal, on a line from 0 a miss by little would not show.
fn accuracy_line(accuracy: f64, goal: f64) -> (f32, f32) {
    let start = (goal - 20.0).max(0.0);
    let at = |value: f64| (100.0 * (value - start) / (100.0 - start)).clamp(0.0, 100.0) as f32;
    (at(accuracy), at(goal))
}

/// The accuracy line of a typing exercise that is over, green from the goal on, red under it.
#[allow(clippy::type_complexity)]
fn update_accuracy_line(
    exercise: Res<Exercise>,
    progress: Res<Progress>,
    mut lines: Query<&mut Node, (With<AccuracyLine>, Without<AccuracyFill>, Without<AccuracyMark>)>,
    mut fills: Query<(Entity, &mut Node), (With<AccuracyFill>, Without<AccuracyMark>)>,
    mut marks: Query<&mut Node, With<AccuracyMark>>,
    mut commands: Commands,
) {
    if !exercise.is_changed() && !progress.is_changed() {
        return;
    }
    let results = exercise.tutor.results.filter(|_| exercise.tutor.query == TutorQuery::End && progress.mode.is_typing());
    for mut node in &mut lines {
        node.display = if results.is_some() { Display::Flex } else { Display::None };
    }
    let Some(results) = results else {
        return;
    };
    let goal = crate::game::goal(exercise.mode);
    let (fill, mark) = accuracy_line(results.accuracy, goal);
    for (entity, mut node) in &mut fills {
        node.width = percent(fill);
        let token = if results.accuracy >= goal { tokens::CHAR_CORRECT } else { tokens::CHAR_WRONG };
        commands.entity(entity).insert(ThemeBackgroundColor(token));
    }
    for mut node in &mut marks {
        node.left = percent(mark);
    }
}


impl Setting {
    fn get(self, progress: &Progress) -> bool {
        match self {
            Setting::FadeLegends => progress.fade_legends,
            Setting::LayerTrainer => progress.layer_trainer,
            Setting::ShowHands => progress.show_hands,
            Setting::ShowKeyboard => progress.show_keyboard,
            Setting::KeyClicks => progress.key_clicks,
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
            Setting::DiscreteGpu => progress.discrete_gpu,
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
        Setting::ShowKeyboard => progress.show_keyboard = change.value,
        Setting::KeyClicks => progress.key_clicks = change.value,
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
        Setting::DiscreteGpu => progress.discrete_gpu = change.value,
    }
    progress.save();
    release_focus(&mut focus);
}

#[allow(clippy::type_complexity)]
/// The selects of the settings follow the progress: the keyboard also changes with an import,
/// the switch with the settings of another window.
fn sync_selects(
    progress: Res<Progress>,
    selects: Query<(Entity, Has<LayoutSelect>), Or<(With<LayoutSelect>, With<SwitchSelect>)>>,
    children: Query<&Children>,
    rows: Query<(&OptionIndex, Has<Selected>)>,
    mut commands: Commands,
) {
    if !progress.is_changed() {
        return;
    }
    for (select, is_layout) in &selects {
        let index = if is_layout {
            LayoutId::ALL.iter().position(|l| *l == progress.layout)
        } else {
            crate::sounds::Switch::ALL.iter().position(|s| *s == progress.switch)
        };
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

fn on_switch_change(change: On<ValueChange<Entity>>, options: Query<&OptionIndex>, mut progress: ResMut<Progress>, mut focus: ResMut<InputFocus>) {
    if let Ok(option) = options.get(change.value)
        && let Some(switch) = crate::sounds::Switch::ALL.get(option.0)
    {
        progress.switch = *switch;
        progress.save();
    }
    release_focus(&mut focus);
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
    progress.words_of = None;
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

fn go_home(_: On<Activate>, mut panel: ResMut<Panel>, mut focus: ResMut<InputFocus>) {
    panel.toggle(Panel::Home);
    release_focus(&mut focus);
}

/// On to the next lesson without its words.
fn skip_words(_: On<Activate>, mut progress: ResMut<Progress>, mut new_exercise: MessageWriter<NewExercise>, mut focus: ResMut<InputFocus>) {
    progress.words_of = None;
    new_exercise.write_default();
    release_focus(&mut focus);
}

/// The arrow beside the words, while they are typed.
fn show_skip_words(exercise: Res<Exercise>, progress: Res<Progress>, mut buttons: Query<&mut Node, With<SkipWordsButton>>) {
    if !exercise.is_changed() && !progress.is_changed() {
        return;
    }
    let shown = exercise.words && progress.mode == ExerciseMode::Basic && exercise.tutor.query != TutorQuery::End;
    for mut node in &mut buttons {
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
}

/// The frosted glass over a paused exercise, and the Enter key on it.
#[allow(clippy::too_many_arguments)]
fn show_pause(
    exercise: Res<Exercise>,
    progress: Res<Progress>,
    layout: Res<ActiveLayout>,
    theme: Res<bevy::feathers::theme::UiTheme>,
    asset_server: Res<AssetServer>,
    mut overlays: Query<(&mut Node, &mut BackgroundColor), With<PauseOverlay>>,
    keys: Query<(Entity, Option<&Children>), With<PauseKey>>,
    mut commands: Commands,
) {
    if !exercise.is_changed() && !progress.is_changed() && !theme.is_changed() {
        return;
    }
    let paused = exercise.paused && progress.mode.is_typing();
    for (mut node, mut background) in &mut overlays {
        let display = if paused { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
        // Frosted: the window's own color, the text and the keys show through faintly.
        let frost = theme.color(&feathers_tokens::WINDOW_BG).with_alpha(0.88);
        if background.0 != frost {
            background.0 = frost;
        }
    }
    for (root, children) in &keys {
        if children.is_none_or(|c| c.is_empty()) || theme.is_changed() {
            commands.entity(root).despawn_children();
            spawn_keycap(&mut commands, root, crate::klavaro::UPSYM, &layout.0, &theme, &asset_server, 1.6);
        }
    }
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
        let display = show(!drawing && !(is_keyboard && (progress.compact || !progress.show_keyboard)));
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
    exercise: Res<Exercise>,
    mut hands: Query<&mut Node, (With<HandsRoot>, Without<CompactSpacer>)>,
    mut spacers: Query<&mut Node, With<CompactSpacer>>,
) {
    if !progress.is_changed() && !exercise.is_changed() {
        return;
    }
    // At the end of a typing exercise the history and the summary take their room.
    let ended = progress.mode.is_typing() && exercise.tutor.query == TutorQuery::End;
    for mut node in &mut spacers {
        node.display = if progress.compact && progress.mode.subject() == Subject::Typing { Display::Flex } else { Display::None };
    }
    // No hands in the hanzi and penmanship modes, nor in the Matrix.
    let display = if progress.show_hands && !ended && progress.mode.subject() == Subject::Typing && progress.theme != ThemeMode::Matrix {
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
    let says_something = !typing || !ended || !summary(&exercise).is_empty();
    for mut node in &mut panes {
        set(&mut node, ended && says_something);
    }
}

/// The Enter key on the frosted glass of a pause.
#[derive(Component, Default, Clone)]
struct PauseKey;

/// The keys an exercise is about.
fn lesson_keys(mode: ExerciseMode, exercise: &Exercise, layout: &crate::layout::Layout) -> Vec<char> {
    match mode {
        ExerciseMode::Basic | ExerciseMode::Review => exercise.char_set.clone(),
        ExerciseMode::Fingers if exercise.finger == 5 => vec![' ', crate::klavaro::UPSYM],
        ExerciseMode::Fingers => layout.finger_chars(exercise.finger).iter().map(|(c, _)| *c).collect(),
        ExerciseMode::BounceLeft | ExerciseMode::BounceRight => {
            let (middle, columns) = crate::drills::bounce_keys(layout, mode == ExerciseMode::BounceLeft);
            middle.into_iter().chain(columns.into_iter().flatten()).collect()
        }
        _ => Vec::new(),
    }
}

/// The small keys beside the label: as on the keyboard, the stripe in the color of the finger.
fn fill_lesson_keys(
    keys: &[char],
    layout: &crate::layout::Layout,
    theme: &bevy::feathers::theme::UiTheme,
    asset_server: &AssetServer,
    root: Entity,
    commands: &mut Commands,
) {
    commands.entity(root).despawn_children();
    for c in keys {
        spawn_keycap(commands, root, *c, layout, theme, asset_server, 1.0);
    }
}

/// A key as on the keyboard, its stripe in the color of its finger, `scale` times the size of
/// the keys of the toolbar.
pub fn spawn_keycap(
    commands: &mut Commands,
    parent: Entity,
    c: char,
    layout: &crate::layout::Layout,
    theme: &bevy::feathers::theme::UiTheme,
    asset_server: &AssetServer,
    scale: f32,
) -> Entity {
    let name = match c {
        ' ' => "Space".to_string(),
        crate::klavaro::UPSYM => "Enter".to_string(),
        c => c.to_string(),
    };
    let finger = layout.locate(c).map_or(tokens::KEY_BORDER, |at| crate::keyboard::stripe_token(&layout.keys[at.key]));
    commands
        .spawn((
            Node {
                min_width: px(24.0 * scale),
                height: px(26.0 * scale),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect { left: px(5.0 * scale), right: px(5.0 * scale), top: px(0), bottom: px(2.0 * scale) },
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4.0 * scale)),
                ..default()
            },
            BackgroundColor(theme.color(&tokens::KEY_BG)),
            BorderColor::all(theme.color(&tokens::KEY_BORDER)),
            Pickable::IGNORE,
            ChildOf(parent),
            children![
                (
                    Text::new(name),
                    TextFont {
                        font: bevy::text::FontSource::Handle(asset_server.load(bevy::feathers::constants::fonts::MONO)),
                        font_size: FontSize::Px(13.0 * scale),
                        ..default()
                    },
                    TextColor(theme.color(&tokens::KEY_TEXT)),
                    Pickable::IGNORE,
                ),
                (
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(4.0 * scale),
                        right: px(4.0 * scale),
                        bottom: px(2.0 * scale),
                        height: px(3.0 * scale),
                        border_radius: BorderRadius::all(px(2.0 * scale)),
                        ..default()
                    },
                    BackgroundColor(theme.color(&finger)),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .id()
}

#[allow(clippy::too_many_arguments)]
fn update_lesson_label(
    exercise: Res<Exercise>,
    layout: Res<ActiveLayout>,
    progress: Res<Progress>,
    hanzi: Res<crate::hanzi::data::HanziData>,
    theme: Res<bevy::feathers::theme::UiTheme>,
    asset_server: Res<AssetServer>,
    mut labels: Query<&mut Text, With<LessonLabel>>,
    roots: Query<Entity, With<LessonKeys>>,
    mut shown: Local<Vec<char>>,
    mut commands: Commands,
) {
    if !exercise.is_changed() && !layout.is_changed() && !progress.is_changed() && !theme.is_changed() {
        return;
    }
    let mode = if progress.mode.is_typing() { exercise.mode } else { progress.mode };
    let keys = lesson_keys(mode, &exercise, &layout.0);
    let text = match mode {
        ExerciseMode::Basic => {
            format!("Lesson {}/{N_BASIC_LESSONS}", exercise.lesson)
        }
        ExerciseMode::Layers => {
            let lessons = crate::course::layer_lessons(&layout.0);
            let name = lessons.get(exercise.lesson.saturating_sub(1)).map(|l| l.name.clone()).unwrap_or_default();
            format!("Layer lesson {}/{}: {name}", exercise.lesson, lessons.len())
        }
        ExerciseMode::Review => "Warm-up".into(),
        ExerciseMode::Adaptability => "Random words, your errors first".into(),
        ExerciseMode::Velocity => "Random words of the dictionary".into(),
        ExerciseMode::Fluidness => "Paragraphs, backspace fixes errors".into(),
        ExerciseMode::Quotes => "Famous quotes and their authors".into(),
        ExerciseMode::Affirmations => "Positive affirmations".into(),
        ExerciseMode::Custom if progress.pasted_text.trim().is_empty() => "Copy a text, then Paste text".into(),
        ExerciseMode::Custom => {
            let lines = crate::pasted::lines(&progress.pasted_text, |c| layout.0.locate(c).is_some());
            format!("Your text: part {}/{}", exercise.lesson, crate::pasted::parts(&lines))
        }
        ExerciseMode::Fingers if exercise.finger == 5 => "Thumbs".into(),
        ExerciseMode::Fingers => capitalize(finger_name(exercise.finger)),
        ExerciseMode::BounceLeft | ExerciseMode::BounceRight => "aLtErNaTe".into(),
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
    if *shown != keys || layout.is_changed() || theme.is_changed() {
        for root in &roots {
            fill_lesson_keys(&keys, &layout.0, &theme, &asset_server, root, &mut commands);
        }
        *shown = keys;
    }
}

/// The tooltip of the button under the pointer, after a short rest, under the button and
/// inside the window.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn show_tooltips(
    hovers: Res<bevy::picking::hover::HoverMap>,
    parents: Query<&ChildOf>,
    tips: Query<(&Tooltip, &ComputedNode, &UiGlobalTransform, &InheritedVisibility)>,
    mut boxes: Query<(&mut Node, &ComputedNode), With<TooltipBox>>,
    mut texts: Query<&mut Text, With<TooltipText>>,
    window: Single<&Window>,
    time: Res<Time<Real>>,
    mut over: Local<Option<(Entity, f64)>>,
) {
    let now = time.elapsed_secs_f64();
    let hovered = hovers.values().flat_map(|hovered| hovered.keys()).find_map(|e| {
        std::iter::successors(Some(*e), |e| parents.get(*e).ok().map(ChildOf::parent)).find(|e| tips.contains(*e))
    });
    match (hovered, *over) {
        (Some(e), Some((was, _))) if e == was => {}
        (Some(e), _) => *over = Some((e, now)),
        (None, _) => *over = None,
    }
    let shown = over.filter(|(_, since)| now - since >= TOOLTIP_DELAY).and_then(|(e, _)| tips.get(e).ok()).filter(|(.., visible)| visible.get());
    for (mut node, tip_size) in &mut boxes {
        let Some((Tooltip(text), button, transform, _)) = shown else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            continue;
        };
        for mut label in &mut texts {
            if label.0 != *text {
                label.0 = text.to_string();
            }
        }
        let scale = button.inverse_scale_factor;
        let (size, center) = (button.size() * scale, transform.translation * scale);
        let width = tip_size.size().x * tip_size.inverse_scale_factor;
        let window_width = window.physical_width() as f32 * scale;
        node.display = Display::Flex;
        node.left = px((center.x - width / 2.0).clamp(4.0, (window_width - width - 4.0).max(4.0)));
        node.top = px(center.y + size.y / 2.0 + 6.0);
    }
}

/// A finger that started at the top of the window and went down: the toolbar comes out.
fn drags_down_from_the_top(start_y: f32, y: f32, scale: f32) -> bool {
    start_y <= 96.0 * scale && y - start_y >= 24.0 * scale
}

/// The toolbar hides, with the setting or in the full screen over the top of the window, in the
/// middle of an exercise keeping its place so that the text and the keyboard do not jump at the
/// first key. It shows while the pointer is near the top or on it, a panel or one of its menus
/// is open, or a moment after a touch at the top or a finger dragged down from there.
#[allow(clippy::too_many_arguments)]
fn show_toolbar(
    exercise: Res<Exercise>,
    drill: Res<Drill>,
    progress: Res<Progress>,
    panel: Res<Panel>,
    window: Single<&Window>,
    scale: Res<UiScale>,
    touches: Res<Touches>,
    time: Res<Time<Real>>,
    hovers: Res<bevy::picking::hover::HoverMap>,
    parents: Query<&ChildOf>,
    popups: Query<&Visibility, With<FeathersMenuPopup>>,
    mut toolbars: Query<(Entity, &mut Node, &mut Visibility, &ComputedNode), (With<Toolbar>, Without<FeathersMenuPopup>)>,
    mut shown_until: Local<f64>,
) {
    let full_screen = !matches!(window.mode, bevy::window::WindowMode::Windowed);
    let hides = progress.auto_hide_toolbar || full_screen;
    let in_lesson = match progress.mode {
        ExerciseMode::Shortcuts => drill.done_steps > 0 && drill.finished.is_none(),
        mode if mode.is_typing() => exercise.tutor.query == TutorQuery::ProcessTouches && !exercise.paused,
        _ => false,
    };
    let menu_open = popups.iter().any(|visibility| *visibility != Visibility::Hidden);
    let now = time.elapsed_secs_f64();
    for (toolbar, mut node, mut visibility, computed) in &mut toolbars {
        let position = if hides { PositionType::Absolute } else { PositionType::Relative };
        if node.position_type != position {
            node.position_type = position;
        }
        // The height of the toolbar in the pixels of the window, a strip of 12 when hidden.
        let height = (computed.size().y * computed.inverse_scale_factor * scale.0).max(12.0);
        let near_top = window.cursor_position().is_some_and(|p| p.y <= height + 8.0);
        let touched = touches.iter_just_pressed().any(|t| t.position().y <= 48.0 * scale.0)
            || touches.iter().any(|t| drags_down_from_the_top(t.start_position().y, t.position().y, scale.0));
        // On the toolbar or its open menus, they hang below it.
        let on_it = hovers.values().flat_map(|hovered| hovered.keys()).any(|e| {
            std::iter::successors(Some(*e), |e| parents.get(*e).ok().map(ChildOf::parent)).any(|e| e == toolbar)
        });
        if near_top || on_it || menu_open || *panel != Panel::None {
            *shown_until = now + 0.6;
        }
        if touched {
            *shown_until = now + 4.0;
        }
        let shown = now < *shown_until;
        let display = if !hides || shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
        let wanted = if in_lesson && !shown { Visibility::Hidden } else { Visibility::Inherited };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}

/// The keys go to the exercise, never to the menus: Tab (with Alt or Ctrl too, Bevy does not
/// look at them) would focus the first control, the "Touch typing" menu, then Space or Enter
/// would open it. The same for a control left focused, by Esc closing its menu. In a panel Tab
/// goes through its controls.
fn keep_keys_off_the_menus(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    panel: Res<Panel>,
    time: Res<Time<Real>>,
    popups: Query<&Visibility, With<FeathersMenuPopup>>,
    mut focus: ResMut<InputFocus>,
    mut visible: ResMut<bevy::input_focus::InputFocusVisible>,
    mut focused_since: Local<Option<f64>>,
) {
    if *panel != Panel::None {
        *focused_since = None;
        return;
    }
    let menu_open = popups.iter().any(|visibility| *visibility != Visibility::Hidden);
    let pressing = mouse.get_pressed().next().is_some() || touches.iter().next().is_some();
    let tab = keys.just_pressed(KeyCode::Tab);
    if !tab && (focus.get().is_none() || menu_open || pressing) {
        *focused_since = None;
        return;
    }
    // A click focuses a menu button before its menu opens: it has a moment.
    let now = time.elapsed_secs_f64();
    let since = *focused_since.get_or_insert(now);
    if tab || now - since > 0.2 {
        focus.clear();
        visible.0 = false;
        *focused_since = None;
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
    mut labels: Query<&mut Text, (With<LiveStats>, Without<TimerText>)>,
    mut timer_texts: Query<&mut Text, With<TimerText>>,
    mut bars: Query<&mut Node, (With<ProgressBar>, Without<Timer>)>,
    mut timers: Query<&mut Node, With<Timer>>,
) {
    let now = time.elapsed_secs_f64();
    let started = !progress.mode.is_typing() || exercise.tutor.query != TutorQuery::Start;
    let seconds = clock(if progress.mode.is_typing() { exercise.tutor.elapsed(now) } else { drill.elapsed(now) });
    let (text, done) = if progress.mode.is_typing() {
        let tutor = &exercise.tutor;
        let elapsed = tutor.elapsed(now);
        let correct = tutor.n_touchs.saturating_sub(tutor.n_errors);
        let wpm = if elapsed > 0.5 { 12.0 * correct as f64 / elapsed } else { 0.0 };
        let mut text = format!("{wpm:.0} wpm");
        if tutor.query == TutorQuery::Start {
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
            format!("{}/{} keys  ·  {} missed", drill.done_steps, drill.n_steps, drill.missed_steps)
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
    for mut label in &mut timer_texts {
        if label.0 != seconds {
            label.0.clone_from(&seconds);
        }
    }
    let display = if started { Display::Flex } else { Display::None };
    for mut node in &mut timers {
        if node.display != display {
            node.display = display;
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_message(
    exercise: Res<Exercise>,
    drill: Res<Drill>,
    progress: Res<Progress>,
    mut texts: ParamSet<(
        Query<(Entity, &mut Text), With<SummaryTitle>>,
        Query<(Entity, &mut Text), With<GradeName>>,
        Query<&mut Node, With<GradeRow>>,
        Query<(&mut Text, &mut Node), With<RecordText>>,
        Query<(&mut Text, &mut Node), With<MessageText>>,
        Query<&mut Node, With<SummaryHeader>>,
    )>,
    mut stars: Query<(Entity, &GradeStar, &mut crate::icons::IconNode)>,
    mut commands: Commands,
) {
    if !exercise.is_changed() && !drill.is_changed() && !progress.is_changed() {
        return;
    }
    let summary = if progress.mode.is_typing() {
        summary(&exercise)
    } else {
        Summary { title: "Round done".into(), tone: feathers_tokens::TEXT_MAIN, grade: None, record: None, notes: drill.summary.clone(), suggestion: false }
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
    for (entity, mut text) in &mut texts.p1() {
        set(&mut text, summary.grade.map_or("", crate::lessons::Grade::name));
        commands.entity(entity).insert(ThemeTextColor(summary.tone.clone()));
    }
    for mut node in &mut texts.p2() {
        show(&mut node, summary.grade.is_some());
    }
    let n_stars = summary.grade.map_or(0, |grade| grade as usize + 1);
    for (entity, GradeStar(i), mut icon) in &mut stars {
        use crate::icons::{Icon, IconTint};
        let gained = *i < n_stars;
        let wanted = if gained { Icon::Star } else { Icon::StarEmpty };
        if icon.0 != wanted {
            icon.0 = wanted;
        }
        commands.entity(entity).insert(IconTint(if gained { summary.tone.clone() } else { feathers_tokens::TEXT_DIM }));
    }
    for (mut text, mut node) in &mut texts.p3() {
        set(&mut text, summary.record.as_deref().unwrap_or_default());
        show(&mut node, summary.record.is_some());
    }
    for (mut text, mut node) in &mut texts.p4() {
        set(&mut text, &summary.notes);
        show(&mut node, !summary.notes.is_empty());
    }
    for mut node in &mut texts.p5() {
        show(&mut node, !summary.title.is_empty());
    }
}

/// The summary at the end of an exercise.
#[derive(Debug, Clone, PartialEq)]
struct Summary {
    /// "Lesson 7 complete"; empty for an exercise failed, the rest of the screen tells.
    title: String,
    /// The color of the title.
    tone: bevy::feathers::theme::ThemeToken,
    /// The grade gained, one to three stars.
    grade: Option<crate::lessons::Grade>,
    record: Option<String>,
    /// The notes of the exercise.
    notes: String,
    /// A finger drill is suggested, on its own line.
    suggestion: bool,
}

impl Summary {
    /// Nothing to say: a failed lesson without a tip, the rest of the screen tells.
    fn is_empty(&self) -> bool {
        self.title.is_empty() && self.grade.is_none() && self.record.is_none() && self.notes.is_empty() && !self.suggestion
    }
}

impl Default for Summary {
    fn default() -> Self {
        Summary { title: String::new(), tone: feathers_tokens::TEXT_MAIN, grade: None, record: None, notes: String::new(), suggestion: false }
    }
}

/// The summary of a typing exercise, empty before its end.
fn summary(exercise: &Exercise) -> Summary {
    let Some(results) = exercise.tutor.results.filter(|_| exercise.tutor.query == TutorQuery::End) else {
        return Summary::default();
    };
    let mode = exercise.mode;
    let grade = crate::lessons::Grade::of(results.accuracy, crate::game::goal(mode));
    let what = match mode {
        ExerciseMode::Basic if exercise.words => format!("Words of lesson {}", exercise.lesson),
        ExerciseMode::Basic => format!("Lesson {}", exercise.lesson),
        ExerciseMode::Layers => format!("Layer lesson {}", exercise.lesson),
        ExerciseMode::Fingers => format!("{} drill", capitalize(finger_name(exercise.finger))),
        ExerciseMode::Review => "Warm-up".into(),
        _ => "Exercise".into(),
    };
    let title = if grade.is_some() { format!("{what} complete") } else { String::new() };
    // The suggested drill has its own line, see [`update_suggestion`].
    let lines: Vec<String> = exercise.message.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect();
    Summary {
        title,
        tone: grade.map_or(feathers_tokens::TEXT_MAIN, crate::lessons::Grade::token),
        grade,
        record: exercise.record.clone(),
        notes: lines.join("\n"),
        suggestion: exercise.suggestion.is_some(),
    }
}

/// The results of a typing exercise once it is over, in columns, the number before its unit
/// ("12 wpm", "90% accuracy", "59% fluidness"): under each ▲ better than the last time of the
/// lesson, ▼ worse, ● the same.
#[allow(clippy::type_complexity)]
fn update_end_stats(
    exercise: Res<Exercise>,
    progress: Res<Progress>,
    theme: Res<bevy::feathers::theme::UiTheme>,
    asset_server: Res<AssetServer>,
    mut nodes: ParamSet<(
        Query<&mut Node, With<EndStats>>,
        Query<&mut Node, With<LiveStats>>,
        Query<(&EndColumn, &mut Node)>,
        Query<(&TrendText, &mut Text, &mut TextColor, &mut TextFont, &mut Node)>,
    )>,
    mut values: Query<(&EndValue, &mut Text), Without<TrendText>>,
) {
    if !exercise.is_changed() && !progress.is_changed() {
        return;
    }
    let results = exercise.tutor.results.filter(|_| exercise.tutor.query == TutorQuery::End && progress.mode.is_typing());
    let show = |visible: bool| if visible { Display::Flex } else { Display::None };
    for mut node in &mut nodes.p0() {
        node.display = show(results.is_some());
    }
    for mut node in &mut nodes.p1() {
        node.display = show(results.is_none());
    }
    let Some(results) = results else {
        return;
    };
    let trend = exercise.trend;
    for (_, mut node) in &mut nodes.p2() {
        node.display = show(true);
    }
    for (EndValue(i), mut text) in &mut values {
        text.0 = match i {
            0 => format!("{:.0} wpm", results.velocity),
            1 => format!("{}% accuracy", accuracy_shown(results.accuracy)),
            _ => format!("{:.0}% fluidness", results.fluidness),
        };
    }
    for (TrendText(i), mut text, mut color, mut font, mut node) in &mut nodes.p3() {
        node.display = show(trend.is_some());
        let Some(trend) = trend else {
            continue;
        };
        // No "-0.0".
        let diff = if trend[*i].abs() < 0.05 { 0.0 } else { trend[*i] };
        let (unit, same) = [("", 0.5), ("%", 0.5), ("%", 0.5)][*i];
        let (mark, token) = if diff > same {
            ("▲", tokens::CHAR_CORRECT)
        } else if diff < -same {
            ("▼", tokens::CHAR_WRONG)
        } else {
            ("●", tokens::NEAR_MISS)
        };
        text.0 = format!("{mark} {diff:+.1}{unit}");
        color.0 = theme.color(&token);
        font.font = bevy::text::FontSource::Handle(asset_server.load(bevy::feathers::constants::fonts::MONO));
        font.font_size = FontSize::Px(13.0);
    }
}

/// The keys of the moment, at the bottom. The key to go on with once an exercise is done comes
/// after a pause only: whoever knows it has pressed it by then, the help lists them all.
#[allow(clippy::too_many_arguments)]
fn update_actions(
    progress: Res<Progress>,
    drill: Res<Drill>,
    hanzi: Res<crate::hanzi::Practice>,
    pen: Res<crate::penmanship::Practice>,
    last: Res<crate::game::LastInput>,
    time: Res<Time<Real>>,
    mut labels: Query<&mut Text, With<ActionsText>>,
) {
    let rested = last.idle(&time) >= REST_SECONDS;
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
            if drill.finished.is_none() {
                actions.push("Esc, the arrows and the shortcuts are the drill".into());
            }
        }
        // Any key goes on.
        _ => {}
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
        "Weak words, a stroke was shown: {} (review them from the lessons)\nMost missed: {}\nBest written: {}\n{} words written, {} levels done · the strokes {} accurate · {glimpses} strokes shown",
        or_none(weak, "none"),
        or_none(missed, "none"),
        or_none(best, "not yet"),
        words.len(),
        progress.hanzi.levels_done.len(),
        if n > 0 { format!("{:.0}%", sum / n as f64) } else { "not yet".into() },
    )
}

/// The line of the suggested drill at the end: "[S] was missed **6** times." and the button
/// "Train left ring finger", the finger underlined with a stripe in its color; or the weakest
/// finger and its accuracy.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_suggestion(
    exercise: Res<Exercise>,
    layout: Res<ActiveLayout>,
    theme: Res<bevy::feathers::theme::UiTheme>,
    asset_server: Res<AssetServer>,
    mut rows: Query<&mut Node, With<SuggestRow>>,
    keys: Query<Entity, With<SuggestKey>>,
    mut texts: ParamSet<(
        Query<(Entity, &mut Text, &TextFont), With<SuggestText>>,
        Query<(&mut TextSpan, &mut TextColor), With<SuggestCount>>,
        Query<(&mut TextSpan, &mut TextColor), (With<SuggestTail>, Without<SuggestCount>)>,
        Query<&mut Text, (With<SuggestCaption>, Without<SuggestText>)>,
        Query<&mut BackgroundColor, With<SuggestFinger>>,
    )>,
    mut spawned: Local<bool>,
    mut commands: Commands,
) {
    // The spans of the text and of the caption, once.
    if !*spawned {
        let Ok((text, font)) = texts.p0().single().map(|(e, _, f)| (e, f.clone())) else {
            return;
        };
        let bold = TextFont { font: bevy::text::FontSource::Handle(asset_server.load(bevy::feathers::constants::fonts::BOLD)), ..font.clone() };
        commands.spawn((TextSpan::new(""), bold, ChildOf(text), SuggestCount));
        commands.spawn((TextSpan::new(""), font, ChildOf(text), SuggestTail));
        *spawned = true;
        return;
    }
    if !exercise.is_changed() && !theme.is_changed() {
        return;
    }
    let suggestion = exercise.suggestion.filter(|_| exercise.tutor.query == TutorQuery::End);
    for mut node in &mut rows {
        node.display = if suggestion.is_some() { Display::Flex } else { Display::None };
    }
    let Some(suggestion) = suggestion else {
        return;
    };
    use crate::game::Reason;
    let (key, head, count, tail) = match suggestion.reason {
        Reason::Missed { key, times } => (Some(key), "was missed ", times.to_string(), if times == 1 { " time." } else { " times." }),
        Reason::Weakest { accuracy } => (None, "Weakest finger at ", format!("{accuracy:.0}%"), "."),
    };
    for root in &keys {
        commands.entity(root).despawn_children();
        if let Some(key) = key {
            spawn_keycap(&mut commands, root, key, &layout.0, &theme, &asset_server, 1.0);
        }
    }
    for (_, mut text, _) in &mut texts.p0() {
        text.0 = head.to_string();
    }
    // The spans in the color of the label, not the white of a text without a theme.
    let ink = theme.color(&feathers_tokens::TEXT_MAIN);
    for (mut span, mut color) in &mut texts.p1() {
        span.0.clone_from(&count);
        color.0 = ink;
    }
    for (mut span, mut color) in &mut texts.p2() {
        span.0 = tail.to_string();
        color.0 = ink;
    }
    let finger_color = theme.color(&tokens::FINGERS[(suggestion.finger as usize).min(9)]);
    for mut caption in &mut texts.p3() {
        caption.0 = finger_name(suggestion.finger).to_string();
    }
    for mut stripe in &mut texts.p4() {
        stripe.0 = finger_color;
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

    /// A window, the first control of the toolbar and a key press through the real focus plugins.
    fn tab_press(keep_off: bool) -> Option<Entity> {
        use bevy::input::ButtonState;
        use bevy::input::keyboard::{Key, KeyboardInput};
        use bevy::input_focus::tab_navigation::{TabIndex, TabNavigationPlugin};
        use bevy::input_focus::{InputDispatchPlugin, InputFocusPlugin};
        use bevy::window::PrimaryWindow;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::input::InputPlugin, InputFocusPlugin, InputDispatchPlugin, TabNavigationPlugin))
            .init_resource::<Panel>();
        if keep_off {
            app.add_systems(Update, keep_keys_off_the_menus);
        }
        let window = app.world_mut().spawn((Window::default(), PrimaryWindow)).id();
        let root = app.world_mut().spawn(TabGroup::default()).id();
        let menu = app.world_mut().spawn((TabIndex(0), ChildOf(root))).id();
        app.update();
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Tab,
            logical_key: Key::Tab,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.update();
        let focused = app.world().resource::<InputFocus>().get();
        assert!(focused.is_none() || focused == Some(menu));
        focused
    }

    #[test]
    fn tab_does_not_leave_the_keys_on_the_first_menu() {
        // Without the system it does, the reason of the system.
        assert!(tab_press(false).is_some());
        assert_eq!(tab_press(true), None);
    }

    #[test]
    fn the_accuracy_shown_never_rounds_up_to_a_pass() {
        assert_eq!(accuracy_shown(94.8), 94);
        assert_eq!(accuracy_shown(95.0), 95);
        assert_eq!(accuracy_shown(99.96), 99);
        assert_eq!(accuracy_shown(100.0), 100);
    }

    #[test]
    fn the_clock_is_minutes_and_seconds_with_zeros() {
        assert_eq!(clock(0.0), "00:00");
        assert_eq!(clock(5.9), "00:05");
        assert_eq!(clock(65.0), "01:05");
        assert_eq!(clock(600.0), "10:00");
        assert_eq!(clock(6000.0), "100:00");
    }

    #[test]
    fn a_close_miss_shows_on_the_accuracy_line() {
        let (fill, mark) = accuracy_line(94.8, 95.0);
        // 0.8% of the 608 pixels of the line, about 5: more than the mark's width.
        assert!(mark - fill > 0.7 && mark - fill < 0.9, "{fill} {mark}");
        assert_eq!(accuracy_line(100.0, 95.0).0, 100.0);
        assert_eq!(accuracy_line(40.0, 95.0).0, 0.0);
        assert_eq!(accuracy_line(95.0, 95.0).0, accuracy_line(95.0, 95.0).1);
    }

    #[test]
    fn only_a_drag_down_from_the_top_brings_out_the_toolbar() {
        assert!(drags_down_from_the_top(10.0, 60.0, 1.0));
        // A tap, a drag up, a drag from the middle of the screen.
        assert!(!drags_down_from_the_top(10.0, 12.0, 1.0));
        assert!(!drags_down_from_the_top(60.0, 10.0, 1.0));
        assert!(!drags_down_from_the_top(300.0, 400.0, 1.0));
    }
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
