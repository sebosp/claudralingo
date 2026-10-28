use bevy::prelude::*;
use claudralingo::ClaudralingoPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Claudralingo".into(),
            resolution: (1280, 880).into(),
            // The web build fills the page, see index.html.
            canvas: Some("#claudralingo".into()),
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }))
    .add_plugins(ClaudralingoPlugin);
    #[cfg(not(target_arch = "wasm32"))]
    snapshot::add(&mut app);
    app.run();
}

/// `CLAUDRALINGO_SCREENSHOT=out.png` saves a screenshot and exits, after typing the characters of
/// `CLAUDRALINGO_TYPE` (`<` is backspace, `*` the expected character), to check the screen without a person at the keyboard.
#[cfg(not(target_arch = "wasm32"))]
mod snapshot {
    use bevy::input::ButtonState;
    use bevy::input::keyboard::{Key, KeyboardInput, NativeKeyCode};
    use bevy::prelude::*;
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    use bevy::window::PrimaryWindow;

    #[derive(Resource)]
    struct Snapshot {
        path: String,
        typing: Vec<char>,
    }

    pub fn add(app: &mut App) {
        let Ok(path) = std::env::var("CLAUDRALINGO_SCREENSHOT") else {
            return;
        };
        let typing = std::env::var("CLAUDRALINGO_TYPE").unwrap_or_default().chars().collect();
        app.insert_resource(Snapshot { path, typing })
            .add_systems(Startup, no_vsync)
            .add_systems(Update, run);
    }

    /// A window in the background is throttled by the compositor with vsync.
    fn no_vsync(mut window: Single<&mut Window, With<PrimaryWindow>>) {
        window.present_mode = bevy::window::PresentMode::AutoNoVsync;
    }

    fn run(
        mut frame: Local<u32>,
        snapshot: Res<Snapshot>,
        window: Single<Entity, With<PrimaryWindow>>,
        exercise: Res<claudralingo::game::Exercise>,
        mut keys: MessageWriter<KeyboardInput>,
        mut commands: Commands,
        mut exit: MessageWriter<AppExit>,
        mut theme: ResMut<claudralingo::theme::ThemeMode>,
    ) {
        *frame += 1;
        let start: usize = 60;
        // `CLAUDRALINGO_TOGGLE_THEME` switches the theme like the toggle switch does.
        if *frame == 40 && std::env::var_os("CLAUDRALINGO_TOGGLE_THEME").is_some() {
            *theme = match *theme {
                claudralingo::theme::ThemeMode::Dark => claudralingo::theme::ThemeMode::Light,
                claudralingo::theme::ThemeMode::Light => claudralingo::theme::ThemeMode::Dark,
            };
        }
        let typed = (*frame as usize).checked_sub(start).filter(|i| i % 3 == 0).map(|i| i / 3);
        if let Some(c) = typed.and_then(|i| snapshot.typing.get(i)) {
            // `*` types the expected character, `<` is backspace.
            let c = match c {
                '*' => match exercise.tutor.expected() {
                    Some(claudralingo::klavaro::UPSYM) => '\n',
                    Some(claudralingo::klavaro::BACKSPACE) => '<',
                    Some(c) => c,
                    // Stays on the results of the exercise.
                    None => return,
                },
                c => *c,
            };
            let c = &c;
            let logical_key = match c {
                '<' => Key::Backspace,
                '\n' => Key::Enter,
                ' ' => Key::Space,
                c => Key::Character(c.to_string().into()),
            };
            let text = (!matches!(c, '<' | '\n')).then(|| c.to_string().into());
            for state in [ButtonState::Pressed, ButtonState::Released] {
                keys.write(KeyboardInput {
                    key_code: KeyCode::Unidentified(NativeKeyCode::Unidentified),
                    logical_key: logical_key.clone(),
                    state,
                    text: text.clone(),
                    repeat: false,
                    window: *window,
                });
            }
        }
        let done = start + 3 * snapshot.typing.len() + 30;
        if *frame as usize == done {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(snapshot.path.clone()));
        }
        if *frame as usize == done + 20 {
            exit.write(AppExit::Success);
        }
    }
}
