//! The keyboard on the screen: the next key in green, the shift in purple, the layer key to hold
//! in amber and backspace in red while there are errors to erase, like layer-tutor.
use bevy::feathers::constants::fonts;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemeToken, ThemedText};
use bevy::prelude::*;
use bevy::text::FontSource;

use crate::game::{ActiveLayout, Exercise};
use crate::klavaro::BACKSPACE;
use crate::layout::{Layout, Special};
use crate::theme::tokens;
use crate::ui::KeyboardRoot;

const GAP: f32 = 5.0;

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

#[derive(Resource, Default)]
pub struct KeyboardView {
    /// The key nodes and their legends, by the index of the key in the layout.
    pub keys: Vec<(Entity, Entity)>,
    built: bool,
    legends: Option<(u8, bool)>,
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
    let mut keys = vec![(Entity::PLACEHOLDER, Entity::PLACEHOLDER); layout.keys.len()];
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
    *view = KeyboardView { keys, built: true, legends: None };
}

fn spawn_key(commands: &mut Commands, layout: &Layout, index: usize, parent: Entity, font: FontSource) -> (Entity, Entity) {
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
    let font_size = if legend.chars().count() > 2 { 13.0 } else { 19.0 };
    let legend = commands
        .spawn((
            Text::new(legend),
            TextFont {
                font,
                font_size: FontSize::Px(font_size),
                ..default()
            },
            ThemedText,
            ThemeTextColor(tokens::KEY_TEXT),
            ChildOf(node),
        ))
        .id();
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
    (node, legend)
}

/// The roles of the keys for the next character, with the layer and shift of the legends.
pub fn hints(layout: &Layout, expected: Option<char>) -> (Vec<KeyRole>, u8, bool) {
    let mut roles = vec![KeyRole::Idle; layout.keys.len()];
    let Some(expected) = expected else {
        return (roles, 0, false);
    };
    if expected == BACKSPACE {
        for key in layout.find_special(Special::Backspace) {
            roles[key] = KeyRole::Erase;
        }
        return (roles, 0, false);
    }
    let Some(location) = layout.locate(expected) else {
        return (roles, 0, false);
    };
    roles[location.key] = KeyRole::Target;
    if location.shift
        && let Some(shift) = layout.shift_for(location.key)
    {
        roles[shift] = KeyRole::Shift;
    }
    if location.layer > 0
        && let Some(hold) = layout.layer_key_for(location.key, location.layer)
    {
        roles[hold] = KeyRole::Hold;
    }
    (roles, location.layer, location.shift)
}

fn update_hints(
    exercise: Res<Exercise>,
    layout: Res<ActiveLayout>,
    mut view: ResMut<KeyboardView>,
    caps: Query<&KeyCap>,
    mut texts: Query<&mut Text>,
    mut commands: Commands,
) {
    if !exercise.is_changed() && !layout.is_changed() && view.legends.is_some() {
        return;
    }
    let layout = &layout.0;
    if view.keys.len() != layout.keys.len() {
        return;
    }
    let (roles, layer, shift) = hints(layout, exercise.tutor.expected());
    for (index, role) in roles.iter().enumerate() {
        let (node, legend) = view.keys[index];
        // The keys spawned this frame are not queryable yet, they start idle.
        let previous = caps.get(node).map(|cap| cap.role).unwrap_or_default();
        if previous == *role && caps.contains(node) {
            continue;
        }
        let (bg, text) = role.tokens();
        commands.entity(node).insert((ThemeBackgroundColor(bg), KeyCap { index, role: *role }));
        commands.entity(legend).insert(ThemeTextColor(text));
    }
    if view.legends != Some((layer, shift)) {
        for (index, key) in layout.keys.iter().enumerate() {
            if let Ok(mut text) = texts.get_mut(view.keys[index].1) {
                text.0 = key.legend(layer, shift, &layout.layer_names);
            } else {
                // Spawned this frame, the legends are refreshed on the next change.
                view.legends = None;
                return;
            }
        }
        view.legends = Some((layer, shift));
    }
}
