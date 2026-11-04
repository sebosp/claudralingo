//! The velocity exercises, `velocity.c` of Klavaro: random words of the dictionary.
use super::{WORDS, draw_paragraph, rand_below, to_upper};

pub const GOAL_ACCURACY: f64 = 95.0;
pub const GOAL_SPEED: f64 = 50.0;

/// `velo_draw_random_words`: 4 paragraphs of 20 words, the first one capitalized.
pub fn draw_random_words() -> String {
    let dict: Vec<&str> = WORDS.lines().map(str::trim).filter(|w| !w.is_empty()).collect();
    let mut text = String::new();
    for _ in 0..4 {
        let mut words: Vec<String> = (0..20).map(|_| dict[rand_below(dict.len())].to_string()).collect();
        if let Some(first) = words.first_mut() {
            let mut chars = first.chars();
            if let Some(c) = chars.next() {
                *first = to_upper(c).to_string() + chars.as_str();
            }
        }
        let paragraph = words.join(" ") + ".";
        draw_paragraph(&mut text, &paragraph);
    }
    text
}
