//! The window: the feathers toolbar, the live statistics, the exercise, the keyboard and the
//! rankings of the keys.
use bevy::feathers::containers::{flex_spacer, group, group_body, group_header};
use bevy::feathers::controls::*;
use bevy::feathers::display::{caption, label, label_dim};
use bevy::feathers::theme::ThemeBackgroundColor;
use bevy::feathers::tokens as feathers_tokens;
use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::ui::{Checked, Selected};
use bevy::picking::events::PointerClick;
use bevy::ui_widgets::{Activate, ValueChange};

use crate::drills::FINGERS;
use crate::game::{ActiveLayout, Exercise, NewExercise, go_to_suggestion};
use crate::klavaro::N_BASIC_LESSONS;
use crate::klavaro::tutor::Query as TutorQuery;
use crate::layout::{LayoutId, finger_name};
use crate::progress::{BackgroundLevel, ExerciseMode, Progress};
use crate::shortcuts::Drill;
use crate::theme::{ThemeMode, tokens};

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
struct RankingText;

#[derive(Component, Default, Clone)]
struct ModeSelect;

/// The toggles and checkboxes of the settings, they start unchecked.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Setting {
    #[default]
    Dark,
    FadeLegends,
    LayerTrainer,
    ShowHands,
    ReduceMotion,
    LargeText,
}

#[derive(Component, Default, Clone)]
struct SettingControl(Setting);

#[derive(Component, Default, Clone)]
struct SuggestButton;

#[derive(Component, Default, Clone)]
struct HelpOverlay;

#[derive(Component, Default, Clone)]
struct SettingsOverlay;

/// The panel over the window, it takes the keys.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    #[default]
    None,
    Help,
    Settings,
}

impl Panel {
    pub fn toggle(&mut self, panel: Panel) {
        *self = if *self == panel { Panel::None } else { panel };
    }
}

const HELP: &str = "\
MODES, the first menu
  Basic course, Adaptability, Velocity, Fluidness: the exercises of Klavaro
  Finger drill: one finger up and down its keys, ‹ › picks the finger
  Left and Right hand finger bounce: middle and index finger in turns, aLtErNaTiNg case
  Shortcuts & keys: Esc, arrows, numbers, symbols, Ctrl and Alt shortcuts, Alt+Tab

TOOLBAR
  ‹ ›  the lesson, the finger or the hand    Restart  a new text
  Settings  the keyboard, dark mode, the background, motion, hands, text size and:
    Fade legends  the letters fade while you type well, an error or a pause shows the next key
    Layer trainer  always shows the layer and the modifier keys to hold

TYPING
  Backspace erases the errors, Ctrl+Backspace all of them
  At the end: Enter for the next exercise, Backspace the previous lesson, Tab the suggested drill

COLORS
  green the next key, purple shift or hold the key longer (auto shift), amber hold, red backspace
  The stripes of the keys and the hands below show the finger of every key

ON-SCREEN KEYBOARD
  Click a key to type it, a long press types it shifted
  Lower and Raise: click to show and use their layer, click again to go back, or hold them
  Ctrl, Alt, Win: click them, then the key: they are never sent to your system

BACKGROUND
  Its colors are the fingers and layers of your most missed keys, named under Your keys

F1 or ? opens this help, Esc or a click closes it. The progress is saved in ~/.config/claudralingo";

#[derive(Component, Default, Clone)]
struct SuggestCaption;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Panel>().add_systems(Startup, (spawn_ui, first_help)).add_systems(
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
                show_hands,
                update_rankings.run_if(resource_changed::<Progress>),
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
    commands.spawn_scene(root(modes, layouts, backgrounds));
}

fn root(modes: Box<dyn SceneList>, layouts: Box<dyn SceneList>, backgrounds: Box<dyn SceneList>) -> impl Scene {
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
            @label("") TrainerHint
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
            --
            @rankings()
            --
            @help_overlay()
            --
            @settings_overlay(layouts, backgrounds)
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

fn settings_overlay(layouts: Box<dyn SceneList>, backgrounds: Box<dyn SceneList>) -> impl Scene {
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
                        on(on_layout_change)
                    ]
                    --
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(12),
                    }
                    Children [
                        @label("Dark mode") Node { min_width: px(110) }
                        --
                        @FeathersToggleSwitch
                        SettingControl(Setting::Dark)
                        on(on_setting_change)
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
                    @setting_checkbox(Setting::FadeLegends, "Fade legends: the letters fade while you type well")
                    --
                    @setting_checkbox(Setting::LayerTrainer, "Layer trainer: show the layer and modifier keys to hold")
                    --
                    @setting_checkbox(Setting::ShowHands, "Show the hands under the keyboard")
                    --
                    @setting_checkbox(Setting::ReduceMotion, "Reduce motion: no drift, sparks, shakes nor breathing keys")
                    --
                    @setting_checkbox(Setting::LargeText, "Large text to type")
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

fn stats_bar() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(6),
            width: px(640),
        }
        Children [
            @label("") LiveStats
            --
            Node {
                width: percent(100),
                height: px(4),
                border_radius: BorderRadius::all(px(2)),
            }
            ThemeBackgroundColor(tokens::PROGRESS_TRACK)
            Children [
                Node {
                    width: percent(0),
                    height: percent(100),
                    border_radius: BorderRadius::all(px(2)),
                }
                ThemeBackgroundColor(tokens::PROGRESS_BAR)
                ProgressBar
            ]
        ]
    }
}

fn rankings() -> impl Scene {
    bsn! {
        Node {
            width: px(820),
            margin: UiRect::bottom(px(10)),
        }
        Children [
            @group()
            Node {
                flex_grow: 1.0,
            }
            Children [
                @group_header() Children [ @label("Your keys") ]
                --
                @group_body() Children [ @label_dim("") RankingText ]
            ]
        ]
    }
}

impl Setting {
    fn get(self, progress: &Progress) -> bool {
        match self {
            Setting::Dark => progress.theme == ThemeMode::Dark,
            Setting::FadeLegends => progress.fade_legends,
            Setting::LayerTrainer => progress.layer_trainer,
            Setting::ShowHands => progress.show_hands,
            Setting::ReduceMotion => progress.reduce_motion,
            Setting::LargeText => progress.large_text,
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
    mut mode: ResMut<ThemeMode>,
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
        Setting::Dark => {
            *mode = if change.value { ThemeMode::Dark } else { ThemeMode::Light };
            progress.theme = *mode;
        }
        Setting::FadeLegends => progress.fade_legends = change.value,
        Setting::LayerTrainer => progress.layer_trainer = change.value,
        Setting::ShowHands => progress.show_hands = change.value,
        Setting::ReduceMotion => progress.reduce_motion = change.value,
        Setting::LargeText => progress.large_text = change.value,
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
    children: Query<&Children>,
    rows: Query<(&OptionIndex, Has<Selected>)>,
    mut commands: Commands,
) {
    if !progress.is_changed() {
        return;
    }
    let index = ExerciseMode::ALL.iter().position(|m| *m == progress.mode);
    for select in &selects {
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
fn change_lesson(progress: &mut Progress, delta: isize) {
    if matches!(progress.mode, ExerciseMode::BounceLeft | ExerciseMode::BounceRight) {
        // The other hand.
        progress.mode = if progress.mode == ExerciseMode::BounceLeft {
            ExerciseMode::BounceRight
        } else {
            ExerciseMode::BounceLeft
        };
    } else if progress.mode == ExerciseMode::Fingers {
        let current = progress.current();
        let i = FINGERS.iter().position(|f| *f == current.finger).unwrap_or(0);
        current.finger = FINGERS[(i as isize + delta).rem_euclid(FINGERS.len() as isize) as usize];
    } else {
        let current = progress.current();
        current.basic_lesson = current.basic_lesson.saturating_add_signed(delta).clamp(1, N_BASIC_LESSONS);
        progress.mode = ExerciseMode::Basic;
    }
    progress.save();
}

fn previous_lesson(
    _: On<Activate>,
    mut progress: ResMut<Progress>,
    mut new_exercise: MessageWriter<NewExercise>,
    mut focus: ResMut<InputFocus>,
) {
    change_lesson(&mut progress, -1);
    new_exercise.write_default();
    release_focus(&mut focus);
}

fn next_lesson(
    _: On<Activate>,
    mut progress: ResMut<Progress>,
    mut new_exercise: MessageWriter<NewExercise>,
    mut focus: ResMut<InputFocus>,
) {
    change_lesson(&mut progress, 1);
    new_exercise.write_default();
    release_focus(&mut focus);
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
    }
}

fn toggle_help(_: On<Activate>, mut panel: ResMut<Panel>, mut focus: ResMut<InputFocus>) {
    panel.toggle(Panel::Help);
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
    mut helps: Query<&mut Node, (With<HelpOverlay>, Without<SettingsOverlay>)>,
    mut settings: Query<&mut Node, With<SettingsOverlay>>,
) {
    let show = |visible: bool| if visible { Display::Flex } else { Display::None };
    for mut node in &mut helps {
        node.display = show(*panel == Panel::Help);
    }
    for mut node in &mut settings {
        node.display = show(*panel == Panel::Settings);
    }
}

fn show_hands(progress: Res<Progress>, mut hands: Query<&mut Node, With<HandsRoot>>) {
    if !progress.is_changed() {
        return;
    }
    let display = if progress.show_hands { Display::Flex } else { Display::None };
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

type PromptOnly = (With<PromptRoot>, Without<ShortcutsRoot>, Without<MessageText>);

/// The text of the typing exercises or the tasks of the shortcuts, the results at the end.
fn show_mode_views(
    progress: Res<Progress>,
    exercise: Res<Exercise>,
    drill: Res<Drill>,
    mut prompts: Query<&mut Node, PromptOnly>,
    mut shortcuts: Query<&mut Node, (With<ShortcutsRoot>, Without<MessageText>)>,
    mut messages: Query<&mut Node, With<MessageText>>,
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
}

fn update_lesson_label(exercise: Res<Exercise>, layout: Res<ActiveLayout>, mut labels: Query<&mut Text, With<LessonLabel>>) {
    if !exercise.is_changed() && !layout.is_changed() {
        return;
    }
    let text = match exercise.mode {
        ExerciseMode::Basic => {
            let chars: Vec<String> = exercise.char_set.iter().copied().map(display_char).collect();
            format!("Lesson {}/{N_BASIC_LESSONS}: {}", exercise.lesson, chars.join(" "))
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
    };
    for mut label in &mut labels {
        label.0.clone_from(&text);
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
) {
    if !exercise.is_changed() && !drill.is_changed() && !progress.is_changed() {
        return;
    }
    let text = if !progress.mode.is_typing() {
        drill.summary.clone()
    } else if exercise.tutor.query == TutorQuery::End {
        let results = exercise.tutor.results.unwrap_or(crate::klavaro::tutor::Results {
            accuracy: 0.0,
            velocity: 0.0,
            fluidness: 0.0,
            elapsed: 0.0,
            n_touchs: 0,
            n_errors: 0,
        });
        let mut keys = match exercise.mode {
            ExerciseMode::Basic => "Enter: next exercise · Backspace: previous lesson".to_string(),
            _ => "Enter: next exercise".to_string(),
        };
        if let Some((finger, reason)) = &exercise.suggestion {
            keys = format!("{} · Tab: {} drill · {keys}", capitalize(reason), finger_name(*finger));
        }
        format!(
            "Accuracy {:.1}% · {:.1} WPM · fluidness {:.0}%\n{}\n{keys}",
            results.accuracy, results.velocity, results.fluidness, exercise.message
        )
    } else {
        String::new()
    };
    for mut label in &mut labels {
        if label.0 != text {
            label.0.clone_from(&text);
        }
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
    // The colors of the background, named.
    let focus: Vec<String> =
        crate::background::focus(&layout.0, current).into_iter().map(|f| f.name(&layout.0)).collect();
    let mut text = format!(
        "Best keys: {}\nMost missed: {}\nWeakest fingers: {}\nFocus, the colors of the background: {}\n{} · {} exercises done",
        list(&current.best_keys),
        if misses.is_empty() { "none".to_string() } else { misses.join("   ") },
        if fingers.is_empty() { "not enough touches yet".to_string() } else { fingers.join("   ") },
        if focus.is_empty() { "no misses yet".to_string() } else { focus.join(", ") },
        layout_id.name(),
        current.sessions.len(),
    );
    if let Some(best) = current.alt_tab_best {
        text.push_str(&format!(" · best Alt+Tab round trip {best:.1} s"));
    }
    for mut label in &mut labels {
        label.0.clone_from(&text);
    }
}
