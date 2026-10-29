//! The hands under the keyboard, each finger in the color of the stripes of its keys. The finger
//! of the next key lifts and glows, the fingers holding a shift, a layer or a modifier key are
//! outlined in the color of the held keys.
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeBorderColor, UiTheme};
use bevy::prelude::*;

use crate::game::{ActiveLayout, Coach};
use crate::layout::Layout;
use crate::theme::tokens;
use crate::ui::HandsRoot;

const FINGER_WIDTH: f32 = 24.0;
/// From the pinky to the index.
const FINGER_HEIGHTS: [f32; 4] = [34.0, 46.0, 52.0, 46.0];
const LIFT: f32 = 8.0;
/// The lift follows the target at this rate per second.
const LIFT_RATE: f32 = 60.0;
const IDLE_ALPHA: f32 = 0.35;

/// Klavaro's finger number, the thumbs are told apart by their hand.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Finger {
    pub finger: u8,
    pub left: bool,
}

impl Finger {
    fn of_key(layout: &Layout, key: usize) -> Finger {
        let spec = &layout.keys[key];
        Finger { finger: spec.finger, left: spec.left }
    }

    fn matches(self, other: Finger) -> bool {
        self.finger == other.finger && (self.finger != 5 || self.left == other.left)
    }
}

pub struct HandsPlugin;

impl Plugin for HandsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_hands, update_hands).chain().after(crate::game::GameSystems));
    }
}

fn spawn_finger(commands: &mut Commands, parent: Entity, finger: Finger, width: f32, height: f32, radius: BorderRadius) {
    commands.spawn((
        Node {
            width: px(width),
            height: px(height),
            border: UiRect::all(px(2)),
            border_radius: radius,
            ..default()
        },
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
        UiTransform::default(),
        finger,
        ChildOf(parent),
    ));
}

fn spawn_hand(commands: &mut Commands, root: Entity, left: bool) {
    let hand = commands
        .spawn((
            Node { flex_direction: FlexDirection::Row, align_items: AlignItems::End, column_gap: px(4), ..default() },
            ChildOf(root),
        ))
        .id();
    let thumb = |commands: &mut Commands| {
        let column = commands
            .spawn((Node { margin: UiRect::bottom(px(6)), ..default() }, ChildOf(hand)))
            .id();
        spawn_finger(commands, column, Finger { finger: 5, left }, 40.0, 22.0, BorderRadius::all(px(11)));
    };
    if !left {
        thumb(commands);
    }
    let palm_column = commands
        .spawn((
            Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: px(3), ..default() },
            ChildOf(hand),
        ))
        .id();
    let fingers = commands
        .spawn((
            Node { flex_direction: FlexDirection::Row, align_items: AlignItems::End, column_gap: px(5), ..default() },
            ChildOf(palm_column),
        ))
        .id();
    // From the left: the pinky of the left hand, the index of the right hand.
    for i in 0..4 {
        let (finger, height) = if left { (1 + i as u8, FINGER_HEIGHTS[i]) } else { (6 + i as u8, FINGER_HEIGHTS[3 - i]) };
        let radius = BorderRadius::new(px(12), px(12), px(5), px(5));
        spawn_finger(commands, fingers, Finger { finger, left }, FINGER_WIDTH, height, radius);
    }
    commands.spawn((
        Node {
            width: px(4.0 * FINGER_WIDTH + 3.0 * 5.0 + 6.0),
            height: px(28),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::new(px(8), px(8), px(16), px(16)),
            ..default()
        },
        ThemeBackgroundColor(tokens::KEY_BG),
        ThemeBorderColor(tokens::KEY_BORDER),
        ChildOf(palm_column),
    ));
    if left {
        thumb(commands);
    }
}

fn spawn_hands(roots: Query<Entity, Added<HandsRoot>>, mut commands: Commands) {
    for root in &roots {
        spawn_hand(&mut commands, root, true);
        spawn_hand(&mut commands, root, false);
    }
}

/// The hands sit under the halves of a split keyboard, closer under a single block.
fn hands_gap(layout: &Layout) -> f32 {
    if layout.blocks.iter().filter(|b| b.rows.len() > 2).count() > 1 { 150.0 } else { 70.0 }
}

fn update_hands(
    coach: Res<Coach>,
    layout: Res<ActiveLayout>,
    theme: Res<UiTheme>,
    time: Res<Time>,
    mut roots: Query<&mut Node, With<HandsRoot>>,
    mut fingers: Query<(Entity, &Finger, &mut BackgroundColor, &mut BorderColor, &mut UiTransform, Has<BoxShadow>)>,
    mut commands: Commands,
) {
    let layout = &layout.0;
    if layout.keys.is_empty() {
        return;
    }
    for mut node in &mut roots {
        let gap = px(hands_gap(layout));
        if node.column_gap != gap {
            node.column_gap = gap;
        }
    }
    let plan = coach.target.and_then(|target| layout.plan(target));
    let target = plan.as_ref().map(|p| Finger::of_key(layout, p.key));
    let holds: Vec<Finger> = plan
        .iter()
        .flat_map(|p| p.shift_key.into_iter().chain(p.layer_key).chain(p.mods.iter().map(|(_, k)| *k)))
        .map(|k| Finger::of_key(layout, k))
        .collect();
    let step = LIFT_RATE * time.delta_secs();
    for (entity, finger, mut background, mut border, mut transform, glows) in &mut fingers {
        let token = &tokens::FINGERS[finger.finger.min(9) as usize];
        let color = theme.color(token);
        let is_target = target.is_some_and(|t| t.matches(*finger));
        let is_hold = holds.iter().any(|h| h.matches(*finger));
        let fill = color.with_alpha(if is_target || is_hold { 1.0 } else { IDLE_ALPHA });
        if background.0 != fill {
            background.0 = fill;
        }
        let outline = if is_hold { theme.color(&tokens::KEY_HOLD) } else { color.with_alpha(0.7) };
        if border.top != outline {
            *border = BorderColor::all(outline);
        }
        // The lift, smoothly.
        let lift = if is_target { -LIFT } else { 0.0 };
        let current = match transform.translation.y {
            Val::Px(y) => y,
            _ => 0.0,
        };
        if current != lift {
            let y = current + (lift - current).clamp(-step, step);
            transform.translation = Val2::px(0.0, y);
        }
        if is_target && !glows {
            commands.entity(entity).insert(BoxShadow::new(color.with_alpha(0.6), px(0), px(0), px(1), px(10)));
        } else if !is_target && glows {
            commands.entity(entity).remove::<BoxShadow>();
        }
    }
}
