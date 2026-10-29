//! The keyboard on the screen: the next key in green, the shift in purple, the layer and the
//! modifier keys to hold in amber and backspace in red while there are errors to erase, like
//! layer-tutor.
//!
//! The keys sit where the physical layout puts them, rotated thumbs included, and are drawn by
//! [`KeyMaterial`] so the rotated ones keep smooth edges. The legends fade out while the typing
//! goes well, the layout and the colors stay. A wrong key or a pause shows the next key again,
//! with its legend. The layer trainer always shows the layer and the modifier keys to hold, with
//! the instructions above the keyboard. A click on a layer key shows the legends of its layer.
use bevy::asset::embedded_asset;
use bevy::feathers::constants::fonts;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeToken, UiTheme};
use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::text::FontSource;

use crate::game::{ActiveLayout, Coach};
use crate::klavaro::BACKSPACE;
use crate::layout::{Layout, Plan, Special, Target};
use crate::theme::tokens;
use crate::ui::{KeyboardRoot, TrainerHint};

/// The space between two keys, in logical pixels.
const GAP: f32 = 5.0;
/// The transparent margin of the key nodes, the anti-aliasing of the edges is drawn in it.
const MARGIN: f32 = 2.0;
const RADIUS: f32 = 8.0;
const BORDER: f32 = 1.0;
/// The padding of the cases around the keys.
const PADDING: f32 = 12.0;
/// Below this fade the next key is not highlighted any more.
const HINT_FADE: f32 = 0.35;
/// The fade of the legends follows the coach at this rate per second.
const FADE_RATE: f32 = 3.0;

/// A key cap, see `key.wgsl`.
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct KeyMaterial {
    #[uniform(0)]
    fill: Vec4,
    #[uniform(1)]
    border: Vec4,
    /// The corner radius, the border width, the margin and the width of the node, in logical
    /// pixels.
    #[uniform(2)]
    shape: Vec4,
    /// The color of the finger.
    #[uniform(3)]
    stripe: Vec4,
}

impl UiMaterial for KeyMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://claudralingo/key.wgsl".into()
    }
}

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
#[derive(Debug, Clone)]
pub struct KeyEntities {
    pub node: Entity,
    pub legend: Entity,
    pub hold: Option<Entity>,
    material: Handle<KeyMaterial>,
    /// The fill, border and finger colors in the material, it is only touched when they change.
    colors: Option<(Color, Color, Color)>,
}

#[derive(Resource, Default)]
pub struct KeyboardView {
    /// By the index of the key in the layout.
    pub keys: Vec<KeyEntities>,
    built: bool,
    legends: Option<(u8, bool)>,
    /// The fade of the legends on the screen, it follows the coach smoothly.
    shown_fade: f32,
    /// The layer shown after a click on its layer key.
    pub preview: Option<u8>,
}

impl KeyboardView {
    pub fn node(&self, key: usize) -> Option<Entity> {
        self.keys.get(key).map(|k| k.node)
    }
}

pub struct KeyboardPlugin;

impl Plugin for KeyboardPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "key.wgsl");
        app.add_plugins(UiMaterialPlugin::<KeyMaterial>::default())
            .init_resource::<KeyboardView>()
            .add_systems(Update, (rebuild_keyboard, update_hints).chain().after(crate::game::GameSystems));
    }
}

fn rebuild_keyboard(
    layout: Res<ActiveLayout>,
    mut view: ResMut<KeyboardView>,
    roots: Query<Entity, With<KeyboardRoot>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<KeyMaterial>>,
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
    let mut keys: Vec<Option<KeyEntities>> = vec![None; layout.keys.len()];
    for block in &layout.blocks {
        let (x0, y0, x1, y1) = block.bounds(&layout.keys);
        let block_node = commands
            .spawn((
                Node {
                    width: px((x1 - x0) * layout.unit - GAP + 2.0 * PADDING),
                    height: px((y1 - y0) * layout.unit - GAP + 2.0 * PADDING),
                    border_radius: BorderRadius::all(px(16)),
                    ..default()
                },
                ThemeBackgroundColor(tokens::KEYBOARD_CASE),
                ChildOf(root),
            ))
            .id();
        for &index in &block.keys {
            let origin = (x0, y0);
            keys[index] = Some(spawn_key(&mut commands, &mut materials, layout, index, origin, block_node, font.clone()));
        }
    }
    *view = KeyboardView {
        keys: keys.into_iter().flatten().collect(),
        built: true,
        legends: None,
        shown_fade: 1.0,
        preview: None,
    };
}

#[allow(clippy::too_many_arguments)]
fn spawn_key(
    commands: &mut Commands,
    materials: &mut Assets<KeyMaterial>,
    layout: &Layout,
    index: usize,
    (x0, y0): (f32, f32),
    parent: Entity,
    font: FontSource,
) -> KeyEntities {
    let key = &layout.keys[index];
    let unit = layout.unit;
    let pos = key.pos;
    // The node is the key cap and its margin, centered where the physical layout puts the key.
    let (width, height) = (pos.w * unit - GAP + 2.0 * MARGIN, pos.h * unit - GAP + 2.0 * MARGIN);
    let (cx, cy) = pos.center();
    let center = ((cx - x0) * unit - GAP / 2.0 + PADDING, (cy - y0) * unit - GAP / 2.0 + PADDING);
    let material = materials.add(KeyMaterial {
        fill: Vec4::ZERO,
        border: Vec4::ZERO,
        shape: Vec4::new(RADIUS, BORDER, MARGIN, width),
        stripe: Vec4::ZERO,
    });
    let node = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(center.0 - width / 2.0),
                top: px(center.1 - height / 2.0),
                width: px(width),
                height: px(height),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            MaterialNode(material.clone()),
            UiTransform::from_rotation(Rot2::degrees(pos.rot)),
            KeyCap { index, role: KeyRole::Idle },
            ChildOf(parent),
        ))
        .id();
    if let Some(Special::Layer(layer)) = key.special {
        commands.entity(node).observe(move |_: On<PointerClick>, mut view: ResMut<KeyboardView>| {
            view.preview = if view.preview == Some(layer) { None } else { Some(layer) };
        });
    }
    let legend = key.legend(0, false, &layout.layer_names);
    let font_size = if legend.chars().count() > 2 { 12.0 } else { 19.0 };
    // The text colors are set every frame, with the fade.
    let legend = commands
        .spawn((
            Text::new(legend),
            TextFont { font: font.clone(), font_size: FontSize::Px(font_size), ..default() },
            TextColor(Color::NONE),
            Pickable::IGNORE,
            ChildOf(node),
        ))
        .id();
    let hold = key.hold_legend().map(|hold| {
        commands
            .spawn((
                Text::new(hold),
                TextFont { font: font.clone(), font_size: FontSize::Px(9.0), ..default() },
                TextColor(Color::NONE),
                Node { position_type: PositionType::Absolute, top: px(2.0 + MARGIN), right: px(4.0 + MARGIN), ..default() },
                Pickable::IGNORE,
                ChildOf(node),
            ))
            .id()
    });
    KeyEntities { node, legend, hold, material, colors: None }
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

fn to_vec4(color: Color) -> Vec4 {
    LinearRgba::from(color).to_vec4()
}

#[allow(clippy::too_many_arguments)]
fn update_hints(
    coach: Res<Coach>,
    layout: Res<ActiveLayout>,
    theme: Res<UiTheme>,
    time: Res<Time>,
    mut view: ResMut<KeyboardView>,
    mut materials: ResMut<Assets<KeyMaterial>>,
    mut caps: Query<&mut KeyCap>,
    mut texts: Query<(&mut Text, &mut TextColor), Without<TrainerHint>>,
    mut trainer: Query<&mut Text, With<TrainerHint>>,
) {
    let layout = &layout.0;
    if view.keys.len() != layout.keys.len() {
        return;
    }
    let step = FADE_RATE * time.delta_secs();
    view.shown_fade += (coach.fade - view.shown_fade).clamp(-step, step);
    let preview = view.preview;
    // The legends of a previewed layer are all shown.
    let fade = if preview.is_some() { 1.0 } else { view.shown_fade };

    let (mut roles, plan) = hints(layout, coach.target);
    let trainer_applies = (coach.layer_trainer && plan.as_ref().is_some_and(Plan::is_combo)) || coach.always_show;
    let show_plan = coach.reveal || trainer_applies || coach.fade > HINT_FADE;
    let in_plan = |key: usize| plan.as_ref().is_some_and(|p| p.keys().any(|k| k == key));
    if let Some(layer) = preview {
        for key in layout.find_special(Special::Layer(layer)) {
            roles[key] = KeyRole::Hold;
        }
    }
    let border = theme.color(&tokens::KEY_BORDER);

    for (index, role) in roles.iter().enumerate() {
        // Backspace is always shown while there are errors to erase.
        let previewed = preview.is_some() && *role == KeyRole::Hold && !in_plan(index);
        let role = if show_plan || *role == KeyRole::Erase || previewed { *role } else { KeyRole::Idle };
        let entities = &mut view.keys[index];
        // The keys spawned this frame are not queryable yet, they start idle.
        if let Ok(mut cap) = caps.get_mut(entities.node)
            && cap.role != role
        {
            cap.role = role;
        }
        let (fill_token, text_token) = role.tokens();
        // The finger stripe of Klavaro's colored keyboard.
        let finger = theme.color(&tokens::FINGERS[layout.keys[index].finger.min(9) as usize]);
        let colors = (theme.color(&fill_token), border, finger);
        if entities.colors != Some(colors)
            && let Some(mut material) = materials.get_mut(&entities.material)
        {
            material.fill = to_vec4(colors.0);
            material.border = to_vec4(colors.1);
            material.stripe = to_vec4(colors.2);
            entities.colors = Some(colors);
        }
        let alpha = if role != KeyRole::Idle && (in_plan(index) || role == KeyRole::Erase || previewed) {
            1.0
        } else {
            fade
        };
        let color = theme.color(&text_token).with_alpha(alpha);
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

    // The legends of the previewed layer, or of the layer, and shifted, of the next key.
    let legends = match (&plan, preview) {
        (_, Some(layer)) => (layer, false),
        (Some(plan), None) if show_plan => (plan.layer, plan.shift),
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

    let instructions = match (&plan, preview) {
        (_, Some(layer)) => {
            let name = Special::Layer(layer).name(&layout.layer_names);
            format!("The {name} layer: what the keys type while {name} is held, click {name} again to hide it")
        }
        (Some(plan), None) if trainer_applies => layout.describe(plan),
        _ => String::new(),
    };
    for mut text in &mut trainer {
        if text.0 != instructions {
            text.0.clone_from(&instructions);
        }
    }
}
