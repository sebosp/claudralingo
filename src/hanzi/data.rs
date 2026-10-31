//! The embedded data of the hanzi mode, see `assets/hanzi/LICENSES.md`: the HSK words, the
//! strokes of their characters, their recordings and a font for their characters.
use std::collections::HashMap;
use std::ops::Range;

use bevy::math::Vec2;
use bevy::prelude::Resource;
use serde::Deserialize;

use super::strokes::{KINDS, Kind, kind_of};

const WORDS: &str = include_str!("../../assets/hanzi/words.json");
const STROKES: &str = include_str!("../../assets/hanzi/strokes.json");
const AUDIO_INDEX: &str = include_str!("../../assets/hanzi/audio.json");
pub const AUDIO: &[u8] = include_bytes!("../../assets/hanzi/audio.bin");
/// Noto Sans CJK SC, only the characters of the words.
pub const FONT: &[u8] = include_bytes!("../../assets/hanzi/NotoSansSC-HSK.otf");

/// The words of a level of Claudralingo, an HSK level is cut in levels of this many words.
pub const WORDS_PER_LEVEL: usize = 10;
/// The baseline of Make Me a Hanzi: the y of its 1024 box grows upwards from 900 below the top.
pub const BASELINE: f32 = 900.0;
pub const BOX: f32 = 1024.0;

#[derive(Debug, Clone, Deserialize)]
pub struct Word {
    #[serde(rename = "w")]
    pub word: String,
    /// With the tone marks, `nǐ hǎo`.
    #[serde(rename = "p")]
    pub pinyin: String,
    #[serde(rename = "m")]
    pub meaning: String,
    /// HSK 2.0, 1 to 4.
    #[serde(rename = "l")]
    pub hsk: u8,
}

#[derive(Deserialize)]
struct RawGlyph {
    s: Vec<String>,
    m: Vec<Vec<[f32; 2]>>,
    /// A letter per stroke, its kind.
    #[serde(default)]
    k: String,
}

/// The strokes of a character in their order: the outlines to draw, the medians to compare,
/// their kinds.
#[derive(Debug, Clone)]
pub struct Glyph {
    pub outlines: Vec<Vec<PathOp>>,
    pub medians: Vec<Vec<Vec2>>,
    pub kinds: Vec<&'static Kind>,
}

/// An absolute command of the SVG paths of Make Me a Hanzi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathOp {
    Move(Vec2),
    Line(Vec2),
    Quad(Vec2, Vec2),
    Cubic(Vec2, Vec2, Vec2),
    Close,
}

/// A level of Claudralingo: a part of an HSK level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Level {
    pub hsk: u8,
    /// 1 based, of `parts`.
    pub part: usize,
    pub parts: usize,
    /// The words, indices in [`HanziData::words`].
    pub words: Range<usize>,
}

#[derive(Resource)]
pub struct HanziData {
    pub words: Vec<Word>,
    pub levels: Vec<Level>,
    glyphs: HashMap<char, Glyph>,
    audio: HashMap<String, Vec<(usize, usize)>>,
}

impl HanziData {
    pub fn load() -> Self {
        let words: Vec<Word> = serde_json::from_str(WORDS).expect("assets/hanzi/words.json");
        let raw: HashMap<String, RawGlyph> = serde_json::from_str(STROKES).expect("assets/hanzi/strokes.json");
        let glyphs = raw
            .into_iter()
            .filter_map(|(c, g)| {
                let c = c.chars().next()?;
                let outlines = g.s.iter().map(|p| parse_path(p)).collect();
                let medians = g.m.iter().map(|m| m.iter().map(|p| Vec2::new(p[0], p[1])).collect()).collect();
                let kinds = g.k.chars().map(|k| kind_of(k).unwrap_or(&KINDS[0])).collect();
                Some((c, Glyph { outlines, medians, kinds }))
            })
            .collect();
        let audio: HashMap<String, Vec<[usize; 2]>> = serde_json::from_str(AUDIO_INDEX).expect("assets/hanzi/audio.json");
        let audio = audio.into_iter().map(|(w, spans)| (w, spans.into_iter().map(|[o, l]| (o, l)).collect())).collect();
        let levels = levels(&words);
        HanziData { words, levels, glyphs, audio }
    }

    pub fn glyph(&self, c: char) -> Option<&Glyph> {
        self.glyphs.get(&c)
    }

    /// Whether the font and the strokes have the character.
    pub fn knows(&self, c: char) -> bool {
        self.glyphs.contains_key(&c)
    }

    /// The recordings of a word: one, or one per syllable.
    pub fn audio(&self, word: &str) -> Vec<&'static [u8]> {
        self.audio
            .get(word)
            .map(|spans| spans.iter().filter_map(|(o, l)| AUDIO.get(*o..o + l)).collect())
            .unwrap_or_default()
    }

    /// The level of a word.
    pub fn level_of(&self, word: usize) -> usize {
        self.levels.iter().position(|l| l.words.contains(&word)).unwrap_or(0)
    }

    /// The index of a word, its first one.
    pub fn index_of(&self, word: &str) -> Option<usize> {
        self.words.iter().position(|w| w.word == word)
    }

    /// The levels of an HSK level, their indices.
    pub fn levels_of_hsk(&self, hsk: u8) -> std::ops::Range<usize> {
        let first = self.levels.iter().position(|l| l.hsk == hsk).unwrap_or(0);
        let n = self.levels.iter().filter(|l| l.hsk == hsk).count();
        first..first + n
    }
}

/// The HSK levels cut in parts of [`WORDS_PER_LEVEL`] words, the last part of a level takes the
/// remaining words.
fn levels(words: &[Word]) -> Vec<Level> {
    let mut levels = Vec::new();
    let mut start = 0;
    while start < words.len() {
        let hsk = words[start].hsk;
        let end = words[start..].iter().position(|w| w.hsk != hsk).map_or(words.len(), |n| start + n);
        let count = end - start;
        let parts = count.div_ceil(WORDS_PER_LEVEL).max(1);
        for part in 0..parts {
            let from = start + part * WORDS_PER_LEVEL;
            let to = if part + 1 == parts { end } else { from + WORDS_PER_LEVEL };
            levels.push(Level { hsk, part: part + 1, parts, words: from..to });
        }
        start = end;
    }
    levels
}

/// The SVG paths of Make Me a Hanzi: absolute M, L, Q, C and Z, separated by spaces.
pub fn parse_path(path: &str) -> Vec<PathOp> {
    let mut ops = Vec::new();
    let mut tokens = path.split_whitespace().peekable();
    let numbers = |tokens: &mut std::iter::Peekable<std::str::SplitWhitespace>, n: usize| -> Option<Vec<Vec2>> {
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            let x: f32 = tokens.next()?.parse().ok()?;
            let y: f32 = tokens.next()?.parse().ok()?;
            v.push(Vec2::new(x, y));
        }
        Some(v)
    };
    while let Some(token) = tokens.next() {
        let op = match token {
            "M" => numbers(&mut tokens, 1).map(|p| PathOp::Move(p[0])),
            "L" => numbers(&mut tokens, 1).map(|p| PathOp::Line(p[0])),
            "Q" => numbers(&mut tokens, 2).map(|p| PathOp::Quad(p[0], p[1])),
            "C" => numbers(&mut tokens, 3).map(|p| PathOp::Cubic(p[0], p[1], p[2])),
            "Z" | "z" => Some(PathOp::Close),
            _ => None,
        };
        if let Some(op) = op {
            ops.push(op);
        }
    }
    ops
}

/// The pinyin without the tones and the spaces, as the input methods take it: `nǚ hái` is
/// `nvhai`.
pub fn toneless(pinyin: &str) -> String {
    pinyin
        .chars()
        .filter_map(|c| {
            let base = match c {
                'ā' | 'á' | 'ǎ' | 'à' | 'a' | 'A' | 'Ā' | 'Á' | 'Ǎ' | 'À' => 'a',
                'ē' | 'é' | 'ě' | 'è' | 'e' | 'E' | 'Ē' | 'É' | 'Ě' | 'È' => 'e',
                'ī' | 'í' | 'ǐ' | 'ì' | 'i' | 'I' | 'Ī' | 'Í' | 'Ǐ' | 'Ì' => 'i',
                'ō' | 'ó' | 'ǒ' | 'ò' | 'o' | 'O' | 'Ō' | 'Ó' | 'Ǒ' | 'Ò' => 'o',
                'ū' | 'ú' | 'ǔ' | 'ù' | 'u' | 'U' | 'Ū' | 'Ú' | 'Ǔ' | 'Ù' => 'u',
                'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' | 'ü' | 'Ü' | 'v' | 'V' => 'v',
                c if c.is_ascii_alphabetic() => c.to_ascii_lowercase(),
                _ => return None,
            };
            Some(base)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_data_loads() {
        let data = HanziData::load();
        assert_eq!(data.words.iter().filter(|w| w.hsk == 1).count(), 150);
        assert_eq!(data.words[data.levels[0].words.start].hsk, 1);
        // HSK 1 in 15 levels of 10 words, every word in a level.
        assert_eq!(data.levels.iter().filter(|l| l.hsk == 1).count(), 15);
        assert_eq!(data.levels.iter().map(|l| l.words.len()).sum::<usize>(), data.words.len());
        assert!(data.words.iter().all(|w| w.word.chars().all(|c| data.knows(c))));
        let wo = data.glyph('我').expect("我");
        assert_eq!(wo.outlines.len(), 7);
        assert_eq!(wo.medians.len(), 7);
        assert!(matches!(wo.outlines[0][0], PathOp::Move(_)));
        assert!(!data.audio("我").is_empty());
    }

    #[test]
    fn toneless_pinyin() {
        assert_eq!(toneless("nǚ hái"), "nvhai");
        assert_eq!(toneless("Zhōngguó"), "zhongguo");
        assert_eq!(toneless("xǐhuan"), "xihuan");
    }
}
