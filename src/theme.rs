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
    /// The colors of the Tokyo Night theme of Neovim and the terminals.
    TokyoNight,
    /// Gruvbox, its dark variant.
    Gruvbox,
    /// Green on black, the code falling in the background, no hands.
    Matrix,
}

impl ThemeMode {
    pub const ALL: [ThemeMode; 5] =
        [ThemeMode::Dark, ThemeMode::Light, ThemeMode::TokyoNight, ThemeMode::Gruvbox, ThemeMode::Matrix];

    pub fn name(self) -> &'static str {
        match self {
            ThemeMode::Dark => "Dark",
            ThemeMode::Light => "Light",
            ThemeMode::TokyoNight => "Tokyo Night",
            ThemeMode::Gruvbox => "Gruvbox",
            ThemeMode::Matrix => "Matrix",
        }
    }

    pub fn is_dark(self) -> bool {
        self != ThemeMode::Light
    }
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
    /// A line typed with one slip: almost.
    pub const NEAR_MISS: ThemeToken = ThemeToken::new_static("claudralingo.near.miss");
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
        ThemeMode::TokyoNight => terminal_theme(&TOKYO_NIGHT),
        ThemeMode::Gruvbox => terminal_theme(&GRUVBOX),
        ThemeMode::Matrix => terminal_theme(&MATRIX),
    }
}

/// The colors of a dark theme of the terminals, from the darkest to the text.
struct Terminal {
    window: &'static str,
    pane: &'static str,
    item: &'static str,
    border: &'static str,
    text: &'static str,
    dim: &'static str,
    accent: &'static str,
    green: &'static str,
    red: &'static str,
    yellow: &'static str,
    purple: &'static str,
    orange: &'static str,
    /// From the pinky to the thumb, and the layers (base, Lower, Raise).
    fingers: [&'static str; 5],
    layers: [&'static str; 3],
}

/// Tokyo Night, the night variant (folke/tokyonight.nvim).
const TOKYO_NIGHT: Terminal = Terminal {
    window: "#1a1b26",
    pane: "#1f2335",
    item: "#292e42",
    border: "#3b4261",
    text: "#c0caf5",
    dim: "#737aa2",
    accent: "#7aa2f7",
    green: "#9ece6a",
    red: "#f7768e",
    yellow: "#e0af68",
    purple: "#bb9af7",
    orange: "#ff9e64",
    fingers: ["#7aa2f7", "#ff9e64", "#9ece6a", "#e0af68", "#bb9af7"],
    layers: ["#565f89", "#1abc9c", "#bb9af7"],
};

/// Gruvbox dark (morhetz/gruvbox).
const GRUVBOX: Terminal = Terminal {
    window: "#1d2021",
    pane: "#282828",
    item: "#3c3836",
    border: "#504945",
    text: "#ebdbb2",
    dim: "#a89984",
    accent: "#83a598",
    green: "#b8bb26",
    red: "#fb4934",
    yellow: "#fabd2f",
    purple: "#d3869b",
    orange: "#fe8019",
    fingers: ["#83a598", "#fe8019", "#b8bb26", "#fabd2f", "#d3869b"],
    layers: ["#928374", "#8ec07c", "#d3869b"],
};

/// The green phosphor of the movie; the errors stay red, they must be seen.
const MATRIX: Terminal = Terminal {
    window: "#000000",
    pane: "#03140a",
    item: "#062712",
    border: "#0d4d22",
    text: "#b6ffc4",
    dim: "#2fa84f",
    accent: "#00ff41",
    green: "#00ff41",
    red: "#ff3355",
    yellow: "#d4ff3d",
    purple: "#7dffa0",
    orange: "#ffb000",
    fingers: ["#00c853", "#69f0ae", "#00ff41", "#b9f6ca", "#2e7d32"],
    layers: ["#1b5e20", "#00e676", "#a5d6a7"],
};

fn terminal_theme(t: &Terminal) -> ThemeProps {
    let mut props = dark_theme();
    let (window, pane, item, accent) = (hex(t.window), hex(t.pane), hex(t.item), hex(t.accent));
    let text = hex(t.text);
    let base = [
        (semantic::SURFACE_WINDOW, window),
        (semantic::SURFACE_PANE_BODY, pane),
        (semantic::SURFACE_PANE_HEADER, window),
        (semantic::SURFACE_SUBPANE_BODY, pane),
        (semantic::SURFACE_SUBPANE_HEADER, item),
        (semantic::SURFACE_GROUP, item),
        (semantic::SURFACE_DIALOG, pane),
        (semantic::SURFACE_MENU, pane),
        (semantic::BORDER_DEFAULT, hex(t.border)),
        (semantic::TEXT_DEFAULT, text),
        (semantic::TEXT_DIM, hex(t.dim)),
        (semantic::TEXT_DISABLED, text.with_alpha(0.4)),
        (semantic::TEXT_ON_ACCENT, window),
        (semantic::FILL_ACCENT_DEFAULT, accent),
        (semantic::FILL_ACCENT_HOVER, accent.lighter(0.05)),
        (semantic::FILL_ACCENT_PRESSED, accent.lighter(0.1)),
        (semantic::FILL_SOLID_DEFAULT, item.lighter(0.04)),
        (semantic::FILL_SOLID_HOVER, item.lighter(0.08)),
        (semantic::FILL_SOLID_PRESSED, item.lighter(0.12)),
        (semantic::FILL_ITEM_HOVER, pane.lighter(0.05)),
        (semantic::FILL_ITEM_PRESSED, pane.lighter(0.1)),
        (semantic::FILL_FIELD_DEFAULT, item),
        (semantic::FILL_FIELD_HOVER, item.lighter(0.05)),
        (semantic::FILL_FIELD_PRESSED, item.lighter(0.1)),
        (semantic::FOCUS_RING, accent.with_alpha(0.5)),
    ];
    props.semantic_base.extend(base);
    // The controls take the text color, not white: the accent buttons have the window's.
    for token in [
        feathers_tokens::BUTTON_TEXT,
        feathers_tokens::SLIDER_TEXT,
        feathers_tokens::MENUITEM_TEXT,
        feathers_tokens::TEXT_INPUT_TEXT,
        feathers_tokens::LISTROW_TEXT,
    ] {
        props.token_assignments.insert(token, semantic::TEXT_DEFAULT);
    }
    props.semantic_overrides = HashMap::new();
    let (green, red, yellow) = (hex(t.green), hex(t.red), hex(t.yellow));
    set(&mut props, &tokens::KEYBOARD_CASE, pane);
    set(&mut props, &tokens::KEY_BG, item);
    set(&mut props, &tokens::KEY_BORDER, hex(t.border));
    set(&mut props, &tokens::KEY_TEXT, text);
    set(&mut props, &tokens::KEY_TARGET, green);
    set(&mut props, &tokens::KEY_HOLD, yellow);
    set(&mut props, &tokens::KEY_SHIFT, hex(t.purple));
    set(&mut props, &tokens::KEY_ERASE, red);
    set(&mut props, &tokens::KEY_TEXT_ON_HINT, window);
    set(&mut props, &tokens::CHAR_PENDING, hex(t.dim));
    set(&mut props, &tokens::CHAR_CURRENT, text);
    set(&mut props, &tokens::CHAR_CORRECT, green);
    set(&mut props, &tokens::CHAR_WRONG, red);
    set(&mut props, &tokens::CHAR_WRONG_BG, red.with_alpha(0.25));
    set(&mut props, &tokens::CHAR_RETOUCHED, yellow);
    set(&mut props, &tokens::SELECTION, accent.with_alpha(0.3));
    set(&mut props, &tokens::CURSOR, accent);
    set(&mut props, &tokens::SPARK_OK, accent);
    set(&mut props, &tokens::NEAR_MISS, hex(t.orange));
    set(&mut props, &tokens::SPARK_ERROR, red);
    set(&mut props, &tokens::PROGRESS_TRACK, item);
    set(&mut props, &tokens::PROGRESS_BAR, accent);
    set_fingers(&mut props, t.fingers, t.layers);
    props
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
    set(&mut props, &tokens::NEAR_MISS, hex("#ffa62b"));
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
    set(&mut props, &tokens::NEAR_MISS, hex("#c46a00"));
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
