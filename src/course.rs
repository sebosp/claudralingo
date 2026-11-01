//! The exercises around the basic course: the layer course, the lines of real words made of
//! the keys learned, and the warm-up on the keys that need it most.
use crate::klavaro::{KbdGrid, N_BASIC_LESSONS, UPSYM, basic::lesson_char_set, rand_below};
use crate::layout::{Layout, Row};
use crate::progress::LayoutProgress;

/// A lesson of the layer course: its name and its characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerLesson {
    pub name: String,
    pub chars: Vec<char>,
}

/// The lessons of the layer course: each layer row by row, then each layer whole, then all of
/// them. A keyboard without layers has its symbols instead, the shifted ones apart.
pub fn layer_lessons(layout: &Layout) -> Vec<LayerLesson> {
    const ROWS: [(Row, &str); 4] =
        [(Row::Home, "home row"), (Row::Top, "top row"), (Row::Bottom, "bottom row"), (Row::Number, "number row")];
    let mut lessons = Vec::new();
    let mut all = Vec::new();
    let push = |lessons: &mut Vec<LayerLesson>, name: String, chars: Vec<char>| {
        if chars.len() >= 2 {
            lessons.push(LayerLesson { name, chars });
        }
    };
    if layout.layer_names.len() > 1 {
        for layer in 1..layout.layer_names.len() {
            let name = layout.layer_names[layer];
            let mut whole = Vec::new();
            for (row, row_name) in ROWS {
                let chars = layer_chars(layout, layer, |k| k.row == row);
                whole.extend(chars.iter().copied());
                push(&mut lessons, format!("{name}: {row_name}"), chars);
            }
            whole.dedup();
            all.extend(whole.iter().copied());
            // Not again when the layer has one row of characters.
            if lessons.last().is_none_or(|l| l.chars != whole) {
                push(&mut lessons, format!("{name}: all its keys"), whole);
            }
        }
    } else {
        // The symbols of the base layer, then with Shift.
        for shifted in [false, true] {
            let chars: Vec<char> = layout
                .keys
                .iter()
                .filter_map(|k| k.outputs.first().copied().flatten())
                .map(|o| if shifted { o.up } else { o.lo })
                .filter(|c| !c.is_ascii_alphabetic() && !c.is_whitespace())
                .collect();
            all.extend(chars.iter().copied());
            push(&mut lessons, if shifted { "Symbols with Shift".into() } else { "Numbers and symbols".into() }, chars);
        }
    }
    let mut seen = Vec::new();
    all.retain(|c| {
        let new = !seen.contains(c);
        seen.push(*c);
        new
    });
    push(&mut lessons, "All the layers".into(), all);
    lessons
}

/// The characters a layer types, not the ones it takes from the base layer.
fn layer_chars(layout: &Layout, layer: usize, keep: impl Fn(&crate::layout::KeySpec) -> bool) -> Vec<char> {
    let mut chars: Vec<char> = layout
        .keys
        .iter()
        .filter(|k| keep(k) && !k.is_inherited(layer as u8))
        .filter_map(|k| k.outputs.get(layer).copied().flatten())
        .map(|o| o.lo)
        .filter(|c| !c.is_whitespace())
        .collect();
    chars.dedup();
    chars
}

/// The characters of the basic lessons up to `lesson`, the ones learned so far.
pub fn learned_chars(grid: &KbdGrid, lesson: usize) -> Vec<char> {
    let mut chars: Vec<char> = Vec::new();
    for l in 1..=lesson.min(N_BASIC_LESSONS) {
        for c in lesson_char_set(l, grid) {
            if !chars.contains(&c) {
                chars.push(c);
            }
        }
    }
    chars
}

/// The common English words of SCOWL, by level, the most common first, see
/// `scripts/prepare_words.py`.
const DICTIONARY: &str = include_str!("../data/scowl/english.txt");

/// The words of the dictionary typed with the characters learned only and with a key of the
/// lesson, with their weight: the common words and the words with more keys of the lesson come
/// up more often.
pub fn words_of(learned: &[char], lesson: &[char]) -> Vec<(&'static str, u32)> {
    let mut weight = 1;
    let mut words = Vec::new();
    for line in DICTIONARY.lines() {
        if let Some(level) = line.strip_prefix("= ") {
            weight = match level.trim() {
                "10" => 4,
                "20" => 2,
                _ => 1,
            };
        } else if !line.starts_with('#') && line.chars().count() >= 2 && line.chars().all(|c| learned.contains(&c)) {
            let focus = line.chars().filter(|c| lesson.contains(c)).count() as u32;
            if focus > 0 {
                words.push((line, weight * (1 + focus)));
            }
        }
    }
    words
}

/// A few lines of real words made of the characters learned with the keys of the lesson, when
/// there are enough of them,
/// in the format of the basic lessons. A word does not come back before the ones drawn after
/// it, unless the keys make only a few words.
pub fn word_lines(learned: &[char], lesson: &[char], lines: usize) -> Option<String> {
    const MIN_WORDS: usize = 12;
    const LINE_LENGTH: usize = 50;
    let words = words_of(learned, lesson);
    if words.len() < MIN_WORDS {
        return None;
    }
    let total: u32 = words.iter().map(|(_, w)| w).sum();
    let memory = (words.len() / 2).min(40);
    let mut recent: Vec<usize> = Vec::new();
    let mut text = String::new();
    for _ in 0..lines {
        let mut line = String::new();
        while line.chars().count() < LINE_LENGTH {
            let mut pick = 0;
            for _ in 0..20 {
                pick = weighted(&words, rand_below(total as usize) as u32);
                if !recent.contains(&pick) {
                    break;
                }
            }
            recent.push(pick);
            if recent.len() > memory {
                recent.remove(0);
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(words[pick].0);
        }
        text.push_str(&line);
        text.push(UPSYM);
        text.push('\n');
    }
    Some(text)
}

/// The word at `r` of the weights laid end to end.
fn weighted(words: &[(&str, u32)], mut r: u32) -> usize {
    for (i, (_, w)) in words.iter().enumerate() {
        if r < *w {
            return i;
        }
        r -= w;
    }
    words.len() - 1
}

/// The keys to warm up on: the misses weigh more on the keys not practiced for a while, a
/// spaced repetition of the weak keys. The most needed first.
pub fn weak_keys(progress: &LayoutProgress, now: u64, n: usize) -> Vec<char> {
    const MIN_TOUCHES: u64 = 10;
    let mut keys: Vec<(char, f64)> = progress
        .keys
        .iter()
        .filter_map(|(k, s)| {
            let c = k.chars().next()?;
            let touches = s.hits + s.misses;
            if touches < MIN_TOUCHES || c == ' ' {
                return None;
            }
            let miss_rate = s.misses as f64 / touches as f64;
            let days = now.saturating_sub(s.last) as f64 / 86_400.0;
            // Forgotten slowly: a week without practice doubles the weight.
            Some((c, (miss_rate + 0.01) * (1.0 + days / 7.0)))
        })
        .collect();
    keys.sort_by(|a, b| b.1.total_cmp(&a.1));
    keys.into_iter().take(n).map(|(c, _)| c).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutId;

    #[test]
    fn the_layer_course_of_the_corne() {
        let lessons = layer_lessons(&LayoutId::Corne36.build());
        let names: Vec<&str> = lessons.iter().map(|l| l.name.as_str()).collect();
        assert!(names.contains(&"Lower: top row"), "{names:?}");
        assert!(names.contains(&"Raise: home row"));
        assert_eq!(names.last(), Some(&"All the layers"));
        let lower_top = &lessons[names.iter().position(|n| *n == "Lower: top row").unwrap()];
        assert_eq!(lower_top.chars.iter().collect::<String>(), "!@#$%^&*()");
        // A keyboard without layers: its symbols.
        let qwerty = layer_lessons(&LayoutId::QwertyUs.build());
        assert!(qwerty.iter().any(|l| l.chars.contains(&'!')));
    }

    #[test]
    fn real_words_of_the_keys_learned() {
        let grid = KbdGrid::qwerty_us();
        // The home row: words as "salad", "flask".
        let learned = learned_chars(&grid, 10);
        assert!(words_of(&learned, &learned).iter().all(|(w, _)| w.chars().all(|c| learned.contains(&c))));
        let text = word_lines(&learned, &learned, 2).expect("enough words");
        assert_eq!(text.matches(UPSYM).count(), 2);
        // Two keys make no words.
        let first = learned_chars(&grid, 1);
        assert!(word_lines(&first, &first, 2).is_none());
        // The words of the top row lesson have one of its keys at least, with the home row.
        let (learned, lesson) = (learned_chars(&grid, 15), lesson_char_set(15, &grid));
        let words = words_of(&learned, &lesson);
        assert!(words.iter().all(|(w, _)| w.chars().any(|c| lesson.contains(&c))));
        assert!(words.iter().any(|(w, _)| *w == "fear"), "{words:?}");
    }

    #[test]
    fn the_home_row_makes_many_words() {
        let grid = KbdGrid::qwerty_us();
        let home: Vec<char> = "asdfghjkl".chars().collect();
        let words: Vec<&str> = words_of(&home, &home).into_iter().map(|(w, _)| w).collect();
        for w in ["ash", "gas", "fad", "flask", "salad", "glass", "half"] {
            assert!(words.contains(&w), "{w} in {words:?}");
        }
        assert!(words.len() > 60, "{}", words.len());
        // Two lines with no word twice.
        let text = word_lines(&home, &home, 2).unwrap();
        let drawn: Vec<&str> = text.split([' ', '\n', UPSYM]).filter(|w| !w.is_empty()).collect();
        let mut unique = drawn.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), drawn.len(), "{text}");
        // The words of each basic lesson, with --nocapture.
        for lesson in 1..=N_BASIC_LESSONS {
            let set = lesson_char_set(lesson, &grid);
            let n = words_of(&learned_chars(&grid, lesson), &set).len();
            println!("{lesson:2} {:>12} {n} words", set.iter().collect::<String>());
        }
    }

    #[test]
    fn weak_keys_first() {
        let mut progress = LayoutProgress::default();
        let now = 100 * 86_400;
        for (c, hits, misses, last) in [('a', 90, 10, now), ('b', 99, 1, now), ('c', 90, 10, now - 30 * 86_400)] {
            let s = progress.keys.entry(c.to_string()).or_default();
            s.hits = hits;
            s.misses = misses;
            s.last = last;
        }
        assert_eq!(weak_keys(&progress, now, 2), vec!['c', 'a']);
    }
}
