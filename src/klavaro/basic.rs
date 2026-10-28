//! The basic course, `basic.c` of Klavaro.
use super::{BASIC_LESSONS, KbdGrid, N_BASIC_LESSONS, UPSYM, rand_below, to_lower};

/// Klavaro asks for this accuracy to go to the next lesson, `goal.basic.accuracy`.
pub const GOAL_ACCURACY: f64 = 95.0;
const N_LINES: usize = 8;

/// `basic_init_char_set`: the characters of a lesson (1 based), the `1` of the masks on the
/// lower and then the upper chars of the keyboard grid.
pub fn lesson_char_set(lesson: usize, grid: &KbdGrid) -> Vec<char> {
    let lesson = lesson.clamp(1, N_BASIC_LESSONS);
    let lines: Vec<&str> = BASIC_LESSONS.lines().skip((lesson - 1) * 11 + 1).take(9).collect();
    let mut char_set = Vec::new();
    for (i, mask) in lines.iter().take(4).enumerate() {
        for (j, bit) in mask.chars().take(14).enumerate() {
            let c = grid.lochars[i][j];
            if bit == '1' && is_graph(c) {
                char_set.push(to_lower(c));
            }
        }
    }
    for (i, mask) in lines.iter().skip(5).take(4).enumerate() {
        for (j, bit) in mask.chars().take(14).enumerate() {
            let c = grid.upchars[i][j];
            if bit == '1' && is_graph(c) {
                char_set.push(to_lower(c));
            }
        }
    }
    char_set
}

fn is_graph(c: char) -> bool {
    !c.is_whitespace() && !c.is_control()
}

/// `basic_draw_lesson`: 8 lines of 9 words of 5 random characters of the lesson.
///
/// The characters are drawn from a pool without repetition until the pool is empty, the pool is
/// doubled for the lessons with 5 to 13 characters.
pub fn draw_lesson(char_set: &[char]) -> String {
    let mut text = String::new();
    if char_set.len() < 2 {
        return text;
    }
    let fill_pool = |pool: &mut Vec<char>| {
        pool.clear();
        pool.extend_from_slice(char_set);
        if char_set.len() > 4 && char_set.len() < 14 {
            pool.extend_from_slice(char_set);
        }
    };
    let mut pool = Vec::new();
    fill_pool(&mut pool);
    for i in 0..N_LINES {
        for j in 0..9 {
            for _ in 0..5 {
                text.push(pool.swap_remove(rand_below(pool.len())));
                if pool.is_empty() {
                    fill_pool(&mut pool);
                }
            }
            text.push(if j == 8 { UPSYM } else { ' ' });
        }
        text.push('\n');
        if pool.len() == 2 && i >= N_LINES / 2 - 1 {
            break;
        }
    }
    text
}

/// `basic_comment`
pub fn comment(accuracy: f64, lesson: usize) -> &'static str {
    if accuracy < GOAL_ACCURACY {
        ":-(\nPress Enter to repeat the lesson, Backspace to go back one lesson."
    } else if lesson >= N_BASIC_LESSONS {
        "Positions of keys seems to be learned!\nCongratulations! You have accomplished the \
         entire basic course.\nGo to the next type of exercise: adaptability."
    } else {
        "All right, now you got it!\nPress Enter to go to the next lesson."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_lessons_are_the_home_row() {
        let grid = KbdGrid::qwerty_us();
        assert_eq!(lesson_char_set(1, &grid), vec!['f', 'j']);
        assert_eq!(lesson_char_set(5, &grid), vec!['a', 's', 'd', 'f']);
    }

    #[test]
    fn every_lesson_has_characters() {
        let grid = KbdGrid::qwerty_us();
        for lesson in 1..=N_BASIC_LESSONS {
            let set = lesson_char_set(lesson, &grid);
            assert!(set.len() >= 2, "lesson {lesson}: {set:?}");
            let text = draw_lesson(&set);
            assert!(text.ends_with('\n'));
            assert!(text.chars().all(|c| set.contains(&c) || " \n".contains(c) || c == UPSYM));
        }
    }
}
