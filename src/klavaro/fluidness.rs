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

/// `fluid_comment`, the levels of `tutor_init_goals`.
pub fn comment(accuracy: f64, speed: f64, fluidness: f64) -> String {
    if accuracy < GOAL_ACCURACY {
        ":-(".into()
    } else if speed < GOAL_SPEED {
        format!("You type accurately but not so fast.\nCan you reach {GOAL_SPEED:.0} WPM?")
    } else if fluidness < 60.0 {
        "Your rhythm is not so constant. Calm down.\nFor now, try to make the fluidness greater \
         than 60%."
            .into()
    } else if fluidness < GOAL_FLUIDITY {
        format!(
            "You are almost getting there. Type more fluently.\nI want a fluidness greater than \
             {GOAL_FLUIDITY:.0}%."
        )
    } else if speed < 90.0 {
        "Congratulations!\nIt seems to me that you are a professional.\nYou don't need this \
         program (me) anymore.\nHope you have enjoyed. Thanks and be happy!"
            .into()
    } else {
        "How can you type so fast?\nYou have exceeded all my expectations.\nAre you a machine? \
         Could you teach me?\nI can not help you anymore. Go to an expert!"
            .into()
    }
}
