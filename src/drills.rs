//! The finger drills: the keys of one finger only, up and down its column, and sideways for the
//! index fingers that have two columns. The thumbs drill presses Space and Enter.
use crate::klavaro::{UPSYM, rand_below};
use crate::layout::{Layout, Row};

/// The accuracy to call a drill done, the goal of Klavaro's basic course.
pub const GOAL_ACCURACY: f64 = 95.0;
const N_LINES: usize = 5;
const WORDS_PER_LINE: usize = 8;
/// The chance to change of row on each key, the drills are about moving the finger.
const ROW_CHANGE: f64 = 0.75;

/// The fingers in the order of the drill selector, the thumbs in the middle.
pub const FINGERS: [u8; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];

pub fn draw_finger_drill(layout: &Layout, finger: u8) -> String {
    if finger == 5 {
        return draw_thumbs_drill();
    }
    let keys = layout.finger_chars(finger);
    if keys.len() < 2 {
        return draw_thumbs_drill();
    }
    let mut text = String::new();
    let mut last = rand_below(keys.len());
    for _ in 0..N_LINES {
        for w in 0..WORDS_PER_LINE {
            for _ in 0..5 {
                last = next_key(&keys, last);
                text.push(keys[last].0);
            }
            text.push(if w + 1 == WORDS_PER_LINE { UPSYM } else { ' ' });
        }
        text.push('\n');
    }
    text
}

/// Another key of the finger, most of the time on another row.
fn next_key(keys: &[(char, Row)], last: usize) -> usize {
    let other_rows: Vec<usize> = (0..keys.len()).filter(|i| keys[*i].1 != keys[last].1).collect();
    if !other_rows.is_empty() && fastrand::f64() < ROW_CHANGE {
        return other_rows[rand_below(other_rows.len())];
    }
    let others: Vec<usize> = (0..keys.len()).filter(|i| *i != last).collect();
    others[rand_below(others.len())]
}

/// Short words of the index home keys, separated by Space and ended by Enter.
fn draw_thumbs_drill() -> String {
    const LETTERS: [char; 4] = ['f', 'j', 'd', 'k'];
    let mut text = String::new();
    for _ in 0..N_LINES + 3 {
        let n_words = 2 + rand_below(3);
        for w in 0..n_words {
            for _ in 0..1 + rand_below(2) {
                text.push(LETTERS[rand_below(LETTERS.len())]);
            }
            text.push(if w + 1 == n_words { UPSYM } else { ' ' });
        }
        text.push('\n');
    }
    text
}

pub fn comment(accuracy: f64, finger: u8) -> String {
    let finger = crate::layout::finger_name(finger);
    if accuracy < GOAL_ACCURACY {
        format!("Keep the other fingers on the home row, only the {finger} moves.\nPress Enter to repeat the drill.")
    } else {
        format!("Your {finger} knows its keys!\nPress Enter for another drill, or pick another finger with ‹ ›.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutId;

    #[test]
    fn drills_only_use_the_finger() {
        for id in LayoutId::ALL {
            let layout = id.build();
            for finger in FINGERS {
                let text = draw_finger_drill(&layout, finger);
                assert!(text.ends_with('\n'));
                for c in text.chars().filter(|c| !matches!(*c, ' ' | '\n' | UPSYM)) {
                    if finger != 5 {
                        assert_eq!(layout.finger_of(c), Some(finger), "{c:?} in the {finger} drill on {id:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_pinky_moves_vertically() {
        let layout = LayoutId::Corne36.build();
        let text: Vec<char> = draw_finger_drill(&layout, 1).chars().collect();
        // q, a and z: no letter twice in a row.
        assert!(text.windows(2).all(|w| w[0] != w[1] || !w[0].is_alphabetic()));
    }
}
