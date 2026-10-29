//! The exercise being typed: the keyboard input, the Klavaro tutor and the progress.
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardFocusLost, KeyboardInput};
use bevy::prelude::*;

use crate::klavaro::tutor::{Outcome, Query, Tutor};
use crate::klavaro::{self, BACKSPACE, HYPER_BACKSPACE, KbdGrid, N_BASIC_LESSONS, UPSYM};
use crate::layout::{Chord, KeyName, Layout, Mods, Special, Target, finger_name};
use crate::progress::{ExerciseMode, Progress, Session, now_unix_seconds};

/// The legends are hidden after this many correct touches in a row.
const FADE_TOUCHES: f32 = 20.0;
/// Without a touch for this long the next key is shown again.
const STUCK_SECONDS: f64 = 2.0;
/// A key missed this many times in an exercise suggests the drill of its finger.
const SUGGEST_MISSES: u32 = 3;
/// The progress changed since this long is saved, the browser has no exit to save on.
const AUTOSAVE_SECONDS: f64 = 5.0;
/// A finger below this accuracy suggests its drill.
const SUGGEST_ACCURACY: f64 = 92.0;

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
    pub finger: u8,
    pub char_set: Vec<char>,
    /// The misses of this exercise per expected character.
    pub misses: BTreeMap<char, u32>,
    /// Klavaro's comment at the end of the exercise.
    pub message: String,
    /// The finger drill suggested at the end, with the reason.
    pub suggestion: Option<(u8, String)>,
    /// The most missed characters of a failed exercise, red on the keyboard until the next one.
    pub review: Vec<char>,
}

/// The accuracy to pass an exercise, Klavaro's goals: the basic course unlocks the next lesson
/// at 95%.
pub fn goal(mode: ExerciseMode) -> f64 {
    match mode {
        ExerciseMode::Basic | ExerciseMode::Layers | ExerciseMode::Review => klavaro::basic::GOAL_ACCURACY,
        ExerciseMode::Adaptability => 98.0,
        ExerciseMode::Velocity => 95.0,
        ExerciseMode::Fluidness => 97.0,
        ExerciseMode::Fingers | ExerciseMode::BounceLeft | ExerciseMode::BounceRight => crate::drills::GOAL_ACCURACY,
        ExerciseMode::Shortcuts | ExerciseMode::Hanzi => 100.0,
    }
}

/// The keys shown in red after a failed exercise.
const N_REVIEW: usize = 4;

/// What to press next and how much help to show on the keyboard.
#[derive(Resource)]
pub struct Coach {
    pub target: Option<Target>,
    /// 1 shows all the legends, 0 hides them.
    pub fade: f32,
    /// The next key is shown after an error or a pause, until it is typed.
    pub reveal: bool,
    pub fade_legends: bool,
    pub layer_trainer: bool,
    /// The plan of every target is shown, for the shortcuts.
    pub always_show: bool,
    last_touch: f64,
    /// A word of advice above the keyboard, until the time given.
    pub flash: Option<(String, f64)>,
}

impl Coach {
    fn reset(&mut self, now: f64) {
        self.fade = 1.0;
        self.reveal = false;
        self.last_touch = now;
    }

    pub fn touched(&mut self, ok: bool, now: f64) {
        self.last_touch = now;
        if ok {
            self.reveal = false;
            self.fade = (self.fade - 1.0 / FADE_TOUCHES).max(0.0);
        } else {
            self.reveal = true;
            self.fade = (self.fade + 0.1).min(1.0);
        }
        if !self.fade_legends {
            self.fade = 1.0;
        }
    }
}

/// The modifiers held, followed through the key events.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct Modifiers {
    pub mods: Mods,
    /// Alt was held when the window lost the keyboard focus, an Alt+Tab.
    pub alt_at_focus_loss: bool,
}

/// A key press, with the modifiers held at that moment.
#[derive(Message, Debug, Clone, Copy)]
pub struct KeyPressed {
    /// The character Klavaro receives, `None` for the keys it ignores.
    pub typed: Option<char>,
    /// The key with its modifiers, `None` for the modifiers themselves.
    pub chord: Option<Chord>,
}

/// The typing and the new exercises, the views run after them.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GameSystems;

/// Asks for a new exercise of the current mode and lesson.
#[derive(Message, Default)]
pub struct NewExercise;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchResult {
    Ok,
    Wrong,
    Other,
}

/// A processed touch, for the animations, with the indices of the keys in the layout.
#[derive(Message, Debug, Clone, Copy)]
pub struct Touched {
    pub pressed: Option<usize>,
    pub expected: Option<usize>,
    pub result: TouchResult,
    /// Shakes the wrong character of the text.
    pub shake: bool,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        let mut progress = Progress::load();
        start_with_warmup(&mut progress);
        let layout = progress.layout.build();
        let grid = layout.klavaro_grid();
        let exercise = generate(&grid, &layout, &mut progress);
        let coach = Coach {
            target: None,
            fade: 1.0,
            reveal: false,
            fade_legends: progress.fade_legends,
            layer_trainer: progress.layer_trainer,
            always_show: false,
            flash: None,
            last_touch: 0.0,
        };
        app.insert_resource(exercise)
            .insert_resource(coach)
            .insert_resource(progress.theme)
            .insert_resource(bevy::feathers::theme::UiTheme(crate::theme::props(progress.theme)))
            .insert_resource(crate::shortcuts::Drill::new(&layout))
            .insert_resource(progress)
            .insert_resource(Grid(grid))
            .insert_resource(ActiveLayout(layout))
            .init_resource::<Modifiers>()
            .add_message::<NewExercise>()
            .add_message::<Touched>()
            .add_message::<KeyPressed>()
            .add_systems(
                Update,
                (
                    read_keys,
                    crate::keyboard::on_screen_keys,
                    handle_typing,
                    crate::shortcuts::handle_shortcuts,
                    new_exercise.run_if(on_message::<NewExercise>),
                    update_coach,
                )
                    .chain()
                    .in_set(GameSystems),
            )
            .add_systems(Last, (autosave, save_on_exit.run_if(on_message::<AppExit>)));
    }
}

/// Generates the text of an exercise, as Klavaro does for each kind of exercise.
pub fn generate(grid: &KbdGrid, layout: &Layout, progress: &mut Progress) -> Exercise {
    let mode = progress.mode;
    let lesson = progress.current().basic_lesson.clamp(1, N_BASIC_LESSONS);
    let finger = progress.current().finger.clamp(1, 9);
    let char_set = klavaro::basic::lesson_char_set(lesson, grid);
    let mut lesson = lesson;
    let mut char_set = char_set;
    let text = match mode {
        // The drill, then real words of the keys learned so far.
        ExerciseMode::Basic => {
            let mut text = klavaro::basic::draw_lesson(&char_set);
            if let Some(words) = crate::course::word_lines(&crate::course::learned_chars(grid, lesson), 2) {
                text.push_str(&words);
            }
            text
        }
        ExerciseMode::Layers => {
            let lessons = crate::course::layer_lessons(layout);
            let current = progress.current();
            current.layer_lesson = current.layer_lesson.clamp(1, lessons.len().max(1));
            lesson = current.layer_lesson;
            char_set = lessons.get(lesson - 1).map(|l| l.chars.clone()).unwrap_or_default();
            klavaro::basic::draw_lesson(&char_set)
        }
        ExerciseMode::Review => {
            let weak = crate::course::weak_keys(progress.current(), crate::progress::now_unix_seconds(), 6);
            if weak.len() >= 2 {
                char_set = weak;
            }
            klavaro::basic::draw_lesson(&char_set)
        }
        ExerciseMode::Adaptability => {
            let accuracy = &progress.current().klavaro;
            klavaro::adaptability::draw_random_pattern(grid, accuracy, accuracy.needs_special_practice())
        }
        ExerciseMode::Velocity => klavaro::velocity::draw_random_words(),
        ExerciseMode::Fluidness => klavaro::fluidness::draw_random_paragraphs(),
        ExerciseMode::Fingers => crate::drills::draw_finger_drill(layout, finger),
        ExerciseMode::BounceLeft => crate::drills::draw_bounce(layout, true),
        ExerciseMode::BounceRight => crate::drills::draw_bounce(layout, false),
        // The shortcuts have their own tasks, see `crate::shortcuts`.
        // The shortcuts and the hanzi have their own exercises.
        ExerciseMode::Shortcuts | ExerciseMode::Hanzi => String::new(),
    };
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    Exercise {
        id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        tutor: Tutor::new(&text),
        mode,
        lesson,
        finger,
        char_set,
        misses: BTreeMap::new(),
        message: String::new(),
        suggestion: None,
        review: Vec::new(),
    }
}

/// A session started after a pause begins with the weak keys, then goes back to its mode.
fn start_with_warmup(progress: &mut Progress) {
    const PAUSE_SECONDS: u64 = 4 * 3600;
    let now = crate::progress::now_unix_seconds();
    let last = progress.layouts.values().flat_map(|l| l.sessions.last()).map(|s| s.finished_at).max();
    let mode = progress.mode;
    if !mode.is_typing() || mode == ExerciseMode::Review || last.is_none_or(|t| now.saturating_sub(t) < PAUSE_SECONDS) {
        return;
    }
    if crate::course::weak_keys(progress.current(), now, 6).len() >= 3 {
        progress.warmup_return = Some(mode);
        progress.mode = ExerciseMode::Review;
    }
}

fn new_exercise(
    mut exercise: ResMut<Exercise>,
    mut progress: ResMut<Progress>,
    mut grid: ResMut<Grid>,
    layout: Res<ActiveLayout>,
    mut drill: ResMut<crate::shortcuts::Drill>,
    mut coach: ResMut<Coach>,
    time: Res<Time<Real>>,
) {
    // After the warm-up, the mode of the session.
    if exercise.mode == ExerciseMode::Review
        && exercise.tutor.query == Query::End
        && let Some(mode) = progress.warmup_return.take()
    {
        progress.mode = mode;
    }
    if layout.is_changed() {
        grid.0 = layout.0.klavaro_grid();
    }
    *exercise = generate(&grid.0, &layout.0, &mut progress);
    *drill = crate::shortcuts::Drill::new(&layout.0);
    coach.reset(time.elapsed_secs_f64());
}

fn is_modifier(key: &Key) -> Option<Special> {
    match key {
        Key::Control => Some(Special::Ctrl),
        Key::Alt => Some(Special::Alt),
        Key::Shift => Some(Special::Shift),
        Key::Super | Key::Meta => Some(Special::Super),
        _ => None,
    }
}

fn named_key(key: &Key) -> Option<Special> {
    Some(match key {
        Key::Escape => Special::Escape,
        Key::Tab => Special::Tab,
        Key::Enter => Special::Enter,
        Key::Space => Special::Space,
        Key::Backspace => Special::Backspace,
        Key::Delete => Special::Delete,
        Key::Insert => Special::Insert,
        Key::ArrowLeft => Special::Left,
        Key::ArrowRight => Special::Right,
        Key::ArrowUp => Special::Up,
        Key::ArrowDown => Special::Down,
        Key::Home => Special::Home,
        Key::End => Special::End,
        Key::PageUp => Special::PageUp,
        Key::PageDown => Special::PageDown,
        _ => return None,
    })
}

/// The chord of a key press: the shift of the characters is part of the character, unless Ctrl,
/// Alt or Super are held too.
fn chord(event: &KeyboardInput, mods: Mods) -> Option<Chord> {
    if let Some(special) = named_key(&event.logical_key) {
        return Some(Chord::new(mods, KeyName::Special(special)));
    }
    let c = match &event.logical_key {
        Key::Character(s) => s.chars().next()?,
        _ => event.text.as_ref()?.chars().next().filter(|c| !c.is_control())?,
    };
    if mods.ctrl || mods.alt || mods.super_key {
        Some(Chord::new(mods, KeyName::Char(c.to_ascii_lowercase())))
    } else {
        Some(Chord::char(c))
    }
}

/// The character Klavaro would receive for a key press, `None` for the keys to ignore.
fn typed_char(event: &KeyboardInput, mods: Mods) -> Option<char> {
    match &event.logical_key {
        Key::Backspace if mods.ctrl => Some(HYPER_BACKSPACE),
        Key::Backspace => Some(BACKSPACE),
        _ if mods.ctrl || mods.alt || mods.super_key || event.repeat => None,
        Key::Enter => Some(UPSYM),
        Key::Space => Some(' '),
        _ => event
            .text
            .as_ref()
            .and_then(|text| text.chars().next())
            .filter(|c| !c.is_control()),
    }
}

/// Follows the modifiers through the key events, in their order, and sends the key presses.
fn read_keys(
    mut events: MessageReader<KeyboardInput>,
    mut focus_lost: MessageReader<KeyboardFocusLost>,
    mut modifiers: ResMut<Modifiers>,
    mut panel: ResMut<crate::ui::Panel>,
    mut pressed: MessageWriter<KeyPressed>,
    mut zoom: MessageWriter<crate::zoom::Zoom>,
) {
    for event in events.read() {
        let down = event.state == ButtonState::Pressed;
        // Ctrl+= Ctrl+- Ctrl+0 zoom, as in the browsers, the browser itself zooms on the web.
        let mods = modifiers.mods;
        if mods.ctrl
            && !mods.alt
            && let Some(change) = crate::zoom::Zoom::of_key(&event.logical_key)
        {
            if down && !cfg!(target_arch = "wasm32") {
                zoom.write(change);
            }
            continue;
        }
        // F1 opens and closes the help, Esc closes the help and the settings, they take the
        // other keys.
        let open = *panel != crate::ui::Panel::None;
        if down && event.logical_key == Key::F1 {
            panel.toggle(crate::ui::Panel::Help);
            continue;
        }
        if down && open && event.logical_key == Key::Escape {
            *panel = crate::ui::Panel::None;
            continue;
        }
        if open {
            continue;
        }
        // AltGr types the symbols of the international layouts, it is not a shortcut modifier.
        if event.logical_key == Key::AltGraph {
            continue;
        }
        if let Some(modifier) = is_modifier(&event.logical_key) {
            let mods = &mut modifiers.mods;
            match modifier {
                Special::Ctrl => mods.ctrl = down,
                Special::Alt => mods.alt = down,
                Special::Shift => mods.shift = down,
                _ => mods.super_key = down,
            }
            if down {
                modifiers.alt_at_focus_loss = false;
            }
            continue;
        }
        if !down {
            continue;
        }
        let mods = modifiers.mods;
        pressed.write(KeyPressed {
            typed: typed_char(event, mods),
            chord: if event.repeat { None } else { chord(event, mods) },
        });
    }
    // The releases are lost with the focus.
    if focus_lost.read().count() > 0 {
        modifiers.alt_at_focus_loss = modifiers.mods.alt;
        modifiers.mods = Mods::NONE;
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_typing(
    mut keys: MessageReader<KeyPressed>,
    time: Res<Time<Real>>,
    mut exercise: ResMut<Exercise>,
    mut progress: ResMut<Progress>,
    grid: Res<Grid>,
    layout: Res<ActiveLayout>,
    mut coach: ResMut<Coach>,
    mut touched: MessageWriter<Touched>,
    mut new_exercise: MessageWriter<NewExercise>,
) {
    if !progress.mode.is_typing() || !exercise.mode.is_typing() {
        keys.clear();
        return;
    }
    let now = time.elapsed_secs_f64();
    for key in keys.read() {
        if exercise.tutor.query == Query::End {
            // Klavaro: Enter starts the next exercise, Backspace goes back one basic lesson.
            if key.typed == Some(UPSYM) {
                new_exercise.write_default();
            } else if key.typed == Some(BACKSPACE) && exercise.mode == ExerciseMode::Basic {
                // As Klavaro, from the lesson already increased when the goal was reached.
                let current = progress.current();
                current.basic_lesson = current.basic_lesson.saturating_sub(1).max(1);
                progress.save();
                new_exercise.write_default();
            } else if key.chord == Some(Chord::special(Special::Tab)) {
                go_to_suggestion(&exercise, &mut progress, &mut new_exercise);
            }
            continue;
        }
        let Some(c) = key.typed else {
            continue;
        };
        let exercise = &mut *exercise;
        let current = progress.current();
        let expected = exercise.tutor.expected();
        let outcome = exercise.tutor.process_touch(c, now, &mut current.klavaro, &grid.0);
        let result = match outcome {
            Outcome::Correct { expected, touch_time } => {
                if let Some(touch_time) = touch_time {
                    let stats = current.key(expected);
                    stats.hits += 1;
                    stats.last = crate::progress::now_unix_seconds();
                    // Klavaro ignores the touch times of 3 seconds or more, the pauses.
                    stats.total_time += touch_time.min(3.0);
                }
                TouchResult::Ok
            }
            Outcome::Wrong { expected, first, .. } => {
                if first {
                    let stats = current.key(expected);
                    stats.misses += 1;
                    stats.last = crate::progress::now_unix_seconds();
                    *exercise.misses.entry(expected).or_default() += 1;
                    // The auto shift: a capital by a key held too long, or not long enough.
                    if layout.0.auto_shift && c != expected {
                        let advice = if c == layout.0.shifted(expected) {
                            current.auto_shift_long += 1;
                            Some(format!("Held too long: {expected} came out as {c}, tap it shorter"))
                        } else if expected == layout.0.shifted(c) {
                            current.auto_shift_short += 1;
                            Some(format!("{expected} needs the key held a moment longer, auto shift"))
                        } else {
                            None
                        };
                        if let Some(advice) = advice {
                            coach.flash = Some((advice, now + 2.5));
                        }
                    }
                }
                TouchResult::Wrong
            }
            Outcome::Finished => TouchResult::Ok,
            Outcome::Erased => TouchResult::Ok,
            Outcome::Beep => TouchResult::Other,
        };
        if outcome == Outcome::Erased {
            // The key typed wrong stays revealed until it is typed right.
            coach.last_touch = now;
        } else if result != TouchResult::Other {
            coach.touched(result == TouchResult::Ok, now);
        }
        let key_of = |c: char| layout.0.locate(c).map(|l| l.key);
        touched.write(Touched {
            pressed: key_of(c),
            expected: expected.and_then(key_of),
            result,
            shake: result == TouchResult::Wrong,
        });
        if outcome == Outcome::Finished {
            finish(exercise, &mut progress, &layout.0);
        }
    }
}

/// Tab at the end of an exercise starts the suggested finger drill.
pub fn go_to_suggestion(exercise: &Exercise, progress: &mut Progress, new_exercise: &mut MessageWriter<NewExercise>) {
    if let Some((finger, _)) = exercise.suggestion {
        progress.current().finger = finger;
        progress.mode = ExerciseMode::Fingers;
        progress.save();
        new_exercise.write_default();
    }
}

/// The drill of the finger of the most missed key of the exercise, or of the weakest finger.
fn suggest(exercise: &Exercise, progress: &mut Progress, layout: &Layout) -> Option<(u8, String)> {
    if !matches!(
        exercise.mode,
        ExerciseMode::Basic | ExerciseMode::Adaptability | ExerciseMode::Velocity | ExerciseMode::Fluidness
    ) {
        return None;
    }
    let worst = exercise.misses.iter().max_by_key(|(_, n)| **n);
    if let Some((c, n)) = worst
        && *n >= SUGGEST_MISSES
        && let Some(finger) = layout.finger_of(*c).filter(|f| *f != 0)
    {
        let shown = if *c == ' ' { "Space".to_string() } else { c.to_string() };
        return Some((finger, format!("{shown} was missed {n} times, it is typed with the {}", finger_name(finger))));
    }
    let fingers = progress.current().finger_accuracy(|c| layout.finger_of(c));
    let (finger, accuracy) = fingers.into_iter().filter(|(f, _)| *f != 0).min_by(|a, b| a.1.total_cmp(&b.1))?;
    (accuracy < SUGGEST_ACCURACY)
        .then(|| (finger, format!("your weakest finger is the {} at {accuracy:.0}%", finger_name(finger))))
}

fn finish(exercise: &mut Exercise, progress: &mut Progress, layout: &Layout) {
    let Some(results) = exercise.tutor.results else {
        return;
    };
    let mode = exercise.mode;
    exercise.message = match mode {
        ExerciseMode::Basic | ExerciseMode::Layers => {
            klavaro::basic::comment(results.accuracy, exercise.lesson).to_string()
        }
        ExerciseMode::Review => {
            let keys: String = exercise.char_set.iter().collect();
            format!("Warm-up done on {keys}. Enter goes on with the session.")
        }
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
        ExerciseMode::Fingers => crate::drills::comment(results.accuracy, exercise.finger),
        ExerciseMode::BounceLeft | ExerciseMode::BounceRight => crate::drills::bounce_comment(results.accuracy),
        ExerciseMode::Shortcuts | ExerciseMode::Hanzi => String::new(),
    };
    if results.n_errors == 0 {
        exercise.message = format!("Congratulations, not a single error! Well done.\n{}", exercise.message);
    }
    exercise.suggestion = suggest(exercise, progress, layout);
    if results.accuracy < goal(mode) {
        let mut missed: Vec<(char, u32)> = exercise.misses.iter().map(|(c, n)| (*c, *n)).collect();
        missed.sort_by_key(|m| std::cmp::Reverse(m.1));
        exercise.review = missed.into_iter().take(N_REVIEW).map(|(c, _)| c).collect();
        if !exercise.review.is_empty() {
            let keys: Vec<String> =
                exercise.review.iter().map(|c| if *c == ' ' { "Space".into() } else { c.to_string() }).collect();
            exercise.message.push_str(&format!("\nFocus next time: {}, red on the keyboard", keys.join("  ")));
        }
    } else if mode == ExerciseMode::Basic && exercise.lesson < N_BASIC_LESSONS {
        exercise.message.push_str(&format!("\nLesson {} unlocked.", exercise.lesson + 1));
    } else if mode == ExerciseMode::Layers && exercise.lesson < crate::course::layer_lessons(layout).len() {
        exercise.message.push_str(&format!("\nLayer lesson {} unlocked.", exercise.lesson + 1));
    }
    let n_layer_lessons = crate::course::layer_lessons(layout).len().max(1);
    let current = progress.current();
    if mode == ExerciseMode::Basic && results.accuracy >= klavaro::basic::GOAL_ACCURACY {
        current.basic_lesson = (exercise.lesson + 1).min(N_BASIC_LESSONS);
        current.basic_unlocked = current.basic_unlocked.max(current.basic_lesson);
    }
    if mode == ExerciseMode::Layers && results.accuracy >= goal(mode) {
        current.layer_lesson = (exercise.lesson + 1).min(n_layer_lessons);
        current.layer_unlocked = current.layer_unlocked.max(current.layer_lesson);
    }
    let may_log = mode != ExerciseMode::Fluidness || results.n_touchs >= klavaro::tutor::MIN_CHARS_TO_LOG;
    if may_log {
        current.sessions.push(Session {
            mode,
            lesson: match mode {
                ExerciseMode::Basic | ExerciseMode::Layers => Some(exercise.lesson),
                ExerciseMode::Fingers => Some(exercise.finger as usize),
                _ => None,
            },
            accuracy: results.accuracy,
            velocity: results.velocity,
            fluidness: results.fluidness,
            finished_at: now_unix_seconds(),
            elapsed: results.elapsed,
        });
    }
    progress.save();
}

/// The next key, and the reveal of the next key when the typing stops.
fn update_coach(
    mut coach: ResMut<Coach>,
    exercise: Res<Exercise>,
    drill: Res<crate::shortcuts::Drill>,
    progress: Res<Progress>,
    time: Res<Time<Real>>,
) {
    let target = if progress.mode.is_typing() {
        exercise.tutor.expected().map(Target::Char)
    } else {
        drill.expected().map(Target::Chord)
    };
    let typing = exercise.tutor.query == Query::ProcessTouches;
    let stuck = typing && time.elapsed_secs_f64() - coach.last_touch > STUCK_SECONDS;
    let always_show = !progress.mode.is_typing();
    // Only the changes, the views follow the changes of the coach.
    if coach.target != target {
        coach.target = target;
    }
    if stuck && !coach.reveal {
        coach.reveal = true;
    }
    if coach.always_show != always_show {
        coach.always_show = always_show;
    }
    if coach.fade_legends != progress.fade_legends || coach.layer_trainer != progress.layer_trainer {
        coach.fade_legends = progress.fade_legends;
        coach.layer_trainer = progress.layer_trainer;
        if !coach.fade_legends {
            coach.fade = 1.0;
        }
    }
}

/// Saves the touches of an exercise not finished yet, a closed browser tab keeps them.
fn autosave(mut progress: ResMut<Progress>, time: Res<Time<Real>>, mut dirty: Local<bool>, mut last: Local<f64>) {
    let now = time.elapsed_secs_f64();
    if progress.is_changed() {
        *dirty = true;
    }
    if *dirty && now - *last >= AUTOSAVE_SECONDS {
        // The save sorts the rankings, it is not a change to save again.
        progress.bypass_change_detection().save();
        *dirty = false;
        *last = now;
    }
}

fn save_on_exit(mut progress: ResMut<Progress>) {
    progress.save();
}
