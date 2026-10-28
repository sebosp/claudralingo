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
use bevy::ui::Checked;
use bevy::ui_widgets::{Activate, ValueChange};

use crate::game::{ActiveLayout, Exercise, NewExercise};
use crate::klavaro::N_BASIC_LESSONS;
use crate::klavaro::tutor::Query as TutorQuery;
use crate::layout::LayoutId;
use crate::progress::{ExerciseMode, Progress};
use crate::theme::{ThemeMode, tokens};

#[derive(Component, Default, Clone)]
pub struct PromptRoot;

#[derive(Component, Default, Clone)]
pub struct KeyboardRoot;

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
struct DarkToggle;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ui).add_systems(
            Update,
            (
                init_dark_toggle,
                update_lesson_label,
                update_live_stats,
                update_message,
                update_rankings.run_if(resource_changed::<Progress>),
            ),
        );
    }
}

fn spawn_ui(mut commands: Commands, progress: Res<Progress>) {
    commands.spawn(Camera2d);
    let mode = ExerciseMode::ALL.iter().position(|m| *m == progress.mode);
    let layout = LayoutId::ALL.iter().position(|l| *l == progress.layout);
    let modes = list_rows_from_strings(ExerciseMode::ALL.map(ExerciseMode::name), mode);
    let layouts = list_rows_from_strings(LayoutId::ALL.map(LayoutId::name), layout);
    commands.spawn_scene(root(modes, layouts));
}

fn root(modes: Box<dyn SceneList>, layouts: Box<dyn SceneList>) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(14),
        }
        TabGroup
        ThemeBackgroundColor(feathers_tokens::WINDOW_BG)
        Children [
            @toolbar(modes, layouts)
            --
            @stats_bar()
            --
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Start,
                min_height: px(4.0 * crate::prompt::LINE_HEIGHT),
                margin: UiRect::top(px(8)),
            }
            PromptRoot
            --
            @label("") MessageText
            Node {
                min_height: px(84),
                margin: UiRect::bottom(px(8)),
            }
            TextLayout {
                justify: Justify::Center,
            }
            --
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Start,
                column_gap: px(56),
                padding: px(12),
            }
            KeyboardRoot
            --
            @flex_spacer()
            --
            @rankings()
        ]
    }
}

fn toolbar(modes: Box<dyn SceneList>, layouts: Box<dyn SceneList>) -> impl Scene {
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
            Node {
                min_width: px(150),
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
                min_width: px(220),
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
            @label_dim("Keyboard")
            --
            @FeathersSelect {
                @options: layouts,
            }
            Node {
                min_width: px(150),
            }
            on(on_layout_change)
            --
            @label_dim("Dark")
            --
            @FeathersToggleSwitch
            DarkToggle
            on(on_dark_toggle)
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
            width: px(760),
            margin: UiRect::bottom(px(12)),
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

/// The toggle has no initial value prop, the dark mode is the default.
fn init_dark_toggle(
    toggles: Query<Entity, Added<DarkToggle>>,
    mode: Res<ThemeMode>,
    mut commands: Commands,
) {
    for entity in &toggles {
        if *mode == ThemeMode::Dark {
            commands.entity(entity).insert(Checked);
        }
    }
}

/// The typing goes to the exercise, not to the last clicked control: Space and Enter would
/// activate it again.
fn release_focus(focus: &mut InputFocus) {
    focus.clear();
}

fn on_dark_toggle(
    change: On<ValueChange<bool>>,
    mut mode: ResMut<ThemeMode>,
    mut progress: ResMut<Progress>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    if change.value {
        commands.entity(change.source).insert(Checked);
        *mode = ThemeMode::Dark;
    } else {
        commands.entity(change.source).remove::<Checked>();
        *mode = ThemeMode::Light;
    }
    progress.theme = *mode;
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

fn change_lesson(progress: &mut Progress, delta: isize) {
    let current = progress.current();
    current.basic_lesson = current.basic_lesson.saturating_add_signed(delta).clamp(1, N_BASIC_LESSONS);
    progress.mode = ExerciseMode::Basic;
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

fn display_char(c: char) -> String {
    match c {
        ' ' => "Space".into(),
        c => c.to_string(),
    }
}

fn update_lesson_label(exercise: Res<Exercise>, mut labels: Query<&mut Text, With<LessonLabel>>) {
    if !exercise.is_changed() {
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
    };
    for mut label in &mut labels {
        label.0.clone_from(&text);
    }
}

fn update_live_stats(
    exercise: Res<Exercise>,
    time: Res<Time<Real>>,
    mut labels: Query<&mut Text, With<LiveStats>>,
    mut bars: Query<&mut Node, With<ProgressBar>>,
) {
    let tutor = &exercise.tutor;
    let elapsed = tutor.elapsed(time.elapsed_secs_f64());
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
    for mut label in &mut labels {
        if label.0 != text {
            label.0.clone_from(&text);
        }
    }
    let progress = if tutor.text.is_empty() { 0.0 } else { 100.0 * tutor.cursor as f32 / tutor.text.len() as f32 };
    let done = if tutor.query == TutorQuery::End { 100.0 } else { progress };
    for mut node in &mut bars {
        node.width = percent(done);
    }
}

fn update_message(exercise: Res<Exercise>, mut labels: Query<&mut Text, With<MessageText>>) {
    if !exercise.is_changed() {
        return;
    }
    let text = if exercise.tutor.query == TutorQuery::End {
        let results = exercise.tutor.results.unwrap_or(crate::klavaro::tutor::Results {
            accuracy: 0.0,
            velocity: 0.0,
            fluidness: 0.0,
            elapsed: 0.0,
            n_touchs: 0,
            n_errors: 0,
        });
        let keys = match exercise.mode {
            ExerciseMode::Basic => "Enter: next exercise · Backspace: previous lesson",
            _ => "Enter: next exercise",
        };
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

fn update_rankings(mut progress: ResMut<Progress>, mut labels: Query<&mut Text, With<RankingText>>) {
    let layout = progress.layout;
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
    let text = format!(
        "Best keys: {}\nMost missed: {}\n{} · {} exercises done",
        list(&current.best_keys),
        if misses.is_empty() { "none".to_string() } else { misses.join("   ") },
        layout.name(),
        current.sessions.len(),
    );
    for mut label in &mut labels {
        label.0.clone_from(&text);
    }
}
