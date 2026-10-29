//! The keyboard layouts shown on the screen.
//!
//! A layout is a list of keys with their physical position (as the ZMK physical layouts: x, y,
//! width, height and a rotation around a point, in key units) and what they type on every layer.
//! The QWERTY layout is made from the grid of Klavaro's `qwerty_us.kbd`, with the modifiers, the
//! escape key and the navigation keys around it. The Corne 3x5+3 is the default 5 column keymap
//! of the Typeractive wireless Corne, parsed from its ZMK bindings, see [`crate::zmk`].
use serde::{Deserialize, Serialize};

use crate::klavaro::{BACKSPACE, KbdGrid, UPSYM};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LayoutId {
    QwertyUs,
    /// The QWERTY of a laptop: no navigation block, no arrows, no numpad.
    QwertyLaptop,
    #[default]
    Corne36,
    Corne42,
    Lily58,
    /// A keymap of ZMK imported in the settings, see [`set_custom_keymap`].
    Custom,
}

impl LayoutId {
    pub const ALL: [LayoutId; 6] = [
        LayoutId::QwertyUs,
        LayoutId::QwertyLaptop,
        LayoutId::Corne36,
        LayoutId::Corne42,
        LayoutId::Lily58,
        LayoutId::Custom,
    ];

    pub fn name(self) -> &'static str {
        match self {
            LayoutId::QwertyUs => "QWERTY (US)",
            LayoutId::QwertyLaptop => "QWERTY laptop (US)",
            LayoutId::Corne36 => "Corne 3x5+3",
            LayoutId::Corne42 => "Corne 3x6+3",
            LayoutId::Lily58 => "Lily58",
            LayoutId::Custom => "My ZMK keymap",
        }
    }

    /// The name used in the progress file.
    pub fn key(self) -> &'static str {
        match self {
            LayoutId::QwertyUs => "qwerty_us",
            LayoutId::QwertyLaptop => "qwerty_laptop",
            LayoutId::Corne36 => "corne_3x5_3",
            LayoutId::Corne42 => "corne_3x6_3",
            LayoutId::Lily58 => "lily58",
            LayoutId::Custom => "custom",
        }
    }

    pub fn build(self) -> Layout {
        match self {
            LayoutId::QwertyUs => qwerty(&KbdGrid::qwerty_us()),
            LayoutId::QwertyLaptop => qwerty_laptop(),
            LayoutId::Corne36 => corne36(),
            LayoutId::Corne42 => corne42(),
            LayoutId::Lily58 => lily58(),
            // Without an imported keymap, the Corne.
            LayoutId::Custom => CUSTOM_KEYMAP
                .read()
                .ok()
                .and_then(|k| k.as_ref().and_then(|text| custom_from_zmk(text).ok()))
                .unwrap_or_else(corne36),
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
    /// The Windows or Command key, ZMK's GUI.
    Super,
    /// Selects a Bluetooth profile, from 0.
    Bluetooth(u8),
    BluetoothClear,
}

impl Special {
    fn legend(self, layer_names: &[&str]) -> String {
        match self {
            Special::Backspace => "⌫".into(),
            Special::Enter => "⏎".into(),
            Special::Shift => "⇧".into(),
            Special::Space => "Spc".into(),
            Special::Insert => "Ins".into(),
            Special::Super => "Win".into(),
            Special::Bluetooth(n) => format!("BT{}", n + 1),
            Special::BluetoothClear => "BT×".into(),
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
            Special::Super => "GUI (Win/Cmd)",
            Special::Bluetooth(n) => return format!("Bluetooth {}", n + 1),
            Special::BluetoothClear => "Bluetooth clear",
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
    /// By layer, the key types what the base layer types (ZMK's `&trans`).
    pub inherited: Vec<bool>,
    pub finger: u8,
    pub row: Row,
    pub left: bool,
    pub pos: KeyPos,
}

/// The physical position of a key, in key units, as in the ZMK physical layouts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyPos {
    /// The top left corner before the rotation.
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Clockwise, in degrees, around `(rx, ry)`.
    pub rot: f32,
    pub rx: f32,
    pub ry: f32,
}

impl KeyPos {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        KeyPos { x, y, w, h, rot: 0.0, rx: 0.0, ry: 0.0 }
    }

    fn rotate(&self, (px, py): (f32, f32)) -> (f32, f32) {
        let (sin, cos) = self.rot.to_radians().sin_cos();
        let (dx, dy) = (px - self.rx, py - self.ry);
        // Clockwise on the screen, the y axis goes down.
        (self.rx + dx * cos - dy * sin, self.ry + dx * sin + dy * cos)
    }

    /// The center of the key after the rotation, a rotated key turns around its center there.
    pub fn center(&self) -> (f32, f32) {
        self.rotate((self.x + self.w / 2.0, self.y + self.h / 2.0))
    }

    /// The corners after the rotation, for the bounding boxes.
    pub fn corners(&self) -> [(f32, f32); 4] {
        let (x, y, w, h) = (self.x, self.y, self.w, self.h);
        [(x, y), (x + w, y), (x + w, y + h), (x, y + h)].map(|p| self.rotate(p))
    }
}

impl KeySpec {
    /// What the key types on a layer: a special key or a character.
    pub fn binding_on(&self, layer: u8) -> (Option<Special>, Option<Output>) {
        match (layer, self.outputs.get(layer as usize).copied().flatten()) {
            (0, output) => (self.special, output),
            (_, Some(output)) => (None, Some(output)),
            (_, None) => (self.special_on(layer).or(self.special), None),
        }
    }

    pub fn is_inherited(&self, layer: u8) -> bool {
        layer > 0 && self.inherited.get(layer as usize).copied().unwrap_or(false)
    }

    pub fn special_on(&self, layer: u8) -> Option<Special> {
        if layer == 0 {
            self.special
        } else {
            self.layer_specials.get(layer as usize).copied().flatten()
        }
    }

    pub fn legend(&self, layer: u8, shift: bool, layer_names: &[&str]) -> String {
        if self.special == Some(Special::Space) && self.pos.w >= 2.0 {
            return "Space".into();
        }
        if let Some(special) = self.special_on(layer).or(self.special) {
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

/// A group of keys drawn on one case: a whole keyboard, the half of a split one, a cluster.
#[derive(Debug, Clone)]
pub struct Block {
    /// Indices into [`Layout::keys`].
    pub keys: Vec<usize>,
}

impl Block {
    /// The bounding box of the keys, rotations included: min x, min y, max x, max y.
    pub fn bounds(&self, keys: &[KeySpec]) -> (f32, f32, f32, f32) {
        let mut b = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (x, y) in self.keys.iter().flat_map(|k| keys[*k].pos.corners()) {
            b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
        }
        b
    }
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub id: LayoutId,
    pub keys: Vec<KeySpec>,
    pub blocks: Vec<Block>,
    pub layer_names: Vec<&'static str>,
    /// The size of a key unit in logical pixels.
    pub unit: f32,
    /// Two halves, one per hand.
    pub split: bool,
    /// The firmware shifts a character when its key is held a moment longer, the shift key is
    /// only for the chords.
    pub auto_shift: bool,
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
    /// The shifted character comes from holding the key a moment longer, no shift key.
    pub auto_shift: bool,
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
        self.layer > 0 || !self.mods.is_empty() || self.auto_shift
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
        self.plan_from(target, None)
    }

    /// The plan with a key of `layer`, when one types the target there: Space is on the thumbs
    /// of Raise too.
    pub fn plan_on(&self, target: Target, layer: u8) -> Option<Plan> {
        self.plan_from(target, Some(layer))
    }

    fn plan_from(&self, target: Target, layer: Option<u8>) -> Option<Plan> {
        let locate = |c: char| match layer {
            None => self.locate(c),
            Some(layer) => self.locate_on(c, layer),
        };
        let locate_special = |s: Special| match layer {
            None => self.locate_special(s),
            Some(layer) => self.locate_special_on(s, layer),
        };
        let (location, mods) = match target {
            Target::Char(c) => (locate(c)?, Mods::NONE),
            Target::Chord(chord) => {
                let location = match chord.key {
                    KeyName::Char(c) => locate(c)?,
                    KeyName::Special(s) => locate_special(s)?,
                };
                (location, chord.mods)
            }
        };
        let key = location.key;
        let shift = location.shift || mods.shift;
        let auto_shift = location.shift && !mods.shift && self.auto_shift;
        Some(Plan {
            key,
            layer: location.layer,
            shift,
            shift_key: if shift && !auto_shift { self.shift_for(key) } else { None },
            auto_shift,
            layer_key: if location.layer > 0 { self.layer_key_for(key, location.layer) } else { None },
            mods: mods.held().filter_map(|m| Some((m, self.modifier_for(key, m)?))).collect(),
        })
    }

    /// A character typed by a key of one layer, the special keys of the thumbs included.
    pub fn locate_on(&self, c: char, layer: u8) -> Option<KeyLocation> {
        for (i, key) in self.keys.iter().enumerate() {
            match key.binding_on(layer) {
                (Some(special), _) if special.typed() == Some(c) => {
                    return Some(KeyLocation { key: i, layer, shift: false });
                }
                (None, Some(out)) if out.lo == c => return Some(KeyLocation { key: i, layer, shift: false }),
                _ => {}
            }
        }
        self.keys.iter().enumerate().find_map(|(i, key)| match key.binding_on(layer) {
            (None, Some(out)) if out.up == c && out.lo != c => Some(KeyLocation { key: i, layer, shift: true }),
            _ => None,
        })
    }

    pub fn locate_special_on(&self, special: Special, layer: u8) -> Option<KeyLocation> {
        let key = self.keys.iter().position(|k| k.binding_on(layer).0 == Some(special))?;
        Some(KeyLocation { key, layer, shift: false })
    }

    /// Every key of the target exists on this layout: Ctrl+Shift+→ needs a shift key, the
    /// keymaps with auto shift may have none.
    pub fn can_press(&self, target: Target) -> bool {
        let Some(plan) = self.plan(target) else {
            return false;
        };
        let held = match target {
            Target::Chord(chord) => chord.mods.held().count(),
            Target::Char(_) => 0,
        };
        (!plan.shift || plan.auto_shift || plan.shift_key.is_some())
            && (plan.layer == 0 || plan.layer_key.is_some())
            && plan.mods.len() == held
    }

    /// The keyboard grid of Klavaro for this layout: its lessons, made for a QWERTY keyboard,
    /// pick the keys by their place. The letter keys of a split keyboard take the places of the
    /// QWERTY keys under the same finger, so a lesson trains the keys of the base layer first:
    /// the home row pinky of the Corne is `'`, not `;`. The places the layout does not have, the
    /// number row and the outer columns, keep their QWERTY characters, typed on the layers in
    /// the later lessons.
    pub fn klavaro_grid(&self) -> KbdGrid {
        let mut grid = KbdGrid::qwerty_us();
        if !self.split {
            return grid;
        }
        // The QWERTY columns of the innermost five keys of each hand, by row: the number row
        // starts at `1`, the bottom row at `z`.
        let rows = [(Row::Number, 0, 1), (Row::Top, 1, 0), (Row::Home, 2, 0), (Row::Bottom, 3, 1)];
        for (row, i, offset) in rows {
            for left in [true, false] {
                let mut keys: Vec<&KeySpec> = self.keys.iter().filter(|k| k.row == row && k.left == left).collect();
                keys.sort_by(|a, b| a.pos.x.total_cmp(&b.pos.x));
                let inner: Vec<&KeySpec> = if left {
                    keys.iter().rev().take(5).rev().copied().collect()
                } else {
                    keys.into_iter().take(5).collect()
                };
                let start = if left { offset + 5 - inner.len() } else { offset + 5 };
                for (n, key) in inner.into_iter().enumerate() {
                    // A modifier there, Ctrl in the place of `,` on the Corne: nothing to type.
                    let out = key.outputs.first().copied().flatten().unwrap_or(Output { lo: ' ', up: ' ' });
                    grid.lochars[i][start + n] = out.lo;
                    grid.upchars[i][start + n] = out.up;
                }
            }
        }
        // The QWERTY characters replaced, `;` and `,` on the Corne, are typed on its layers.
        for c in KbdGrid::qwerty_us().chars_typed() {
            if !grid.is_inset(c) && self.locate(c).is_some() {
                grid.extra.push(c);
            }
        }
        grid
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
        let press = if plan.auto_shift && plan.layer > 0 {
            format!("hold {legend} a moment longer where {} is, auto shift ({place})", key.base_name(names))
        } else if plan.auto_shift {
            format!("hold {legend} a moment longer, auto shift ({place})")
        } else if plan.layer > 0 {
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

    /// The shifted character of the key of a character of the base layer, `C` for `c`.
    pub fn shifted(&self, c: char) -> char {
        self.locate(c)
            .and_then(|l| self.keys[l.key].outputs.first().copied().flatten())
            .map_or(c, |out| out.up)
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
        inherited: vec![],
        finger,
        row,
        left: finger < 5,
        pos: KeyPos::new(0.0, 0.0, width, 1.0),
    }
}

fn special_key(special: Special, finger: u8, left: bool, width: f32) -> KeySpec {
    KeySpec {
        outputs: vec![],
        special: Some(special),
        layer_specials: vec![],
        hold: None,
        inherited: vec![],
        finger,
        row: Row::Other,
        left,
        pos: KeyPos::new(0.0, 0.0, width, 1.0),
    }
}

/// Puts the keys side by side from `x` on the row `y`.
fn place_row(keys: &mut [KeySpec], row: &[usize], x: f32, y: f32) {
    let mut x = x;
    for k in row {
        keys[*k].pos.x = x;
        keys[*k].pos.y = y;
        x += keys[*k].pos.w;
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
/// The QWERTY without the navigation block and the arrows, as on a laptop.
pub fn qwerty_laptop() -> Layout {
    let mut layout = qwerty(&KbdGrid::qwerty_us());
    let main = layout.blocks[0].keys.len();
    layout.keys.truncate(main);
    layout.blocks.truncate(1);
    layout.id = LayoutId::QwertyLaptop;
    layout
}

pub fn qwerty(grid: &KbdGrid) -> Layout {
    let mut keys = Vec::new();
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let push = |keys: &mut Vec<KeySpec>, key: KeySpec| {
        keys.push(key);
        keys.len() - 1
    };
    const GRID_ROWS: [Row; 4] = [Row::Number, Row::Top, Row::Home, Row::Bottom];
    let grid_key = |i: usize, j: usize, width: f32| {
        char_key(grid.lochars[i][j], grid.upchars[i][j], QWERTY_FINGERS[i][j], GRID_ROWS[i], width)
    };
    rows.push(vec![push(&mut keys, special_key(Special::Escape, 1, true, 1.0))]);
    let mut row: Vec<usize> = (0..13).map(|j| push(&mut keys, grid_key(0, j, 1.0))).collect();
    row.push(push(&mut keys, special_key(Special::Backspace, 9, false, 2.0)));
    rows.push(row);
    let mut row = vec![push(&mut keys, special_key(Special::Tab, 1, true, 1.5))];
    row.extend((0..13).map(|j| push(&mut keys, grid_key(1, j, if j == 12 { 1.5 } else { 1.0 }))));
    rows.push(row);
    let mut row = vec![push(&mut keys, special_key(Special::Caps, 1, true, 1.75))];
    row.extend((0..11).map(|j| push(&mut keys, grid_key(2, j, 1.0))));
    row.push(push(&mut keys, special_key(Special::Enter, 9, false, 2.25)));
    rows.push(row);
    // The first column of the grid is the extra key of the ISO keyboards.
    let mut row = vec![push(&mut keys, special_key(Special::Shift, 1, true, 2.25))];
    row.extend((1..11).map(|j| push(&mut keys, grid_key(3, j, 1.0))));
    row.push(push(&mut keys, special_key(Special::Shift, 9, false, 2.75)));
    rows.push(row);
    let bottom = [
        (Special::Ctrl, 1, true, 1.25),
        (Special::Super, 1, true, 1.25),
        (Special::Alt, 5, true, 1.25),
        (Special::Space, 5, false, 6.25),
        (Special::Alt, 5, false, 1.25),
        (Special::Super, 9, false, 1.25),
        (Special::Ctrl, 9, false, 2.5),
    ];
    rows.push(bottom.map(|(special, finger, left, width)| push(&mut keys, special_key(special, finger, left, width))).to_vec());
    for (y, row) in rows.iter().enumerate() {
        place_row(&mut keys, row, 0.0, y as f32);
    }
    for key in &mut keys {
        if key.special == Some(Special::Space) || key.special == Some(Special::Alt) {
            key.row = Row::Thumb;
        }
    }
    let main = Block { keys: (0..keys.len()).collect() };

    // The navigation keys of the right pinky, the arrows of the right hand, lined up with the
    // number and the bottom rows.
    let first_nav = keys.len();
    let nav = [
        (Special::Insert, 0.0, 1.0, 9),
        (Special::Home, 1.0, 1.0, 9),
        (Special::PageUp, 2.0, 1.0, 9),
        (Special::Delete, 0.0, 2.0, 9),
        (Special::End, 1.0, 2.0, 9),
        (Special::PageDown, 2.0, 2.0, 9),
        (Special::Up, 1.0, 4.0, 7),
        (Special::Left, 0.0, 5.0, 6),
        (Special::Down, 1.0, 5.0, 7),
        (Special::Right, 2.0, 5.0, 8),
    ];
    for (special, x, y, finger) in nav {
        let mut key = special_key(special, finger, false, 1.0);
        key.pos = KeyPos::new(x, y, 1.0, 1.0);
        keys.push(key);
    }
    let nav = Block { keys: (first_nav..keys.len()).collect() };
    Layout {
        id: LayoutId::QwertyUs,
        keys,
        blocks: vec![main, nav],
        layer_names: vec!["Base"],
        unit: 44.0,
        split: false,
        auto_shift: false,
    }
}

/// The default 5 column keymap of the Typeractive wireless Corne, `config/corne.keymap` of
/// <https://github.com/typeractivexyz/corne-wireless-5-col-zmk-config>: auto shift on the
/// letters (`AS`), Ctrl and Alt on the right bottom row, GUI, Lower and Space on the left thumbs,
/// Enter, Raise and Backspace on the right ones, symbols and arrows on Lower, numbers and brackets
/// on Raise.
const CORNE_5COL_KEYMAP: [&str; 3] = [
    "AS(Q) AS(W) AS(E) AS(R) AS(T)     AS(Y)   AS(U)  AS(I)     AS(O)     AS(P)
     AS(A) AS(S) AS(D) AS(F) AS(G)     AS(H)   AS(J)  AS(K)     AS(L)     AS(SQT)
     AS(Z) AS(X) AS(C) AS(V) AS(B)     AS(N)   AS(M)  &kp LCTL  &kp RALT  AS(DOT)
          &kp LGUI &mo 1 &kp SPACE     AS(RET) &mo 2  &kp BSPC",
    "&kp EXCL     &kp AT       &kp HASH     &kp DLLR     &kp PRCNT        &kp CARET &kp AMPS  &kp KP_MULTIPLY &kp LPAR  &kp RPAR
     &bt BT_SEL 0 &bt BT_SEL 1 &bt BT_SEL 2 &bt BT_SEL 3 &bt BT_SEL 4     &kp LEFT  &kp DOWN  &kp UP          &kp RIGHT &trans
     &bt BT_CLR   &trans       &trans       &trans       &trans           &trans    &trans    &trans          &trans    &trans
                               &kp LGUI     &trans       &kp SPACE        AS(RET)   &kp ESC   &kp TAB",
    "AS(N1)    AS(N2)    AS(N3)    AS(N4)    AS(N5)        AS(N6)    AS(N7)    AS(N8)    AS(N9)   AS(N0)
     AS(MINUS) AS(EQUAL) &kp LBRC  &kp LBKT  &kp LPAR      &kp RPAR  &kp RBKT  &kp RBRC  AS(FSLH) AS(BSLH)
     AS(GRAVE) &trans    &trans    AS(COMMA) &kp COLON     &kp SEMI  AS(DOT)   &kp LCTRL &kp RALT &trans
                         &kp LGUI  &kp ESC   &kp SPACE     AS(RET)   &trans    &kp TAB",
];

/// The default keymap of ZMK for the Corne with six columns, `app/boards/shields/corne/corne.keymap`.
const CORNE_6COL_KEYMAP: [&str; 3] = [
    "&kp TAB   &kp Q &kp W &kp E &kp R &kp T   &kp Y &kp U  &kp I     &kp O   &kp P    &kp BSPC
     &kp LCTRL &kp A &kp S &kp D &kp F &kp G   &kp H &kp J  &kp K     &kp L   &kp SEMI &kp SQT
     &kp LSHFT &kp Z &kp X &kp C &kp V &kp B   &kp N &kp M  &kp COMMA &kp DOT &kp FSLH &kp ESC
                    &kp LGUI &mo 1 &kp SPACE   &kp RET &mo 2 &kp RALT",
    "&kp TAB    &kp N1       &kp N2       &kp N3       &kp N4       &kp N5         &kp N6   &kp N7   &kp N8 &kp N9    &kp N0 &kp BSPC
     &bt BT_CLR &bt BT_SEL 0 &bt BT_SEL 1 &bt BT_SEL 2 &bt BT_SEL 3 &bt BT_SEL 4   &kp LEFT &kp DOWN &kp UP &kp RIGHT &trans &trans
     &kp LSHFT  &trans       &trans       &trans       &trans       &trans         &trans   &trans   &trans &trans    &trans &trans
                             &kp LGUI     &trans       &kp SPACE      &kp RET  &trans   &kp RALT",
    "&kp TAB   &kp EXCL &kp AT &kp HASH &kp DLLR &kp PRCNT   &kp CARET &kp AMPS  &kp ASTRK &kp LPAR &kp RPAR &kp BSPC
     &kp LCTRL &trans   &trans &trans   &trans   &trans      &kp MINUS &kp EQUAL &kp LBKT  &kp RBKT &kp BSLH &kp GRAVE
     &kp LSHFT &trans   &trans &trans   &trans   &trans      &kp UNDER &kp PLUS  &kp LBRC  &kp RBRC &kp PIPE &kp TILDE
                        &kp LGUI &trans   &kp SPACE   &kp RET   &trans    &kp RALT",
];

/// `foostan_corne_6col_layout` of ZMK (`app/dts/layouts/foostan/corne/6column.dtsi`).
#[rustfmt::skip]
const CORNE_6COL_PHYSICAL: [(i32, i32, i32, i32, i32, i32, i32); 42] = [
    (0, 37, 100, 100, 0, 0, 0), (100, 37, 100, 100, 0, 0, 0), (200, 12, 100, 100, 0, 0, 0),
    (300, 0, 100, 100, 0, 0, 0), (400, 12, 100, 100, 0, 0, 0), (500, 24, 100, 100, 0, 0, 0),
    (800, 24, 100, 100, 0, 0, 0), (900, 12, 100, 100, 0, 0, 0), (1000, 0, 100, 100, 0, 0, 0),
    (1100, 12, 100, 100, 0, 0, 0), (1200, 37, 100, 100, 0, 0, 0), (1300, 37, 100, 100, 0, 0, 0),
    (0, 137, 100, 100, 0, 0, 0), (100, 137, 100, 100, 0, 0, 0), (200, 112, 100, 100, 0, 0, 0),
    (300, 100, 100, 100, 0, 0, 0), (400, 112, 100, 100, 0, 0, 0), (500, 124, 100, 100, 0, 0, 0),
    (800, 124, 100, 100, 0, 0, 0), (900, 112, 100, 100, 0, 0, 0), (1000, 100, 100, 100, 0, 0, 0),
    (1100, 112, 100, 100, 0, 0, 0), (1200, 137, 100, 100, 0, 0, 0), (1300, 137, 100, 100, 0, 0, 0),
    (0, 237, 100, 100, 0, 0, 0), (100, 237, 100, 100, 0, 0, 0), (200, 212, 100, 100, 0, 0, 0),
    (300, 200, 100, 100, 0, 0, 0), (400, 212, 100, 100, 0, 0, 0), (500, 224, 100, 100, 0, 0, 0),
    (800, 224, 100, 100, 0, 0, 0), (900, 212, 100, 100, 0, 0, 0), (1000, 200, 100, 100, 0, 0, 0),
    (1100, 212, 100, 100, 0, 0, 0), (1200, 237, 100, 100, 0, 0, 0), (1300, 237, 100, 100, 0, 0, 0),
    (350, 312, 100, 100, 0, 0, 0), (450, 312, 100, 100, 1200, 450, 412),
    (548, 283, 100, 150, 2400, 548, 433), (752, 283, 100, 150, -2400, 852, 433),
    (850, 312, 100, 100, -1200, 950, 412), (950, 312, 100, 100, 0, 0, 0),
];

/// The default keymap of ZMK for the Lily58, `app/boards/shields/lily58/lily58.keymap`.
const LILY58_KEYMAP: [&str; 3] = [
    "&kp ESC   &kp N1 &kp N2 &kp N3   &kp N4   &kp N5                     &kp N6 &kp N7   &kp N8    &kp N9  &kp N0   &kp GRAVE
     &kp TAB   &kp Q  &kp W  &kp E    &kp R    &kp T                      &kp Y  &kp U    &kp I     &kp O   &kp P    &kp MINUS
     &kp LCTRL &kp A  &kp S  &kp D    &kp F    &kp G                      &kp H  &kp J    &kp K     &kp L   &kp SEMI &kp SQT
     &kp LSHFT &kp Z  &kp X  &kp C    &kp V    &kp B  &kp LBKT   &kp RBKT &kp N  &kp M    &kp COMMA &kp DOT &kp FSLH &kp RSHFT
                             &kp LALT &kp LGUI &mo 1  &kp SPACE  &kp RET  &mo 2  &kp BSPC &kp RGUI",
    "&bt BT_CLR &bt BT_SEL 0 &bt BT_SEL 1 &bt BT_SEL 2 &bt BT_SEL 3 &bt BT_SEL 4    &trans &trans &trans &trans &trans &trans
     &kp F1     &kp F2       &kp F3       &kp F4       &kp F5       &kp F6          &kp F7 &kp F8 &kp F9 &kp F10 &kp F11 &kp F12
     &kp GRAVE  &kp EXCL     &kp AT       &kp HASH     &kp DOLLAR   &kp PRCNT       &kp CARET &kp AMPS &kp ASTRK &kp LPAR &kp RPAR &kp TILDE
     &trans     &ext_power EP_ON &ext_power EP_OFF &ext_power EP_TOG &trans &trans &trans &trans  &trans &kp MINUS &kp PLUS &kp LBRC &kp RBRC &kp PIPE
                             &trans &trans &trans &trans   &trans &trans &trans &trans",
    "&trans    &trans &trans &trans  &trans  &trans                 &trans      &trans    &trans    &trans   &trans    &trans
     &kp GRAVE &kp N1 &kp N2 &kp N3  &kp N4  &kp N5                 &kp N6      &kp N7    &kp N8    &kp N9   &kp N0    &trans
     &kp F1    &kp F2 &kp F3 &kp F4  &kp F5  &kp F6                 &trans      &kp LEFT  &kp DOWN  &kp UP   &kp RIGHT &trans
     &kp F7    &kp F8 &kp F9 &kp F10 &kp F11 &kp F12 &trans &trans  &kp KP_PLUS &kp MINUS &kp EQUAL &kp LBKT &kp RBKT  &kp BSLH
                             &trans &trans &trans &trans   &trans &trans &trans &trans",
];

/// `kata0510_lily58_layout` of ZMK (`app/dts/layouts/kata0510/lily58.dtsi`).
#[rustfmt::skip]
const LILY58_PHYSICAL: [(i32, i32, i32, i32, i32, i32, i32); 58] = [
    (0, 50, 100, 100, 0, 0, 0), (100, 37, 100, 100, 0, 0, 0), (200, 12, 100, 100, 0, 0, 0),
    (300, 0, 100, 100, 0, 0, 0), (400, 12, 100, 100, 0, 0, 0), (500, 25, 100, 100, 0, 0, 0),
    (1050, 25, 100, 100, 0, 0, 0), (1150, 12, 100, 100, 0, 0, 0), (1250, 0, 100, 100, 0, 0, 0),
    (1350, 12, 100, 100, 0, 0, 0), (1450, 37, 100, 100, 0, 0, 0), (1550, 50, 100, 100, 0, 0, 0),
    (0, 150, 100, 100, 0, 0, 0), (100, 137, 100, 100, 0, 0, 0), (200, 112, 100, 100, 0, 0, 0),
    (300, 100, 100, 100, 0, 0, 0), (400, 112, 100, 100, 0, 0, 0), (500, 125, 100, 100, 0, 0, 0),
    (1050, 125, 100, 100, 0, 0, 0), (1150, 112, 100, 100, 0, 0, 0), (1250, 100, 100, 100, 0, 0, 0),
    (1350, 112, 100, 100, 0, 0, 0), (1450, 137, 100, 100, 0, 0, 0), (1550, 150, 100, 100, 0, 0, 0),
    (0, 250, 100, 100, 0, 0, 0), (100, 237, 100, 100, 0, 0, 0), (200, 212, 100, 100, 0, 0, 0),
    (300, 200, 100, 100, 0, 0, 0), (400, 212, 100, 100, 0, 0, 0), (500, 225, 100, 100, 0, 0, 0),
    (1050, 225, 100, 100, 0, 0, 0), (1150, 212, 100, 100, 0, 0, 0), (1250, 200, 100, 100, 0, 0, 0),
    (1350, 212, 100, 100, 0, 0, 0), (1450, 237, 100, 100, 0, 0, 0), (1550, 250, 100, 100, 0, 0, 0),
    (0, 350, 100, 100, 0, 0, 0), (100, 337, 100, 100, 0, 0, 0), (200, 312, 100, 100, 0, 0, 0),
    (300, 300, 100, 100, 0, 0, 0), (400, 312, 100, 100, 0, 0, 0), (500, 325, 100, 100, 0, 0, 0),
    (600, 275, 100, 100, 0, 0, 0), (950, 275, 100, 100, 0, 0, 0), (1050, 325, 100, 100, 0, 0, 0),
    (1150, 312, 100, 100, 0, 0, 0), (1250, 300, 100, 100, 0, 0, 0), (1350, 312, 100, 100, 0, 0, 0),
    (1450, 337, 100, 100, 0, 0, 0), (1550, 350, 100, 100, 0, 0, 0),
    (250, 412, 100, 100, 0, 0, 0), (350, 415, 100, 100, 0, 0, 0), (450, 425, 100, 100, 0, 0, 0),
    (575, 400, 100, 150, 3000, 625, 475), (975, 400, 100, 150, -3000, 1025, 475),
    (1100, 425, 100, 100, 0, 0, 0), (1200, 415, 100, 100, 0, 0, 0), (1300, 415, 100, 100, 0, 0, 0),
];

/// `foostan_corne_5col_layout` of ZMK (`app/dts/layouts/foostan/corne/5column.dtsi`), in
/// hundredths of a key: x, y, w, h, rotation in hundredths of a degree, rotation point.
#[rustfmt::skip]
const CORNE_5COL_PHYSICAL: [(i32, i32, i32, i32, i32, i32, i32); 36] = [
    (0, 37, 100, 100, 0, 0, 0), (100, 12, 100, 100, 0, 0, 0), (200, 0, 100, 100, 0, 0, 0),
    (300, 12, 100, 100, 0, 0, 0), (400, 24, 100, 100, 0, 0, 0), (700, 24, 100, 100, 0, 0, 0),
    (800, 12, 100, 100, 0, 0, 0), (900, 0, 100, 100, 0, 0, 0), (1000, 12, 100, 100, 0, 0, 0),
    (1100, 37, 100, 100, 0, 0, 0), (0, 137, 100, 100, 0, 0, 0), (100, 112, 100, 100, 0, 0, 0),
    (200, 100, 100, 100, 0, 0, 0), (300, 112, 100, 100, 0, 0, 0), (400, 124, 100, 100, 0, 0, 0),
    (700, 124, 100, 100, 0, 0, 0), (800, 112, 100, 100, 0, 0, 0), (900, 100, 100, 100, 0, 0, 0),
    (1000, 112, 100, 100, 0, 0, 0), (1100, 137, 100, 100, 0, 0, 0), (0, 237, 100, 100, 0, 0, 0),
    (100, 212, 100, 100, 0, 0, 0), (200, 200, 100, 100, 0, 0, 0), (300, 212, 100, 100, 0, 0, 0),
    (400, 224, 100, 100, 0, 0, 0), (700, 224, 100, 100, 0, 0, 0), (800, 212, 100, 100, 0, 0, 0),
    (900, 200, 100, 100, 0, 0, 0), (1000, 212, 100, 100, 0, 0, 0), (1100, 237, 100, 100, 0, 0, 0),
    (250, 312, 100, 100, 0, 0, 0), (350, 312, 100, 100, 1200, 350, 412),
    (448, 283, 100, 150, 2400, 448, 433), (652, 283, 100, 150, -2400, 752, 433),
    (750, 312, 100, 100, -1200, 850, 412), (850, 312, 100, 100, 0, 0, 0),
];

/// Corne 3x5+3 with the Typeractive default keymap.
pub fn corne36() -> Layout {
    let layers: Vec<Vec<crate::zmk::Binding>> =
        CORNE_5COL_KEYMAP.iter().map(|layer| crate::zmk::parse_bindings(layer)).collect();
    split_from_zmk(LayoutId::Corne36, &layers, &CORNE_5COL_PHYSICAL, vec!["Base", "Lower", "Raise"], true)
}

/// The Corne with six columns, the default keymap of ZMK (`corne.keymap`).
pub fn corne42() -> Layout {
    let layers: Vec<Vec<crate::zmk::Binding>> =
        CORNE_6COL_KEYMAP.iter().map(|layer| crate::zmk::parse_bindings_with(layer, true)).collect();
    split_from_zmk(LayoutId::Corne42, &layers, &CORNE_6COL_PHYSICAL, vec!["Base", "Lower", "Raise"], false)
}

/// The Lily58, the default keymap of ZMK (`lily58.keymap`).
pub fn lily58() -> Layout {
    let layers: Vec<Vec<crate::zmk::Binding>> =
        LILY58_KEYMAP.iter().map(|layer| crate::zmk::parse_bindings_with(layer, true)).collect();
    split_from_zmk(LayoutId::Lily58, &layers, &LILY58_PHYSICAL, vec!["Base", "Lower", "Raise"], false)
}

static CUSTOM_KEYMAP: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

/// The `.keymap` file of ZMK of the custom layout, see [`LayoutId::Custom`].
pub fn set_custom_keymap(text: Option<String>) {
    if let Ok(mut keymap) = CUSTOM_KEYMAP.write() {
        *keymap = text;
    }
}

/// A keymap of ZMK imported by the user: its physical layout is the one with as many keys,
/// a Corne with 5 or 6 columns or a Lily58.
pub fn custom_from_zmk(text: &str) -> Result<Layout, String> {
    let parsed = crate::zmk::parse_keymap(text);
    let Some(first) = parsed.first() else {
        return Err("no layer with bindings in the keymap".into());
    };
    let n = first.1.len();
    let physical: &[(i32, i32, i32, i32, i32, i32, i32)] = match n {
        36 => &CORNE_5COL_PHYSICAL,
        42 => &CORNE_6COL_PHYSICAL,
        58 => &LILY58_PHYSICAL,
        n => return Err(format!("{n} keys: only the Corne (36 or 42 keys) and the Lily58 (58) are known")),
    };
    let auto_shift = text.contains("AS(") || text.contains("auto-shift") || text.contains("&as ");
    let names: Vec<&'static str> = parsed
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            let name = if i == 0 { "Base".to_string() } else { name.replace("_layer", "").replace(" Layer", "") };
            &*Box::leak(capitalize(&name).into_boxed_str())
        })
        .collect();
    let layers: Vec<Vec<crate::zmk::Binding>> = parsed.into_iter().map(|(_, b)| b).collect();
    Ok(split_from_zmk(LayoutId::Custom, &layers, physical, names, auto_shift))
}

/// A column staggered split keyboard of ZMK, its keys in the order of the keymap with their
/// physical attributes in hundredths of a key (x, y, w, h, rotation in hundredths of a degree,
/// rotation point). The fingers and the rows come from the places: the rotated keys and the
/// keys under the outer column are thumbs, the two inner columns are the index fingers, then
/// the middle, the ring and the pinky for the outer columns.
pub fn split_from_zmk(
    id: LayoutId,
    layers: &[Vec<crate::zmk::Binding>],
    physical: &[(i32, i32, i32, i32, i32, i32, i32)],
    layer_names: Vec<&'static str>,
    auto_shift: bool,
) -> Layout {
    let min_x = physical.iter().map(|p| p.0).min().unwrap_or(0);
    let max_x = physical.iter().map(|p| p.0 + p.2).max().unwrap_or(0);
    let middle = (min_x + max_x) / 2;
    // The lowest key of the outer column, the thumbs are below.
    let outer_bottom = physical.iter().filter(|p| p.0 == min_x).map(|p| p.1).max().unwrap_or(0);
    let is_thumb = |p: &(i32, i32, i32, i32, i32, i32, i32)| p.4 != 0 || p.1 > outer_bottom + 50;
    let left_of = |p: &(i32, i32, i32, i32, i32, i32, i32)| p.0 + p.2 / 2 < middle;
    // The columns of each half, from the outer one: a lone key is not a column, as the inner
    // keys of the Lily58.
    let in_column = |x: i32| physical.iter().filter(|q| q.0 == x && !is_thumb(q)).count() >= 2;
    let columns = |left: bool| {
        let mut xs: Vec<i32> =
            physical.iter().filter(|p| !is_thumb(p) && left_of(p) == left && in_column(p.0)).map(|p| p.0).collect();
        xs.sort_unstable();
        xs.dedup();
        if !left {
            xs.reverse();
        }
        xs
    };
    let (left_columns, right_columns) = (columns(true), columns(false));
    let keys: Vec<KeySpec> = physical
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let &(x, y, w, h, rot, rx, ry) = p;
            let left = left_of(p);
            let (row, finger) = if is_thumb(p) {
                (Row::Thumb, 5)
            } else {
                let cols = if left { &left_columns } else { &right_columns };
                // A lone key beside the columns: inside, the index finger, outside, the pinky.
                let inner = cols.last().copied().unwrap_or(x);
                let column = cols.iter().position(|c| *c == x).unwrap_or_else(|| {
                    let inside = if left { x > inner } else { x < inner };
                    if inside { cols.len().saturating_sub(1) } else { 0 }
                });
                let from_inner = cols.len().saturating_sub(1 + column);
                let finger = match from_inner {
                    0 | 1 => 4,
                    2 => 3,
                    3 => 2,
                    _ => 1,
                };
                // The rows of the column, from the top.
                let mut ys: Vec<i32> = physical.iter().filter(|q| q.0 == x && !is_thumb(q)).map(|q| q.1).collect();
                ys.sort_unstable();
                let index = ys.iter().position(|v| *v == y).unwrap_or(0);
                let names: &[Row] = match ys.len() {
                    // A lone key, beside the rows.
                    1 => &[Row::Other],
                    2 => &[Row::Home, Row::Bottom],
                    3 => &[Row::Top, Row::Home, Row::Bottom],
                    _ => &[Row::Number, Row::Top, Row::Home, Row::Bottom],
                };
                (names[index.min(names.len() - 1)], if left { finger } else { 10 - finger })
            };
            let units = |v: i32| v as f32 / 100.0;
            let pos = KeyPos {
                x: units(x - min_x),
                y: units(y),
                w: units(w),
                h: units(h),
                rot: rot as f32 / 100.0,
                rx: if rot == 0 { 0.0 } else { units(rx - min_x) },
                ry: units(ry),
            };
            crate::zmk::key_spec(layers, i, finger, row, left, pos)
        })
        .collect();
    let left: Vec<usize> = (0..keys.len()).filter(|k| keys[*k].left).collect();
    let right: Vec<usize> = (0..keys.len()).filter(|k| !keys[*k].left).collect();
    Layout {
        id,
        keys,
        blocks: vec![Block { keys: left }, Block { keys: right }],
        layer_names,
        unit: if physical.len() > 42 { 44.0 } else { 52.0 },
        split: true,
        auto_shift,
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
            for s in [Escape, Tab, Left, Right, Up, Down, Backspace, Enter, Space] {
                // A laptop without arrows: the shortcuts skip them.
                let arrow = matches!(s, Left | Right | Up | Down);
                if arrow && id == LayoutId::QwertyLaptop {
                    assert!(layout.locate_special(s).is_none());
                    continue;
                }
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

    /// The Typeractive default keymap.
    #[test]
    fn corne_layers() {
        let layout = corne36();
        assert_eq!(layout.keys.len(), 36);
        let one = layout.locate('1').unwrap();
        assert_eq!((one.layer, one.shift), (2, false));
        // The shifted digits are on Lower, without shift.
        let bang = layout.locate('!').unwrap();
        assert_eq!((bang.layer, bang.shift), (1, false));
        let a = layout.locate('A').unwrap();
        assert_eq!((a.layer, a.shift), (0, true));
        assert_eq!(layout.locate(',').unwrap().layer, 2);
        assert_eq!(layout.locate('"').map(|l| (l.layer, l.shift)), Some((0, true)));
        let right = layout.locate_special(Special::Right).unwrap();
        assert_eq!(right.layer, 1);
        // Esc and Tab are thumb keys of the layers.
        let esc = layout.locate_special(Special::Escape).unwrap();
        assert_eq!((esc.layer, layout.keys[esc.key].row), (1, Row::Thumb));
        // No shift, no Home: auto shift and the arrows do it.
        assert!(layout.find_special(Special::Shift).next().is_none());
        assert!(layout.locate_special(Special::Home).is_none());
        // The GUI key is the outer left thumb, Ctrl and Alt are on the right bottom row.
        let gui = layout.find_special(Special::Super).next().unwrap();
        assert_eq!((layout.keys[gui].row, layout.keys[gui].left), (Row::Thumb, true));
        assert_eq!(layout.keys[gui].legend(0, false, &layout.layer_names), "Win");
        let ctrl = layout.find_special(Special::Ctrl).next().unwrap();
        assert_eq!((layout.keys[ctrl].row, layout.keys[ctrl].finger), (Row::Bottom, 7));
        // The inner thumbs are rotated and 1.5 keys tall.
        let enter = layout.find_special(Special::Enter).next().unwrap();
        assert_eq!((layout.keys[enter].pos.h, layout.keys[enter].pos.rot), (1.5, -24.0));
        // The layer keys show what their layer types.
        let h = layout.locate('h').unwrap().key;
        assert_eq!(layout.keys[h].legend(1, false, &layout.layer_names), "←");
        // &trans types the base layer.
        let x = layout.locate('x').unwrap().key;
        assert_eq!(layout.keys[x].legend(1, false, &layout.layer_names), "x");
        let z = layout.locate('z').unwrap().key;
        assert_eq!(layout.keys[z].legend(1, false, &layout.layer_names), "BT×");
    }

    #[test]
    fn plans_on_the_layer_shown() {
        let layout = corne36();
        // Raise has its own Space, the base f is not on Raise.
        let space = layout.plan_on(Target::Char(' '), 2).unwrap();
        assert_eq!((space.layer, layout.keys[space.key].left), (2, true));
        assert!(layout.plan_on(Target::Char('f'), 2).is_none());
        // &trans keys type the base character on the layer.
        assert_eq!(layout.plan_on(Target::Char('x'), 1).map(|p| p.layer), Some(1));
        // Auto shift on a layer: ? is the long press of /.
        let question = layout.plan_on(Target::Char('?'), 2).unwrap();
        assert!(question.auto_shift);
    }

    #[test]
    fn plans_use_the_other_hand() {
        let layout = corne36();
        // Ctrl+C: Ctrl is on the right bottom row.
        let plan = layout.plan(Target::Chord(Chord::new(Mods::CTRL, KeyName::Char('c')))).unwrap();
        assert_eq!(
            layout.describe(&plan),
            "Hold Ctrl (right middle finger), then press c (left middle finger, bottom row)"
        );
        let plan = layout.plan(Target::Char('!')).unwrap();
        assert_eq!(
            layout.describe(&plan),
            "Hold Lower (left thumb), then press ! where Q is (left pinky, top row)"
        );
        let plan = layout.plan(Target::Char('?')).unwrap();
        assert_eq!(
            layout.describe(&plan),
            "Hold Raise (right thumb), then hold ? a moment longer where L is, auto shift (right ring finger, home row)"
        );
        let plan = layout.plan(Target::Char('C')).unwrap();
        assert!(plan.auto_shift && plan.shift_key.is_none());
        assert_eq!(layout.describe(&plan), "Hold C a moment longer, auto shift (left middle finger, bottom row)");
        // The chords with shift can not be pressed without a shift key.
        let select = Target::Chord(Chord::new(Mods::CTRL_SHIFT, KeyName::Special(Special::Right)));
        assert!(!layout.can_press(select));
        assert!(layout.can_press(Target::Chord(Chord::new(Mods::ALT, KeyName::Special(Special::Tab)))));
        let qwerty = qwerty(&KbdGrid::qwerty_us());
        assert!(qwerty.can_press(select));
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

#[cfg(test)]
mod klavaro_grid_tests {
    use super::*;
    use crate::klavaro::basic::{draw_lesson, lesson_char_set};

    #[test]
    fn corne_lessons_use_the_base_layer_first() {
        let grid = corne36().klavaro_grid();
        let set = |lesson| lesson_char_set(lesson, &grid).into_iter().collect::<String>();
        // The home row pinky of the Corne types `'`, `;` is on Raise.
        assert_eq!(set(4), "a'");
        assert_eq!(set(10), "asdfghjkl'");
        // Ctrl and Alt sit where `,` and `.` are on QWERTY, `.` where `/` is: the home keys of
        // the fingers complete the lessons.
        assert_eq!(set(22), "cdk");
        assert_eq!(set(23), "xsl");
        assert_eq!(set(30), "zxcvbnm.");
        // The number row is on the layers, later.
        assert_eq!(set(32), "1234");
        assert!(grid.is_inset(';') && grid.is_inset(','));
        for lesson in 1..=crate::klavaro::N_BASIC_LESSONS {
            let chars = lesson_char_set(lesson, &grid);
            assert!(!chars.is_empty(), "lesson {lesson}");
            assert!(!draw_lesson(&chars).is_empty());
        }
        // QWERTY keeps the lessons of Klavaro.
        assert_eq!(lesson_char_set(4, &LayoutId::QwertyUs.build().klavaro_grid()), vec!['a', ';']);
    }
}

#[cfg(test)]
mod zmk_layout_tests {
    use super::*;

    fn char_key(layout: &Layout, c: char) -> &KeySpec {
        let at = layout.locate(c).unwrap_or_else(|| panic!("{c}"));
        assert_eq!(at.layer, 0, "{c} on the base layer");
        &layout.keys[at.key]
    }

    #[test]
    fn fingers_and_rows_from_the_places() {
        for layout in [corne36(), corne42(), lily58()] {
            for (c, finger, row) in [
                ('a', 1, Row::Home),
                ('s', 2, Row::Home),
                ('d', 3, Row::Home),
                ('f', 4, Row::Home),
                ('g', 4, Row::Home),
                ('j', 6, Row::Home),
                ('l', 8, Row::Home),
                ('q', 1, Row::Top),
                ('b', 4, Row::Bottom),
                ('p', 9, Row::Top),
            ] {
                let key = char_key(&layout, c);
                assert_eq!((key.finger, key.row), (finger, row), "{c} on {:?}", layout.id);
            }
            assert!(layout.keys.iter().filter(|k| k.row == Row::Thumb).count() >= 6, "{:?}", layout.id);
        }
        // The number row of the Lily58, Shift for the capitals.
        let lily = lily58();
        assert_eq!(char_key(&lily, '1').row, Row::Number);
        assert!(!lily.auto_shift);
        assert!(lily.plan(Target::Char('A')).is_some_and(|p| p.shift));
        assert!(lily.plan(Target::Char('!')).is_some());
    }

    #[test]
    fn imported_keymap() {
        let text = include_str!("../tests/data/corne.keymap");
        let layout = custom_from_zmk(text).expect("the keymap");
        assert_eq!(layout.keys.len(), 42);
        assert_eq!(layout.layer_names, vec!["Base", "Lower", "Raise"]);
        assert!(layout.plan(Target::Char('{')).is_some_and(|p| p.layer == 2));
        assert!(custom_from_zmk("/ { keymap { }; };").is_err());
    }
}
