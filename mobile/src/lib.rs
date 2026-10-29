//! Claudralingo on Android and iOS: full screen, the same plugin as the desktop and the web.
use bevy::prelude::*;
use bevy::window::WindowMode;
use bevy::winit::WinitSettings;
use claudralingo::ClaudralingoPlugin;

#[bevy_main]
pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Claudralingo".into(),
                resizable: false,
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                // Only on iOS.
                prefers_home_indicator_hidden: true,
                prefers_status_bar_hidden: true,
                ..default()
            }),
            ..default()
        }))
        // The loop waits for input when nothing moves, for the battery.
        .insert_resource(WinitSettings::mobile())
        .add_plugins(ClaudralingoPlugin)
        .run();
}
