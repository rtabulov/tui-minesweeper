//! Persistent state: theme choice, last difficulty, and score history,
//! stored as JSON in `~/.tui-minesweeper.json`.

use std::path::PathBuf;

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

fn state_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".tui-minesweeper.json")
}

pub fn load() -> PersistentState {
    std::fs::read_to_string(state_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(state: &PersistentState) {
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(state_path(), json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
