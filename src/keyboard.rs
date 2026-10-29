//! The keyboard on the screen: the next key in green, the shift in purple, the layer and the
//! modifier keys to hold in amber and backspace in red while there are errors to erase, like
//! layer-tutor.
//!
//! The legends fade out while the typing goes well, the layout and the colors stay. A wrong key
//! or a pause shows the next key again, with its legend. The layer trainer always shows the
//! layer and the modifier keys to hold, with the instructions above the keyboard.
use bevy::feathers::constants::fonts;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeToken, UiTheme};
use bevy::prelude::*;
use bevy::text::FontSource;

use crate::game::{ActiveLayout, Coach};
use crate::klavaro::BACKSPACE;
use crate::layout::{Layout, Plan, Special, Target};
use crate::theme::tokens;
use crate::ui::{KeyboardRoot, TrainerHint};

const GAP: f32 = 5.0;
/// Below this fade the next key is not highlighted any more.
const HINT_FADE: f32 = 0.35;
/// The fade of the legends follows the coach at this rate per second.
const FADE_RATE: f32 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyRole {
    #[default]
    Idle,
    Target,
    Shift,
    Hold,
    Erase,
}

impl KeyRole {
    fn tokens(self) -> (ThemeToken, ThemeToken) {
        match self {
            KeyRole::Idle => (tokens::KEY_BG, tokens::KEY_TEXT),
            KeyRole::Target => (tokens::KEY_TARGET, tokens::KEY_TEXT_ON_HINT),
            KeyRole::Shift => (tokens::KEY_SHIFT, tokens::KEY_TEXT_ON_HINT),
            KeyRole::Hold => (tokens::KEY_HOLD, tokens::KEY_TEXT_ON_HINT),
            KeyRole::Erase => (tokens::KEY_ERASE, tokens::KEY_TEXT_ON_HINT),
        }
    }
}

#[derive(Component)]
pub struct KeyCap {
    pub index: usize,
    pub role: KeyRole,
}

/// The node, the legend and the home row mod legend of a key.
#[derive(Debug, Clone, Copy)]
pub struct KeyEntities {
    pub node: Entity,
    pub legend: Entity,
    pub hold: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct KeyboardView {
    /// By the index of the key in the layout.
    pub keys: Vec<KeyEntities>,
    built: bool,
    legends: Option<(u8, bool)>,
    /// The fade of the legends on the screen, it follows the coach smoothly.
    shown_fade: f32,
}

impl KeyboardView {
    pub fn node(&self, key: usize) -> Option<Entity> {
        self.keys.get(key).map(|k| k.node)
    }
}

pub struct KeyboardPlugin;

impl Plugin for KeyboardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyboardView>()
            .add_systems(Update, (rebuild_keyboard, update_hints).chain().after(crate::game::GameSystems));
    }
}

fn rebuild_keyboard(
    layout: Res<ActiveLayout>,
    mut view: ResMut<KeyboardView>,
    roots: Query<Entity, With<KeyboardRoot>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let Ok(root) = roots.single() else {
        return;
    };
    if view.built && !layout.is_changed() {
        return;
    }
    commands.entity(root).despawn_children();
    let layout = &layout.0;
    let font = FontSource::Handle(asset_server.load(fonts::MONO));
    let placeholder = KeyEntities { node: Entity::PLACEHOLDER, legend: Entity::PLACEHOLDER, hold: None };
    let mut keys = vec![placeholder; layout.keys.len()];
    for block in &layout.blocks {
        let block_node = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(GAP),
                    padding: UiRect::all(px(12)),
                    border_radius: BorderRadius::all(px(16)),
                    ..default()
                },
                ThemeBackgroundColor(tokens::KEYBOARD_CASE),
                UiTransform::from_rotation(Rot2::degrees(block.tilt)),
                ChildOf(root),
            ))
            .id();
        for (offset, row) in &block.rows {
            let row_node = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Start,
                        column_gap: px(GAP),
                        margin: UiRect::left(px(offset * layout.unit)),
                        ..default()
                    },
                    ChildOf(block_node),
                ))
                .id();
            for &index in row {
                keys[index] = spawn_key(&mut commands, layout, index, row_node, font.clone());
            }
        }
    }
    *view = KeyboardView { keys, built: true, legends: None, shown_fade: 1.0 };
}

fn spawn_key(commands: &mut Commands, layout: &Layout, index: usize, parent: Entity, font: FontSource) -> KeyEntities {
    let key = &layout.keys[index];
    let unit = layout.unit;
    let node = commands
        .spawn((
            Node {
                width: px(key.width * unit - GAP),
                height: px(unit - GAP),
                margin: UiRect::top(px(key.y_offset * unit)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            ThemeBackgroundColor(tokens::KEY_BG),
            ThemeBorderColor(tokens::KEY_BORDER),
            UiTransform::default(),
            KeyCap { index, role: KeyRole::Idle },
            ChildOf(parent),
        ))
        .id();
    let legend = key.legend(0, false, &layout.layer_names);
    let font_size = if legend.chars().count() > 2 { 12.0 } else { 19.0 };
    // The text colors are set every frame, with the fade.
    let legend = commands
        .spawn((
            Text::new(legend),
            TextFont { font: font.clone(), font_size: FontSize::Px(font_size), ..default() },
            TextColor(Color::NONE),
            ChildOf(node),
        ))
        .id();
    let hold = key.hold_legend().map(|hold| {
        commands
            .spawn((
                Text::new(hold),
                TextFont { font: font.clone(), font_size: FontSize::Px(9.0), ..default() },
                TextColor(Color::NONE),
                Node { position_type: PositionType::Absolute, top: px(2), right: px(4), ..default() },
                ChildOf(node),
            ))
            .id()
    });
    // The finger stripe of Klavaro's colored keyboard.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(4),
            width: percent(36),
            height: px(3),
            border_radius: BorderRadius::all(px(2)),
            ..default()
        },
        ThemeBackgroundColor(tokens::FINGERS[key.finger.min(9) as usize].clone()),
        ChildOf(node),
    ));
    KeyEntities { node, legend, hold }
}

/// The roles of the keys for the next target, with its plan.
pub fn hints(layout: &Layout, target: Option<Target>) -> (Vec<KeyRole>, Option<Plan>) {
    let mut roles = vec![KeyRole::Idle; layout.keys.len()];
    let Some(target) = target else {
        return (roles, None);
    };
    if target == Target::Char(BACKSPACE) {
        for key in layout.find_special(Special::Backspace) {
            roles[key] = KeyRole::Erase;
        }
        return (roles, None);
    }
    let Some(plan) = layout.plan(target) else {
        return (roles, None);
    };
    // Purple: the key itself is held for the shift.
    roles[plan.key] = if plan.auto_shift { KeyRole::Shift } else { KeyRole::Target };
    if let Some(shift) = plan.shift_key {
        roles[shift] = KeyRole::Shift;
    }
    if let Some(hold) = plan.layer_key {
        roles[hold] = KeyRole::Hold;
    }
    for (_, key) in &plan.mods {
        roles[*key] = KeyRole::Hold;
    }
    (roles, Some(plan))
}

#[allow(clippy::too_many_arguments)]
fn update_hints(
    coach: Res<Coach>,
    layout: Res<ActiveLayout>,
    theme: Res<UiTheme>,
    time: Res<Time>,
    mut view: ResMut<KeyboardView>,
    mut caps: Query<&mut KeyCap>,
    mut texts: Query<(&mut Text, &mut TextColor), Without<TrainerHint>>,
    mut trainer: Query<&mut Text, With<TrainerHint>>,
    mut commands: Commands,
) {
    let layout = &layout.0;
    if view.keys.len() != layout.keys.len() {
        return;
    }
    let step = FADE_RATE * time.delta_secs();
    view.shown_fade += (coach.fade - view.shown_fade).clamp(-step, step);
    let fade = view.shown_fade;

    let (roles, plan) = hints(layout, coach.target);
    let trainer_applies = (coach.layer_trainer && plan.as_ref().is_some_and(Plan::is_combo)) || coach.always_show;
    let show_plan = coach.reveal || trainer_applies || coach.fade > HINT_FADE;
    let in_plan = |key: usize| plan.as_ref().is_some_and(|p| p.keys().any(|k| k == key));

    for (index, role) in roles.iter().enumerate() {
        // Backspace is always shown while there are errors to erase.
        let role = if show_plan || *role == KeyRole::Erase { *role } else { KeyRole::Idle };
        let entities = view.keys[index];
        // The keys spawned this frame are not queryable yet, they start idle.
        if let Ok(mut cap) = caps.get_mut(entities.node)
            && cap.role != role
        {
            cap.role = role;
            commands.entity(entities.node).insert(ThemeBackgroundColor(role.tokens().0));
        }
        let alpha = if role != KeyRole::Idle && (in_plan(index) || role == KeyRole::Erase) && show_plan {
            1.0
        } else {
            fade
        };
        let color = theme.color(&role.tokens().1).with_alpha(alpha);
        if let Ok((_, mut text_color)) = texts.get_mut(entities.legend)
            && text_color.0 != color
        {
            text_color.0 = color;
        }
        if let Some(hold) = entities.hold
            && let Ok((_, mut text_color)) = texts.get_mut(hold)
        {
            // The home row mods are shown when they are held, and dimmed with the legends.
            let alpha = if role == KeyRole::Hold { 1.0 } else { 0.55 * fade };
            let color = color.with_alpha(alpha);
            if text_color.0 != color {
                text_color.0 = color;
            }
        }
    }

    // The legends of the layer, and shifted, of the next key.
    let legends = match &plan {
        Some(plan) if show_plan => (plan.layer, plan.shift),
        _ => (0, false),
    };
    if view.legends != Some(legends) {
        for (index, key) in layout.keys.iter().enumerate() {
            if let Ok((mut text, _)) = texts.get_mut(view.keys[index].legend) {
                text.0 = key.legend(legends.0, legends.1, &layout.layer_names);
            } else {
                // Spawned this frame, the legends are refreshed on the next frame.
                return;
            }
        }
        view.legends = Some(legends);
    }

    let instructions = match &plan {
        Some(plan) if trainer_applies => layout.describe(plan),
        _ => String::new(),
    };
    for mut text in &mut trainer {
        if text.0 != instructions {
            text.0.clone_from(&instructions);
        }
    }
}
