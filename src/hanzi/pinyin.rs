//! The pinyin input of the hanzi words: letters typed on the keyboard, or the nine keys of a
//! phone (T9, 九宫格), 2 for abc up to 9 for wxyz. The digits are spelled with the valid
//! syllables of Mandarin, the candidates are the HSK words first and then the words of the
//! dictionary of the `inputx-pinyin` input method, the most frequent first. Only the words
//! whose characters the embedded font and strokes have are offered.
use std::collections::HashMap;

use inputx_pinyin::{PinyinEngine, VALID_SYLLABLES};

use super::data::{HanziData, toneless};

/// The letters of the keys 2 to 9.
pub const KEYS: [(char, &str); 8] = [
    ('2', "abc"),
    ('3', "def"),
    ('4', "ghi"),
    ('5', "jkl"),
    ('6', "mno"),
    ('7', "pqrs"),
    ('8', "tuv"),
    ('9', "wxyz"),
];
/// The candidates shown.
pub const MAX_CANDIDATES: usize = 8;
/// The spellings of a digit sequence explored, the long sequences have many.
const MAX_SPELLINGS: usize = 256;

pub fn digit_of(letter: char) -> Option<char> {
    KEYS.iter().find(|(_, letters)| letters.contains(letter)).map(|(d, _)| *d)
}

fn digits_of(word: &str) -> String {
    word.chars().filter_map(digit_of).collect()
}

/// A spelling of digits with syllables, the last one may be the start of a syllable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spelling {
    pub letters: String,
    pub complete: bool,
}

/// The pinyin spellings of the digits of the nine keys, the fewest syllables first.
pub fn t9_spellings(digits: &str) -> Vec<Spelling> {
    let syllables: Vec<(&str, String)> = VALID_SYLLABLES.iter().map(|s| (*s, digits_of(s))).collect();
    let mut out = Vec::new();
    let mut stack = vec![(0usize, String::new())];
    while let Some((at, letters)) = stack.pop() {
        if out.len() >= MAX_SPELLINGS {
            break;
        }
        let rest = &digits[at..];
        if rest.is_empty() {
            out.push(Spelling { letters, complete: true });
            continue;
        }
        let mut partials: Vec<String> = Vec::new();
        for (syllable, code) in &syllables {
            if rest.starts_with(code.as_str()) {
                stack.push((at + code.len(), format!("{letters}{syllable}")));
            } else if code.starts_with(rest) {
                // The start of the last syllable.
                let start: String = syllable.chars().take(rest.len()).collect();
                if !partials.contains(&start) {
                    partials.push(start);
                }
            }
        }
        for start in partials {
            out.push(Spelling { letters: format!("{letters}{start}"), complete: false });
        }
    }
    out.sort_by_key(|s| (!s.complete, s.letters.len()));
    out.dedup();
    out
}

/// A candidate word and its pinyin with the tones when it is an HSK word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub word: String,
    pub pinyin: String,
}

/// The input method: the dictionary and the HSK words by their toneless pinyin.
pub struct Ime {
    engine: PinyinEngine,
    hsk: HashMap<String, Vec<usize>>,
}

impl Ime {
    pub fn new(data: &HanziData) -> Self {
        let mut hsk: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, w) in data.words.iter().enumerate() {
            hsk.entry(toneless(&w.pinyin)).or_default().push(i);
        }
        Ime { engine: PinyinEngine::new(), hsk }
    }

    /// The candidates of the input: letters, or digits with `t9`.
    pub fn candidates(&self, data: &HanziData, input: &str, t9: bool) -> Vec<Candidate> {
        let spellings = if t9 {
            t9_spellings(input)
        } else {
            vec![Spelling { letters: input.to_ascii_lowercase(), complete: true }]
        };
        let mut out: Vec<Candidate> = Vec::new();
        let push = |word: &str, pinyin: String, out: &mut Vec<Candidate>| {
            if out.len() < MAX_CANDIDATES
                && word.chars().all(|c| data.knows(c))
                && !out.iter().any(|c| c.word == word)
            {
                out.push(Candidate { word: word.to_string(), pinyin });
            }
        };
        // The HSK words spelled exactly, then those starting with the spelling.
        for exact in [true, false] {
            for s in &spellings {
                let mut found: Vec<&usize> = if exact {
                    self.hsk.get(&s.letters).map(|v| v.iter().collect()).unwrap_or_default()
                } else {
                    self.hsk.iter().filter(|(k, _)| k.starts_with(&s.letters)).flat_map(|(_, v)| v).collect()
                };
                found.sort();
                for i in found {
                    let w = &data.words[*i];
                    push(&w.word, w.pinyin.clone(), &mut out);
                }
            }
        }
        // The other words of the dictionary.
        let dict = self.engine.dict();
        for s in spellings.iter().filter(|s| s.complete) {
            for word in dict.lookup(&s.letters) {
                push(&word, s.letters.clone(), &mut out);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_keys() {
        // 96 is `wo`, 94 `xi`, `yi` or `zh`, the start of `zhong`.
        let s = t9_spellings("96");
        assert!(s.contains(&Spelling { letters: "wo".into(), complete: true }));
        let s = t9_spellings("94");
        assert!(s.contains(&Spelling { letters: "xi".into(), complete: true }));
        assert!(s.iter().any(|s| s.letters == "zh" && !s.complete));
        // ni hao
        let s = t9_spellings("64426");
        assert!(s.iter().any(|s| s.letters == "nihao" && s.complete));
    }

    #[test]
    fn hsk_words_first() {
        let data = HanziData::load();
        let ime = Ime::new(&data);
        let c = ime.candidates(&data, "96", true);
        assert_eq!(c.first().map(|c| c.word.as_str()), Some("我"));
        let c = ime.candidates(&data, "zhongguo", false);
        assert_eq!(c.first().map(|c| c.word.as_str()), Some("中国"));
        assert!(c.len() <= MAX_CANDIDATES);
        let c = ime.candidates(&data, "nvhai", false);
        assert!(c.iter().any(|c| c.word == "女孩儿" || c.word == "女儿" || c.word == "女孩"));
    }
}
