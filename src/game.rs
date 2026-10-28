//! The exercise being typed: the keyboard input, the Klavaro tutor and the progress.
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::klavaro::tutor::{Outcome, Query, Tutor};
use crate::klavaro::{self, BACKSPACE, HYPER_BACKSPACE, KbdGrid, N_BASIC_LESSONS, UPSYM};
use crate::layout::Layout;
use crate::progress::{ExerciseMode, Progress, Session, now_unix_seconds};

/// The keyboard grid of the basic lessons.
#[derive(Resource)]
pub struct Grid(pub KbdGrid);

#[derive(Resource)]
pub struct ActiveLayout(pub Layout);

#[derive(Resource)]
pub struct Exercise {
    /// Different for every generated exercise.
    pub id: u64,
    pub tutor: Tutor,
    pub mode: ExerciseMode,
    pub lesson: usize,
    pub char_set: Vec<char>,
    /// Klavaro's comment at the end of the exercise.
    pub message: String,
}

/// The typing and the new exercises, the views run after them.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GameSystems;

/// Asks for a new exercise of the current mode and lesson.
#[derive(Message, Default)]
pub struct NewExercise;

/// A processed touch, for the animations.
#[derive(Message, Debug, Clone, Copy)]
pub struct Touched {
    pub typed: char,
    pub outcome: Outcome,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        let mut progress = Progress::load();
        let layout = progress.layout.build();
        let grid = KbdGrid::qwerty_us();
        let mode = progress.mode;
        let lesson = progress.current().basic_lesson.clamp(1, N_BASIC_LESSONS);
        let exercise = generate(mode, lesson, &grid, &mut progress);
        app.insert_resource(exercise)
            .insert_resource(progress.theme)
            .insert_resource(bevy::feathers::theme::UiTheme(crate::theme::props(progress.theme)))
            .insert_resource(progress)
            .insert_resource(Grid(grid))
            .insert_resource(ActiveLayout(layout))
            .add_message::<NewExercise>()
            .add_message::<Touched>()
            .add_systems(
                Update,
                (handle_typing, new_exercise.run_if(on_message::<NewExercise>))
                    .chain()
                    .in_set(GameSystems),
            )
            .add_systems(Last, save_on_exit.run_if(on_message::<AppExit>));
    }
}

/// Generates the text of an exercise, as Klavaro does for each kind of exercise.
pub fn generate(mode: ExerciseMode, lesson: usize, grid: &KbdGrid, progress: &mut Progress) -> Exercise {
    let char_set = klavaro::basic::lesson_char_set(lesson, grid);
    let text = match mode {
        ExerciseMode::Basic => klavaro::basic::draw_lesson(&char_set),
        ExerciseMode::Adaptability => {
            let accuracy = &progress.current().klavaro;
            klavaro::adaptability::draw_random_pattern(grid, accuracy, accuracy.needs_special_practice())
        }
        ExerciseMode::Velocity => klavaro::velocity::draw_random_words(),
        ExerciseMode::Fluidness => klavaro::fluidness::draw_random_paragraphs(),
    };
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    Exercise {
        id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        tutor: Tutor::new(&text),
        mode,
        lesson,
        char_set,
        message: String::new(),
    }
}

fn new_exercise(mut exercise: ResMut<Exercise>, mut progress: ResMut<Progress>, grid: Res<Grid>) {
    let mode = progress.mode;
    let lesson = progress.current().basic_lesson.clamp(1, N_BASIC_LESSONS);
    *exercise = generate(mode, lesson, &grid.0, &mut progress);
}

/// The character Klavaro would receive for a key press, `None` for the keys to ignore.
fn typed_char(event: &KeyboardInput, keys: &ButtonInput<KeyCode>) -> Option<char> {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let other_modifier = keys.any_pressed([
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    match &event.logical_key {
        Key::Backspace if ctrl => Some(HYPER_BACKSPACE),
        Key::Backspace => Some(BACKSPACE),
        _ if ctrl || other_modifier || event.repeat => None,
        Key::Enter => Some(UPSYM),
        Key::Space => Some(' '),
        _ => event
            .text
            .as_ref()
            .and_then(|text| text.chars().next())
            .filter(|c| !c.is_control()),
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_typing(
    mut events: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time<Real>>,
    mut exercise: ResMut<Exercise>,
    mut progress: ResMut<Progress>,
    grid: Res<Grid>,
    mut touched: MessageWriter<Touched>,
    mut new_exercise: MessageWriter<NewExercise>,
) {
    let now = time.elapsed_secs_f64();
    for event in events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        let Some(c) = typed_char(event, &keys) else {
            continue;
        };
        if exercise.tutor.query == Query::End {
            // Klavaro: Enter starts the next exercise, Backspace goes back one basic lesson.
            if c == UPSYM {
                new_exercise.write_default();
            } else if c == BACKSPACE && exercise.mode == ExerciseMode::Basic {
                // As Klavaro, from the lesson already increased when the goal was reached.
                let current = progress.current();
                current.basic_lesson = current.basic_lesson.saturating_sub(1).max(1);
                progress.save();
                new_exercise.write_default();
            }
            continue;
        }
        let exercise = &mut *exercise;
        let current = progress.current();
        let outcome = exercise.tutor.process_touch(c, now, &mut current.klavaro, &grid.0);
        match outcome {
            Outcome::Correct { expected, touch_time: Some(touch_time) } => {
                let stats = current.key(expected);
                stats.hits += 1;
                // Klavaro ignores the touch times of 3 seconds or more, the pauses.
                stats.total_time += touch_time.min(3.0);
            }
            Outcome::Wrong { expected, first: true, .. } => current.key(expected).misses += 1,
            _ => {}
        }
        touched.write(Touched { typed: c, outcome });
        if outcome == Outcome::Finished {
            finish(exercise, &mut progress);
        }
    }
}

fn finish(exercise: &mut Exercise, progress: &mut Progress) {
    let Some(results) = exercise.tutor.results else {
        return;
    };
    let mode = exercise.mode;
    exercise.message = match mode {
        ExerciseMode::Basic => klavaro::basic::comment(results.accuracy, exercise.lesson).to_string(),
        ExerciseMode::Adaptability => klavaro::adaptability::comment(results.accuracy),
        ExerciseMode::Velocity => klavaro::velocity::comment(results.accuracy, results.velocity),
        ExerciseMode::Fluidness => {
            let mut comment =
                klavaro::fluidness::comment(results.accuracy, results.velocity, results.fluidness);
            if results.n_touchs < klavaro::tutor::MIN_CHARS_TO_LOG {
                comment.push_str(&format!(
                    "\nps.: logging not performed for this session: the number of typed \
                     characters ({}) must be greater than {}.",
                    results.n_touchs,
                    klavaro::tutor::MIN_CHARS_TO_LOG
                ));
            }
            comment
        }
    };
    let current = progress.current();
    if mode == ExerciseMode::Basic && results.accuracy >= klavaro::basic::GOAL_ACCURACY {
        current.basic_lesson = (exercise.lesson + 1).min(N_BASIC_LESSONS);
    }
    let may_log = mode != ExerciseMode::Fluidness || results.n_touchs >= klavaro::tutor::MIN_CHARS_TO_LOG;
    if may_log {
        current.sessions.push(Session {
            mode,
            lesson: (mode == ExerciseMode::Basic).then_some(exercise.lesson),
            accuracy: results.accuracy,
            velocity: results.velocity,
            fluidness: results.fluidness,
            finished_at: now_unix_seconds(),
        });
    }
    progress.save();
}

fn save_on_exit(mut progress: ResMut<Progress>) {
    progress.save();
}
