//! Claudralingo: a touch typing tutor, the Chinese hanzi calligraphy comes later.
use bevy::feathers::FeathersPlugins;
use bevy::prelude::*;

pub mod effects;
pub mod game;
pub mod keyboard;
pub mod klavaro;
pub mod layout;
pub mod progress;
pub mod prompt;
pub mod theme;
pub mod ui;

pub struct ClaudralingoPlugin;

impl Plugin for ClaudralingoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FeathersPlugins,
            theme::ThemePlugin,
            game::GamePlugin,
            ui::UiPlugin,
            prompt::PromptPlugin,
            keyboard::KeyboardPlugin,
            effects::EffectsPlugin,
        ));
    }
}
