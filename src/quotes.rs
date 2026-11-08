//! The Quotes and the Affirmations exercises: famous quotes with their authors, positive
//! affirmations, see `data/quotes/LICENSES.md`.
use crate::klavaro::{draw_paragraph, rand_below};

const QUOTES: &str = include_str!("../data/quotes/quotes.txt");
const AFFIRMATIONS: &str = include_str!("../data/quotes/affirmations.txt");
const N_QUOTES: usize = 3;
const N_AFFIRMATIONS: usize = 6;

fn quotes() -> Vec<String> {
    QUOTES
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(author, quote)| format!("{quote} - {author}"))
        .collect()
}

fn affirmations() -> Vec<String> {
    AFFIRMATIONS.lines().map(str::to_string).collect()
}

/// `count` different entries of `pool`, one per paragraph.
fn draw(mut pool: Vec<String>, count: usize) -> String {
    let mut text = String::new();
    for _ in 0..count.min(pool.len()) {
        draw_paragraph(&mut text, &pool.swap_remove(rand_below(pool.len())));
    }
    text
}

pub fn draw_quotes() -> String {
    draw(quotes(), N_QUOTES)
}

pub fn draw_affirmations() -> String {
    draw(affirmations(), N_AFFIRMATIONS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_texts_only_use_the_keys_of_the_keyboards() {
        for entry in quotes().iter().chain(&affirmations()) {
            assert_eq!(entry.trim(), entry);
            for id in crate::layout::LayoutId::ALL {
                let layout = id.build();
                for c in entry.chars() {
                    assert!(layout.locate(c).is_some(), "{id:?} has no {c:?}: {entry}");
                }
            }
        }
        assert!(quotes().len() >= 100);
        assert_eq!(QUOTES.lines().count(), quotes().len());
    }

    #[test]
    fn an_exercise_has_a_paragraph_per_entry() {
        assert_eq!(draw_quotes().lines().count(), N_QUOTES);
        assert_eq!(draw_affirmations().lines().count(), N_AFFIRMATIONS);
    }
}
