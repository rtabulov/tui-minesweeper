//! Persistent state: theme choice, last difficulty, and score history,
//! stored as JSON in `~/.tui-minesweeper.json`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::app::Difficulty;

#[derive(Clone, Copy, Serialize, Deserialize, Default)]
pub struct StatItem {
    pub wins: u32,
    pub total: u32,
    pub total_duration: u64,
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
        item.total_duration += seconds;
        if won {
            item.wins += 1;
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
