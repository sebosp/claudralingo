//! The background: the edges of a low poly mesh drifting slowly behind the window, its
//! triangles join and dissolve, blurred under the text to type and the hands. The vertices wear the colors of the most missed keys: the color of their finger, or
//! of their layer when the key is on Lower or Raise, the same colors as the stripes of the keys.
//! It shows what to practice next, the rankings name the same fingers and layers.
use bevy::asset::embedded_asset;
use bevy::feathers::theme::{ThemeToken, UiTheme};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

use crate::game::ActiveLayout;
use crate::layout::{Layout, Target, finger_name};
use crate::progress::{LayoutProgress, Progress};
use crate::theme::{ThemeMode, tokens};
use crate::ui::{BackgroundRoot, ExerciseArea, HandsRoot};

/// The colors of the background, at most.
const N_COLORS: usize = 4;
const COLUMNS: f32 = 14.0;

/// See `background.wgsl`.
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct BackgroundMaterial {
    #[uniform(0)]
    color0: Vec4,
    #[uniform(1)]
    color1: Vec4,
    #[uniform(2)]
    color2: Vec4,
    #[uniform(3)]
    color3: Vec4,
    /// The time, unused, the opacity of the edges and vertices, the columns of the mesh.
    #[uniform(4)]
    params: Vec4,
    /// The areas where the mesh blurs, in uv units: the text to type and the hands.
    #[uniform(5)]
    blur0: Vec4,
    #[uniform(6)]
    blur1: Vec4,
}

impl UiMaterial for BackgroundMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://claudralingo/background.wgsl".into()
    }
}

/// What a weak key asks to practice: its finger, or the layer it is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Finger(u8),
    Layer(u8),
}

impl Focus {
    fn token(self) -> ThemeToken {
        match self {
            Focus::Finger(f) => tokens::FINGERS[f.min(9) as usize].clone(),
            Focus::Layer(l) => tokens::LAYERS[(l as usize).min(tokens::LAYERS.len() - 1)].clone(),
        }
    }

    pub fn name(self, layout: &Layout) -> String {
        match self {
            Focus::Finger(f) => finger_name(f).to_string(),
            Focus::Layer(l) => format!("{} layer", layout.layer_names.get(l as usize).copied().unwrap_or("?")),
        }
    }
}

/// The fingers and layers of the most missed keys, the most missed first.
pub fn focus(layout: &Layout, progress: &LayoutProgress) -> Vec<Focus> {
    let mut focus = Vec::new();
    for key in &progress.most_missed {
        let Some(plan) = key.chars().next().and_then(|c| layout.plan(Target::Char(c))) else {
            continue;
        };
        let item = if plan.layer > 0 { Focus::Layer(plan.layer) } else { Focus::Finger(layout.keys[plan.key].finger) };
        if !focus.contains(&item) {
            focus.push(item);
        }
        if focus.len() == N_COLORS {
            break;
        }
    }
    focus
}

pub struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "background.wgsl");
        app.add_plugins(UiMaterialPlugin::<BackgroundMaterial>::default())
            .add_systems(Update, (spawn_background, update_background).chain());
    }
}

fn spawn_background(
    roots: Query<Entity, Added<BackgroundRoot>>,
    mut materials: ResMut<Assets<BackgroundMaterial>>,
    mut commands: Commands,
) {
    for root in &roots {
        let material = materials.add(BackgroundMaterial {
            color0: Vec4::ZERO,
            color1: Vec4::ZERO,
            color2: Vec4::ZERO,
            color3: Vec4::ZERO,
            params: Vec4::new(0.0, 0.0, 0.0, COLUMNS),
            blur0: Vec4::ZERO,
            blur1: Vec4::ZERO,
        });
        commands.entity(root).insert(MaterialNode(material));
    }
}

fn to_vec4(color: Color) -> Vec4 {
    LinearRgba::from(color).to_vec4()
}

/// The nodes the background blurs under.
type BlurredUnder = Or<(With<ExerciseArea>, With<HandsRoot>)>;

#[allow(clippy::too_many_arguments)]
fn update_background(
    time: Res<Time<Real>>,
    progress: Res<Progress>,
    layout: Res<ActiveLayout>,
    theme: Res<UiTheme>,
    mode: Res<ThemeMode>,
    nodes: Query<(&MaterialNode<BackgroundMaterial>, &ComputedNode)>,
    areas: Query<(&ComputedNode, &UiGlobalTransform), BlurredUnder>,
    mut materials: ResMut<Assets<BackgroundMaterial>>,
    mut colors: Local<Option<[Vec4; N_COLORS]>>,
) {
    let Ok((node, background)) = nodes.single() else {
        return;
    };
    // The areas to blur under, from the layout of this frame, the background fills the window.
    let size = background.size();
    let mut blur = [Vec4::ZERO; 2];
    if size.x > 0.0 && size.y > 0.0 {
        for (slot, (computed, transform)) in blur.iter_mut().zip(&areas) {
            let center = transform.affine().translation;
            let half = computed.size() / 2.0;
            *slot = Vec4::new(
                (center.x - half.x) / size.x,
                (center.y - half.y) / size.y,
                (center.x + half.x) / size.x,
                (center.y + half.y) / size.y,
            );
        }
    }
    let Some(mut material) = materials.get_mut(&node.0) else {
        return;
    };
    if progress.is_changed() || layout.is_changed() || theme.is_changed() || colors.is_none() {
        let current = progress.layouts.get(progress.layout.key()).cloned().unwrap_or_default();
        let mut tokens_used: Vec<ThemeToken> = focus(&layout.0, &current).into_iter().map(Focus::token).collect();
        // Without misses yet, the colors of the cursor and of the thumbs.
        if tokens_used.is_empty() {
            tokens_used = vec![tokens::CURSOR, tokens::FINGERS[5].clone()];
        }
        let palette: [Vec4; N_COLORS] =
            std::array::from_fn(|i| to_vec4(theme.color(&tokens_used[i % tokens_used.len()])));
        *colors = Some(palette);
        [material.color0, material.color1, material.color2, material.color3] = palette;
    }
    // Quiet on the dark windows, a little stronger on the light ones.
    let edges = match *mode {
        ThemeMode::Dark => 0.13,
        ThemeMode::Light => 0.24,
    } * progress.background.strength();
    // A wrapped time keeps the precision of the shader, reduced motion stops the mesh.
    let t = if progress.reduce_motion { 120.0 } else { (time.elapsed_secs_f64() % 10_000.0) as f32 };
    material.params = Vec4::new(t, 0.0, edges, COLUMNS);
    [material.blur0, material.blur1] = blur;
}
