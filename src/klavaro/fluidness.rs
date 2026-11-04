//! The fluidness exercises, `fluidness.c` of Klavaro: whole paragraphs of a text.
use super::{PARAGRAPHS, draw_paragraph, rand_below};

pub const GOAL_ACCURACY: f64 = 97.0;
pub const GOAL_SPEED: f64 = 50.0;
pub const GOAL_FLUIDITY: f64 = 70.0;
/// The default of the "fluid_paragraphs" preference.
const N_PARAGRAPHS: usize = 3;

/// `fluid_filter_utf8`: the paragraphs are separated by empty lines.
pub fn paragraphs() -> Vec<String> {
    PARAGRAPHS
        .split("\n\n")
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|p| !p.is_empty())
        .collect()
}

/// `fluid_draw_random_paragraphs`: some different paragraphs, pseudo randomly.
pub fn draw_random_paragraphs() -> String {
    let all = paragraphs();
    let mut picked: Vec<usize> = Vec::new();
    while picked.len() < N_PARAGRAPHS.min(all.len()) {
        let i = rand_below(all.len());
        if !picked.contains(&i) {
            picked.push(i);
        }
    }
    let mut text = String::new();
    for i in picked {
        draw_paragraph(&mut text, &all[i]);
    }
    text
}
