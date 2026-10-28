//! The adaptability exercises, `adaptability.c` of Klavaro: random weird words made of the keys
//! of the keyboard, or of the keys with errors and slow touch times when there are too many.
use super::accuracy::Accuracy;
use super::{KbdGrid, draw_paragraph, rand_below, to_upper};

pub const GOAL_ACCURACY: f64 = 98.0;
const LINES: usize = 4;
const WORDS: usize = 22;
pub const MAX_WORD_LEN: usize = 9;

/// `adapt_draw_random_pattern`, `special` is the "too much errors" toggle of Klavaro.
pub fn draw_random_pattern(grid: &KbdGrid, accuracy: &Accuracy, special: bool) -> String {
    let mut text = String::new();
    let mut profile_type = 0;
    for _ in 0..LINES {
        let mut line = String::new();
        for j in 0..WORDS {
            let mut word = if special { accuracy.create_word(&mut profile_type) } else { None }
                .unwrap_or_else(|| {
                    if rand_below(15) != 0 { create_word(grid) } else { create_number() }
                });
            if j == 0 {
                word[0] = to_upper(word[0]);
            } else {
                line.push(' ');
            }
            line.extend(word);
        }
        line.push('.');
        draw_paragraph(&mut text, &line);
    }
    text
}

/// `adapt_create_word`: alternating consonants and vowels, sometimes a capital or a symbol.
pub fn create_word(grid: &KbdGrid) -> Vec<char> {
    let vowels = grid.vowels();
    let consonants = grid.consonants();
    let symbols = grid.symbols();
    let vowel = || vowels[rand_below(vowels.len())];
    let consonant = || consonants[rand_below(consonants.len())];
    let n = rand_below(MAX_WORD_LEN - 1) + 1;
    let mut word = Vec::with_capacity(n + 1);
    for i in 0..n {
        if rand_below(25) != 0 {
            let mut c = if i % 2 == 1 {
                if rand_below(30) != 0 { vowel() } else { consonant() }
            } else if rand_below(50) != 0 {
                consonant()
            } else {
                vowel()
            };
            if i == 0 && rand_below(7) == 0 {
                c = to_upper(c);
            }
            word.push(c);
        } else {
            let mut c = symbols[rand_below(symbols.len())];
            if c == '\\' && i > 0 {
                c = '-';
            }
            word.push(c);
            // A space after the symbol ends the word, most often.
            if rand_below(5) != 0 || c == '-' || c == '\\' {
                return word;
            }
        }
    }
    word.push(if rand_below(20) != 0 { vowel() } else { ',' });
    word
}

/// `adapt_create_number`
pub fn create_number() -> Vec<char> {
    (0..4).map(|_| char::from(b'0' + rand_below(10) as u8)).collect()
}

/// `adapt_comment`, the levels of `tutor_init_goals`.
pub fn comment(accuracy: f64) -> String {
    if accuracy < 50.0 {
        ":-(".into()
    } else if accuracy < 90.0 {
        "Your accuracy rate is below 90%...\nCould you please try again to improve it?".into()
    } else if accuracy < 95.0 {
        "You are doing well. But...\nCould you make the accuracy reach 95%?".into()
    } else if accuracy < GOAL_ACCURACY {
        format!(
            "You are almost there, but your accuracy rate is still below {GOAL_ACCURACY:.0}%.\n\
             Try a few more times, or maybe you're getting upset, so go to another kind of exercise."
        )
    } else {
        format!(
            "Very good!\nYou succeeded with an accuracy rate above {GOAL_ACCURACY:.0}%.\n\
             Now it is time to increase your velocity.\nGo to the velocity exercises."
        )
    }
}
