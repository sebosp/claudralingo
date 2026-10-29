//! The animations, inspired by layer-tutor: the pressed keys sink, the next key breathes, the
//! correct touches send sparks up from the key, the errors short circuit it and shake the
//! character, the cursor pulses and the comments fade in.
use std::f32::consts::TAU;

use bevy::feathers::theme::UiTheme;
use bevy::prelude::*;

use crate::game::{Exercise, TouchResult, Touched};
use crate::keyboard::{KeyCap, KeyRole, KeyboardView};
use crate::klavaro::tutor::{CharState, Query as TutorQuery};
use crate::prompt::PromptView;
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
}

#[derive(Component)]
struct FadeIn {
    age: f32,
}

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                on_touch,
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
            Spark {
                position: at,
                velocity: Vec2::from_angle(angle) * speed,
                age: 0.0,
                life: if error { 0.35 } else { 0.5 },
                color,
            },
        ));
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
        if cap.role == KeyRole::Target && !progress.reduce_motion {
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
        spark.velocity *= 1.0 - 3.0 * dt;
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
