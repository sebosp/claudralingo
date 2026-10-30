use bevy::prelude::*;
use claudralingo::ClaudralingoPlugin;

fn main() {
    #[cfg(target_os = "linux")]
    if use_x11_for_pens() {
        // SAFETY: before any thread is started.
        unsafe { std::env::remove_var("WAYLAND_DISPLAY") };
    }
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Claudralingo".into(),
            // The app id of Wayland and the class of X11, as the desktop file.
            name: Some("claudralingo".into()),
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
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("CLAUDRALINGO_INPUT_LOG").is_some() {
        app.add_systems(Update, log_input);
    }
    app.run();
}

/// The pens through XWayland: the Wayland backend of winit 0.30 has no tablet support, KDE sends
/// it nothing from a pen, XWayland sends the pen as a mouse. With a pen plugged in and an X
/// server, the window opens on X11; `CLAUDRALINGO_X11=1` forces it, `CLAUDRALINGO_WAYLAND=1`
/// keeps Wayland.
#[cfg(target_os = "linux")]
fn use_x11_for_pens() -> bool {
    if std::env::var_os("CLAUDRALINGO_X11").is_some() {
        return true;
    }
    if std::env::var_os("CLAUDRALINGO_WAYLAND").is_some()
        || std::env::var_os("WAYLAND_DISPLAY").is_none()
        || std::env::var_os("DISPLAY").is_none()
    {
        return false;
    }
    let devices = std::fs::read_to_string("/proc/bus/input/devices").unwrap_or_default();
    let pen = has_pen(&devices);
    if pen {
        eprintln!("A pen is plugged in: the window opens through XWayland, CLAUDRALINGO_WAYLAND=1 keeps Wayland");
    }
    pen
}

/// A pen or a stylus among the input devices of `/proc/bus/input/devices`.
#[cfg(target_os = "linux")]
fn has_pen(devices: &str) -> bool {
    devices
        .lines()
        .filter_map(|l| l.strip_prefix("N: Name="))
        .map(str::to_lowercase)
        .any(|name| name.contains("stylus") || name.contains(" pen") || name.contains("wacom"))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn pens() {
        let portia = "N: Name=\"ELAN9008:00 04F3:4359\"\nN: Name=\"ELAN9008:00 04F3:4359 Stylus\"\n";
        assert!(super::has_pen(portia));
        assert!(!super::has_pen("N: Name=\"Corne Keyboard\"\nN: Name=\"Corne Mouse\"\nN: Name=\"PC Speaker\"\n"));
    }
}

/// `CLAUDRALINGO_INPUT_LOG=1`: the mouse, touch and picking events on stderr, to see what a pen
/// sends.
#[cfg(not(target_arch = "wasm32"))]
fn log_input(
    mut buttons: MessageReader<bevy::input::mouse::MouseButtonInput>,
    mut touches: MessageReader<TouchInput>,
    mut moved: MessageReader<CursorMoved>,
    mut presses: MessageReader<bevy::picking::events::PointerPress>,
    mut drags: MessageReader<bevy::picking::events::PointerDrag>,
    mut moves: Local<u32>,
) {
    for b in buttons.read() {
        eprintln!("mouse button {:?} {:?}", b.button, b.state);
    }
    for t in touches.read() {
        eprintln!("touch {:?} id {} at {:?} force {:?}", t.phase, t.id, t.position, t.force);
    }
    for m in moved.read() {
        *moves += 1;
        if moves.is_multiple_of(30) {
            eprintln!("cursor moved {} times, at {:?}", *moves, m.position);
        }
    }
    for p in presses.read() {
        eprintln!("picking press {:?} on {:?} by {:?}", p.button, p.entity, p.pointer.id);
    }
    for d in drags.read() {
        eprintln!("picking drag {:?} by {:?}", d.delta, d.pointer.id);
    }
}

/// `CLAUDRALINGO_SCREENSHOT=out.png` saves a screenshot and exits, after typing the characters of
/// `CLAUDRALINGO_TYPE` (`<` is backspace, `*` the expected character or chord), to check the screen
/// without a person at the keyboard.
#[cfg(not(target_arch = "wasm32"))]
mod snapshot {
    use bevy::input::ButtonState;
    use bevy::input::keyboard::{Key, KeyboardInput, NativeKey, NativeKeyCode};
    use bevy::prelude::*;
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    use bevy::window::{PrimaryWindow, WindowFocused};
    use claudralingo::game::Coach;
    use claudralingo::klavaro::{BACKSPACE, UPSYM};
    use claudralingo::layout::{Chord, KeyName, Mods, Special, Target};
    use claudralingo::shortcuts::Drill;

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
            .add_systems(First, drop_real_keys)
            .add_systems(Update, run.before(claudralingo::game::GameSystems));
    }

    /// The keys typed on the real keyboard while the window has the focus are not the test's:
    /// the keys of the test are read in the frame they are sent, the others are dropped.
    fn drop_real_keys(mut keys: ResMut<Messages<KeyboardInput>>) {
        keys.drain().for_each(drop);
    }

    /// A window in the background is throttled by the compositor with vsync.
    fn no_vsync(mut window: Single<&mut Window, With<PrimaryWindow>>) {
        window.present_mode = bevy::window::PresentMode::AutoNoVsync;
    }

    fn key_of(special: Special) -> Key {
        match special {
            Special::Escape => Key::Escape,
            Special::Tab => Key::Tab,
            Special::Enter => Key::Enter,
            Special::Space => Key::Space,
            Special::Backspace => Key::Backspace,
            Special::Delete => Key::Delete,
            Special::Insert => Key::Insert,
            Special::Left => Key::ArrowLeft,
            Special::Right => Key::ArrowRight,
            Special::Up => Key::ArrowUp,
            Special::Down => Key::ArrowDown,
            Special::Home => Key::Home,
            Special::End => Key::End,
            Special::PageUp => Key::PageUp,
            Special::PageDown => Key::PageDown,
            _ => Key::Unidentified(NativeKey::Unidentified),
        }
    }

    fn send(keys: &mut MessageWriter<KeyboardInput>, window: Entity, key_code: KeyCode, logical_key: Key, text: Option<String>, state: ButtonState) {
        keys.write(KeyboardInput {
            key_code,
            logical_key,
            state,
            text: text.map(Into::into),
            repeat: false,
            window,
        });
    }

    /// Types a character of `CLAUDRALINGO_TYPE`, `<` is backspace and a tab is the Tab key.
    fn tap(keys: &mut MessageWriter<KeyboardInput>, window: Entity, c: char) {
        match c {
            '<' => press_chord(keys, window, Chord::special(Special::Backspace)),
            '\t' => press_chord(keys, window, Chord::special(Special::Tab)),
            c => type_char(keys, window, c),
        }
    }

    fn type_char(keys: &mut MessageWriter<KeyboardInput>, window: Entity, c: char) {
        let logical_key = match c {
            '\n' => Key::Enter,
            ' ' => Key::Space,
            c => Key::Character(c.to_string().into()),
        };
        let text = (c != '\n').then(|| c.to_string());
        for state in [ButtonState::Pressed, ButtonState::Released] {
            send(keys, window, UNIDENTIFIED, logical_key.clone(), text.clone(), state);
        }
    }

    const UNIDENTIFIED: KeyCode = KeyCode::Unidentified(NativeKeyCode::Unidentified);

    /// Presses a chord: the modifiers, the key, then releases the modifiers.
    fn press_chord(keys: &mut MessageWriter<KeyboardInput>, window: Entity, chord: Chord) {
        let mods = [
            (chord.mods.ctrl, KeyCode::ControlLeft, Key::Control),
            (chord.mods.alt, KeyCode::AltLeft, Key::Alt),
            (chord.mods.shift, KeyCode::ShiftLeft, Key::Shift),
            (chord.mods.super_key, KeyCode::SuperLeft, Key::Super),
        ];
        for (on, code, key) in &mods {
            if *on {
                send(keys, window, *code, key.clone(), None, ButtonState::Pressed);
            }
        }
        match chord.key {
            KeyName::Char(c) => type_char(keys, window, c),
            KeyName::Special(special) => {
                for state in [ButtonState::Pressed, ButtonState::Released] {
                    send(keys, window, UNIDENTIFIED, key_of(special), None, state);
                }
            }
        }
        for (on, code, key) in &mods {
            if *on {
                send(keys, window, *code, key.clone(), None, ButtonState::Released);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn run(
        mut frame: Local<u32>,
        (mut typed_at, mut shot, mut after_shot): (Local<f32>, Local<bool>, Local<u32>),
        time: Res<Time<Real>>,
        snapshot: Res<Snapshot>,
        window: Single<Entity, With<PrimaryWindow>>,
        coach: Res<Coach>,
        drill: Res<Drill>,
        mut keys: MessageWriter<KeyboardInput>,
        mut focus: MessageWriter<WindowFocused>,
        mut commands: Commands,
        mut exit: MessageWriter<AppExit>,
        mut theme: ResMut<claudralingo::theme::ThemeMode>,
        mut keyboard: ResMut<claudralingo::keyboard::KeyboardView>,
        mut panel: ResMut<claudralingo::ui::Panel>,
        layout: Res<claudralingo::game::ActiveLayout>,
    ) {
        *frame += 1;
        let window = *window;
        let start: usize = 60;
        // `CLAUDRALINGO_TOGGLE_THEME` switches the theme like the toggle switch does.
        if *frame == 40 && std::env::var_os("CLAUDRALINGO_TOGGLE_THEME").is_some() {
            *theme = match *theme {
                claudralingo::theme::ThemeMode::Light => claudralingo::theme::ThemeMode::Dark,
                _ => claudralingo::theme::ThemeMode::Light,
            };
        }
        // The help of the first start, kept with `CLAUDRALINGO_HELP`, `CLAUDRALINGO_SETTINGS`
        // opens the settings, `CLAUDRALINGO_LESSONS` the lessons.
        if *frame == 45 {
            if std::env::var_os("CLAUDRALINGO_SETTINGS").is_some() {
                *panel = claudralingo::ui::Panel::Settings;
            } else if std::env::var_os("CLAUDRALINGO_LESSONS").is_some() {
                *panel = claudralingo::ui::Panel::Lessons;
            } else if std::env::var_os("CLAUDRALINGO_STATS").is_some() {
                *panel = claudralingo::ui::Panel::Stats;
            } else if std::env::var_os("CLAUDRALINGO_HELP").is_none() {
                *panel = claudralingo::ui::Panel::None;
            }
        }
        // `CLAUDRALINGO_PREVIEW_LAYER=1` shows a layer as a click on its layer key does.
        if *frame == 50
            && let Some(layer) = std::env::var("CLAUDRALINGO_PREVIEW_LAYER").ok().and_then(|l| l.parse().ok())
        {
            keyboard.preview = Some(layer);
        }
        let typed = (*frame as usize).checked_sub(start).filter(|i| i % 3 == 0).map(|i| i / 3);
        match typed.and_then(|i| snapshot.typing.get(i)) {
            // `*` presses what the coach expects: a character or a chord, a round trip for Alt+Tab.
            Some('*') => match coach.target {
                Some(Target::Char(UPSYM)) => tap(&mut keys, window, '\n'),
                Some(Target::Char(BACKSPACE)) => tap(&mut keys, window, '<'),
                Some(Target::Char(c)) => type_char(&mut keys, window, c),
                Some(Target::Chord(chord)) if chord == Chord::new(Mods::ALT, KeyName::Special(Special::Tab)) => {
                    if drill.is_away() {
                        focus.write(WindowFocused { window, focused: true });
                    } else {
                        send(&mut keys, window, KeyCode::AltLeft, Key::Alt, None, ButtonState::Pressed);
                        focus.write(WindowFocused { window, focused: false });
                    }
                }
                Some(Target::Chord(chord)) => press_chord(&mut keys, window, chord),
                // Stays on the results of the exercise.
                None => {}
            },
            // `^` clicks the on-screen key the hints point at: the layer key, a modifier, the key.
            Some('^') => {
                let layout = &layout.0;
                let shown = keyboard.shown_layer().unwrap_or(0);
                let plan = coach.target.and_then(|t| layout.plan_on(t, shown).or_else(|| layout.plan(t)));
                if let Some(plan) = plan {
                    let sticky = keyboard.sticky;
                    let unarmed = plan.mods.iter().find(|(m, _)| match m {
                        Special::Ctrl => !sticky.ctrl,
                        Special::Alt => !sticky.alt,
                        _ => !sticky.super_key,
                    });
                    let layer_key = |l: u8| layout.find_special(Special::Layer(l)).next();
                    if plan.layer != shown {
                        let leave = if shown > 0 { shown } else { plan.layer };
                        if let Some(key) = layer_key(leave) {
                            keyboard.click(key, false);
                        }
                    } else if let Some((_, key)) = unarmed {
                        keyboard.click(*key, false);
                    } else if let Some(shift) = plan.shift_key.filter(|_| !sticky.shift) {
                        keyboard.click(shift, false);
                    } else {
                        keyboard.click(plan.key, plan.auto_shift);
                    }
                }
            }
            Some(c) => tap(&mut keys, window, *c),
            None => {}
        }
        // `CLAUDRALINGO_WAIT` seconds after the typing, for the animations at the end.
        let typed_until = start + 3 * snapshot.typing.len() + 30;
        let wait: f32 = std::env::var("CLAUDRALINGO_WAIT").ok().and_then(|w| w.parse().ok()).unwrap_or(0.0);
        if *frame as usize == typed_until {
            *typed_at = time.elapsed_secs();
        }
        let done = if (*frame as usize) < typed_until || time.elapsed_secs() - *typed_at < wait {
            usize::MAX
        } else {
            *frame as usize
        };
        if *frame as usize == done && !*shot {
            *shot = true;
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(snapshot.path.clone()));
        }
        if *shot {
            *after_shot += 1;
            if *after_shot == 20 {
                exit.write(AppExit::Success);
            }
        }
    }
}
