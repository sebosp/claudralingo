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

/// `velo_comment`, the levels of `tutor_init_goals`.
pub fn comment(accuracy: f64, speed: f64) -> String {
    if accuracy < GOAL_ACCURACY {
        ":-(".into()
    } else if speed < 10.0 {
        "You are just beginning.\nBe patient, try it again every day, rest and don't worry so \
         much:\npersistence and practice will improve your velocity."
            .into()
    } else if speed < 20.0 {
        "Still away from the highway. You can do better...\nTry to reach at least 20 WPM.".into()
    } else if speed < 30.0 {
        "You are doing well, but need to go faster.\nAnd don't forget the accuracy. Try to get 30 WPM."
            .into()
    } else if speed < 40.0 {
        "Fine. Now you need to start running.\nCan you reach 40 WPM?".into()
    } else if speed < GOAL_SPEED {
        format!("Very good. You are almost there.\nCan you finally reach {GOAL_SPEED:.0} WPM?")
    } else if speed < 60.0 {
        "Excellent. For this course, that is enough.\nTry now the fluidness exercises, OK?".into()
    } else if speed < 70.0 {
        "Fast! Are you a professional?\nSo, try to get 70 WPM!".into()
    } else if speed < 80.0 {
        "Ranking good, Speed Racer! Are you afraid of reaching 80 WPM?".into()
    } else if speed < 90.0 {
        "Kung-fu mastery!\nCan you fly at 90 WPM?".into()
    } else {
        "Dvorak master!\nI have no words to express my admiration!".into()
    }
}
