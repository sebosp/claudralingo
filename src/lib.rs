//! Claudralingo: a touch typing tutor, the Chinese hanzi calligraphy comes later.
use bevy::feathers::FeathersPlugins;
use bevy::prelude::*;

pub mod background;
pub mod drills;
pub mod effects;
pub mod game;
pub mod hands;
pub mod hanzi;
pub mod keyboard;
pub mod klavaro;
pub mod layout;
pub mod progress;
pub mod prompt;
pub mod shortcuts;
pub mod theme;
pub mod ui;
pub mod zmk;
pub mod zoom;

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
            hands::HandsPlugin,
            effects::EffectsPlugin,
            shortcuts::ShortcutsPlugin,
            background::BackgroundPlugin,
            zoom::ZoomPlugin,
            hanzi::HanziPlugin,
        ));
    }
}
