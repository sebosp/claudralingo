//! The progress file: the settings, the misses and the touch times per key and the best keys.
//!
//! Native builds save to `~/.config/claudralingo/progress.json` (or `$XDG_CONFIG_HOME`), the web
//! build to the `localStorage` of the browser.
use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::klavaro::accuracy::Accuracy;
use crate::layout::LayoutId;
use crate::theme::ThemeMode;

/// The sessions kept per layout, the browser storage is a few megabytes.
const MAX_SESSIONS: usize = 5000;
/// The keys with less touches are not ranked.
const MIN_TOUCHES_TO_RANK: u64 = 20;
const N_RANKED_KEYS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ExerciseMode {
    #[default]
    Basic,
    /// The keys of the layers, row by row, see [`crate::course`].
    Layers,
    /// The keys most needed, at the start of a session.
    Review,
    Adaptability,
    Velocity,
    Fluidness,
    /// The keys of one finger, up and down its column.
    Fingers,
    /// The middle and the index finger of the left hand in turns, in alternating case.
    BounceLeft,
    BounceRight,
    /// Escape, the arrows, the numbers, the symbols and the Ctrl and Alt shortcuts.
    Shortcuts,
    /// The words of HSK 1 to 4, written with a pen and typed in pinyin, see [`crate::hanzi`].
    Hanzi,
}

impl ExerciseMode {
    pub const ALL: [ExerciseMode; 11] = [
        ExerciseMode::Basic,
        ExerciseMode::Layers,
        ExerciseMode::Review,
        ExerciseMode::Adaptability,
        ExerciseMode::Velocity,
        ExerciseMode::Fluidness,
        ExerciseMode::Fingers,
        ExerciseMode::BounceLeft,
        ExerciseMode::BounceRight,
        ExerciseMode::Shortcuts,
        ExerciseMode::Hanzi,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ExerciseMode::Basic => "Basic course",
            ExerciseMode::Layers => "Layer course",
            ExerciseMode::Review => "Warm-up: weak keys",
            ExerciseMode::Adaptability => "Adaptability",
            ExerciseMode::Velocity => "Velocity",
            ExerciseMode::Fluidness => "Fluidness",
            ExerciseMode::Fingers => "Finger drill",
            ExerciseMode::BounceLeft => "Left hand finger bounce",
            ExerciseMode::BounceRight => "Right hand finger bounce",
            ExerciseMode::Shortcuts => "Shortcuts & keys",
            ExerciseMode::Hanzi => "Hanzi (HSK 1-4)",
        }
    }

    /// The exercises with a text typed through the Klavaro tutor.
    pub fn is_typing(self) -> bool {
        !matches!(self, ExerciseMode::Shortcuts | ExerciseMode::Hanzi)
    }
}

/// How much the background shows, see [`crate::background`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BackgroundLevel {
    Off,
    #[default]
    Subtle,
    Lively,
}

impl BackgroundLevel {
    pub const ALL: [BackgroundLevel; 3] = [BackgroundLevel::Off, BackgroundLevel::Subtle, BackgroundLevel::Lively];

    pub fn name(self) -> &'static str {
        match self {
            BackgroundLevel::Off => "Off",
            BackgroundLevel::Subtle => "Subtle",
            BackgroundLevel::Lively => "Lively",
        }
    }

    /// The opacity of the lines, relative to the lively one.
    pub fn strength(self) -> f32 {
        match self {
            BackgroundLevel::Off => 0.0,
            BackgroundLevel::Subtle => 0.4,
            BackgroundLevel::Lively => 0.8,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyStats {
    /// When the key was last typed, seconds since the UNIX epoch, for the warm-up.
    pub last: u64,
    /// Typed right on the first try.
    pub hits: u64,
    /// Typed wrong when this key was expected.
    pub misses: u64,
    /// The sum of the touch times of the hits, in seconds.
    pub total_time: f64,
}

impl KeyStats {
    pub fn accuracy(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { 100.0 * self.hits as f64 / total as f64 }
    }

    pub fn average_time(&self) -> f64 {
        if self.hits == 0 { 0.0 } else { self.total_time / self.hits as f64 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// The seconds of typing, for the minutes of the day.
    #[serde(default)]
    pub elapsed: f64,
    pub mode: ExerciseMode,
    pub lesson: Option<usize>,
    pub accuracy: f64,
    pub velocity: f64,
    pub fluidness: f64,
    /// Seconds since the UNIX epoch.
    pub finished_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutProgress {
    pub basic_lesson: usize,
    /// Per character, the key `" "` is the space.
    pub keys: BTreeMap<String, KeyStats>,
    /// The most accurate and then fastest keys, updated on every save.
    pub best_keys: Vec<String>,
    /// The keys with the most misses, updated on every save.
    pub most_missed: Vec<String>,
    /// Klavaro's accumulators, they drive the adaptability exercises.
    pub klavaro: Accuracy,
    pub sessions: Vec<Session>,
    /// The finger of the finger drill, 1 is the left pinky, 5 the thumbs, 9 the right pinky.
    pub finger: u8,
    /// Per chord of the shortcuts exercise, like `Ctrl+C`, the times to press it.
    pub shortcuts: BTreeMap<String, KeyStats>,
    /// The fastest Alt+Tab round trip, in seconds.
    pub alt_tab_best: Option<f64>,
    /// The last basic lesson open with the lessons locked: the ones passed and the next one.
    pub basic_unlocked: usize,
    /// The lesson of the layer course, 1 based, and the last one open.
    pub layer_lesson: usize,
    pub layer_unlocked: usize,
    /// The auto shift missed: a capital held too long by mistake, a capital not held enough.
    pub auto_shift_long: u64,
    pub auto_shift_short: u64,
}

impl Default for LayoutProgress {
    fn default() -> Self {
        LayoutProgress {
            basic_lesson: 1,
            keys: BTreeMap::new(),
            best_keys: Vec::new(),
            most_missed: Vec::new(),
            klavaro: Accuracy::default(),
            sessions: Vec::new(),
            finger: 4,
            shortcuts: BTreeMap::new(),
            alt_tab_best: None,
            basic_unlocked: 1,
            layer_lesson: 1,
            layer_unlocked: 1,
            auto_shift_long: 0,
            auto_shift_short: 0,
        }
    }
}

impl LayoutProgress {
    pub fn key(&mut self, c: char) -> &mut KeyStats {
        self.keys.entry(c.to_string()).or_default()
    }

    pub fn update_rankings(&mut self) {
        let mut ranked: Vec<(&String, &KeyStats)> = self
            .keys
            .iter()
            .filter(|(_, s)| s.hits + s.misses >= MIN_TOUCHES_TO_RANK)
            .collect();
        ranked.sort_by(|a, b| {
            b.1.accuracy()
                .total_cmp(&a.1.accuracy())
                .then(a.1.average_time().total_cmp(&b.1.average_time()))
        });
        self.best_keys = ranked.iter().take(N_RANKED_KEYS).map(|(k, _)| (*k).clone()).collect();
        let mut missed: Vec<(&String, &KeyStats)> = self.keys.iter().filter(|(_, s)| s.misses > 0).collect();
        missed.sort_by(|a, b| b.1.misses.cmp(&a.1.misses).then(a.1.accuracy().total_cmp(&b.1.accuracy())));
        self.most_missed = missed.iter().take(N_RANKED_KEYS).map(|(k, _)| (*k).clone()).collect();
    }

    /// The accuracy of every finger with enough touches, from the per key statistics.
    pub fn finger_accuracy(&self, finger_of: impl Fn(char) -> Option<u8>) -> Vec<(u8, f64)> {
        let mut fingers: BTreeMap<u8, (u64, u64)> = BTreeMap::new();
        for (key, stats) in &self.keys {
            if let Some(finger) = key.chars().next().and_then(&finger_of) {
                let entry = fingers.entry(finger).or_default();
                entry.0 += stats.hits;
                entry.1 += stats.misses;
            }
        }
        fingers
            .into_iter()
            .filter(|(_, (hits, misses))| hits + misses >= MIN_TOUCHES_TO_RANK)
            .map(|(finger, (hits, misses))| (finger, 100.0 * hits as f64 / (hits + misses) as f64))
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
#[serde(default)]
pub struct Progress {
    pub theme: ThemeMode,
    pub layout: LayoutId,
    pub mode: ExerciseMode,
    /// The legends of the keys fade out while typing, they come back on errors.
    pub fade_legends: bool,
    /// Shows the layer and modifier keys to hold for the characters of the other layers.
    pub layer_trainer: bool,
    /// The help was shown once, on the first start.
    pub help_seen: bool,
    pub background: BackgroundLevel,
    /// No drifting background, no sparks, shakes nor breathing keys.
    pub reduce_motion: bool,
    pub show_hands: bool,
    /// A bigger text to type, for the tired or weaker eyes.
    pub large_text: bool,
    /// The window is scaled to fit its size, see [`crate::zoom`].
    pub fit_window: bool,
    /// The zoom over the fit to the window, or over 100% without it.
    pub zoom_factor: f32,
    /// The pinyin of the hanzi words is typed on the nine keys of a phone.
    pub pinyin_keypad: bool,
    /// The words of the hanzi mode are spoken.
    pub sound: bool,
    pub hanzi: HanziProgress,
    /// The `.keymap` file of ZMK imported, the layout "My ZMK keymap".
    pub custom_keymap: Option<String>,
    /// A lesson opens when the previous one is passed.
    pub lock_lessons: bool,
    /// The mode to go back to after the warm-up of a session.
    pub warmup_return: Option<ExerciseMode>,
    /// The minutes of practice of a day, for the streak.
    pub daily_goal_minutes: u32,
    pub layouts: BTreeMap<String, LayoutProgress>,
}

/// The progress of the hanzi mode, whatever the keyboard.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HanziProgress {
    /// The level of Claudralingo, 0 based, see [`crate::hanzi::data::Level`].
    pub level: usize,
    /// The word of the level being learned.
    pub word: usize,
    pub words: BTreeMap<String, HanziWordStats>,
    /// The levels done, with their misses.
    pub levels_done: BTreeMap<usize, u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HanziWordStats {
    pub practiced: u32,
    /// The strokes written right and the misses.
    pub strokes: u32,
    pub misses: u32,
    pub pinyin_misses: u32,
}

impl Default for Progress {
    fn default() -> Self {
        Progress {
            theme: ThemeMode::default(),
            layout: LayoutId::default(),
            mode: ExerciseMode::default(),
            fade_legends: true,
            layer_trainer: true,
            help_seen: false,
            background: BackgroundLevel::default(),
            reduce_motion: false,
            show_hands: true,
            large_text: false,
            fit_window: true,
            zoom_factor: 1.0,
            pinyin_keypad: false,
            sound: true,
            hanzi: HanziProgress::default(),
            custom_keymap: None,
            lock_lessons: true,
            warmup_return: None,
            daily_goal_minutes: 10,
            layouts: BTreeMap::new(),
        }
    }
}

impl Progress {
    pub fn current(&mut self) -> &mut LayoutProgress {
        self.layouts.entry(self.layout.key().to_string()).or_default()
    }

    pub fn load() -> Self {
        let mut progress: Progress = storage::read()
            .and_then(|json| match serde_json::from_str(&json) {
                Ok(progress) => Some(progress),
                Err(err) => {
                    warn!("Ignoring the unreadable progress file: {err}");
                    None
                }
            })
            .unwrap_or_default();
        for layout in progress.layouts.values_mut() {
            layout.klavaro.sanitize();
            // The lessons reached before the locks are open.
            layout.basic_unlocked = layout.basic_unlocked.max(layout.basic_lesson);
            layout.layer_unlocked = layout.layer_unlocked.max(layout.layer_lesson);
        }
        crate::layout::set_custom_keymap(progress.custom_keymap.clone());
        progress
    }

    pub fn save(&mut self) {
        for layout in self.layouts.values_mut() {
            layout.update_rankings();
            layout.klavaro.sort();
            layout.klavaro.prune();
            let excess = layout.sessions.len().saturating_sub(MAX_SESSIONS);
            layout.sessions.drain(..excess);
        }
        // Compact in the browser storage, readable in the file.
        #[cfg(target_arch = "wasm32")]
        let json = serde_json::to_string(self);
        #[cfg(not(target_arch = "wasm32"))]
        let json = serde_json::to_string_pretty(self);
        match json {
            Ok(json) => storage::write(&json),
            Err(err) => error!("Unable to serialize the progress: {err}"),
        }
    }
}

pub fn now_unix_seconds() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default()
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod storage {
    use std::path::PathBuf;

    use bevy::prelude::*;

    pub fn config_dir() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
        Some(base.join("claudralingo"))
    }

    pub fn read() -> Option<String> {
        std::fs::read_to_string(config_dir()?.join("progress.json")).ok()
    }

    pub fn write(json: &str) {
        let Some(dir) = config_dir() else {
            warn!("No HOME, the progress is not saved");
            return;
        };
        let path = dir.join("progress.json");
        // Write to a temporary file first, the progress survives a crash while saving.
        let tmp = dir.join("progress.json.tmp");
        let result = std::fs::create_dir_all(&dir)
            .and_then(|_| std::fs::write(&tmp, json))
            .and_then(|_| std::fs::rename(&tmp, &path));
        if let Err(err) = result {
            error!("Unable to save the progress to {}: {err}", path.display());
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod storage {
    const KEY: &str = "claudralingo.progress";

    fn local_storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    pub fn read() -> Option<String> {
        local_storage()?.get_item(KEY).ok().flatten()
    }

    pub fn write(json: &str) {
        if let Some(storage) = local_storage() {
            let _ = storage.set_item(KEY, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rankings() {
        let mut p = LayoutProgress::default();
        *p.key('a') = KeyStats { hits: 30, misses: 0, total_time: 6.0, last: 0 };
        *p.key('s') = KeyStats { hits: 30, misses: 0, total_time: 3.0, last: 0 };
        *p.key('d') = KeyStats { hits: 20, misses: 10, total_time: 3.0, last: 0 };
        *p.key('f') = KeyStats { hits: 1, misses: 1, total_time: 3.0, last: 0 };
        p.update_rankings();
        assert_eq!(p.best_keys, vec!["s", "a", "d"]);
        assert_eq!(p.most_missed, vec!["d", "f"]);
    }

    #[test]
    fn round_trip() {
        let mut progress = Progress::default();
        progress.current().key('x').misses = 3;
        let json = serde_json::to_string(&progress).unwrap();
        let mut back: Progress = serde_json::from_str(&json).unwrap();
        assert_eq!(back.current().key('x').misses, 3);
        assert_eq!(back.theme, ThemeMode::Dark);
        assert!(back.fade_legends && back.layer_trainer);
        // The files saved before the settings existed.
        let old: Progress = serde_json::from_str(r#"{"theme":"Light","layouts":{}}"#).unwrap();
        assert!(old.fade_legends);
        assert_eq!(old.theme, ThemeMode::Light);
    }
}
