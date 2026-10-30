//! The animations, inspired by layer-tutor: the pressed keys sink, the next key breathes, the
//! correct touches send sparks up from the key, the errors short circuit it and shake the
//! character, the cursor pulses and the comments fade in. A line typed without an error clears
//! as a row of Tetris, with a small firework, a perfect exercise ends with fireworks and sparks.
use std::f32::consts::TAU;

use bevy::feathers::theme::UiTheme;
use bevy::prelude::*;

use crate::game::{Exercise, TouchResult, Touched};
use crate::keyboard::{KeyCap, KeyRole, KeyboardView};
use crate::klavaro::tutor::{CharState, Query as TutorQuery};
use crate::prompt::{PromptChar, PromptView};
use crate::theme::tokens;
use crate::ui::MessageText;

const PRESS_SECONDS: f32 = 0.16;
const SHAKE_SECONDS: f32 = 0.25;
const FADE_SECONDS: f32 = 0.35;

#[derive(Component)]
struct Pressed {
    age: f32,
}

#[derive(Component)]
struct Shake {
    age: f32,
}

#[derive(Component)]
struct Spark {
    position: Vec2,
    velocity: Vec2,
    age: f32,
    life: f32,
    color: Color,
    /// Pixels per second squared, the sparks of the fireworks fall.
    gravity: f32,
}

/// A firework going up, it bursts when its fuse is out.
#[derive(Component)]
struct Rocket {
    position: Vec2,
    velocity: Vec2,
    fuse: f32,
    color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Celebration {
    /// A perfect exercise.
    Fireworks,
    /// An exercise passed with a few errors.
    Sparkles,
}

/// The celebration at the end of an exercise, for a few seconds.
#[derive(Resource, Default)]
struct Show {
    celebration: Option<Celebration>,
    age: f32,
    launched: u32,
    /// The sparks of a perfect exercise from its comment, after the first rockets.
    burst_done: bool,
}

const FIREWORKS_SECONDS: f32 = 3.0;
const ROCKETS: u32 = 12;

/// A line typed without an error: it flashes, its characters pop away from the left to the
/// right and it closes up.
#[derive(Component)]
struct RowClear {
    age: f32,
    /// The height of the line before.
    height: f32,
    /// Green when perfect, orange with one slip.
    color: Color,
}

/// A word of encouragement over a cleared line, rising and fading.
#[derive(Component)]
struct FloatText {
    age: f32,
    start: Vec2,
    color: Color,
}

const FLOAT_SECONDS: f32 = 1.4;
/// Over a perfect line.
const CHEERS: [&str; 8] = ["GREAT!", "WOW!", "AMAZING!", "Are you a bot?", "Flawless!", "Perfect!", "On fire!", "Superb!"];
/// Over a line with one slip.
const ALMOST: [&str; 6] = ["Almost!", "So close!", "Next time!", "That one shouldn't count!", "One slip, no worries", "Nearly perfect!"];

/// A streak of light out of a finger that missed its key.
#[derive(Component)]
struct Streak {
    position: Vec2,
    velocity: Vec2,
    age: f32,
    life: f32,
    color: Color,
}

const ROW_FLASH_SECONDS: f32 = 0.3;
const ROW_POP_SECONDS: f32 = 0.35;
const ROW_CLOSE_SECONDS: f32 = 0.2;
/// The delay of a character after the one on its left.
const ROW_STAGGER: f32 = 0.008;

#[derive(Component)]
struct FadeIn {
    age: f32,
}

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Show>().add_systems(
            Update,
            (
                on_touch,
                start_celebration,
                detect_row_clears,
                animate_row_clears,
                animate_float_texts,
                animate_streaks,
                celebrate,
                animate_rockets,
                fade_in_message,
                animate_keys,
                animate_shakes,
                animate_sparks,
                pulse_cursor,
                animate_fades,
            )
                .chain()
                .after(crate::game::GameSystems),
        );
    }
}

/// The center of a node in logical pixels.
fn center(transform: &UiGlobalTransform, node: &ComputedNode) -> Vec2 {
    transform.affine().translation * node.inverse_scale_factor()
}

fn spawn_spark(commands: &mut Commands, at: Vec2, velocity: Vec2, size: f32, life: f32, color: Color, gravity: f32) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(at.x),
            top: px(at.y),
            width: px(size),
            height: px(size),
            border_radius: BorderRadius::all(percent(50)),
            ..default()
        },
        BackgroundColor(color),
        GlobalZIndex(100),
        Pickable::IGNORE,
        Spark { position: at, velocity, age: 0.0, life, color, gravity },
    ));
}

/// The fireworks for a perfect exercise, sparkles when passed, nothing when failed: the missed
/// keys are red on the keyboard then.
fn start_celebration(
    exercise: Res<Exercise>,
    progress: Res<crate::progress::Progress>,
    mut show: ResMut<Show>,
    mut was_end: Local<bool>,
) {
    let is_end = exercise.tutor.query == TutorQuery::End;
    if is_end && !*was_end && !progress.reduce_motion
        && let Some(results) = exercise.tutor.results
    {
        let celebration = if results.n_errors == 0 {
            Some(Celebration::Fireworks)
        } else if results.accuracy >= crate::game::goal(exercise.mode) {
            Some(Celebration::Sparkles)
        } else {
            None
        };
        *show = Show { celebration, age: 0.0, launched: 0, burst_done: false };
    }
    *was_end = is_end;
}

/// The colors of the fingers and the layers, the fireworks wear them all.
fn festive(theme: &UiTheme) -> Color {
    let tokens = [
        &tokens::FINGERS[1],
        &tokens::FINGERS[2],
        &tokens::FINGERS[3],
        &tokens::FINGERS[4],
        &tokens::FINGERS[5],
        &tokens::LAYERS[1],
        &tokens::LAYERS[2],
        &tokens::CURSOR,
    ];
    theme.color(tokens[fastrand::usize(..tokens.len())])
}

fn celebrate(
    time: Res<Time>,
    theme: Res<UiTheme>,
    window: Single<&Window>,
    messages: Query<(&UiGlobalTransform, &ComputedNode), With<MessageText>>,
    mut show: ResMut<Show>,
    mut commands: Commands,
) {
    let Some(celebration) = show.celebration else {
        return;
    };
    show.age += time.delta_secs();
    let (width, height) = (window.width(), window.height());
    // One soft burst from the comment, rising, once the comment is laid out: it shows up with
    // the end of the exercise.
    let message = messages.iter().find(|(_, node)| node.size().x > 0.0).map(|(transform, node)| center(transform, node));
    let burst = |commands: &mut Commands, at: Vec2, n: usize| {
        for _ in 0..n {
            let angle = -TAU / 4.0 + (fastrand::f32() - 0.5) * 2.4;
            let velocity = Vec2::from_angle(angle) * (90.0 + 140.0 * fastrand::f32());
            let start = at + Vec2::new((fastrand::f32() - 0.5) * 300.0, 0.0);
            spawn_spark(commands, start, velocity, 3.0 + 2.0 * fastrand::f32(), 1.1, festive(&theme), 60.0);
        }
    };
    match celebration {
        Celebration::Fireworks => {
            // The rockets go up from the bottom, one after the other.
            let due = ((show.age / FIREWORKS_SECONDS) * ROCKETS as f32).ceil() as u32;
            while show.launched < due.min(ROCKETS) {
                show.launched += 1;
                let x = width * (0.12 + 0.76 * fastrand::f32());
                commands.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(4),
                        height: px(4),
                        border_radius: BorderRadius::all(percent(50)),
                        ..default()
                    },
                    BackgroundColor(Color::WHITE),
                    GlobalZIndex(100),
                    Pickable::IGNORE,
                    Rocket {
                        position: Vec2::new(x, height),
                        velocity: Vec2::new((fastrand::f32() - 0.5) * 80.0, -(height * 0.9 + 200.0 * fastrand::f32())),
                        fuse: 0.55 + 0.35 * fastrand::f32(),
                        color: festive(&theme),
                    },
                ));
            }
            // The congratulations sparkle too, a lot.
            if !show.burst_done
                && show.age > 0.4
                && let Some(at) = message
            {
                burst(&mut commands, at, 60);
                show.burst_done = true;
            }
            if show.age > FIREWORKS_SECONDS {
                show.celebration = None;
            }
        }
        Celebration::Sparkles => {
            let Some(at) = message else {
                return;
            };
            burst(&mut commands, at, 28);
            show.celebration = None;
        }
    }
}

/// The lines typed without an error, once the cursor left them.
#[allow(clippy::too_many_arguments)]
fn detect_row_clears(
    exercise: Res<Exercise>,
    prompt: Res<PromptView>,
    progress: Res<crate::progress::Progress>,
    theme: Res<UiTheme>,
    lines: Query<(&Node, &UiGlobalTransform, &ComputedNode)>,
    mut commands: Commands,
    mut cleared: Local<(u64, Vec<bool>)>,
) {
    let tutor = &exercise.tutor;
    if !exercise.mode.is_typing() || tutor.states.len() != prompt.chars.len() || prompt.lines().is_empty() {
        return;
    }
    if cleared.0 != exercise.id || cleared.1.len() != prompt.lines().len() {
        *cleared = (exercise.id, vec![false; prompt.lines().len()]);
    }
    let ended = tutor.query == TutorQuery::End;
    for (i, (entity, start, end)) in prompt.lines().iter().enumerate() {
        if cleared.1[i] || !(ended || tutor.cursor >= *end) {
            continue;
        }
        cleared.1[i] = true;
        // The slips: the runs of characters not typed right at once, one slip with the keys
        // typed on before backspace.
        let slips = (*start..*end)
            .filter(|c| tutor.states[*c] != CharState::Correct && (*c == *start || tutor.states[*c - 1] == CharState::Correct))
            .count();
        let Ok((node, transform, computed)) = lines.get(*entity) else {
            continue;
        };
        if slips > 1 || ended {
            continue;
        }
        let at = center(transform, computed);
        // A word over the line, still with reduced motion.
        let (words, token) = if slips == 0 { (&CHEERS[..], tokens::CHAR_CORRECT) } else { (&ALMOST[..], tokens::NEAR_MISS) };
        let color = theme.color(&token);
        commands.spawn((
            Text::new(words[fastrand::usize(..words.len())]),
            TextFont { font_size: FontSize::Px(30.0), ..default() },
            TextColor(color),
            Node { position_type: PositionType::Absolute, left: px(at.x - 80.0), top: px(at.y - 62.0), ..default() },
            GlobalZIndex(120),
            Pickable::IGNORE,
            FloatText { age: 0.0, start: Vec2::new(at.x - 80.0, at.y - 62.0), color },
        ));
        if progress.reduce_motion {
            continue;
        }
        let height = match node.height {
            Val::Px(h) => h,
            _ => computed.size().y * computed.inverse_scale_factor(),
        };
        commands.entity(*entity).insert(RowClear { age: 0.0, height, color });
        if slips > 0 {
            continue;
        }
        // A small firework from the middle of the row, and sparks along it.
        let width = computed.size().x * computed.inverse_scale_factor();
        for k in 0..24 {
            let angle = TAU * k as f32 / 24.0 + fastrand::f32() * 0.2;
            let speed = 90.0 + 90.0 * fastrand::f32();
            spawn_spark(&mut commands, at, Vec2::from_angle(angle) * speed, 3.0 + 2.0 * fastrand::f32(), 0.9, festive(&theme), 80.0);
        }
        for _ in 0..16 {
            let x = (fastrand::f32() - 0.5) * width;
            let velocity = Vec2::new((fastrand::f32() - 0.5) * 40.0, -60.0 - 80.0 * fastrand::f32());
            spawn_spark(&mut commands, at + Vec2::new(x, 0.0), velocity, 2.0 + 2.0 * fastrand::f32(), 0.7, festive(&theme), 40.0);
        }
    }
}

fn animate_row_clears(
    time: Res<Time>,
    mut rows: Query<(Entity, &mut RowClear, &mut Node, &Children)>,
    mut chars: Query<(&PromptChar, &mut UiTransform, &mut BackgroundColor), Without<RowClear>>,
    mut commands: Commands,
) {
    for (entity, mut row, mut node, children) in &mut rows {
        row.age += time.delta_secs();
        let flash = row.color;
        let first = children.iter().find_map(|c| chars.get(c).ok().map(|(p, _, _)| p.0)).unwrap_or(0);
        for child in children.iter() {
            let Ok((PromptChar(index), mut transform, mut background)) = chars.get_mut(child) else {
                continue;
            };
            let t = row.age - (index - first) as f32 * ROW_STAGGER;
            // The flash, bright then fading, the characters swell a little.
            let glow = if t < 0.0 { 0.0 } else { 1.0 - (t / ROW_FLASH_SECONDS).min(1.0) };
            let swell = (t / ROW_FLASH_SECONDS).clamp(0.0, 1.0);
            let mut scale = 1.0 + 0.25 * (swell * TAU / 2.0).sin();
            // Then they pop away.
            let pop = ((t - ROW_FLASH_SECONDS) / ROW_POP_SECONDS).clamp(0.0, 1.0);
            scale *= 1.0 - pop * pop;
            background.0 = flash.with_alpha(0.85 * glow.max(0.5 * (1.0 - pop) * swell.min(1.0)));
            transform.scale = Vec2::splat(scale);
            transform.translation = Val2::px(0.0, -10.0 * pop);
        }
        // The rows below move up into its place.
        let span = ROW_FLASH_SECONDS + ROW_POP_SECONDS + ROW_STAGGER * children.len() as f32;
        let close = ((row.age - span) / ROW_CLOSE_SECONDS).clamp(0.0, 1.0);
        node.height = px(row.height * (1.0 - close * close * (3.0 - 2.0 * close)));
        if close >= 1.0 {
            commands.entity(entity).remove::<RowClear>();
        }
    }
}

fn animate_float_texts(
    time: Res<Time>,
    progress: Res<crate::progress::Progress>,
    mut texts: Query<(Entity, &mut FloatText, &mut Node, &mut TextColor)>,
    mut commands: Commands,
) {
    for (entity, mut float, mut node, mut color) in &mut texts {
        float.age += time.delta_secs();
        let k = float.age / FLOAT_SECONDS;
        if k >= 1.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let rise = if progress.reduce_motion { 0.0 } else { 36.0 * k };
        node.top = px(float.start.y - rise);
        // Shown, then fading in the second half.
        color.0 = float.color.with_alpha(if k < 0.5 { 1.0 } else { 2.0 * (1.0 - k) });
    }
}

fn animate_streaks(time: Res<Time>, mut streaks: Query<(Entity, &mut Streak, &mut Node, &mut BackgroundColor)>, mut commands: Commands) {
    let dt = time.delta_secs();
    for (entity, mut streak, mut node, mut color) in &mut streaks {
        streak.age += dt;
        if streak.age >= streak.life {
            commands.entity(entity).despawn();
            continue;
        }
        let velocity = streak.velocity;
        streak.position += velocity * dt;
        node.left = px(streak.position.x);
        node.top = px(streak.position.y);
        let k = streak.age / streak.life;
        color.0 = streak.color.with_alpha(0.7 * (1.0 - k));
    }
}

fn animate_rockets(time: Res<Time>, mut rockets: Query<(Entity, &mut Rocket, &mut Node)>, mut commands: Commands) {
    let dt = time.delta_secs();
    for (entity, mut rocket, mut node) in &mut rockets {
        rocket.fuse -= dt;
        let velocity = rocket.velocity;
        rocket.position += velocity * dt;
        rocket.velocity.y += 500.0 * dt;
        node.left = px(rocket.position.x);
        node.top = px(rocket.position.y);
        if rocket.fuse <= 0.0 {
            // The burst: a ring of sparks falling slowly.
            let n = 36;
            for i in 0..n {
                let angle = TAU * i as f32 / n as f32 + fastrand::f32() * 0.2;
                let speed = 120.0 + 120.0 * fastrand::f32();
                let size = 3.0 + 3.0 * fastrand::f32();
                spawn_spark(&mut commands, rocket.position, Vec2::from_angle(angle) * speed, size, 1.3, rocket.color, 90.0);
            }
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_sparks(commands: &mut Commands, at: Vec2, color: Color, error: bool) {
    let n = if error { 10 } else { 6 };
    for _ in 0..n {
        let angle = if error {
            fastrand::f32() * TAU
        } else {
            // Upwards, the signal goes from the key to the text.
            -TAU / 4.0 + (fastrand::f32() - 0.5) * 1.6
        };
        let speed = if error { 80.0 + 160.0 * fastrand::f32() } else { 120.0 + 160.0 * fastrand::f32() };
        let size = 3.0 + 3.0 * fastrand::f32();
        spawn_spark(commands, at, Vec2::from_angle(angle) * speed, size, if error { 0.35 } else { 0.5 }, color, 0.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn on_touch(
    mut touches: MessageReader<Touched>,
    keyboard: Res<KeyboardView>,
    prompt: Res<PromptView>,
    exercise: Res<Exercise>,
    theme: Res<UiTheme>,
    progress: Res<crate::progress::Progress>,
    nodes: Query<(&UiGlobalTransform, &ComputedNode)>,
    fingers: Query<(&crate::hands::Finger, &UiGlobalTransform, &ComputedNode)>,
    layout: Res<crate::game::ActiveLayout>,
    mut commands: Commands,
) {
    let calm = progress.reduce_motion;
    for touch in touches.read() {
        if let Some(key) = touch.pressed.and_then(|k| keyboard.node(k)) {
            commands.entity(key).insert(Pressed { age: 0.0 });
        }
        let (token, error) = match touch.result {
            TouchResult::Ok => (tokens::SPARK_OK, false),
            TouchResult::Wrong => (tokens::SPARK_ERROR, true),
            TouchResult::Other => continue,
        };
        // Reduced motion: the keys still sink when pressed, no sparks nor shakes.
        if calm {
            continue;
        }
        if let Some(key) = touch.expected.and_then(|k| keyboard.node(k))
            && let Ok((transform, node)) = nodes.get(key)
        {
            spawn_sparks(&mut commands, center(transform, node), theme.color(&token), error);
        }
        // The lines stream out of the finger that missed, in its color.
        if error
            && let Some(spec) = touch.expected.and_then(|k| layout.0.keys.get(k))
            && let Some((transform, node)) = fingers
                .iter()
                .find(|(f, _, _)| f.finger == spec.finger && (spec.finger != 5 || f.left == spec.left))
                .map(|(_, t, n)| (t, n))
        {
            let at = center(transform, node);
            let color = theme.color(&tokens::FINGERS[spec.finger.min(9) as usize]);
            for _ in 0..9 {
                let angle = -TAU / 4.0 + (fastrand::f32() - 0.5) * 2.2;
                let speed = 160.0 + 220.0 * fastrand::f32();
                let length = 18.0 + 30.0 * fastrand::f32();
                commands.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(at.x),
                        top: px(at.y),
                        width: px(2),
                        height: px(length),
                        ..default()
                    },
                    // Along its way.
                    UiTransform { rotation: Rot2::radians(angle - TAU / 4.0), ..default() },
                    BackgroundColor(color),
                    GlobalZIndex(90),
                    Pickable::IGNORE,
                    Streak { position: at, velocity: Vec2::from_angle(angle) * speed, age: 0.0, life: 1.1, color },
                ));
            }
        }
        if !touch.shake || exercise.tutor.states.is_empty() {
            continue;
        }
        // The wrong character is the last one before the cursor.
        let tutor = &exercise.tutor;
        let wrong = (0..=tutor.cursor.min(tutor.states.len() - 1))
            .rev()
            .find(|i| tutor.states[*i] == CharState::Wrong);
        if let Some(entity) = wrong.and_then(|i| prompt.char_entity(i)) {
            commands.entity(entity).insert(Shake { age: 0.0 });
        }
    }
}

fn fade_in_message(
    exercise: Res<Exercise>,
    progress: Res<crate::progress::Progress>,
    messages: Query<Entity, With<MessageText>>,
    mut commands: Commands,
    mut was_end: Local<bool>,
) {
    let is_end = exercise.tutor.query == TutorQuery::End;
    if is_end && !*was_end && !progress.reduce_motion {
        for entity in &messages {
            commands.entity(entity).insert((FadeIn { age: 0.0 }, UiTransform::default()));
        }
    }
    *was_end = is_end;
}

/// The pressed keys sink and come back, the next key breathes.
fn animate_keys(
    time: Res<Time>,
    progress: Res<crate::progress::Progress>,
    mut keys: Query<(Entity, &KeyCap, &mut UiTransform, Option<&mut Pressed>)>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    for (entity, cap, mut transform, pressed) in &mut keys {
        let mut scale = 1.0;
        if matches!(cap.role, KeyRole::Target | KeyRole::Missed) && !progress.reduce_motion {
            scale += 0.045 * (0.5 + 0.5 * (t * 5.0).sin());
        }
        if let Some(mut pressed) = pressed {
            pressed.age += dt;
            let k = (pressed.age / PRESS_SECONDS).min(1.0);
            scale *= 0.86 + 0.14 * (k * TAU / 4.0).sin();
            if pressed.age >= PRESS_SECONDS {
                commands.entity(entity).remove::<Pressed>();
            }
        }
        let scale = Vec2::splat(scale);
        if transform.scale != scale {
            transform.scale = scale;
        }
    }
}

fn animate_shakes(time: Res<Time>, mut shakes: Query<(Entity, &mut Shake, &mut UiTransform)>, mut commands: Commands) {
    for (entity, mut shake, mut transform) in &mut shakes {
        shake.age += time.delta_secs();
        let decay = 1.0 - (shake.age / SHAKE_SECONDS).min(1.0);
        transform.translation = Val2::px(4.0 * (shake.age * 70.0).sin() * decay, 0.0);
        if shake.age >= SHAKE_SECONDS {
            transform.translation = Val2::ZERO;
            commands.entity(entity).remove::<Shake>();
        }
    }
}

fn animate_sparks(time: Res<Time>, mut sparks: Query<(Entity, &mut Spark, &mut Node, &mut BackgroundColor)>, mut commands: Commands) {
    let dt = time.delta_secs();
    for (entity, mut spark, mut node, mut color) in &mut sparks {
        spark.age += dt;
        if spark.age >= spark.life {
            commands.entity(entity).despawn();
            continue;
        }
        let velocity = spark.velocity;
        spark.position += velocity * dt;
        spark.velocity *= 1.0 - 2.0 * dt;
        spark.velocity.y += spark.gravity * dt;
        node.left = px(spark.position.x);
        node.top = px(spark.position.y);
        color.0 = spark.color.with_alpha(1.0 - spark.age / spark.life);
    }
}

fn pulse_cursor(
    time: Res<Time>,
    exercise: Res<Exercise>,
    prompt: Res<PromptView>,
    theme: Res<UiTheme>,
    progress: Res<crate::progress::Progress>,
    mut borders: Query<&mut BorderColor>,
) {
    if exercise.tutor.query == TutorQuery::End {
        return;
    }
    if let Some(entity) = prompt.char_entity(exercise.tutor.cursor)
        && let Ok(mut border) = borders.get_mut(entity)
    {
        let pulse = 0.5 + 0.5 * (time.elapsed_secs() * TAU / 1.2).cos();
        let alpha = if progress.reduce_motion { 1.0 } else { 0.3 + 0.7 * pulse };
        border.set_all(theme.color(&tokens::CURSOR).with_alpha(alpha));
    }
}

fn animate_fades(
    time: Res<Time>,
    mut fades: Query<(Entity, &mut FadeIn, &mut UiTransform, Option<&mut TextColor>)>,
    mut commands: Commands,
) {
    for (entity, mut fade, mut transform, color) in &mut fades {
        fade.age += time.delta_secs();
        let k = (fade.age / FADE_SECONDS).min(1.0);
        transform.translation = Val2::px(0.0, 6.0 * (1.0 - k));
        if let Some(mut color) = color {
            color.0 = color.0.with_alpha(k);
        }
        if k >= 1.0 {
            commands.entity(entity).remove::<FadeIn>();
        }
    }
}
