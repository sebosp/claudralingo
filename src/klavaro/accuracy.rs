//! The touch errors and touch times accumulators, `accuracy.c` of Klavaro.
//!
//! Klavaro saves them to `accuracy.log` and `proficiency.log`, here they are part of the progress
//! file, see [`crate::progress`].
use serde::{Deserialize, Serialize};

use super::{BACKSPACE, HYPER_BACKSPACE, KbdGrid, UPSYM, rand_below, to_lower};
use super::adaptability::MAX_WORD_LEN;

pub const MAX_CHARS_EVALUATED: usize = 130;
pub const MAX_TT_SAVED: usize = 30;
/// How many correct touches to eliminate a wrong one.
pub const ERROR_INERTIA: u64 = 10;
/// How many errors to start a special practice on adaptability.
pub const ERROR_LIMIT: u64 = 60;
/// How many times the 10th slowest key touch time should be relative to the 40th slowest one,
/// to start a special practice on adaptability.
pub const PROFI_LIMIT: f64 = 2.2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchError {
    pub ch: char,
    pub wrong: u64,
    pub correct: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchTime {
    pub ch: char,
    pub dt: Vec<f64>,
    pub idx: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Accuracy {
    pub terror: Vec<TouchError>,
    pub ttime: Vec<TouchTime>,
}

fn is_evaluated(c: char, grid: &KbdGrid) -> bool {
    !matches!(c, ' ' | UPSYM | HYPER_BACKSPACE | BACKSPACE) && grid.is_inset(c)
}

impl Accuracy {
    /// `accur_init` drops the entries that are not worth keeping.
    pub fn sanitize(&mut self) {
        self.terror
            .retain(|e| 1234 > e.wrong && e.wrong > 0 && e.correct <= ERROR_INERTIA);
        self.terror.truncate(MAX_CHARS_EVALUATED);
        for t in &mut self.ttime {
            t.dt.resize(MAX_TT_SAVED + 2, 0.0);
            t.idx = t.idx.min(MAX_TT_SAVED - 2);
        }
        self.ttime.truncate(MAX_CHARS_EVALUATED);
    }

    /// `accur_close` does not save the errors already compensated by correct touches.
    pub fn prune(&mut self) {
        self.terror.retain(|e| e.wrong > 0 && e.correct <= ERROR_INERTIA);
    }

    /// `accur_correct`
    pub fn correct(&mut self, c: char, touch_time: f64, grid: &KbdGrid) {
        if !is_evaluated(c, grid) {
            return;
        }
        if let Some(e) = self.terror.iter_mut().find(|e| e.ch == c) {
            if e.correct > ERROR_INERTIA {
                e.wrong = e.wrong.saturating_sub(1);
                e.correct = 1;
            } else {
                e.correct += 1;
            }
        }
        if touch_time < 0.001 {
            return;
        }
        let c = to_lower(c);
        if let Some(t) = self.ttime.iter_mut().find(|t| t.ch == c) {
            t.dt[t.idx] = touch_time;
            t.idx += 1;
            if t.idx >= MAX_TT_SAVED - 1 {
                t.idx = 0;
            }
            return;
        }
        if self.ttime.len() >= MAX_CHARS_EVALUATED {
            return;
        }
        let mut dt = vec![0.0; MAX_TT_SAVED + 2];
        dt[0] = touch_time;
        self.ttime.push(TouchTime { ch: c, dt, idx: 1 });
    }

    /// `accur_wrong`: when the pool is full the entry with the lowest error ratio is replaced.
    pub fn wrong(&mut self, c: char, grid: &KbdGrid) {
        if !is_evaluated(c, grid) {
            return;
        }
        let mut i_min = None;
        let mut correct_min = 1e9;
        for (i, e) in self.terror.iter_mut().enumerate() {
            if e.ch == c {
                e.wrong += 1;
                return;
            }
            let hlp = e.wrong as f64 / (e.correct + e.wrong + 1) as f64;
            if hlp <= correct_min {
                correct_min = hlp;
                i_min = Some(i);
            }
        }
        let entry = TouchError { ch: c, wrong: 1, correct: 1 };
        if self.terror.len() < MAX_CHARS_EVALUATED {
            self.terror.push(entry);
        } else {
            let i = i_min.unwrap_or(self.terror.len() - 1);
            self.terror[i] = entry;
        }
    }

    /// `accur_error_total`
    pub fn error_total(&self) -> u64 {
        self.terror.iter().map(|e| if e.wrong < 12345 { e.wrong } else { 0 }).sum()
    }

    /// `accur_profi_aver`: the average of the touch times below 3 seconds.
    pub fn profi_aver(&self, idx: usize) -> f64 {
        let Some(t) = self.ttime.get(idx) else {
            return -10.0;
        };
        let valid: Vec<f64> = t.dt.iter().take(MAX_TT_SAVED).copied().filter(|dt| *dt > 0.0 && *dt < 3.0).collect();
        if valid.is_empty() { 0.0 } else { valid.iter().sum::<f64>() / valid.len() as f64 }
    }

    /// `accur_profi_aver_norm`
    fn profi_aver_norm(&self, idx: usize) -> i64 {
        let last = self.profi_aver(self.ttime.len().saturating_sub(1));
        if last == 0.0 {
            return -3;
        }
        (self.profi_aver(idx) / last).round() as i64
    }

    /// `accur_sort`: the errors by decreasing wrongness then increasing correctness, the touch
    /// times by decreasing average, removing the keys without touch times.
    pub fn sort(&mut self) {
        self.terror.sort_by_key(|e| e.correct);
        self.terror.sort_by_key(|e| std::cmp::Reverse(e.wrong));
        let mut averages: Vec<(f64, TouchTime)> =
            (0..self.ttime.len()).map(|i| (self.profi_aver(i), self.ttime[i].clone())).collect();
        averages.sort_by(|a, b| b.0.total_cmp(&a.0));
        while averages.last().is_some_and(|(aver, _)| *aver < 0.001) {
            averages.pop();
        }
        self.ttime = averages.into_iter().map(|(_, t)| t).collect();
    }

    /// The adaptability exercise is "special" when there are too many errors or too slow keys.
    pub fn needs_special_practice(&self) -> bool {
        let mut special = self.error_total() >= ERROR_LIMIT;
        if self.ttime.len() >= 40 {
            special = special || self.profi_aver(9) / self.profi_aver(39) > PROFI_LIMIT;
        }
        special
    }

    /// `accur_create_word`: a random weird word based on the error profile, alternating between
    /// the error and the touch time profiles.
    pub fn create_word(&self, profile_type: &mut u8) -> Option<Vec<char>> {
        if self.terror.len() < 10 && self.ttime.len() < 5 {
            return None;
        }
        let ptype_terror = self.error_total() >= ERROR_LIMIT;
        let ptype_ttime =
            self.ttime.len() >= 40 && self.profi_aver(9) / self.profi_aver(39) > PROFI_LIMIT;
        let n = rand_below(MAX_WORD_LEN) + 1;
        let mut word = Vec::with_capacity(n);
        let mut last = '\0';
        for _ in 0..n {
            *profile_type = if *profile_type == 0 {
                u8::from(ptype_ttime)
            } else {
                u8::from(!ptype_terror)
            };
            let c = if *profile_type == 0 && !self.terror.is_empty() {
                let mut ind = 0;
                for _ in 0..100 {
                    ind = rand_below(self.terror.len());
                    if self.terror[ind].ch == last {
                        continue;
                    }
                    if (rand_below(self.terror[0].wrong.max(1) as usize) as u64) < self.terror[ind].wrong {
                        break;
                    }
                }
                self.terror[ind].ch
            } else if !self.ttime.is_empty() {
                let mut ind = 0;
                for _ in 0..100 {
                    ind = rand_below(self.ttime.len());
                    if self.ttime[ind].ch == last {
                        continue;
                    }
                    let norm0 = self.profi_aver_norm(0).max(1) as usize;
                    if (rand_below(norm0) as i64) < self.profi_aver_norm(ind) {
                        break;
                    }
                }
                self.ttime[ind].ch
            } else {
                continue;
            };
            word.push(c);
            last = c;
        }
        if word.is_empty() { None } else { Some(word) }
    }
}
