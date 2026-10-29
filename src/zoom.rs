//! The zoom of the whole window, for the small or far screens: Ctrl and the mouse wheel, a pinch
//! of the trackpad, Ctrl+= Ctrl+- Ctrl+0, or the buttons of the settings. It is saved with the
//! progress. In the browser, its own zoom takes Ctrl, the wheel and the keys.
use bevy::input::gestures::PinchGesture;
use bevy::input::keyboard::Key;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::progress::Progress;

pub const MIN_ZOOM: f32 = 0.5;
pub const MAX_ZOOM: f32 = 3.0;
/// The zoom of a key press or a button, a factor.
const STEP: f32 = 1.1;

/// Changes the zoom.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub enum Zoom {
    /// Multiplies it.
    By(f32),
    Reset,
}

impl Zoom {
    pub const IN: Zoom = Zoom::By(STEP);
    pub const OUT: Zoom = Zoom::By(1.0 / STEP);

    /// The zoom of a key pressed with Ctrl, as in the browsers.
    pub fn of_key(key: &Key) -> Option<Zoom> {
        let Key::Character(c) = key else {
            return None;
        };
        match c.as_str() {
            "=" | "+" => Some(Zoom::IN),
            "-" | "_" => Some(Zoom::OUT),
            "0" => Some(Zoom::Reset),
            _ => None,
        }
    }

    fn apply(self, zoom: f32) -> f32 {
        match self {
            Zoom::By(factor) => (zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM),
            Zoom::Reset => 1.0,
        }
    }
}

pub struct ZoomPlugin;

impl Plugin for ZoomPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Zoom>().add_systems(Update, (gestures, apply_zoom).chain());
    }
}

/// Ctrl and the wheel, or a pinch.
fn gestures(
    mut wheel: MessageReader<MouseWheel>,
    mut pinch: MessageReader<PinchGesture>,
    keys: Res<ButtonInput<KeyCode>>,
    mut zoom: MessageWriter<Zoom>,
) {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    for event in wheel.read() {
        if !ctrl || cfg!(target_arch = "wasm32") {
            continue;
        }
        let notches = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / 40.0,
        };
        if notches != 0.0 {
            zoom.write(Zoom::By(STEP.powf(notches.clamp(-3.0, 3.0))));
        }
    }
    for PinchGesture(delta) in pinch.read() {
        zoom.write(Zoom::By((1.0 + delta).clamp(0.5, 2.0)));
    }
}

fn apply_zoom(mut changes: MessageReader<Zoom>, mut progress: ResMut<Progress>, mut scale: ResMut<UiScale>) {
    let mut zoom = progress.zoom;
    for change in changes.read() {
        zoom = change.apply(zoom);
    }
    if zoom != progress.zoom {
        // The autosave writes it.
        progress.zoom = zoom;
    }
    if scale.0 != progress.zoom {
        scale.0 = progress.zoom;
    }
}

/// The zoom in percent, for the settings.
pub fn percent(zoom: f32) -> String {
    format!("{:.0}%", zoom * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_stay_in_range() {
        let mut zoom = 1.0;
        for _ in 0..50 {
            zoom = Zoom::IN.apply(zoom);
        }
        assert_eq!(zoom, MAX_ZOOM);
        assert_eq!(Zoom::Reset.apply(zoom), 1.0);
        assert!((Zoom::OUT.apply(Zoom::IN.apply(1.0)) - 1.0).abs() < 1e-6);
        assert_eq!(Zoom::of_key(&Key::Character("=".into())), Some(Zoom::IN));
        assert_eq!(Zoom::of_key(&Key::Character("a".into())), None);
    }
}
