//! Persistent state: theme choice, last Difficulty, and score history.
//!
//! Stored as JSON under the platform config dir (`…/tui-minesweeper/state.json`).
//! Legacy `~/.tui-minesweeper.json` is migrated once on load.

use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::app::Difficulty;

#[derive(Clone, Copy, Serialize, Deserialize, Default)]
pub struct StatItem {
    pub wins: u32,
    pub total: u32,
    #[serde(default)]
    pub win_duration: u64,
    pub best_time: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Stats {
    pub beginner: StatItem,
    pub intermediate: StatItem,
    pub expert: StatItem,
    pub custom: StatItem,
}

impl Stats {
    pub fn item(&self, d: Difficulty) -> &StatItem {
        match d {
            Difficulty::Beginner => &self.beginner,
            Difficulty::Intermediate => &self.intermediate,
            Difficulty::Expert => &self.expert,
            Difficulty::Custom { .. } => &self.custom,
        }
    }

    fn item_mut(&mut self, d: Difficulty) -> &mut StatItem {
        match d {
            Difficulty::Beginner => &mut self.beginner,
            Difficulty::Intermediate => &mut self.intermediate,
            Difficulty::Expert => &mut self.expert,
            Difficulty::Custom { .. } => &mut self.custom,
        }
    }

    /// Record the outcome of a finished game.
    pub fn record(&mut self, d: Difficulty, won: bool, seconds: u64) {
        let item = self.item_mut(d);
        item.total += 1;
        if won {
            item.wins += 1;
            item.win_duration += seconds;
            item.best_time = Some(item.best_time.map_or(seconds, |b| b.min(seconds)));
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PersistentState {
    pub theme: usize,
    pub difficulty: Difficulty,
    pub stats: Stats,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            theme: 0,
            difficulty: Difficulty::Intermediate,
            stats: Stats::default(),
        }
    }
}

struct StatePaths {
    current: PathBuf,
    legacy: PathBuf,
}

fn state_paths() -> StatePaths {
    StatePaths {
        current: current_state_path(),
        legacy: legacy_state_path(),
    }
}

fn current_state_path() -> PathBuf {
    if let Ok(dir) = std::env::var("TUI_MINESWEEPER_CONFIG_DIR") {
        return PathBuf::from(dir).join("state.json");
    }
    ProjectDirs::from("", "", "tui-minesweeper")
        .map(|p| p.config_dir().join("state.json"))
        .unwrap_or_else(|| PathBuf::from("tui-minesweeper-state.json"))
}

fn legacy_state_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".tui-minesweeper.json")
}

fn read_file(path: &Path) -> Option<PersistentState> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

fn write_file(path: &Path, state: &PersistentState) -> bool {
    let Ok(json) = serde_json::to_string_pretty(state) else {
        return false;
    };
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, json).is_ok()
}

fn load_at(paths: &StatePaths) -> PersistentState {
    if let Some(state) = read_file(&paths.current) {
        return state;
    }
    if let Some(state) = read_file(&paths.legacy) {
        if write_file(&paths.current, &state) {
            let _ = std::fs::remove_file(&paths.legacy);
        }
        return state;
    }
    PersistentState::default()
}

pub fn load() -> PersistentState {
    load_at(&state_paths())
}

pub fn save(state: &PersistentState) {
    let _ = write_file(&current_state_path(), state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("tui-ms-{label}-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn record_loss_counts_game_but_not_win_duration() {
        let mut stats = Stats::default();
        stats.record(Difficulty::Expert, false, 120);
        let item = stats.item(Difficulty::Expert);
        assert_eq!(item.total, 1);
        assert_eq!(item.wins, 0);
        assert_eq!(item.win_duration, 0);
        assert!(item.best_time.is_none());
    }

    #[test]
    fn record_win_accumulates_win_duration_and_best_time() {
        let mut stats = Stats::default();
        stats.record(Difficulty::Expert, true, 120);
        stats.record(Difficulty::Expert, true, 90);
        let item = stats.item(Difficulty::Expert);
        assert_eq!(item.total, 2);
        assert_eq!(item.wins, 2);
        assert_eq!(item.win_duration, 210);
        assert_eq!(item.best_time, Some(90));
    }

    #[test]
    fn record_mixed_outcomes_only_accumulates_win_durations() {
        let mut stats = Stats::default();
        stats.record(Difficulty::Expert, true, 100);
        stats.record(Difficulty::Expert, false, 999);
        stats.record(Difficulty::Expert, true, 50);
        let item = stats.item(Difficulty::Expert);
        assert_eq!(item.total, 3);
        assert_eq!(item.wins, 2);
        assert_eq!(item.win_duration, 150);
    }

    #[test]
    fn deserializes_legacy_total_duration_without_win_duration() {
        let json = r#"{"wins":2,"total":5,"total_duration":999,"best_time":60}"#;
        let item: StatItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.wins, 2);
        assert_eq!(item.total, 5);
        assert_eq!(item.win_duration, 0);
        assert_eq!(item.best_time, Some(60));
    }

    #[test]
    fn load_migrates_legacy_dotfile_into_config_dir() {
        let root = temp_dir("migrate");
        let config = root.join("config");
        let legacy = root.join(".tui-minesweeper.json");
        let current = config.join("state.json");

        let mut legacy_state = PersistentState::default();
        legacy_state.theme = 3;
        legacy_state.stats.record(Difficulty::Beginner, true, 42);
        write_file(&legacy, &legacy_state);
        assert!(legacy.exists());
        assert!(!current.exists());

        let loaded = load_at(&StatePaths {
            current: current.clone(),
            legacy: legacy.clone(),
        });
        assert_eq!(loaded.theme, 3);
        assert_eq!(loaded.stats.beginner.wins, 1);
        assert_eq!(loaded.stats.beginner.best_time, Some(42));
        assert!(current.exists(), "should write new config path");
        assert!(!legacy.exists(), "should remove legacy file after migrate");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn load_prefers_current_over_legacy() {
        let root = temp_dir("prefer");
        let current = root.join("state.json");
        let legacy = root.join(".tui-minesweeper.json");

        let mut new_state = PersistentState::default();
        new_state.theme = 1;
        write_file(&current, &new_state);

        let mut old_state = PersistentState::default();
        old_state.theme = 9;
        write_file(&legacy, &old_state);

        let loaded = load_at(&StatePaths {
            current: current.clone(),
            legacy: legacy.clone(),
        });
        assert_eq!(loaded.theme, 1);
        assert!(legacy.exists(), "legacy untouched when current exists");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn save_creates_parent_directories() {
        let root = temp_dir("save-dirs");
        let path = root.join("nested").join("tui-minesweeper").join("state.json");
        let mut state = PersistentState::default();
        state.theme = 2;
        assert!(write_file(&path, &state));
        assert!(path.exists());
        let loaded = read_file(&path).unwrap();
        assert_eq!(loaded.theme, 2);
        let _ = std::fs::remove_dir_all(root);
    }
}
