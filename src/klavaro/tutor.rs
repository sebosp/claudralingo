//! The evaluation of the touches and the final results, `tutor.c` of Klavaro.
//!
//! Klavaro uses `tutor_eval_forward` for the basic, adaptability and velocity exercises, where
//! backspace only beeps, and `tutor_eval_forward_backward` for the fluidness exercises. Here
//! every exercise uses `tutor_eval_forward_backward`: a wrong touch moves forward, the following
//! touches are wrong until the errors are erased with backspace, and the retyped characters count
//! as errors.
use super::accuracy::Accuracy;
use super::{BACKSPACE, HYPER_BACKSPACE, KbdGrid, UPSYM};

/// The fluidness exercises are not logged with less touches than this, `MIN_CHARS_TO_LOG`.
pub const MIN_CHARS_TO_LOG: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharState {
    Untouched,
    Correct,
    Wrong,
    /// Typed correctly after a correction, Klavaro's `char_retouched`.
    Retouched,
}

/// Klavaro's `TutorQuery` without the intro, the first touch starts the timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Query {
    Start,
    ProcessTouches,
    End,
}

/// What a touch did, for the animations and the per key statistics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// Klavaro beeps: a backspace without errors to erase, or a touch after the end.
    Beep,
    Correct {
        expected: char,
        /// `None` when retyped after a correction.
        touch_time: Option<f64>,
    },
    Wrong {
        expected: char,
        typed: char,
        /// Only the first error of a sequence is accumulated.
        first: bool,
    },
    Erased,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Results {
    pub accuracy: f64,
    /// Words per minute, 12 times the correct touches per second.
    pub velocity: f64,
    pub fluidness: f64,
    pub elapsed: f64,
    pub n_touchs: usize,
    pub n_errors: usize,
}

#[derive(Debug, Clone)]
pub struct Tutor {
    pub text: Vec<char>,
    pub states: Vec<CharState>,
    pub cursor: usize,
    pub query: Query,
    pub n_touchs: usize,
    pub n_errors: usize,
    /// The number of wrong characters before the cursor, to be erased.
    pub retro_pos: usize,
    correcting: usize,
    touch_times: Vec<f64>,
    last_touch: f64,
    start: f64,
    pub results: Option<Results>,
}

impl Tutor {
    pub fn new(text: &str) -> Self {
        let text: Vec<char> = text.chars().collect();
        Tutor {
            states: vec![CharState::Untouched; text.len()],
            text,
            cursor: 0,
            query: Query::Start,
            n_touchs: 0,
            n_errors: 0,
            retro_pos: 0,
            correcting: 0,
            touch_times: Vec::new(),
            last_touch: 0.0,
            start: 0.0,
            results: None,
        }
    }

    pub fn current_char(&self) -> Option<char> {
        self.text.get(self.cursor).copied()
    }

    /// The character to type next: the one at the cursor, or backspace while there are errors.
    pub fn expected(&self) -> Option<char> {
        match self.query {
            Query::End => None,
            _ if self.retro_pos > 0 => Some(BACKSPACE),
            _ => self.current_char(),
        }
    }

    /// The elapsed time since the first touch.
    pub fn elapsed(&self, now: f64) -> f64 {
        match (self.query, self.results) {
            (_, Some(results)) => results.elapsed,
            (Query::Start, _) => 0.0,
            _ => now - self.start,
        }
    }

    fn advance(&mut self) -> bool {
        if self.cursor + 1 < self.text.len() {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    /// `tutor_process_touch` for the touches while the exercise runs, `now` in seconds.
    pub fn process_touch(&mut self, user_chr: char, now: f64, accur: &mut Accuracy, grid: &KbdGrid) -> Outcome {
        match self.query {
            Query::End => return Outcome::Beep,
            Query::Start => {
                if user_chr == BACKSPACE || user_chr == HYPER_BACKSPACE {
                    return Outcome::Beep;
                }
                self.query = Query::ProcessTouches;
                self.start = now;
                self.last_touch = now;
            }
            Query::ProcessTouches => {}
        }
        let outcome = self.eval_forward_backward(user_chr, now, accur, grid);
        if outcome == Outcome::Finished {
            self.query = Query::End;
            self.results = Some(self.calc_stats(now - self.start));
        }
        outcome
    }

    /// `tutor_eval_forward_backward`
    fn eval_forward_backward(&mut self, user_chr: char, now: f64, accur: &mut Accuracy, grid: &KbdGrid) -> Outcome {
        if user_chr == BACKSPACE || user_chr == HYPER_BACKSPACE {
            self.last_touch = now;
            // Only the errors can be erased.
            if self.retro_pos == 0 || self.cursor == 0 {
                return Outcome::Beep;
            }
            self.cursor -= 1;
            if self.text[self.cursor] == '\n' {
                if self.cursor == 0 {
                    return Outcome::Beep;
                }
                self.cursor -= 1;
            }
            self.states[self.cursor] = CharState::Untouched;
            self.retro_pos -= 1;
            self.correcting += 1;
            if user_chr == HYPER_BACKSPACE && self.retro_pos > 0 {
                return self.eval_forward_backward(HYPER_BACKSPACE, now, accur, grid);
            }
            return Outcome::Erased;
        }

        let real_chr = self.text[self.cursor];
        // Minimizing the line breaking bug.
        let user_chr = if user_chr == UPSYM && real_chr == ' ' { ' ' } else { user_chr };

        let outcome = if user_chr == real_chr && self.retro_pos == 0 {
            self.n_touchs += 1;
            let touch_time = now - self.last_touch;
            self.touch_times.push(touch_time);
            self.last_touch = now;
            if self.correcting != 0 {
                self.states[self.cursor] = CharState::Retouched;
                self.n_errors += 1;
                Outcome::Correct { expected: real_chr, touch_time: None }
            } else {
                self.states[self.cursor] = CharState::Correct;
                accur.correct(real_chr, touch_time, grid);
                Outcome::Correct { expected: real_chr, touch_time: Some(touch_time) }
            }
        } else {
            self.last_touch = now;
            self.states[self.cursor] = CharState::Wrong;
            self.retro_pos += 1;
            let first = self.retro_pos == 1;
            if first {
                accur.wrong(real_chr, grid);
            }
            Outcome::Wrong { expected: real_chr, typed: user_chr, first }
        };

        self.correcting = self.correcting.saturating_sub(1);

        // Go forward and test the end of the text.
        if !self.advance() {
            if self.retro_pos == 0 {
                return Outcome::Finished;
            }
            self.retro_pos -= 1;
        }
        // Test the end of the paragraph.
        if self.text[self.cursor] == '\n' && !self.advance() && self.retro_pos == 0 {
            return Outcome::Finished;
        }
        outcome
    }

    /// `tutor_calc_stats`: accuracy, velocity and the "magic" fluidness.
    fn calc_stats(&self, elapsed: f64) -> Results {
        let elapsed = elapsed.max(1e-3);
        let n_touchs = self.n_touchs.max(1) as f64;
        let accuracy = 100.0 * (1.0 - self.n_errors as f64 / n_touchs);
        let touchs_per_second = (self.n_touchs - self.n_errors.min(self.n_touchs)) as f64 / elapsed;
        let velocity = 12.0 * touchs_per_second;

        let samples: Vec<f64> = self
            .touch_times
            .iter()
            .skip(2)
            .map(|t| (1.0 / t.max(1.0e-8)).sqrt())
            .collect();
        let i = samples.len() + 2;
        let mut average = samples.iter().sum::<f64>() / (if i == 2 { 3 } else { i } - 2) as f64;
        if average <= 0.0 {
            average = 1.0e-9;
        }
        let sum: f64 = samples.iter().map(|s| (s - average) * (s - average)).sum();
        let standard_deviation = (sum / (i.max(4) - 3) as f64).sqrt();
        let fluidness = (100.0 * (1.0 - standard_deviation / average)).max(2.0);
        Results {
            accuracy,
            velocity,
            fluidness,
            elapsed,
            n_touchs: self.n_touchs,
            n_errors: self.n_errors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_str(tutor: &mut Tutor, s: &str, t: &mut f64, accur: &mut Accuracy) -> Outcome {
        let grid = KbdGrid::qwerty_us();
        let mut last = Outcome::Beep;
        for c in s.chars() {
            *t += 0.2;
            last = tutor.process_touch(c, *t, accur, &grid);
        }
        last
    }

    #[test]
    fn backspace_erases_the_errors_and_retyped_chars_count() {
        let mut tutor = Tutor::new("fj fj\u{b6}\n");
        let mut accur = Accuracy::default();
        let mut t = 0.0;
        type_str(&mut tutor, "fk", &mut t, &mut accur);
        assert_eq!(tutor.retro_pos, 1);
        assert_eq!(tutor.expected(), Some(BACKSPACE));
        // The errors block the progress until erased.
        type_str(&mut tutor, " ", &mut t, &mut accur);
        assert_eq!(tutor.retro_pos, 2);
        type_str(&mut tutor, "\u{8}\u{8}", &mut t, &mut accur);
        assert_eq!(tutor.retro_pos, 0);
        assert_eq!(tutor.cursor, 1);
        assert_eq!(tutor.states[1], CharState::Untouched);
        // Backspace over correct characters only beeps.
        assert_eq!(type_str(&mut tutor, "\u{8}", &mut t, &mut accur), Outcome::Beep);
        let last = type_str(&mut tutor, "j fj\u{b6}", &mut t, &mut accur);
        assert_eq!(last, Outcome::Finished);
        // Two erased characters, the next two typed count as errors.
        assert_eq!(tutor.states[1], CharState::Retouched);
        assert_eq!(tutor.states[2], CharState::Retouched);
        assert_eq!(tutor.states[3], CharState::Correct);
        let results = tutor.results.unwrap();
        assert_eq!(results.n_touchs, 6);
        assert_eq!(results.n_errors, 2);
        assert_eq!(accur.terror[0].ch, 'j');
    }

    #[test]
    fn enter_types_a_space_at_the_end_of_a_line() {
        let mut tutor = Tutor::new("a b\u{b6}\n");
        let mut accur = Accuracy::default();
        let mut t = 0.0;
        assert_eq!(type_str(&mut tutor, "a\u{b6}b\u{b6}", &mut t, &mut accur), Outcome::Finished);
        assert_eq!(tutor.results.unwrap().accuracy, 100.0);
    }
}
