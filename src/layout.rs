//! The keyboard layouts shown on the screen.
//!
//! The QWERTY layout is made from the grid of Klavaro's `qwerty_us.kbd`, the Corne is a split
//! 3x5+3 keyboard (36 keys) with a QWERTY base layer, the numbers and symbols live on the Lower
//! and Raise layers held with the thumbs.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Special {
    Backspace,
    Tab,
    Caps,
    Enter,
    Shift,
    Space,
    /// Holds a layer while pressed.
    Layer(u8),
}

impl Special {
    fn legend(self, layer_names: &[&str]) -> String {
        match self {
            Special::Backspace => "⌫".into(),
            Special::Tab => "Tab".into(),
            Special::Caps => "Caps".into(),
            Special::Enter => "⏎".into(),
            Special::Shift => "⇧".into(),
            Special::Space => "Spc".into(),
            Special::Layer(l) => layer_names.get(l as usize).copied().unwrap_or("?").into(),
        }
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

/// What a key types on a layer: the char without and with shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Output {
    pub lo: char,
    pub up: char,
}

#[derive(Debug, Clone)]
pub struct KeySpec {
    /// One output per layer of the layout.
    pub outputs: Vec<Option<Output>>,
    pub special: Option<Special>,
    /// Klavaro's finger numbers, 1 is the left pinky, 5 the thumbs, 9 the right pinky.
    pub finger: u8,
    /// In key units.
    pub width: f32,
    /// Vertical offset in key units, for the column stagger of the split keyboards.
    pub y_offset: f32,
}

impl KeySpec {
    pub fn legend(&self, layer: u8, shift: bool, layer_names: &[&str]) -> String {
        if self.special == Some(Special::Space) && self.width >= 2.0 {
            return "Space".into();
        }
        if let Some(special) = self.special {
            return special.legend(layer_names);
        }
        match self.outputs.get(layer as usize).copied().flatten() {
            Some(out) if shift => out.up.to_string(),
            Some(out) => out.lo.to_string(),
            None => String::new(),
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

    pub fn find_special(&self, special: Special) -> impl Iterator<Item = usize> + '_ {
        self.keys
            .iter()
            .enumerate()
            .filter(move |(_, k)| k.special == Some(special))
            .map(|(i, _)| i)
    }

    /// The shift key of the other hand, Klavaro's hints use the same rule.
    pub fn shift_for(&self, key: usize) -> Option<usize> {
        let left_hand = self.keys[key].finger < 5;
        let shifts: Vec<usize> = self.find_special(Special::Shift).collect();
        shifts
            .iter()
            .copied()
            .find(|s| (self.keys[*s].finger < 5) != left_hand)
            .or(shifts.first().copied())
    }

    /// The layer key of the other hand.
    pub fn layer_key_for(&self, key: usize, layer: u8) -> Option<usize> {
        let left_hand = self.keys[key].finger < 5 || self.is_left(key);
        let keys: Vec<usize> = self.find_special(Special::Layer(layer)).collect();
        keys.iter()
            .copied()
            .find(|k| self.is_left(*k) != left_hand)
            .or(keys.first().copied())
    }

    fn is_left(&self, key: usize) -> bool {
        self.blocks
            .first()
            .is_some_and(|b| self.blocks.len() > 1 && b.rows.iter().any(|(_, r)| r.contains(&key)))
    }
}

fn char_key(lo: char, up: char, finger: u8, width: f32) -> KeySpec {
    KeySpec {
        outputs: vec![Some(Output { lo, up })],
        special: None,
        finger,
        width,
        y_offset: 0.0,
    }
}

fn special_key(special: Special, finger: u8, width: f32) -> KeySpec {
    KeySpec {
        outputs: vec![],
        special: Some(special),
        finger,
        width,
        y_offset: 0.0,
    }
}

/// A row staggered keyboard from the 4x14 grid of a Klavaro `.kbd` file.
pub fn qwerty(grid: &KbdGrid) -> Layout {
    let mut keys = Vec::new();
    let mut rows = Vec::new();
    let push = |keys: &mut Vec<KeySpec>, key: KeySpec| {
        keys.push(key);
        keys.len() - 1
    };
    let grid_key = |i: usize, j: usize, width: f32| {
        char_key(grid.lochars[i][j], grid.upchars[i][j], grid.fingers[i][j], width)
    };
    // Number row
    let mut row: Vec<usize> = (0..13).map(|j| push(&mut keys, grid_key(0, j, 1.0))).collect();
    row.push(push(&mut keys, special_key(Special::Backspace, 9, 2.0)));
    rows.push((0.0, row));
    // Top row
    let mut row = vec![push(&mut keys, special_key(Special::Tab, 1, 1.5))];
    row.extend((0..13).map(|j| push(&mut keys, grid_key(1, j, if j == 12 { 1.5 } else { 1.0 }))));
    rows.push((0.0, row));
    // Home row
    let mut row = vec![push(&mut keys, special_key(Special::Caps, 1, 1.75))];
    row.extend((0..11).map(|j| push(&mut keys, grid_key(2, j, 1.0))));
    row.push(push(&mut keys, special_key(Special::Enter, 9, 2.25)));
    rows.push((0.0, row));
    // Bottom row, the first column is the extra key of the ISO keyboards.
    let mut row = vec![push(&mut keys, special_key(Special::Shift, 1, 2.25))];
    row.extend((1..11).map(|j| push(&mut keys, grid_key(3, j, 1.0))));
    row.push(push(&mut keys, special_key(Special::Shift, 9, 2.75)));
    rows.push((0.0, row));
    rows.push((3.75, vec![push(&mut keys, special_key(Special::Space, 5, 6.25))]));
    Layout {
        id: LayoutId::QwertyUs,
        keys,
        blocks: vec![Block { rows, tilt: 0.0 }],
        layer_names: vec!["Base"],
        unit: 46.0,
    }
}

/// Corne 3x5+3: QWERTY on the base layer, numbers and shifted numbers on Lower, the rest of the
/// symbols on Raise. Thumbs: Shift, Lower and Space on the left, Enter, Raise and Backspace on
/// the right.
pub fn corne36() -> Layout {
    const BASE: [&str; 6] = ["qwert", "asdfg", "zxcvb", "yuiop", "hjkl;", "nm,./"];
    const BASE_UP: [&str; 6] = ["QWERT", "ASDFG", "ZXCVB", "YUIOP", "HJKL:", "NM<>?"];
    const LOWER: [&str; 6] = ["12345", "!@#$%", "`~'\"\\", "67890", "^&*()", "|-_=+"];
    const RAISE: [&str; 6] = ["     ", "     ", "     ", "     ", "[]{} ", "     "];
    // Column stagger of the Corne, from the outer to the inner column of a half.
    const STAGGER: [f32; 5] = [0.35, 0.12, 0.0, 0.12, 0.22];
    const LEFT_FINGERS: [u8; 5] = [1, 2, 3, 4, 4];

    let mut keys = Vec::new();
    let mut blocks = Vec::new();
    for half in 0..2 {
        let mut rows = Vec::new();
        for r in 0..3 {
            let idx = half * 3 + r;
            let (lo, up) = (BASE[idx].chars().collect::<Vec<_>>(), BASE_UP[idx].chars().collect::<Vec<_>>());
            let (lower, raise) = (LOWER[idx].chars().collect::<Vec<_>>(), RAISE[idx].chars().collect::<Vec<_>>());
            let mut row = Vec::new();
            for c in 0..5 {
                // The stagger and the fingers mirror on the right half.
                let outer = if half == 0 { c } else { 4 - c };
                let layer_output = |ch: char| (ch != ' ').then_some(Output { lo: ch, up: ch });
                keys.push(KeySpec {
                    outputs: vec![
                        Some(Output { lo: lo[c], up: up[c] }),
                        layer_output(lower[c]),
                        layer_output(raise[c]),
                    ],
                    special: None,
                    finger: if half == 0 { LEFT_FINGERS[outer] } else { 10 - LEFT_FINGERS[outer] },
                    width: 1.0,
                    y_offset: STAGGER[outer],
                });
                row.push(keys.len() - 1);
            }
            rows.push((0.0, row));
        }
        let thumbs = if half == 0 {
            [Special::Shift, Special::Layer(1), Special::Space]
        } else {
            [Special::Enter, Special::Layer(2), Special::Backspace]
        };
        let mut row = Vec::new();
        for (t, special) in thumbs.into_iter().enumerate() {
            let mut key = special_key(special, 5, 1.0);
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
    }
}
