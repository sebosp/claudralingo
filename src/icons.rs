//! The icons of the toolbar, drawn with tiny-skia when the app starts: no icon font to ship,
//! the same on every platform, tinted with the text color of the buttons so they follow the
//! theme.
use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::feathers::theme::UiTheme;
use bevy::feathers::tokens as feathers_tokens;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

/// The side of an icon image, shown at [`SHOWN`] pixels.
const SIZE: u32 = 64;
pub const SHOWN: f32 = 18.0;

/// An icon of the toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Icon {
    /// A circling arrow: a new text.
    #[default]
    Restart,
    /// A notebook: the lessons.
    Lessons,
    /// Bars: the statistics.
    Stats,
    /// A gear: the settings.
    Settings,
    /// A speaker: listen to the word.
    Listen,
    /// A printer: the exercise on paper.
    Print,
    /// A stopwatch: the time of an exercise.
    Stopwatch,
    /// A star: a grade gained.
    Star,
    /// The outline of a star: a grade not gained, an exercise not started.
    StarEmpty,
    /// A house: the main menu.
    Home,
    /// An arrow to the right: on to the next exercise.
    Forward,
    /// Two bars: the exercise waits.
    Pause,
}

impl Icon {
    pub const ALL: [Icon; 12] = [
        Icon::Restart,
        Icon::Lessons,
        Icon::Stats,
        Icon::Settings,
        Icon::Listen,
        Icon::Print,
        Icon::Stopwatch,
        Icon::Star,
        Icon::StarEmpty,
        Icon::Home,
        Icon::Forward,
        Icon::Pause,
    ];
}

/// The image node that shows an icon.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct IconNode(pub Icon);

/// The color of an icon, instead of the one of the text of the buttons.
#[derive(Component, Debug, Clone)]
pub struct IconTint(pub bevy::feathers::theme::ThemeToken);

#[derive(Resource, Default)]
pub struct Icons(Vec<(Icon, Handle<Image>)>);

/// The icon as the caption of a button.
pub fn icon(icon: Icon) -> impl Scene {
    bsn! {
        Node {
            width: px(SHOWN),
            height: px(SHOWN),
        }
        ImageNode
        IconNode({icon})
    }
}

pub struct IconsPlugin;

impl Plugin for IconsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Icons>().add_systems(Startup, render_icons).add_systems(Update, set_icons);
    }
}

fn render_icons(mut icons: ResMut<Icons>, mut images: ResMut<Assets<Image>>) {
    icons.0 = Icon::ALL
        .iter()
        .map(|icon| {
            let pixmap = draw(*icon);
            let image = Image::new(
                Extent3d { width: SIZE, height: SIZE, depth_or_array_layers: 1 },
                TextureDimension::D2,
                pixmap.data().to_vec(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
            );
            (*icon, images.add(image))
        })
        .collect();
}

/// The image of a new or changed icon node, and the tint of all of them when the theme changes.
fn set_icons(icons: Res<Icons>, theme: Res<UiTheme>, mut nodes: Query<(Ref<IconNode>, &mut ImageNode, Option<Ref<IconTint>>)>) {
    for (icon, mut node, tint) in &mut nodes {
        if icon.is_changed()
            && let Some((_, handle)) = icons.0.iter().find(|(i, _)| *i == icon.0)
            && node.image != *handle
        {
            node.image = handle.clone();
        }
        let tint_changed = tint.as_ref().is_some_and(|tint| tint.is_changed());
        if icon.is_added() || theme.is_changed() || tint_changed {
            let color = theme.color(tint.as_ref().map_or(&feathers_tokens::BUTTON_TEXT, |tint| &tint.0));
            if node.color != color {
                node.color = color;
            }
        }
    }
}

/// The icon in white: a stroke of 6 on a 64 pixel square.
fn draw(icon: Icon) -> Pixmap {
    let mut pixmap = Pixmap::new(SIZE, SIZE).expect("an icon");
    let mut paint = Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);
    paint.anti_alias = true;
    let stroke = Stroke { width: 6.0, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() };
    let ts = Transform::identity();
    match icon {
        Icon::Restart => {
            // An open ring, the arrow head at its end.
            let (cx, cy, r) = (32.0, 33.0, 19.0);
            let mut pb = PathBuilder::new();
            let (from, to) = (-0.12 * TAU, 0.58 * TAU);
            for k in 0..=48 {
                let a = from + (to - from) * k as f32 / 48.0;
                let (x, y) = (cx + r * a.cos(), cy + r * a.sin());
                if k == 0 { pb.move_to(x, y) } else { pb.line_to(x, y) }
            }
            if let Some(path) = pb.finish() {
                pixmap.stroke_path(&path, &paint, &stroke, ts, None);
            }
            let tip = Vec2::new(cx + r * from.cos(), cy + r * from.sin());
            let tangent = Vec2::new(-from.sin(), from.cos());
            let normal = Vec2::new(from.cos(), from.sin());
            let mut pb = PathBuilder::new();
            let a = tip - tangent * 11.0 - normal * 8.0;
            let b = tip - tangent * 11.0 + normal * 8.0;
            let head = tip + tangent * 3.0;
            pb.move_to(a.x, a.y);
            pb.line_to(head.x, head.y);
            pb.line_to(b.x, b.y);
            pb.close();
            if let Some(path) = pb.finish() {
                pixmap.fill_path(&path, &paint, FillRule::Winding, ts, None);
            }
        }
        Icon::Lessons => {
            // A closed notebook, the spine on the left, two lines on the cover.
            let mut pb = PathBuilder::new();
            if let Some(rect) = Rect::from_ltrb(14.0, 10.0, 50.0, 54.0) {
                pb.push_rect(rect);
            }
            pb.move_to(23.0, 10.0);
            pb.line_to(23.0, 54.0);
            pb.move_to(31.0, 24.0);
            pb.line_to(42.0, 24.0);
            pb.move_to(31.0, 34.0);
            pb.line_to(42.0, 34.0);
            if let Some(path) = pb.finish() {
                pixmap.stroke_path(&path, &paint, &Stroke { width: 5.0, ..stroke.clone() }, ts, None);
            }
        }
        Icon::Stats => {
            // Three bars, rising.
            let mut pb = PathBuilder::new();
            for (i, height) in [20.0, 34.0, 46.0].into_iter().enumerate() {
                let x = 11.0 + i as f32 * 16.0;
                if let Some(rect) = Rect::from_ltrb(x, 54.0 - height, x + 10.0, 54.0) {
                    pb.push_rect(rect);
                }
            }
            if let Some(path) = pb.finish() {
                pixmap.fill_path(&path, &paint, FillRule::Winding, ts, None);
            }
        }
        Icon::Settings => {
            // A gear of eight teeth with a hole.
            let (cx, cy) = (32.0, 32.0);
            let (root, tip, hole) = (19.0, 26.0, 8.0);
            let teeth = 8;
            let mut pb = PathBuilder::new();
            let steps = teeth * 4;
            for k in 0..steps {
                let a = TAU * k as f32 / steps as f32;
                // Two points at the tip, two at the root, per tooth.
                let r = if (k % 4) < 2 { tip } else { root };
                let a = a + 0.5 * TAU / steps as f32;
                let (x, y) = (cx + r * a.cos(), cy + r * a.sin());
                if k == 0 { pb.move_to(x, y) } else { pb.line_to(x, y) }
            }
            pb.close();
            pb.push_circle(cx, cy, hole);
            if let Some(path) = pb.finish() {
                pixmap.fill_path(&path, &paint, FillRule::EvenOdd, ts, None);
            }
        }
        Icon::Stopwatch => {
            // The dial and its hand, the button on top.
            let (cx, cy, r) = (32.0, 36.0, 20.0);
            if let Some(path) = PathBuilder::from_circle(cx, cy, r) {
                pixmap.stroke_path(&path, &paint, &stroke, ts, None);
            }
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy);
            pb.line_to(cx + 8.0, cy - 9.0);
            pb.move_to(25.0, 7.0);
            pb.line_to(39.0, 7.0);
            pb.move_to(32.0, 7.0);
            pb.line_to(32.0, 15.0);
            if let Some(path) = pb.finish() {
                pixmap.stroke_path(&path, &paint, &stroke, ts, None);
            }
        }
        Icon::Star | Icon::StarEmpty => {
            let mut pb = PathBuilder::new();
            for k in 0..10 {
                let r = if k % 2 == 0 { 27.0 } else { 11.5 };
                let a = -TAU / 4.0 + TAU * k as f32 / 10.0;
                let (x, y) = (32.0 + r * a.cos(), 34.0 + r * a.sin());
                if k == 0 { pb.move_to(x, y) } else { pb.line_to(x, y) }
            }
            pb.close();
            if let Some(path) = pb.finish() {
                if icon == Icon::Star {
                    pixmap.fill_path(&path, &paint, FillRule::Winding, ts, None);
                }
                pixmap.stroke_path(&path, &paint, &Stroke { width: 4.0, ..stroke.clone() }, ts, None);
            }
        }
        Icon::Print => {
            // A printer: the body, the sheet going in above, the sheet coming out below.
            let mut pb = PathBuilder::new();
            if let Some(rect) = Rect::from_ltrb(10.0, 26.0, 54.0, 46.0) {
                pb.push_rect(rect);
            }
            if let Some(path) = pb.finish() {
                pixmap.fill_path(&path, &paint, FillRule::Winding, ts, None);
            }
            let mut pb = PathBuilder::new();
            if let Some(rect) = Rect::from_ltrb(20.0, 12.0, 44.0, 26.0) {
                pb.push_rect(rect);
            }
            if let Some(rect) = Rect::from_ltrb(20.0, 40.0, 44.0, 54.0) {
                pb.push_rect(rect);
            }
            if let Some(path) = pb.finish() {
                pixmap.stroke_path(&path, &paint, &Stroke { width: 4.0, ..stroke.clone() }, ts, None);
            }
            // The sheet coming out, white on the body.
            let mut sheet = Paint::default();
            sheet.set_color_rgba8(255, 255, 255, 255);
            let mut hole = PathBuilder::new();
            if let Some(rect) = Rect::from_ltrb(22.0, 42.0, 42.0, 52.0) {
                hole.push_rect(rect);
            }
            if let Some(path) = hole.finish() {
                pixmap.fill_path(&path, &sheet, FillRule::Winding, ts, None);
                let mut clear = Paint::default();
                clear.set_color_rgba8(0, 0, 0, 0);
                clear.blend_mode = tiny_skia::BlendMode::Source;
                pixmap.fill_path(&path, &clear, FillRule::Winding, ts, None);
            }
        }
        Icon::Home => {
            // The roof, the walls and the door.
            let mut pb = PathBuilder::new();
            pb.move_to(9.0, 31.0);
            pb.line_to(32.0, 10.0);
            pb.line_to(55.0, 31.0);
            pb.move_to(16.0, 26.0);
            pb.line_to(16.0, 53.0);
            pb.line_to(48.0, 53.0);
            pb.line_to(48.0, 26.0);
            pb.move_to(27.0, 53.0);
            pb.line_to(27.0, 39.0);
            pb.line_to(37.0, 39.0);
            pb.line_to(37.0, 53.0);
            if let Some(path) = pb.finish() {
                pixmap.stroke_path(&path, &paint, &Stroke { width: 5.0, ..stroke.clone() }, ts, None);
            }
        }
        Icon::Forward => {
            // A shaft and its head.
            let mut pb = PathBuilder::new();
            pb.move_to(12.0, 32.0);
            pb.line_to(50.0, 32.0);
            pb.move_to(36.0, 18.0);
            pb.line_to(50.0, 32.0);
            pb.line_to(36.0, 46.0);
            if let Some(path) = pb.finish() {
                pixmap.stroke_path(&path, &paint, &stroke, ts, None);
            }
        }
        Icon::Pause => {
            let mut pb = PathBuilder::new();
            for x in [18.0, 37.0] {
                if let Some(rect) = Rect::from_ltrb(x, 14.0, x + 9.0, 50.0) {
                    pb.push_rect(rect);
                }
            }
            if let Some(path) = pb.finish() {
                pixmap.fill_path(&path, &paint, FillRule::Winding, ts, None);
            }
        }
        Icon::Listen => {
            // A speaker: the box, the cone, two waves.
            let mut pb = PathBuilder::new();
            pb.move_to(10.0, 25.0);
            pb.line_to(20.0, 25.0);
            pb.line_to(32.0, 14.0);
            pb.line_to(32.0, 50.0);
            pb.line_to(20.0, 39.0);
            pb.line_to(10.0, 39.0);
            pb.close();
            if let Some(path) = pb.finish() {
                pixmap.fill_path(&path, &paint, FillRule::Winding, ts, None);
            }
            for r in [12.0, 21.0] {
                let mut pb = PathBuilder::new();
                let (from, to) = (-0.13 * TAU, 0.13 * TAU);
                for k in 0..=16 {
                    let a = from + (to - from) * k as f32 / 16.0;
                    let (x, y) = (34.0 + r * a.cos(), 32.0 + r * a.sin());
                    if k == 0 { pb.move_to(x, y) } else { pb.line_to(x, y) }
                }
                if let Some(path) = pb.finish() {
                    pixmap.stroke_path(&path, &paint, &Stroke { width: 5.0, ..stroke.clone() }, ts, None);
                }
            }
        }
    }
    pixmap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_has_ink_in_the_middle_and_none_at_the_corners() {
        for icon in Icon::ALL {
            let pixmap = draw(icon);
            let inked = pixmap.pixels().iter().filter(|p| p.alpha() > 0).count();
            assert!(inked > 200 && inked < 3000, "{icon:?}: {inked} pixels");
            assert_eq!(pixmap.pixel(1, 1).map(|p| p.alpha()), Some(0), "{icon:?}");
        }
    }
}
