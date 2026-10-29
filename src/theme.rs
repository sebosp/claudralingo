//! The dark and light themes: the feathers themes plus the tokens of the game.
//!
//! Feathers 0.20 only ships a dark theme, the light one overrides its semantic colors. The colors
//! of the keys and the exercise are inspired by <https://layer-tutor.pages.dev/>.
use bevy::color::Luminance;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::{InheritableThemeTextColor, SemanticToken, SurfaceLevel, ThemeProps, ThemeToken, UiTheme};
use bevy::feathers::tokens::{self as feathers_tokens, semantic};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Resource)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

pub mod tokens {
    use bevy::feathers::theme::ThemeToken;

    pub const TRANSPARENT: ThemeToken = ThemeToken::new_static("claudralingo.transparent");
    pub const KEYBOARD_CASE: ThemeToken = ThemeToken::new_static("claudralingo.keyboard.case");
    pub const KEY_BG: ThemeToken = ThemeToken::new_static("claudralingo.key.bg");
    pub const KEY_BORDER: ThemeToken = ThemeToken::new_static("claudralingo.key.border");
    pub const KEY_TEXT: ThemeToken = ThemeToken::new_static("claudralingo.key.text");
    pub const KEY_TARGET: ThemeToken = ThemeToken::new_static("claudralingo.key.target");
    pub const KEY_HOLD: ThemeToken = ThemeToken::new_static("claudralingo.key.hold");
    pub const KEY_SHIFT: ThemeToken = ThemeToken::new_static("claudralingo.key.shift");
    pub const KEY_ERASE: ThemeToken = ThemeToken::new_static("claudralingo.key.erase");
    pub const KEY_TEXT_ON_HINT: ThemeToken = ThemeToken::new_static("claudralingo.key.text.hint");
    pub const CHAR_PENDING: ThemeToken = ThemeToken::new_static("claudralingo.char.pending");
    pub const CHAR_CURRENT: ThemeToken = ThemeToken::new_static("claudralingo.char.current");
    pub const CHAR_CORRECT: ThemeToken = ThemeToken::new_static("claudralingo.char.correct");
    pub const CHAR_WRONG: ThemeToken = ThemeToken::new_static("claudralingo.char.wrong");
    pub const CHAR_WRONG_BG: ThemeToken = ThemeToken::new_static("claudralingo.char.wrong.bg");
    pub const CHAR_RETOUCHED: ThemeToken = ThemeToken::new_static("claudralingo.char.retouched");
    pub const SELECTION: ThemeToken = ThemeToken::new_static("claudralingo.selection");
    pub const CURSOR: ThemeToken = ThemeToken::new_static("claudralingo.cursor");
    pub const SPARK_OK: ThemeToken = ThemeToken::new_static("claudralingo.spark.ok");
    pub const SPARK_ERROR: ThemeToken = ThemeToken::new_static("claudralingo.spark.error");
    pub const PROGRESS_TRACK: ThemeToken = ThemeToken::new_static("claudralingo.progress.track");
    pub const PROGRESS_BAR: ThemeToken = ThemeToken::new_static("claudralingo.progress.bar");
    /// One color per finger, Klavaro's finger numbers, 0 is unused.
    pub const FINGERS: [ThemeToken; 10] = [
        ThemeToken::new_static("claudralingo.finger.0"),
        ThemeToken::new_static("claudralingo.finger.1"),
        ThemeToken::new_static("claudralingo.finger.2"),
        ThemeToken::new_static("claudralingo.finger.3"),
        ThemeToken::new_static("claudralingo.finger.4"),
        ThemeToken::new_static("claudralingo.finger.5"),
        ThemeToken::new_static("claudralingo.finger.6"),
        ThemeToken::new_static("claudralingo.finger.7"),
        ThemeToken::new_static("claudralingo.finger.8"),
        ThemeToken::new_static("claudralingo.finger.9"),
    ];
    /// One color per layer of the split keyboards, 0 (the base layer) wears the finger colors.
    pub const LAYERS: [ThemeToken; 3] = [
        ThemeToken::new_static("claudralingo.layer.0"),
        ThemeToken::new_static("claudralingo.layer.1"),
        ThemeToken::new_static("claudralingo.layer.2"),
    ];
}

/// Assigns a game token to a semantic token of the same name with a color.
fn set(props: &mut ThemeProps, token: &ThemeToken, color: Color) {
    let semantic = SemanticToken::new(token.to_string().into());
    props.token_assignments.insert(token.clone(), semantic.clone());
    props.semantic_base.insert(semantic, color);
}

fn hex(s: &str) -> Color {
    Srgba::hex(s).map(Color::from).unwrap_or(Color::BLACK)
}

/// From the pinky to the index, the index finger colors are shared by both hands.
// The finger and the layer colors are the categorical palette of the dataviz reference (blue,
// orange, aqua, yellow, magenta, green, violet), in the order of the physical neighbors: pinky,
// ring, middle, index, thumb, then Lower and Raise, the thumb keys. That order clears the
// color-blind separation of neighboring colors (protan ΔE 8.4 dark, 9.1 light) and the normal
// vision floor (ΔE 19.3 / 19.6), thumb, Lower and Raise clear it all-pairs. Red is left out, it
// is the erase hint.
const DARK_FINGERS: [&str; 5] = ["#3987e5", "#d95926", "#199e70", "#c98500", "#d55181"];
const DARK_LAYERS: [&str; 3] = ["#8a93b3", "#008300", "#9085e9"];
const LIGHT_FINGERS: [&str; 5] = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4"];
const LIGHT_LAYERS: [&str; 3] = ["#7a8196", "#008300", "#6250d6"];

/// The colors of the fingers from the pinky to the thumb, mirrored for the right hand.
fn set_fingers(props: &mut ThemeProps, fingers: [&str; 5], layers: [&str; 3]) {
    set(props, &tokens::FINGERS[0], hex(layers[0]));
    for (i, color) in fingers.iter().enumerate() {
        set(props, &tokens::FINGERS[i + 1], hex(color));
        set(props, &tokens::FINGERS[9 - i], hex(color));
    }
    for (token, color) in tokens::LAYERS.iter().zip(layers) {
        set(props, token, hex(color));
    }
}

pub fn props(mode: ThemeMode) -> ThemeProps {
    match mode {
        ThemeMode::Dark => dark_theme(),
        ThemeMode::Light => light_theme(),
    }
}

pub fn dark_theme() -> ThemeProps {
    let mut props = create_dark_theme();
    set(&mut props, &tokens::TRANSPARENT, Color::NONE);
    set(&mut props, &tokens::KEYBOARD_CASE, hex("#0c1120"));
    set(&mut props, &tokens::KEY_BG, hex("#131a2e"));
    set(&mut props, &tokens::KEY_BORDER, hex("#29365a"));
    set(&mut props, &tokens::KEY_TEXT, hex("#edf0fa"));
    set(&mut props, &tokens::KEY_TARGET, hex("#22ff88"));
    set(&mut props, &tokens::KEY_HOLD, hex("#ffb200"));
    set(&mut props, &tokens::KEY_SHIFT, hex("#d400ff"));
    set(&mut props, &tokens::KEY_ERASE, hex("#ff1f4c"));
    set(&mut props, &tokens::KEY_TEXT_ON_HINT, hex("#06281c"));
    set(&mut props, &tokens::CHAR_PENDING, hex("#8a93b3"));
    set(&mut props, &tokens::CHAR_CURRENT, hex("#edf0fa"));
    set(&mut props, &tokens::CHAR_CORRECT, hex("#22ff88"));
    set(&mut props, &tokens::CHAR_WRONG, hex("#ff1f4c"));
    set(&mut props, &tokens::CHAR_WRONG_BG, hex("#ff1f4c").with_alpha(0.25));
    set(&mut props, &tokens::CHAR_RETOUCHED, hex("#ffb200"));
    set(&mut props, &tokens::SELECTION, hex("#00e5ff").with_alpha(0.3));
    set(&mut props, &tokens::CURSOR, hex("#00e5ff"));
    set(&mut props, &tokens::SPARK_OK, hex("#00e5ff"));
    set(&mut props, &tokens::SPARK_ERROR, hex("#ff1f4c"));
    set(&mut props, &tokens::PROGRESS_TRACK, hex("#1b2542"));
    set(&mut props, &tokens::PROGRESS_BAR, hex("#00e5ff"));
    set_fingers(&mut props, DARK_FINGERS, DARK_LAYERS);
    props
}

pub fn light_theme() -> ThemeProps {
    let mut props = dark_theme();
    let window = hex("#f4f5f8");
    let pane = hex("#e8eaf0");
    let item = hex("#dde0e8");
    let item_active = hex("#cfd3de");
    let accent = hex("#1a6fd1");
    let base = [
        (semantic::SURFACE_WINDOW, window),
        (semantic::SURFACE_PANE_BODY, pane),
        (semantic::SURFACE_PANE_HEADER, window),
        (semantic::SURFACE_SUBPANE_BODY, pane),
        (semantic::SURFACE_SUBPANE_HEADER, item),
        (semantic::SURFACE_GROUP, item),
        (semantic::SURFACE_DIALOG, pane),
        (semantic::SURFACE_MENU, window),
        (semantic::BORDER_DEFAULT, hex("#b8bdca")),
        (semantic::TEXT_DEFAULT, hex("#23262f")),
        (semantic::TEXT_DIM, hex("#5d6375")),
        (semantic::TEXT_DISABLED, hex("#23262f").with_alpha(0.4)),
        (semantic::TEXT_ON_ACCENT, Color::WHITE),
        (semantic::FILL_ACCENT_DEFAULT, accent),
        (semantic::FILL_ACCENT_HOVER, accent.lighter(0.05)),
        (semantic::FILL_ACCENT_PRESSED, accent.lighter(0.1)),
        (semantic::FILL_ACCENT_DISABLED, item_active.with_alpha(0.7)),
        (semantic::FILL_SOLID_DEFAULT, item_active),
        (semantic::FILL_SOLID_HOVER, item_active.darker(0.04)),
        (semantic::FILL_SOLID_PRESSED, item_active.darker(0.08)),
        (semantic::FILL_SOLID_DISABLED, item.with_alpha(0.7)),
        (semantic::FILL_ITEM_HOVER, pane.darker(0.04)),
        (semantic::FILL_ITEM_PRESSED, pane.darker(0.08)),
        (semantic::FILL_FIELD_DEFAULT, Color::WHITE),
        (semantic::FILL_FIELD_HOVER, Color::WHITE.darker(0.02)),
        (semantic::FILL_FIELD_PRESSED, Color::WHITE.darker(0.04)),
        (semantic::FOCUS_RING, accent.with_alpha(0.5)),
    ];
    props.semantic_base.extend(base);
    // The dark theme uses white text on its gray controls, dark text reads better on light ones.
    for token in [
        feathers_tokens::BUTTON_TEXT,
        feathers_tokens::SLIDER_TEXT,
        feathers_tokens::MENUITEM_TEXT,
        feathers_tokens::TEXT_INPUT_TEXT,
        feathers_tokens::LISTROW_TEXT,
    ] {
        props.token_assignments.insert(token, semantic::TEXT_DEFAULT);
    }
    // The dark overrides lighten the controls on the higher surfaces, darken them here.
    props.semantic_overrides = HashMap::from([(
        SurfaceLevel::Highest,
        HashMap::from([
            (semantic::FILL_SOLID_DEFAULT, item_active.darker(0.04)),
            (semantic::FILL_FIELD_DEFAULT, Color::WHITE),
            (semantic::FILL_ITEM_HOVER, item.darker(0.04)),
        ]),
    )]);
    set(&mut props, &tokens::KEYBOARD_CASE, hex("#e3e6ee"));
    set(&mut props, &tokens::KEY_BG, hex("#ffffff"));
    set(&mut props, &tokens::KEY_BORDER, hex("#c4cad8"));
    set(&mut props, &tokens::KEY_TEXT, hex("#1d2130"));
    set(&mut props, &tokens::KEY_TARGET, hex("#19b866"));
    set(&mut props, &tokens::KEY_HOLD, hex("#e59a00"));
    set(&mut props, &tokens::KEY_SHIFT, hex("#a32ad6"));
    set(&mut props, &tokens::KEY_ERASE, hex("#d8173f"));
    set(&mut props, &tokens::KEY_TEXT_ON_HINT, hex("#ffffff"));
    set(&mut props, &tokens::CHAR_PENDING, hex("#7a8196"));
    set(&mut props, &tokens::CHAR_CURRENT, hex("#1d2130"));
    set(&mut props, &tokens::CHAR_CORRECT, hex("#138a4b"));
    set(&mut props, &tokens::CHAR_WRONG, hex("#d8173f"));
    set(&mut props, &tokens::CHAR_WRONG_BG, hex("#d8173f").with_alpha(0.18));
    set(&mut props, &tokens::CHAR_RETOUCHED, hex("#b87800"));
    set(&mut props, &tokens::SELECTION, hex("#0088b8").with_alpha(0.25));
    set(&mut props, &tokens::CURSOR, hex("#0088b8"));
    set(&mut props, &tokens::SPARK_OK, hex("#0088b8"));
    set(&mut props, &tokens::SPARK_ERROR, hex("#d8173f"));
    set(&mut props, &tokens::PROGRESS_TRACK, hex("#d5d9e3"));
    set(&mut props, &tokens::PROGRESS_BAR, hex("#0088b8"));
    set_fingers(&mut props, LIGHT_FINGERS, LIGHT_LAYERS);
    props
}

pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        // The GamePlugin inserts the saved theme.
        app.init_resource::<ThemeMode>()
            .add_systems(PreUpdate, apply_theme_mode.run_if(resource_changed::<ThemeMode>));
    }
}

/// Swaps the [`UiTheme`], feathers updates the backgrounds, the borders and the direct text
/// colors by itself, the inherited text colors are refreshed by inserting them again.
fn apply_theme_mode(
    mode: Res<ThemeMode>,
    mut theme: ResMut<UiTheme>,
    mut clear_color: ResMut<ClearColor>,
    inherited: Query<(Entity, &InheritableThemeTextColor)>,
    mut commands: Commands,
) {
    theme.0 = props(*mode);
    clear_color.0 = theme.color(&feathers_tokens::WINDOW_BG);
    for (entity, color) in &inherited {
        commands.entity(entity).insert(color.clone());
    }
}
