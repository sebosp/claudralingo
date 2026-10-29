//! The keyboard layouts shown on the screen.
//!
//! The QWERTY layout is made from the grid of Klavaro's `qwerty_us.kbd`, with the modifiers, the
//! escape key and the navigation keys around it. The Corne is a split 3x5+3 keyboard (36 keys)
//! with a QWERTY base layer and home row modifiers, the numbers, the symbols and the navigation
//! keys live on the Lower and Raise layers held with the thumbs.
use serde::{Deserialize, Serialize};

use crate::klavaro::{BACKSPACE, KbdGrid, UPSYM};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LayoutId {
    QwertyUs,
    #[default]
    Corne36,
}

impl LayoutId {
    pub const ALL: [LayoutId; 2] = [LayoutId::QwertyUs, LayoutId::Corne36];

    pub fn name(self) -> &'static str {
        match self {
            LayoutId::QwertyUs => "QWERTY (US)",
            LayoutId::Corne36 => "Corne 3x5+3",
        }
    }

    /// The name used in the progress file.
    pub fn key(self) -> &'static str {
        match self {
            LayoutId::QwertyUs => "qwerty_us",
            LayoutId::Corne36 => "corne_3x5_3",
        }
    }

    pub fn build(self) -> Layout {
        match self {
            LayoutId::QwertyUs => qwerty(&KbdGrid::qwerty_us()),
            LayoutId::Corne36 => corne36(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Special {
    Backspace,
    Tab,
    Caps,
    Enter,
    Shift,
    Space,
    /// Holds a layer while pressed.
    Layer(u8),
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    Insert,
    Ctrl,
    Alt,
    Super,
}

impl Special {
    fn legend(self, layer_names: &[&str]) -> String {
        match self {
            Special::Backspace => "⌫".into(),
            Special::Enter => "⏎".into(),
            Special::Shift => "⇧".into(),
            Special::Space => "Spc".into(),
            Special::Insert => "Ins".into(),
            special => special.name(layer_names),
        }
    }

    /// The name in the instructions.
    pub fn name(self, layer_names: &[&str]) -> String {
        match self {
            Special::Backspace => "Backspace",
            Special::Tab => "Tab",
            Special::Caps => "Caps",
            Special::Enter => "Enter",
            Special::Shift => "Shift",
            Special::Space => "Space",
            Special::Layer(l) => return layer_names.get(l as usize).copied().unwrap_or("?").into(),
            Special::Escape => "Esc",
            Special::Left => "←",
            Special::Right => "→",
            Special::Up => "↑",
            Special::Down => "↓",
            Special::Home => "Home",
            Special::End => "End",
            Special::PageUp => "PgUp",
            Special::PageDown => "PgDn",
            Special::Delete => "Del",
            Special::Insert => "Insert",
            Special::Ctrl => "Ctrl",
            Special::Alt => "Alt",
            Special::Super => "Super",
        }
        .into()
    }

    /// The character the tutor receives from this key.
    pub fn typed(self) -> Option<char> {
        match self {
            Special::Backspace => Some(BACKSPACE),
            Special::Enter => Some(UPSYM),
            Special::Space => Some(' '),
            _ => None,
        }
    }
}

/// The modifiers held with a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

impl Mods {
    pub const NONE: Mods = Mods { ctrl: false, alt: false, shift: false, super_key: false };
    pub const CTRL: Mods = Mods { ctrl: true, ..Mods::NONE };
    pub const ALT: Mods = Mods { alt: true, ..Mods::NONE };
    pub const SHIFT: Mods = Mods { shift: true, ..Mods::NONE };
    pub const CTRL_SHIFT: Mods = Mods { ctrl: true, shift: true, ..Mods::NONE };

    /// The modifier keys other than shift, which is part of the characters.
    fn held(self) -> impl Iterator<Item = Special> {
        [(self.ctrl, Special::Ctrl), (self.alt, Special::Alt), (self.super_key, Special::Super)]
            .into_iter()
            .filter(|(on, _)| *on)
            .map(|(_, special)| special)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyName {
    Char(char),
    Special(Special),
}

/// A key with modifiers, like Ctrl+C.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    pub mods: Mods,
    pub key: KeyName,
}

impl Chord {
    pub const fn new(mods: Mods, key: KeyName) -> Self {
        Chord { mods, key }
    }

    pub const fn special(special: Special) -> Self {
        Chord { mods: Mods::NONE, key: KeyName::Special(special) }
    }

    pub const fn char(c: char) -> Self {
        Chord { mods: Mods::NONE, key: KeyName::Char(c) }
    }

    /// `Ctrl+Shift+→`, the names used in the progress file too.
    pub fn name(&self) -> String {
        let mut parts: Vec<String> = self.mods.held().map(|s| s.name(&[])).collect();
        if self.mods.shift {
            parts.push("Shift".into());
        }
        parts.push(match self.key {
            KeyName::Char(' ') => "Space".into(),
            KeyName::Char(c) if self.mods != Mods::NONE => c.to_ascii_uppercase().to_string(),
            KeyName::Char(c) => c.to_string(),
            KeyName::Special(s) => s.name(&[]),
        });
        parts.join("+")
    }
}

/// What to press next: a character of a text or a chord.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Char(char),
    Chord(Chord),
}

/// What a key types on a layer: the char without and with shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Output {
    pub lo: char,
    pub up: char,
}

/// The rows used in the instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Number,
    Top,
    Home,
    Bottom,
    Thumb,
    Other,
}

impl Row {
    fn name(self) -> &'static str {
        match self {
            Row::Number => "number row",
            Row::Top => "top row",
            Row::Home => "home row",
            Row::Bottom => "bottom row",
            Row::Thumb => "thumb cluster",
            Row::Other => "side keys",
        }
    }
}

/// Klavaro's finger numbers, 1 is the left pinky, 5 the thumbs, 9 the right pinky.
pub fn finger_name(finger: u8) -> &'static str {
    match finger {
        1 => "left pinky",
        2 => "left ring finger",
        3 => "left middle finger",
        4 => "left index",
        5 => "thumbs",
        6 => "right index",
        7 => "right middle finger",
        8 => "right ring finger",
        9 => "right pinky",
        _ => "any finger",
    }
}

#[derive(Debug, Clone)]
pub struct KeySpec {
    /// One output per layer of the layout.
    pub outputs: Vec<Option<Output>>,
    /// The special key of the base layer, the thumb keys and the modifiers.
    pub special: Option<Special>,
    /// The special keys of the other layers, by layer.
    pub layer_specials: Vec<Option<Special>>,
    /// The modifier of a home row mod, sent while the key is held.
    pub hold: Option<Special>,
    pub finger: u8,
    pub row: Row,
    pub left: bool,
    /// In key units.
    pub width: f32,
    /// Vertical offset in key units, for the column stagger of the split keyboards.
    pub y_offset: f32,
}

impl KeySpec {
    pub fn special_on(&self, layer: u8) -> Option<Special> {
        if layer == 0 {
            self.special
        } else {
            self.layer_specials.get(layer as usize).copied().flatten()
        }
    }

    pub fn legend(&self, layer: u8, shift: bool, layer_names: &[&str]) -> String {
        if self.special == Some(Special::Space) && self.width >= 2.0 {
            return "Space".into();
        }
        if let Some(special) = self.special.or_else(|| self.special_on(layer)) {
            return special.legend(layer_names);
        }
        match self.outputs.get(layer as usize).copied().flatten() {
            Some(out) if shift => out.up.to_string(),
            Some(out) => out.lo.to_string(),
            None => String::new(),
        }
    }

    /// The small legend of a home row mod.
    pub fn hold_legend(&self) -> Option<&'static str> {
        match self.hold? {
            Special::Ctrl => Some("ctl"),
            Special::Alt => Some("alt"),
            Special::Super => Some("sup"),
            Special::Shift => Some("sft"),
            _ => None,
        }
    }

    /// The base layer letter, to find the key on a layer.
    fn base_name(&self, layer_names: &[&str]) -> String {
        match (self.special, self.outputs.first().copied().flatten()) {
            (Some(special), _) => special.name(layer_names),
            (None, Some(out)) => out.up.to_string(),
            (None, None) => "?".into(),
        }
    }
}

/// A group of rows drawn together, a whole keyboard or the half of a split one.
#[derive(Debug, Clone)]
pub struct Block {
    /// Indices into [`Layout::keys`], with the left offset of the row in key units.
    pub rows: Vec<(f32, Vec<usize>)>,
    /// The tilt of the block in degrees, clockwise.
    pub tilt: f32,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub id: LayoutId,
    pub keys: Vec<KeySpec>,
    pub blocks: Vec<Block>,
    pub layer_names: Vec<&'static str>,
    /// The size of a key unit in logical pixels.
    pub unit: f32,
}

/// Where to find a character: the key, the layer to hold and whether shift is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyLocation {
    pub key: usize,
    pub layer: u8,
    pub shift: bool,
}

/// All the keys to press for a [`Target`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub key: usize,
    pub layer: u8,
    /// Shift is needed, with the key to hold when the layout has one.
    pub shift: bool,
    pub shift_key: Option<usize>,
    pub layer_key: Option<usize>,
    /// The Ctrl, Alt and Super keys to hold.
    pub mods: Vec<(Special, usize)>,
}

impl Plan {
    pub fn keys(&self) -> impl Iterator<Item = usize> + '_ {
        std::iter::once(self.key)
            .chain(self.shift_key)
            .chain(self.layer_key)
            .chain(self.mods.iter().map(|(_, k)| *k))
    }

    /// Needs more than a key of the base layer, for the layer trainer.
    pub fn is_combo(&self) -> bool {
        self.layer > 0 || !self.mods.is_empty()
    }
}

impl Layout {
    /// The keys to press for a character, the base layer first.
    pub fn locate(&self, c: char) -> Option<KeyLocation> {
        for (i, key) in self.keys.iter().enumerate() {
            if key.special.and_then(Special::typed) == Some(c) {
                return Some(KeyLocation { key: i, layer: 0, shift: false });
            }
        }
        let n_layers = self.layer_names.len();
        for layer in 0..n_layers {
            for shift in [false, true] {
                for (i, key) in self.keys.iter().enumerate() {
                    if let Some(Some(out)) = key.outputs.get(layer) {
                        let typed = if shift { out.up } else { out.lo };
                        if typed == c && (!shift || out.up != out.lo) {
                            return Some(KeyLocation { key: i, layer: layer as u8, shift });
                        }
                    }
                }
            }
        }
        None
    }

    /// A special key, the base layer first.
    pub fn locate_special(&self, special: Special) -> Option<KeyLocation> {
        (0..self.layer_names.len() as u8).find_map(|layer| {
            self.keys
                .iter()
                .position(|k| k.special_on(layer) == Some(special))
                .map(|key| KeyLocation { key, layer, shift: false })
        })
    }

    pub fn find_special(&self, special: Special) -> impl Iterator<Item = usize> + '_ {
        self.keys
            .iter()
            .enumerate()
            .filter(move |(_, k)| k.special == Some(special))
            .map(|(i, _)| i)
    }

    /// A key of the other hand among `keys`, Klavaro's hints use the same rule for shift.
    fn other_hand(&self, key: usize, keys: &[usize]) -> Option<usize> {
        let left = self.keys[key].left;
        keys.iter().copied().find(|k| self.keys[*k].left != left).or(keys.first().copied())
    }

    pub fn shift_for(&self, key: usize) -> Option<usize> {
        let shifts: Vec<usize> = self.find_special(Special::Shift).collect();
        self.other_hand(key, &shifts)
    }

    pub fn layer_key_for(&self, key: usize, layer: u8) -> Option<usize> {
        let keys: Vec<usize> = self.find_special(Special::Layer(layer)).collect();
        self.other_hand(key, &keys)
    }

    /// A Ctrl, Alt or Super key, or a home row mod, of the other hand.
    fn modifier_for(&self, key: usize, modifier: Special) -> Option<usize> {
        let keys: Vec<usize> = (0..self.keys.len())
            .filter(|k| self.keys[*k].special == Some(modifier) || self.keys[*k].hold == Some(modifier))
            .filter(|k| *k != key)
            .collect();
        self.other_hand(key, &keys)
    }

    pub fn plan(&self, target: Target) -> Option<Plan> {
        let (location, mods) = match target {
            Target::Char(c) => (self.locate(c)?, Mods::NONE),
            Target::Chord(chord) => {
                let location = match chord.key {
                    KeyName::Char(c) => self.locate(c)?,
                    KeyName::Special(s) => self.locate_special(s)?,
                };
                (location, chord.mods)
            }
        };
        let key = location.key;
        let shift = location.shift || mods.shift;
        Some(Plan {
            key,
            layer: location.layer,
            shift,
            shift_key: if shift { self.shift_for(key) } else { None },
            layer_key: if location.layer > 0 { self.layer_key_for(key, location.layer) } else { None },
            mods: mods.held().filter_map(|m| Some((m, self.modifier_for(key, m)?))).collect(),
        })
    }

    /// How to press a plan, for the layer trainer.
    pub fn describe(&self, plan: &Plan) -> String {
        let names = &self.layer_names;
        let finger = |k: usize| self.finger_label(k);
        let mut holds = Vec::new();
        for (modifier, k) in &plan.mods {
            let name = modifier.name(names);
            if self.keys[*k].special == Some(*modifier) {
                holds.push(format!("{name} ({})", finger(*k)));
            } else {
                holds.push(format!("{name} on {} ({})", self.keys[*k].base_name(names), finger(*k)));
            }
        }
        if let Some(k) = plan.layer_key {
            holds.push(format!("{} ({})", Special::Layer(plan.layer).name(names), finger(k)));
        }
        if let Some(k) = plan.shift_key {
            holds.push(format!("Shift ({})", finger(k)));
        }
        let key = &self.keys[plan.key];
        let legend = key.legend(plan.layer, plan.shift, names);
        let place = format!("{}, {}", finger(plan.key), key.row.name());
        let press = if plan.layer > 0 {
            format!("press {legend} where {} is ({place})", key.base_name(names))
        } else {
            format!("press {legend} ({place})")
        };
        if holds.is_empty() {
            capitalize(&press)
        } else {
            format!("Hold {}, then {press}", holds.join(" + "))
        }
    }

    /// The characters of the base layer typed by a finger, with their row, for the finger drills.
    pub fn finger_chars(&self, finger: u8) -> Vec<(char, Row)> {
        self.keys
            .iter()
            .filter(|k| k.finger == finger && k.special.is_none())
            .filter_map(|k| Some((k.outputs.first().copied().flatten()?.lo, k.row)))
            .filter(|(c, _)| !c.is_whitespace())
            .collect()
    }

    /// The finger of a key, the thumbs with their hand.
    pub fn finger_label(&self, key: usize) -> &'static str {
        match (self.keys[key].finger, self.keys[key].left) {
            (5, true) => "left thumb",
            (5, false) => "right thumb",
            (finger, _) => finger_name(finger),
        }
    }

    /// The finger of a character, for the per finger statistics.
    pub fn finger_of(&self, c: char) -> Option<u8> {
        Some(self.keys[self.locate(c)?.key].finger)
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

fn char_key(lo: char, up: char, finger: u8, row: Row, width: f32) -> KeySpec {
    KeySpec {
        outputs: vec![Some(Output { lo, up })],
        special: None,
        layer_specials: vec![],
        hold: None,
        finger,
        row,
        left: finger < 5,
        width,
        y_offset: 0.0,
    }
}

fn special_key(special: Special, finger: u8, left: bool, width: f32) -> KeySpec {
    KeySpec {
        outputs: vec![],
        special: Some(special),
        layer_specials: vec![],
        hold: None,
        finger,
        row: Row::Other,
        left,
        width,
        y_offset: 0.0,
    }
}

/// The fingers of the columns of the 4 rows of a `.kbd` grid, the usual touch typing ones.
const QWERTY_FINGERS: [[u8; 14]; 4] = [
    [1, 1, 2, 3, 4, 4, 6, 6, 7, 8, 9, 9, 9, 9],
    [1, 2, 3, 4, 4, 6, 6, 7, 8, 9, 9, 9, 9, 9],
    [1, 2, 3, 4, 4, 6, 6, 7, 8, 9, 9, 9, 9, 9],
    [1, 1, 2, 3, 4, 4, 6, 6, 7, 8, 9, 9, 9, 9],
];

/// A row staggered keyboard from the 4x14 grid of a Klavaro `.kbd` file, with the escape key,
/// the modifiers and the navigation keys of a full size keyboard.
pub fn qwerty(grid: &KbdGrid) -> Layout {
    let mut keys = Vec::new();
    let mut rows = Vec::new();
    let push = |keys: &mut Vec<KeySpec>, key: KeySpec| {
        keys.push(key);
        keys.len() - 1
    };
    const GRID_ROWS: [Row; 4] = [Row::Number, Row::Top, Row::Home, Row::Bottom];
    let grid_key = |i: usize, j: usize, width: f32| {
        char_key(grid.lochars[i][j], grid.upchars[i][j], QWERTY_FINGERS[i][j], GRID_ROWS[i], width)
    };
    rows.push((0.0, vec![push(&mut keys, special_key(Special::Escape, 1, true, 1.0))]));
    let mut row: Vec<usize> = (0..13).map(|j| push(&mut keys, grid_key(0, j, 1.0))).collect();
    row.push(push(&mut keys, special_key(Special::Backspace, 9, false, 2.0)));
    rows.push((0.0, row));
    let mut row = vec![push(&mut keys, special_key(Special::Tab, 1, true, 1.5))];
    row.extend((0..13).map(|j| push(&mut keys, grid_key(1, j, if j == 12 { 1.5 } else { 1.0 }))));
    rows.push((0.0, row));
    let mut row = vec![push(&mut keys, special_key(Special::Caps, 1, true, 1.75))];
    row.extend((0..11).map(|j| push(&mut keys, grid_key(2, j, 1.0))));
    row.push(push(&mut keys, special_key(Special::Enter, 9, false, 2.25)));
    rows.push((0.0, row));
    // The first column of the grid is the extra key of the ISO keyboards.
    let mut row = vec![push(&mut keys, special_key(Special::Shift, 1, true, 2.25))];
    row.extend((1..11).map(|j| push(&mut keys, grid_key(3, j, 1.0))));
    row.push(push(&mut keys, special_key(Special::Shift, 9, false, 2.75)));
    rows.push((0.0, row));
    let bottom = [
        (Special::Ctrl, 1, true, 1.25),
        (Special::Super, 1, true, 1.25),
        (Special::Alt, 5, true, 1.25),
        (Special::Space, 5, false, 6.25),
        (Special::Alt, 5, false, 1.25),
        (Special::Super, 9, false, 1.25),
        (Special::Ctrl, 9, false, 2.5),
    ];
    let row = bottom.map(|(special, finger, left, width)| push(&mut keys, special_key(special, finger, left, width)));
    rows.push((0.0, row.to_vec()));
    for key in &mut keys {
        if key.special == Some(Special::Space) || key.special == Some(Special::Alt) {
            key.row = Row::Thumb;
        }
    }
    let main = Block { rows, tilt: 0.0 };

    // The navigation keys of the right pinky, the arrows of the right hand.
    let mut nav = |specials: &[Special], y_offset: f32, finger: u8| -> Vec<usize> {
        specials
            .iter()
            .map(|s| {
                let mut key = special_key(*s, finger, false, 1.0);
                key.y_offset = y_offset;
                push(&mut keys, key)
            })
            .collect()
    };
    // The offsets line the clusters up with the number and the bottom rows.
    let nav_rows = vec![
        (0.0, nav(&[Special::Insert, Special::Home, Special::PageUp], 1.1, 9)),
        (0.0, nav(&[Special::Delete, Special::End, Special::PageDown], 0.0, 9)),
        (1.11, nav(&[Special::Up], 1.11, 7)),
        (0.0, nav(&[Special::Left, Special::Down, Special::Right], 0.0, 7)),
    ];
    let left_arrow = keys.len() - 3;
    keys[left_arrow].finger = 6;
    keys[left_arrow + 2].finger = 8;
    Layout {
        id: LayoutId::QwertyUs,
        keys,
        blocks: vec![main, Block { rows: nav_rows, tilt: 0.0 }],
        layer_names: vec!["Base"],
        unit: 44.0,
    }
}

/// Corne 3x5+3: QWERTY on the base layer with Super, Alt and Ctrl on the home rows (A S D and
/// ; L K), numbers and shifted numbers on Lower, brackets, Escape, Tab and the navigation keys on
/// Raise. Thumbs: Shift, Lower and Space on the left, Enter, Raise and Backspace on the right.
pub fn corne36() -> Layout {
    const BASE: [&str; 6] = ["qwert", "asdfg", "zxcvb", "yuiop", "hjkl;", "nm,./"];
    const BASE_UP: [&str; 6] = ["QWERT", "ASDFG", "ZXCVB", "YUIOP", "HJKL:", "NM<>?"];
    const LOWER: [&str; 6] = ["12345", "!@#$%", "`~'\"\\", "67890", "^&*()", "|-_=+"];
    const RAISE: [&str; 6] = ["     ", "     ", "     ", "[]{} ", "     ", "     "];
    use Special::*;
    const RAISE_SPECIALS: [[Option<Special>; 5]; 6] = [
        [Some(Escape), Some(Tab), Some(Home), Some(End), Some(Delete)],
        [None; 5],
        [None; 5],
        [None; 5],
        [Some(Left), Some(Down), Some(Up), Some(Right), None],
        [Some(PageUp), Some(PageDown), Some(Insert), None, None],
    ];
    // From the outer to the inner column of a half.
    const HOLDS: [Option<Special>; 5] = [Some(Super), Some(Alt), Some(Ctrl), None, None];
    const STAGGER: [f32; 5] = [0.35, 0.12, 0.0, 0.12, 0.22];
    const LEFT_FINGERS: [u8; 5] = [1, 2, 3, 4, 4];
    const ROWS: [Row; 3] = [Row::Top, Row::Home, Row::Bottom];

    let mut keys = Vec::new();
    let mut blocks = Vec::new();
    for half in 0..2 {
        let mut rows = Vec::new();
        for (r, row_kind) in ROWS.into_iter().enumerate() {
            let idx = half * 3 + r;
            let (lo, up) = (BASE[idx].chars().collect::<Vec<_>>(), BASE_UP[idx].chars().collect::<Vec<_>>());
            let (lower, raise) = (LOWER[idx].chars().collect::<Vec<_>>(), RAISE[idx].chars().collect::<Vec<_>>());
            let mut row = Vec::new();
            for c in 0..5 {
                // The stagger, the fingers and the home row mods mirror on the right half.
                let outer = if half == 0 { c } else { 4 - c };
                let layer_output = |ch: char| (ch != ' ').then_some(Output { lo: ch, up: ch });
                keys.push(KeySpec {
                    outputs: vec![
                        Some(Output { lo: lo[c], up: up[c] }),
                        layer_output(lower[c]),
                        layer_output(raise[c]),
                    ],
                    special: None,
                    layer_specials: vec![None, None, RAISE_SPECIALS[idx][c]],
                    hold: if r == 1 { HOLDS[outer] } else { None },
                    finger: if half == 0 { LEFT_FINGERS[outer] } else { 10 - LEFT_FINGERS[outer] },
                    row: row_kind,
                    left: half == 0,
                    width: 1.0,
                    y_offset: STAGGER[outer],
                });
                row.push(keys.len() - 1);
            }
            rows.push((0.0, row));
        }
        let thumbs = if half == 0 {
            [Shift, Layer(1), Space]
        } else {
            [Enter, Layer(2), Backspace]
        };
        let mut row = Vec::new();
        for (t, special) in thumbs.into_iter().enumerate() {
            let mut key = special_key(special, 5, half == 0, 1.0);
            key.row = Row::Thumb;
            // The thumb keys fan out from the middle of the keyboard.
            let from_inner = if half == 0 { 2 - t } else { t };
            key.y_offset = 0.25 + 0.12 * from_inner as f32;
            keys.push(key);
            row.push(keys.len() - 1);
        }
        rows.push((if half == 0 { 2.2 } else { -0.2 }, row));
        blocks.push(Block { rows, tilt: if half == 0 { 4.0 } else { -4.0 } });
    }
    Layout {
        id: LayoutId::Corne36,
        keys,
        blocks,
        layer_names: vec!["Base", "Lower", "Raise"],
        unit: 52.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klavaro::basic::lesson_char_set;
    use crate::klavaro::{N_BASIC_LESSONS, UPSYM};

    #[test]
    fn every_lesson_char_is_on_every_layout() {
        let grid = KbdGrid::qwerty_us();
        for id in LayoutId::ALL {
            let layout = id.build();
            for lesson in 1..=N_BASIC_LESSONS {
                for c in lesson_char_set(lesson, &grid) {
                    assert!(layout.locate(c).is_some(), "{c:?} of lesson {lesson} on {id:?}");
                    let up = crate::klavaro::to_upper(c);
                    assert!(layout.locate(up).is_some(), "{up:?} of lesson {lesson} on {id:?}");
                }
            }
            for c in [' ', UPSYM, BACKSPACE] {
                assert!(layout.locate(c).is_some(), "{c:?} on {id:?}");
            }
        }
    }

    #[test]
    fn every_shortcut_key_is_on_every_layout() {
        use Special::*;
        for id in LayoutId::ALL {
            let layout = id.build();
            for s in [Escape, Tab, Left, Right, Up, Down, Home, End, Delete, Backspace, Enter, Space] {
                assert!(layout.locate_special(s).is_some(), "{s:?} on {id:?}");
            }
            for m in [Ctrl, Alt, Super] {
                let plan = layout.plan(Target::Chord(Chord::new(
                    Mods { ctrl: m == Ctrl, alt: m == Alt, super_key: m == Super, shift: false },
                    KeyName::Char('c'),
                )));
                assert_eq!(plan.map(|p| p.mods.len()), Some(1), "{m:?}+C on {id:?}");
            }
        }
    }

    #[test]
    fn corne_layers() {
        let layout = corne36();
        assert_eq!(layout.keys.len(), 36);
        let one = layout.locate('1').unwrap();
        assert_eq!((one.layer, one.shift), (1, false));
        let a = layout.locate('A').unwrap();
        assert_eq!((a.layer, a.shift), (0, true));
        let brace = layout.locate('{').unwrap();
        assert_eq!(brace.layer, 2);
        // '1' is on the left half, Lower is on the left thumb too.
        assert!(layout.layer_key_for(one.key, 1).is_some());
        let right = layout.locate_special(Special::Right).unwrap();
        assert_eq!(right.layer, 2);
    }

    #[test]
    fn plans_use_the_other_hand() {
        let layout = corne36();
        // Ctrl+C: C is on the left, Ctrl is the K home row mod.
        let plan = layout.plan(Target::Chord(Chord::new(Mods::CTRL, KeyName::Char('c')))).unwrap();
        let (_, ctrl) = plan.mods[0];
        assert_eq!(layout.keys[ctrl].outputs[0].unwrap().lo, 'k');
        assert_eq!(
            layout.describe(&plan),
            "Hold Ctrl on K (right middle finger), then press c (left middle finger, bottom row)"
        );
        let plan = layout.plan(Target::Char('!')).unwrap();
        assert_eq!(
            layout.describe(&plan),
            "Hold Lower (left thumb), then press ! where A is (left pinky, home row)"
        );
        let qwerty = qwerty(&KbdGrid::qwerty_us());
        let plan = qwerty.plan(Target::Char('A')).unwrap();
        assert!(!qwerty.keys[plan.shift_key.unwrap()].left);
    }

    #[test]
    fn qwerty_fingers() {
        let layout = qwerty(&KbdGrid::qwerty_us());
        let chars = |f| layout.finger_chars(f).into_iter().map(|(c, _)| c).collect::<String>();
        assert_eq!(chars(1), "`1qaz");
        assert_eq!(chars(4), "45rtfgvb");
        assert_eq!(layout.finger_of('j'), Some(6));
        assert_eq!(chord_name(), "Ctrl+Shift+→");
    }

    fn chord_name() -> String {
        Chord::new(Mods::CTRL_SHIFT, KeyName::Special(Special::Right)).name()
    }
}
