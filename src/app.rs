//! Application state and input handling: screens (menu / game / stats),
//! difficulty selection, timer, theme, score history, and modal overlays.

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use serde::{Deserialize, Serialize};

use crate::game::{CellState, Game, Pos, Status};
use crate::state::{PersistentState, Stats};
use crate::theme::{theme, Theme, THEME_COUNT};

/// Rendered size of one board cell in terminal cells.
pub const CELL_W: u16 = 2;
pub const CELL_H: u16 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Expert,
    Custom { width: usize, height: usize, mines: usize },
}

impl Difficulty {
    pub fn dims(self) -> (usize, usize, usize) {
        match self {
            Difficulty::Beginner => (9, 9, 10),
            Difficulty::Intermediate => (16, 16, 40),
            Difficulty::Expert => (30, 16, 99),
            Difficulty::Custom {
                width,
                height,
                mines,
            } => (width, height, mines),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Beginner => "Beginner",
            Difficulty::Intermediate => "Intermediate",
            Difficulty::Expert => "Expert",
            Difficulty::Custom { .. } => "Custom",
        }
    }

    pub fn describe(self) -> String {
        let (w, h, m) = self.dims();
        format!("{}×{} · {} mines", w, h, m)
    }

    pub fn index(self) -> usize {
        match self {
            Difficulty::Beginner => 0,
            Difficulty::Intermediate => 1,
            Difficulty::Expert => 2,
            Difficulty::Custom { .. } => 3,
        }
    }
}

pub const DIFFICULTIES: [Difficulty; 4] = [
    Difficulty::Beginner,
    Difficulty::Intermediate,
    Difficulty::Expert,
    Difficulty::Custom {
        width: 16,
        height: 16,
        mines: 40,
    },
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Menu,
    Game,
    Stats,
}

#[derive(Clone, Copy)]
enum Origin {
    Menu,
    Game,
}

/// Modal layered over a screen.
pub enum Overlay {
    None,
    Help,
    Win,
    CustomInput { buf: String, error: Option<String> },
}

pub struct App {
    pub screen: Screen,
    stats_origin: Origin,
    pub game: Game,
    pub difficulty: Difficulty,
    pub cursor: Pos,
    started_at: Option<Instant>,
    elapsed: Duration,
    pub overlay: Overlay,
    pub theme_index: usize,
    pub stats: Stats,
    pub menu_selected: usize,
    pub should_quit: bool,
    /// Top-left column/row of the board content, set during render so mouse
    /// clicks can be mapped back to cells.
    pub board_origin: (u16, u16),
}

impl App {
    pub fn new(state: PersistentState) -> Self {
        let (w, h, m) = state.difficulty.dims();
        let mut app = Self {
            screen: Screen::Menu,
            stats_origin: Origin::Menu,
            game: Game::new(w, h, m, rand::random()),
            difficulty: state.difficulty,
            cursor: Pos { x: w / 2, y: h / 2 },
            started_at: None,
            elapsed: Duration::ZERO,
            overlay: Overlay::None,
            theme_index: state.theme % THEME_COUNT,
            stats: state.stats,
            menu_selected: state.difficulty.index(),
            should_quit: false,
            board_origin: (0, 0),
        };
        app.clamp_cursor();
        app
    }

    pub fn theme(&self) -> &'static Theme {
        theme(self.theme_index)
    }

    fn clamp_cursor(&mut self) {
        if self.cursor.x >= self.game.width {
            self.cursor.x = self.game.width - 1;
        }
        if self.cursor.y >= self.game.height {
            self.cursor.y = self.game.height - 1;
        }
    }

    /// Begin a fresh game with the given difficulty and a new seed, then enter
    /// the game screen.
    pub fn start_game(&mut self, difficulty: Difficulty, seed: u64) {
        self.difficulty = difficulty;
        self.new_game(difficulty, seed);
        self.screen = Screen::Game;
        self.save_state();
    }

    pub fn new_game(&mut self, difficulty: Difficulty, seed: u64) {
        self.difficulty = difficulty;
        let (w, h, m) = difficulty.dims();
        self.game = Game::new(w, h, m, seed);
        self.cursor = Pos { x: w / 2, y: h / 2 };
        self.clamp_cursor();
        self.started_at = None;
        self.elapsed = Duration::ZERO;
        self.overlay = Overlay::None;
    }

    pub fn elapsed_time(&self) -> Duration {
        match self.started_at {
            Some(t) => t.elapsed(),
            None => self.elapsed,
        }
    }

    fn cycle_theme(&mut self, delta: isize) {
        let n = THEME_COUNT as isize;
        self.theme_index = ((self.theme_index as isize + delta).rem_euclid(n)) as usize;
        self.save_state();
    }

    fn save_state(&self) {
        crate::state::save(&PersistentState {
            theme: self.theme_index,
            difficulty: self.difficulty,
            stats: self.stats.clone(),
        });
    }

    fn after_move(&mut self, prev: Status) {
        let now = self.game.status;
        if prev == Status::Ready && now == Status::Playing {
            self.started_at = Some(Instant::now());
        }
        if now != prev && (now == Status::Won || now == Status::Lost) {
            if let Some(t) = self.started_at.take() {
                self.elapsed = t.elapsed();
            }
            let won = now == Status::Won;
            self.stats.record(self.difficulty, won, self.elapsed.as_secs());
            if won {
                self.overlay = Overlay::Win;
            }
            self.save_state();
        }
    }

    pub fn reveal(&mut self, x: usize, y: usize) {
        let prev = self.game.status;
        self.game.reveal(x, y);
        self.after_move(prev);
    }

    /// Reveal-or-chord: reveal a hidden cell, or chord a revealed number.
    pub fn reveal_or_chord(&mut self, x: usize, y: usize) {
        let revealed = self.game.state(x, y) == CellState::Revealed;
        if revealed {
            let prev = self.game.status;
            self.game.chord(x, y);
            self.after_move(prev);
        } else {
            self.reveal(x, y);
        }
    }

    pub fn toggle_flag(&mut self, x: usize, y: usize) {
        self.game.toggle_flag(x, y);
    }

    pub fn parse_custom(&self, input: &str) -> Result<(usize, usize, usize), String> {
        let nums: Vec<usize> = input
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<usize>())
            .collect::<Result<_, _>>()
            .map_err(|_| "numbers only, e.g. 20 20 60".to_string())?;

        if nums.len() != 3 {
            return Err("enter three numbers: width height mines".to_string());
        }
        let (w, h, m) = (nums[0], nums[1], nums[2]);
        if w == 0 || h == 0 {
            return Err("width and height must be > 0".to_string());
        }
        if m >= w * h {
            return Err("too many mines for that board".to_string());
        }
        Ok((w, h, m))
    }

    pub fn apply_custom(&mut self, input: &str, seed: u64) -> Result<(), String> {
        let (w, h, m) = self.parse_custom(input)?;
        self.start_game(
            Difficulty::Custom {
                width: w,
                height: h,
                mines: m,
            },
            seed,
        );
        Ok(())
    }

    fn move_cursor(&mut self, dx: isize, dy: isize) {
        let nx = self.cursor.x as isize + dx;
        let ny = self.cursor.y as isize + dy;
        if (0..self.game.width as isize).contains(&nx) {
            self.cursor.x = nx as usize;
        }
        if (0..self.game.height as isize).contains(&ny) {
            self.cursor.y = ny as usize;
        }
    }

    pub fn open_stats(&mut self) {
        self.stats_origin = match self.screen {
            Screen::Game => Origin::Game,
            _ => Origin::Menu,
        };
        self.screen = Screen::Stats;
    }

    fn close_stats(&mut self) {
        self.screen = match self.stats_origin {
            Origin::Game => Screen::Game,
            Origin::Menu => Screen::Menu,
        };
    }

    /// Handle a keyboard event, routing through any active overlay first.
    pub fn on_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        if modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char('c' | 'q') = code {
                self.should_quit = true;
            }
            return;
        }

        if !matches!(self.overlay, Overlay::None) {
            self.on_key_overlay(code);
            return;
        }

        match self.screen {
            Screen::Menu => self.on_key_menu(code),
            Screen::Game => self.on_key_game(code),
            Screen::Stats => self.on_key_stats(code),
        }
    }

    fn on_key_menu(&mut self, code: KeyCode) {
        use KeyCode::*;
        match code {
            Char('q') => self.should_quit = true,
            Up | Char('k') => self.menu_selected = self.menu_selected.saturating_sub(1),
            Down | Char('j') => self.menu_selected = (self.menu_selected + 1).min(3),
            Char('1') => self.menu_selected = 0,
            Char('2') => self.menu_selected = 1,
            Char('3') => self.menu_selected = 2,
            Char('4') => self.menu_selected = 3,
            Char('<') => self.cycle_theme(-1),
            Char('>') => self.cycle_theme(1),
            Char('s') => self.open_stats(),
            Enter | Char(' ') => {
                let d = DIFFICULTIES[self.menu_selected];
                if let Difficulty::Custom { .. } = d {
                    self.overlay = Overlay::CustomInput {
                        buf: String::new(),
                        error: None,
                    };
                } else {
                    self.start_game(d, rand::random());
                }
            }
            _ => {}
        }
    }

    fn on_key_game(&mut self, code: KeyCode) {
        use KeyCode::*;
        match code {
            Char('q') => self.should_quit = true,
            Left | Char('h') => self.move_cursor(-1, 0),
            Right | Char('l') => self.move_cursor(1, 0),
            Up | Char('k') => self.move_cursor(0, -1),
            Down | Char('j') => self.move_cursor(0, 1),
            Home => self.cursor.x = 0,
            End => self.cursor.x = self.game.width - 1,
            Char(' ') | Enter | Char('x') | Char('o') => {
                let (x, y) = (self.cursor.x, self.cursor.y);
                self.reveal_or_chord(x, y);
            }
            Char('f') => {
                let (x, y) = (self.cursor.x, self.cursor.y);
                self.toggle_flag(x, y);
            }
            Char('r') => {
                let d = self.difficulty;
                self.new_game(d, rand::random());
            }
            Char('n') => self.screen = Screen::Menu,
            Char('s') => self.open_stats(),
            Char('<') => self.cycle_theme(-1),
            Char('>') => self.cycle_theme(1),
            Char('?') => self.overlay = Overlay::Help,
            _ => {}
        }
    }

    fn on_key_stats(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q' | 's') => self.close_stats(),
            _ => {}
        }
    }

    fn on_key_overlay(&mut self, code: KeyCode) {
        let overlay = std::mem::replace(&mut self.overlay, Overlay::None);
        self.overlay = match overlay {
            Overlay::Help => {
                if matches!(code, KeyCode::Esc | KeyCode::Char('q' | '?')) {
                    Overlay::None
                } else {
                    Overlay::Help
                }
            }
            Overlay::Win => match code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => Overlay::None,
                KeyCode::Char('r') => {
                    let d = self.difficulty;
                    self.new_game(d, rand::random());
                    Overlay::None
                }
                KeyCode::Char('n') => {
                    self.screen = Screen::Menu;
                    Overlay::None
                }
                KeyCode::Char('q') => {
                    self.should_quit = true;
                    Overlay::None
                }
                _ => Overlay::Win,
            },
            Overlay::CustomInput { mut buf, error } => match code {
                KeyCode::Esc | KeyCode::Char('q') => Overlay::None,
                KeyCode::Enter => match self.apply_custom(&buf, rand::random()) {
                    Ok(()) => return,
                    Err(e) => Overlay::CustomInput {
                        buf,
                        error: Some(e),
                    },
                },
                KeyCode::Backspace => {
                    buf.pop();
                    Overlay::CustomInput { buf, error }
                }
                KeyCode::Char(c) => {
                    buf.push(c);
                    Overlay::CustomInput { buf, error }
                }
                _ => Overlay::CustomInput { buf, error },
            },
            Overlay::None => Overlay::None,
        };
    }

    /// Map a mouse click to a board action.
    pub fn on_mouse(&mut self, kind: MouseEventKind, column: u16, row: u16) {
        if self.screen != Screen::Game || !matches!(self.overlay, Overlay::None) {
            return;
        }
        let MouseEventKind::Down(button) = kind else {
            return;
        };
        let (ox, oy) = self.board_origin;
        let cx = column as isize - ox as isize;
        let cy = row as isize - oy as isize;
        if cx < 0 || cy < 0 {
            return;
        }
        let x = cx as usize / CELL_W as usize;
        let y = cy as usize / CELL_H as usize;
        if x >= self.game.width || y >= self.game.height {
            return;
        }
        match button {
            MouseButton::Left => self.reveal_or_chord(x, y),
            MouseButton::Right => self.toggle_flag(x, y),
            MouseButton::Middle => {
                let prev = self.game.status;
                self.game.chord(x, y);
                self.after_move(prev);
            }
        }
    }
}
