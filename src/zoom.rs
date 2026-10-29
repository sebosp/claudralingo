//! The zoom of the whole window. By default the window is scaled to fit its size: a big screen
//! shows everything bigger. Over that, Ctrl and the mouse wheel, a pinch of the trackpad,
//! Ctrl+= Ctrl+- Ctrl+0, or the buttons of the settings zoom in and out, the window scrolls
//! when it does not fit any more. It is saved with the progress. In the browser, its own zoom
//! takes Ctrl, the wheel and the keys.
use bevy::input::gestures::PinchGesture;
use bevy::input::keyboard::Key;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::progress::Progress;

pub const MIN_ZOOM: f32 = 0.5;
pub const MAX_ZOOM: f32 = 3.0;
/// The zoom of a key press or a button, a factor.
const STEP: f32 = 1.1;
/// The size of the window at 100%, from the toolbar down to the hands: the rankings may
/// scroll.
const DESIGN: Vec2 = Vec2::new(1180.0, 760.0);

/// The scale that fits the window.
pub fn fit(window: Vec2) -> f32 {
    if window.x <= 0.0 || window.y <= 0.0 {
        return 1.0;
    }
    (window / DESIGN).min_element().clamp(MIN_ZOOM, MAX_ZOOM)
}

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

    /// The new zoom factor over the base scale, the zoom stays between the bounds.
    fn apply(self, factor: f32, base: f32) -> f32 {
        match self {
            Zoom::By(by) => (factor * by).clamp(MIN_ZOOM / base, MAX_ZOOM / base),
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

fn apply_zoom(
    mut changes: MessageReader<Zoom>,
    mut progress: ResMut<Progress>,
    mut scale: ResMut<UiScale>,
    window: Single<&Window>,
) {
    let base = if progress.fit_window { fit(window.size()) } else { 1.0 };
    let mut factor = progress.zoom_factor;
    for change in changes.read() {
        factor = change.apply(factor, base);
    }
    if factor != progress.zoom_factor {
        // The autosave writes it.
        progress.zoom_factor = factor;
    }
    let zoom = (base * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    if scale.0 != zoom {
        scale.0 = zoom;
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
        let base = 2.0;
        let mut factor = 1.0;
        for _ in 0..50 {
            factor = Zoom::IN.apply(factor, base);
        }
        assert_eq!(factor * base, MAX_ZOOM);
        assert_eq!(Zoom::Reset.apply(factor, base), 1.0);
        assert!((Zoom::OUT.apply(Zoom::IN.apply(1.0, base), base) - 1.0).abs() < 1e-6);
        // A big window shows everything bigger, the window of the native build fits.
        assert_eq!(fit(Vec2::new(2360.0, 1520.0)), 2.0);
        assert!(fit(Vec2::new(1280.0, 880.0)) > 1.0);
        assert_eq!(Zoom::of_key(&Key::Character("=".into())), Some(Zoom::IN));
        assert_eq!(Zoom::of_key(&Key::Character("a".into())), None);
    }
}
