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

/// The keys of the middle finger from the top row down, and the columns of the index, the home
/// column first.
pub fn bounce_keys(layout: &Layout, left: bool) -> (Vec<char>, Vec<Vec<char>>) {
    const ROWS: [Row; 3] = [Row::Top, Row::Home, Row::Bottom];
    let (middle, index) = if left { (3, 4) } else { (7, 6) };
    let in_row = |finger: u8, row: Row| -> Vec<char> {
        layout.finger_chars(finger).into_iter().filter(|(_, r)| *r == row).map(|(c, _)| c).collect()
    };
    let middle_keys: Vec<char> = ROWS.iter().filter_map(|r| in_row(middle, *r).first().copied()).collect();
    let rows: Vec<Vec<char>> = ROWS.iter().map(|r| in_row(index, *r)).collect();
    let n_columns = rows.iter().map(Vec::len).min().unwrap_or(0);
    // The home column is the outer one of the index, on the left of the right hand.
    let columns = (0..n_columns)
        .map(|k| {
            let k = if left { k } else { n_columns - 1 - k };
            rows.iter().map(|row| row[k]).collect()
        })
        .collect();
    (middle_keys, columns)
}

/// The finger bounce of a hand: the middle finger presses a key, the index runs down its column
/// (top, home, bottom row), the middle finger goes to its next row, the index runs again, for
/// both index columns. Then the same moves in a random order. The case alternates on every
/// letter: on a keyboard with auto shift every other key is held a moment longer.
pub fn draw_bounce(layout: &Layout, left: bool) -> String {
    const WORDS_PER_BOUNCE_LINE: usize = 6;
    let (middle_keys, columns) = bounce_keys(layout, left);
    let index_keys: Vec<char> = columns.concat();
    if middle_keys.is_empty() || index_keys.len() < 3 {
        return draw_thumbs_drill();
    }
    let mut words: Vec<Vec<char>> = Vec::new();
    for column in &columns {
        for m in &middle_keys {
            words.push(std::iter::once(*m).chain(column.iter().copied()).collect());
        }
    }
    let n_ordered = words.len();
    while words.len() < n_ordered + N_LINES * WORDS_PER_BOUNCE_LINE {
        let mut keys = index_keys.clone();
        fastrand::shuffle(&mut keys);
        let m = middle_keys[rand_below(middle_keys.len())];
        words.push(std::iter::once(m).chain(keys.into_iter().take(3)).collect());
    }
    let mut text = String::new();
    for (i, word) in words.iter().enumerate() {
        for (j, c) in word.iter().enumerate() {
            // Every other letter shifted, every other word starts shifted.
            let upper = (i + j) % 2 == 1;
            text.push(if upper { layout.shifted(*c) } else { *c });
        }
        // The ordered bounces on the first lines, then the random ones.
        let line_end = if i < n_ordered {
            i + 1 == n_ordered
        } else {
            (i + 1 - n_ordered).is_multiple_of(WORDS_PER_BOUNCE_LINE)
        };
        if line_end || i + 1 == words.len() {
            text.push(UPSYM);
            text.push('\n');
        } else {
            text.push(' ');
        }
    }
    text
}

pub fn bounce_comment(accuracy: f64) -> String {
    if accuracy < GOAL_ACCURACY {
        "Slow down: the middle finger stays on its column, the index visits every row.\nPress Enter to bounce again.".into()
    } else {
        "Clean bounces! The speed is lower with the alternating case, that is expected.\nPress Enter to bounce again, ‹ › for the other hand.".into()
    }
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
    fn bounces() {
        for id in LayoutId::ALL {
            let layout = id.build();
            for (left, fingers) in [(true, [3, 4]), (false, [7, 6])] {
                let text = draw_bounce(&layout, left);
                let first = text.split([' ', UPSYM]).next().unwrap();
                assert_eq!(first, if left { "eRfV" } else { "iUjM" }, "{id:?}");
                for c in text.chars().filter(|c| !matches!(*c, ' ' | '\n' | UPSYM)) {
                    let finger = layout.finger_of(c).unwrap();
                    assert!(fingers.contains(&finger), "{c:?} on {id:?}");
                }
                // The case alternates inside the words.
                for word in text.split([' ', '\n', UPSYM]).filter(|w| !w.is_empty()) {
                    let ups: Vec<bool> = word.chars().map(|c| layout.shifted(c) == c && c.is_alphabetic()).collect();
                    assert!(ups.windows(2).all(|w| w[0] != w[1] || !word.chars().all(char::is_alphabetic)), "{word}");
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
