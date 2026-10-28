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

/// The keys with less touches are not ranked.
const MIN_TOUCHES_TO_RANK: u64 = 20;
const N_RANKED_KEYS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ExerciseMode {
    #[default]
    Basic,
    Adaptability,
    Velocity,
    Fluidness,
}

impl ExerciseMode {
    pub const ALL: [ExerciseMode; 4] = [
        ExerciseMode::Basic,
        ExerciseMode::Adaptability,
        ExerciseMode::Velocity,
        ExerciseMode::Fluidness,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ExerciseMode::Basic => "Basic course",
            ExerciseMode::Adaptability => "Adaptability",
            ExerciseMode::Velocity => "Velocity",
            ExerciseMode::Fluidness => "Fluidness",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeyStats {
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Resource)]
#[serde(default)]
pub struct Progress {
    pub theme: ThemeMode,
    pub layout: LayoutId,
    pub mode: ExerciseMode,
    pub layouts: BTreeMap<String, LayoutProgress>,
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
        }
        progress
    }

    pub fn save(&mut self) {
        for layout in self.layouts.values_mut() {
            layout.update_rankings();
            layout.klavaro.sort();
            layout.klavaro.prune();
        }
        match serde_json::to_string_pretty(self) {
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
        *p.key('a') = KeyStats { hits: 30, misses: 0, total_time: 6.0 };
        *p.key('s') = KeyStats { hits: 30, misses: 0, total_time: 3.0 };
        *p.key('d') = KeyStats { hits: 20, misses: 10, total_time: 3.0 };
        *p.key('f') = KeyStats { hits: 1, misses: 1, total_time: 3.0 };
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
    }
}
