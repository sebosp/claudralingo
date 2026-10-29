//! The keyboard on the screen: the next key in green, the shift in purple, the layer and the
//! modifier keys to hold in amber and backspace in red while there are errors to erase, like
//! layer-tutor.
//!
//! The keys sit where the physical layout puts them, rotated thumbs included, and are drawn by
//! [`KeyMaterial`] so the rotated ones keep smooth edges. The legends fade out while the typing
//! goes well, the layout and the colors stay. A wrong key or a pause shows the next key again,
//! with its legend. The layer trainer always shows the layer and the modifier keys to hold, with
//! the instructions above the keyboard.
//!
//! It is an on-screen keyboard too: a click types the key, a long press types it shifted (as
//! the auto shift of the firmware), a click on a layer key shows and uses its layer until the
//! next click, holding it uses it while held (with a second finger on a touch screen), a click on
//! Ctrl, Alt, Win or Shift applies it to the next key. The clicks only make key presses for the
//! game, nothing reaches the system: Win does not open its menu.
use bevy::asset::embedded_asset;
use bevy::feathers::constants::fonts;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeToken, UiTheme};
use bevy::picking::events::{PointerClick, PointerPress, PointerRelease};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::text::FontSource;

use crate::game::{ActiveLayout, Coach, KeyPressed};
use crate::klavaro::{HYPER_BACKSPACE, UPSYM};
use crate::klavaro::BACKSPACE;
use crate::layout::{Chord, KeyName, Layout, Mods, Output, Plan, Special, Target};
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
/// A longer press of an on-screen key types it shifted, ZMK's `tapping_term_ms` of the auto
/// shift.
pub const TAPPING_TERM: f32 = 0.2;
/// The legends of the keys that type what the base layer types, on the other layers.
const INHERITED_ALPHA: f32 = 0.4;

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
    /// The layer shown and used after a click on its layer key.
    pub preview: Option<u8>,
    /// The layer of an on-screen layer key held down.
    pub held_layer: Option<u8>,
    /// The modifiers clicked on the screen, for the next key.
    pub sticky: Mods,
    /// The keys clicked, with a long press or not, for [`on_screen_keys`].
    clicks: Vec<(usize, bool)>,
}

impl KeyboardView {
    /// Clicks an on-screen key, as the pointer does.
    pub fn click(&mut self, key: usize, long: bool) {
        self.clicks.push((key, long));
    }

    /// The layer of the keys on the screen, `None` for the layer of the next key.
    pub fn shown_layer(&self) -> Option<u8> {
        self.held_layer.or(self.preview)
    }
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
        held_layer: None,
        sticky: Mods::NONE,
        clicks: Vec::new(),
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
    commands.entity(node).observe(move |click: On<PointerClick>, mut view: ResMut<KeyboardView>| {
        if click.button == PointerButton::Primary {
            view.clicks.push((index, click.duration.as_secs_f32() >= TAPPING_TERM));
        }
    });
    if let Some(Special::Layer(layer)) = key.special {
        commands
            .entity(node)
            .observe(move |press: On<PointerPress>, mut view: ResMut<KeyboardView>| {
                if press.button == PointerButton::Primary {
                    view.held_layer = Some(layer);
                }
            })
            .observe(move |_: On<PointerRelease>, mut view: ResMut<KeyboardView>| {
                if view.held_layer == Some(layer) {
                    view.held_layer = None;
                }
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
pub fn hints(layout: &Layout, target: Option<Target>, shown_layer: Option<u8>) -> (Vec<KeyRole>, Option<Plan>) {
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
    let plan = shown_layer.and_then(|layer| layout.plan_on(target, layer)).or_else(|| layout.plan(target));
    let Some(plan) = plan else {
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

fn press_special(special: Special, mods: Mods) -> KeyPressed {
    let plain = !(mods.ctrl || mods.alt || mods.super_key);
    let typed = match special {
        Special::Backspace if mods.ctrl => Some(HYPER_BACKSPACE),
        Special::Backspace => Some(BACKSPACE),
        Special::Enter if plain => Some(UPSYM),
        Special::Space if plain => Some(' '),
        _ => None,
    };
    KeyPressed { typed, chord: Some(Chord::new(mods, KeyName::Special(special))) }
}

fn press_char(out: Output, long: bool, mods: Mods) -> KeyPressed {
    let shifted = long || mods.shift;
    if mods.ctrl || mods.alt || mods.super_key {
        let mods = Mods { shift: shifted, ..mods };
        return KeyPressed { typed: None, chord: Some(Chord::new(mods, KeyName::Char(out.lo))) };
    }
    let c = if shifted { out.up } else { out.lo };
    KeyPressed { typed: Some(c), chord: Some(Chord::char(c)) }
}

/// The keys clicked on the screen become key presses of the game, on the layer shown.
pub fn on_screen_keys(
    mut view: ResMut<KeyboardView>,
    layout: Res<ActiveLayout>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    mut pressed: MessageWriter<KeyPressed>,
) {
    // A layer key released away from it.
    if view.held_layer.is_some() && !mouse.pressed(MouseButton::Left) && touches.iter().next().is_none() {
        view.held_layer = None;
    }
    if view.clicks.is_empty() {
        return;
    }
    for (key, long) in std::mem::take(&mut view.clicks) {
        let Some(spec) = layout.0.keys.get(key) else {
            continue;
        };
        let layer = view.shown_layer().unwrap_or(0);
        let mods = view.sticky;
        match spec.binding_on(layer) {
            // A long press held the layer, a click toggles it.
            (Some(Special::Layer(l)), _) => {
                if !long {
                    view.preview = if view.preview == Some(l) { None } else { Some(l) };
                }
            }
            (Some(Special::Ctrl), _) => view.sticky.ctrl = !mods.ctrl,
            (Some(Special::Alt), _) => view.sticky.alt = !mods.alt,
            (Some(Special::Super), _) => view.sticky.super_key = !mods.super_key,
            (Some(Special::Shift), _) => view.sticky.shift = !mods.shift,
            (Some(Special::Bluetooth(_) | Special::BluetoothClear | Special::Caps), _) => {}
            (Some(special), _) => {
                pressed.write(press_special(special, mods));
                view.sticky = Mods::NONE;
            }
            (None, Some(out)) => {
                pressed.write(press_char(out, long, mods));
                view.sticky = Mods::NONE;
            }
            (None, None) => {}
        }
    }
}

/// The finger of a key, the layer keys wear the color of their layer.
pub fn stripe_token(key: &crate::layout::KeySpec) -> ThemeToken {
    match key.special {
        Some(Special::Layer(l)) if (l as usize) < tokens::LAYERS.len() => tokens::LAYERS[l as usize].clone(),
        _ => tokens::FINGERS[key.finger.min(9) as usize].clone(),
    }
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
    let shown_layer = view.shown_layer();
    // The legends of a layer on the screen are all shown.
    let fade = if shown_layer.is_some() { 1.0 } else { view.shown_fade };
    let names = &layout.layer_names;

    // On a layer on the screen, a key of that layer when one types the target.
    let (mut roles, plan) = hints(layout, coach.target, shown_layer);
    let trainer_applies = (coach.layer_trainer && plan.as_ref().is_some_and(Plan::is_combo)) || coach.always_show;
    let show_plan = coach.reveal || trainer_applies || coach.fade > HINT_FADE;
    let mut in_plan = vec![false; layout.keys.len()];
    for key in plan.iter().flat_map(Plan::keys) {
        in_plan[key] = true;
    }
    // The keys the view itself highlights, they are always shown.
    let mut forced = vec![false; layout.keys.len()];
    let mut force = |roles: &mut Vec<KeyRole>, key: usize| {
        roles[key] = KeyRole::Hold;
        forced[key] = true;
    };
    let mut layer_hint = None;
    if let Some(layer) = shown_layer {
        let layer_name = Special::Layer(layer).name(names);
        if let Some(p) = plan.as_ref().filter(|p| p.layer != layer) {
            // The next key is not on the layer shown: the layer key to leave it, not a key
            // that types something else here.
            for key in p.keys() {
                roles[key] = KeyRole::Idle;
                in_plan[key] = false;
            }
            let legend = layout.keys[p.key].legend(p.layer, p.shift, names);
            layer_hint = Some(if p.layer == 0 {
                format!("{legend} is on the Base layer: click {layer_name} to go back")
            } else {
                let name = Special::Layer(p.layer).name(names);
                format!("{legend} is on the {name} layer: click {layer_name} to go back, then use {name}")
            });
        } else if view.held_layer.is_some() {
            layer_hint = Some(format!("{layer_name} held: the keys type its layer"));
        } else {
            layer_hint = Some(format!(
                "The {layer_name} layer: the keys type what they type while {layer_name} is held, click {layer_name} again to go back"
            ));
        }
        for key in layout.find_special(Special::Layer(layer)) {
            force(&mut roles, key);
        }
    }
    // The modifiers clicked on the screen.
    let sticky = view.sticky;
    let armed: Vec<Special> = [
        (sticky.ctrl, Special::Ctrl),
        (sticky.alt, Special::Alt),
        (sticky.super_key, Special::Super),
        (sticky.shift, Special::Shift),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .map(|(_, s)| s)
    .collect();
    for modifier in &armed {
        for key in layout.find_special(*modifier) {
            force(&mut roles, key);
        }
    }
    let in_plan = |key: usize| in_plan[key];
    let border = theme.color(&tokens::KEY_BORDER);

    for (index, role) in roles.iter().enumerate() {
        // Backspace is always shown while there are errors to erase.
        let previewed = forced[index];
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
        let finger = theme.color(&stripe_token(&layout.keys[index]));
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
        } else if shown_layer.is_some_and(|l| layout.keys[index].is_inherited(l)) {
            // `&trans`: the key types what the base layer types.
            INHERITED_ALPHA
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
    let legends = match (&plan, shown_layer) {
        (_, Some(layer)) => (layer, sticky.shift),
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

    let instructions = if let Some(hint) = layer_hint {
        hint
    } else if !armed.is_empty() {
        let names: Vec<String> = armed.iter().map(|m| m.name(names)).collect();
        format!("{} for the next key, click it again to cancel", names.join(" + "))
    } else {
        match &plan {
            Some(plan) if trainer_applies => layout.describe(plan),
            _ => String::new(),
        }
    };
    for mut text in &mut trainer {
        if text.0 != instructions {
            text.0.clone_from(&instructions);
        }
    }
}
