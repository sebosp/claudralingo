//! The exercises and the tutor logic of Klavaro, ported from its C sources (version 3.14).
//!
//! Klavaro is GPL-3.0-or-later, Copyright (C) 2005-2021 Felipe Emmanuel Ferreira de Castro,
//! <https://sourceforge.net/projects/klavaro/>. The data files under `data/klavaro` are copied
//! as is. The differences to Klavaro are documented on each function, the main one is that every
//! exercise allows backspace, Klavaro only allowed it on the fluidness exercises.

pub mod accuracy;
pub mod adaptability;
pub mod basic;
pub mod fluidness;
pub mod tutor;
pub mod velocity;

/// The lessons of the basic course, as masks over the 4x14 key grid of a `.kbd` file.
pub const BASIC_LESSONS: &str = include_str!("../../data/klavaro/basic_lessons.txt");
/// The keyboard Klavaro uses to turn the basic lessons masks into characters.
pub const QWERTY_US_KBD: &str = include_str!("../../data/klavaro/qwerty_us.kbd");
/// The word list of the velocity exercises, the "C" locale of Klavaro.
pub const WORDS: &str = include_str!("../../data/klavaro/C.words");
/// The paragraphs of the fluidness exercises, the "C" locale of Klavaro.
pub const PARAGRAPHS: &str = include_str!("../../data/klavaro/C.paragraphs");

/// The paragraph symbol Klavaro shows at the end of the lines, typed with Enter.
pub const UPSYM: char = '\u{b6}';
/// Klavaro sends a tab for Ctrl+Backspace, it removes all the errors in one go.
pub const HYPER_BACKSPACE: char = '\t';
pub const BACKSPACE: char = '\u{8}';

/// The number of lessons of the basic course in `basic_lessons.txt`.
pub const N_BASIC_LESSONS: usize = 43;

const VOWELS: [char; 5] = ['a', 'e', 'i', 'o', 'u'];

/// The 4x14 key grid of a Klavaro `.kbd` file: 4 lines of lower chars, 4 of upper chars and
/// 4 of finger numbers (1 is the left pinky, 5 the thumbs, 9 the right pinky).
#[derive(Debug, Clone)]
pub struct KbdGrid {
    pub lochars: [[char; 14]; 4],
    pub upchars: [[char; 14]; 4],
    pub fingers: [[u8; 14]; 4],
    /// The characters the keyboard types outside of the grid, on the layers of a Corne.
    pub extra: Vec<char>,
}

impl KbdGrid {
    pub fn parse(kbd: &str) -> Self {
        let mut grid = KbdGrid {
            lochars: [[' '; 14]; 4],
            upchars: [[' '; 14]; 4],
            fingers: [[0; 14]; 4],
            extra: Vec::new(),
        };
        for (i, line) in kbd.lines().take(12).enumerate() {
            for (j, c) in line.chars().take(14).enumerate() {
                match i {
                    0..4 => grid.lochars[i][j] = c,
                    4..8 => grid.upchars[i - 4][j] = c,
                    _ => grid.fingers[i - 8][j] = c.to_digit(10).unwrap_or(0) as u8,
                }
            }
        }
        grid
    }

    pub fn qwerty_us() -> Self {
        Self::parse(QWERTY_US_KBD)
    }

    fn chars(&self) -> impl Iterator<Item = (char, char)> + '_ {
        (0..4).flat_map(move |i| (0..14).map(move |j| (self.lochars[i][j], self.upchars[i][j])))
    }

    /// The lower and the upper characters of the grid.
    pub fn chars_typed(&self) -> Vec<char> {
        self.chars().flat_map(|(lo, up)| [lo, up]).filter(|c| !c.is_whitespace()).collect()
    }

    /// `keyb_is_inset`
    pub fn is_inset(&self, c: char) -> bool {
        self.chars().any(|(lo, up)| c == lo || c == up) || self.extra.contains(&c)
    }

    /// `keyb_get_vowels`
    pub fn vowels(&self) -> Vec<char> {
        let vowels: Vec<char> = self.chars().map(|(lo, _)| lo).filter(|c| is_vowel(*c)).take(20).collect();
        if vowels.is_empty() {
            return self.lochars[2][..5].to_vec();
        }
        vowels
    }

    /// `keyb_get_consonants`
    pub fn consonants(&self) -> Vec<char> {
        let mut consonants = Vec::new();
        for (lo, up) in self.chars() {
            if lo.is_alphabetic() && !is_vowel(lo) {
                consonants.push(lo);
            }
            let up = to_lower(up);
            if up.is_alphabetic() && !is_vowel(up) && up != lo {
                consonants.push(up);
            }
        }
        consonants
    }

    /// `keyb_get_symbols`
    pub fn symbols(&self) -> Vec<char> {
        let mut symbols = Vec::new();
        for (lo, up) in self.chars() {
            if lo.is_ascii_punctuation() {
                symbols.push(lo);
            }
            if up.is_ascii_punctuation() {
                symbols.push(up);
            }
        }
        symbols
    }
}

/// `keyb_is_vowel`
pub fn is_vowel(c: char) -> bool {
    VOWELS.contains(&to_lower(c))
}

pub fn to_lower(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

pub fn to_upper(c: char) -> char {
    c.to_uppercase().next().unwrap_or(c)
}

/// Klavaro uses `rand () % n`, this is the same with a guard for `n == 0`.
pub fn rand_below(n: usize) -> usize {
    if n == 0 { 0 } else { fastrand::usize(..n) }
}

/// `tutor_draw_paragraph`: the paragraphs end with the paragraph symbol and a new line.
pub fn draw_paragraph(text: &mut String, paragraph: &str) {
    text.push_str(paragraph.trim_end_matches('\n'));
    text.push(UPSYM);
    text.push('\n');
}
