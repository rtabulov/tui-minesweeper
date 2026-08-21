//! Core minesweeper game logic, independent of any UI.
//!
//! The board is stored as flat `Vec`s indexed by `y * width + x`. Mine
//! placement is deferred until the first reveal so that the first click is
//! always safe (the clicked cell and its neighbours are guaranteed mine-free,
//! which also produces the classic flood-fill opening). Layouts are
//! reject-sampled until No-guess search accepts one or the budget falls back.

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

/// What a cell contains once the board is laid out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cell {
    /// Empty cell; the `u8` is the count of adjacent mines (0..=8).
    Empty(u8),
    Mine,
}

/// The player-visible state of a cell.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CellState {
    Hidden,
    Revealed,
    Flagged,
}

/// Overall game progression.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    /// Before the first reveal: mines not yet placed.
    Ready,
    Playing,
    Won,
    Lost,
}

/// A grid coordinate.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

pub struct Game {
    pub width: usize,
    pub height: usize,
    pub mine_count: usize,
    pub seed: u64,
    cells: Vec<Cell>,
    states: Vec<CellState>,
    pub status: Status,
    revealed_count: usize,
    /// The cell that detonated, set when the game is lost.
    pub lost_at: Option<Pos>,
    rng: StdRng,
    /// Max layouts to try in No-guess search before accepting a Fallback board.
    search_budget: u32,
    /// True when the accepted layout may contain Forced guesses.
    pub is_fallback: bool,
}

impl Game {
    /// Create a new, un-mined board. Mines are laid out on first reveal.
    ///
    /// `mine_count` is clamped so the board always has at least one free cell.
    pub fn new(width: usize, height: usize, mine_count: usize, seed: u64) -> Self {
        assert!(width > 0 && height > 0, "board must be non-empty");
        let cells = width * height;
        let mine_count = mine_count.min(cells - 1);

        Self {
            width,
            height,
            mine_count,
            seed,
            cells: vec![Cell::Empty(0); cells],
            states: vec![CellState::Hidden; cells],
            status: Status::Ready,
            revealed_count: 0,
            lost_at: None,
            rng: StdRng::seed_from_u64(seed),
            // Snappy default; App overrides per Difficulty.
            search_budget: 64,
            is_fallback: false,
        }
    }

    /// Cap how many candidate layouts No-guess search may try.
    pub fn with_search_budget(mut self, search_budget: u32) -> Self {
        self.search_budget = search_budget.max(1);
        self
    }

    /// Whether the current revealed position can be finished with only Deductions.
    #[cfg(test)]
    pub fn is_no_guess_from_here(&self) -> bool {
        let is_mine = self.mine_mask();
        let revealed: Vec<bool> = self
            .states
            .iter()
            .map(|s| matches!(s, CellState::Revealed))
            .collect();
        crate::solver::is_no_guess(self.width, self.height, self.mine_count, &is_mine, &revealed)
    }

    fn mine_mask(&self) -> Vec<bool> {
        self.cells.iter().map(|c| matches!(c, Cell::Mine)).collect()
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn cell(&self, x: usize, y: usize) -> Cell {
        self.cells[self.idx(x, y)]
    }

    pub fn state(&self, x: usize, y: usize) -> CellState {
        self.states[self.idx(x, y)]
    }

    fn in_bounds(&self, x: isize, y: isize) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height
    }

    /// All in-bounds orthogonal and diagonal neighbours of a cell.
    pub fn neighbors(&self, x: usize, y: usize) -> Vec<Pos> {
        let mut out = Vec::with_capacity(8);
        for dy in -1isize..=1 {
            for dx in -1isize..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if self.in_bounds(nx, ny) {
                    out.push(Pos {
                        x: nx as usize,
                        y: ny as usize,
                    });
                }
            }
        }
        out
    }

    /// Lay out mines via No-guess search (reject sampling from the Seed stream).
    fn place_mines(&mut self, ex: usize, ey: usize) {
        let budget = self.search_budget.max(1);
        for _ in 0..budget {
            self.clear_cells();
            self.place_mines_once(ex, ey);
            if self.layout_is_no_guess_after_opening(ex, ey) {
                self.is_fallback = false;
                return;
            }
        }
        self.is_fallback = true;
    }

    fn clear_cells(&mut self) {
        for c in &mut self.cells {
            *c = Cell::Empty(0);
        }
    }

    /// Place one candidate layout, excluding the opening safe zone.
    fn place_mines_once(&mut self, ex: usize, ey: usize) {
        let total = self.width * self.height;
        let mut excluded = vec![false; total];
        excluded[self.idx(ex, ey)] = true;
        for p in self.neighbors(ex, ey) {
            excluded[self.idx(p.x, p.y)] = true;
        }

        let safe = excluded.iter().filter(|&&e| e).count();
        if total - safe < self.mine_count {
            // Not enough room for the full 3x3 safe zone; only protect the click.
            excluded.fill(false);
            excluded[self.idx(ex, ey)] = true;
        }

        let mut placed = 0;
        while placed < self.mine_count {
            let i = self.rng.random_range(0..total);
            if excluded[i] || self.cells[i] == Cell::Mine {
                continue;
            }
            self.cells[i] = Cell::Mine;
            placed += 1;
        }

        // Compute adjacency counts for the non-mine cells.
        for y in 0..self.height {
            for x in 0..self.width {
                let i = self.idx(x, y);
                if self.cells[i] == Cell::Mine {
                    continue;
                }
                let count = self
                    .neighbors(x, y)
                    .iter()
                    .filter(|p| self.cells[self.idx(p.x, p.y)] == Cell::Mine)
                    .count();
                self.cells[i] = Cell::Empty(count as u8);
            }
        }
    }

    fn layout_is_no_guess_after_opening(&self, ox: usize, oy: usize) -> bool {
        let is_mine = self.mine_mask();
        let mut revealed = vec![false; self.width * self.height];
        self.simulate_opening(ox, oy, &mut revealed);
        crate::solver::is_no_guess(
            self.width,
            self.height,
            self.mine_count,
            &is_mine,
            &revealed,
        )
    }

    fn simulate_opening(&self, x: usize, y: usize, revealed: &mut [bool]) {
        let i = self.idx(x, y);
        if revealed[i] {
            return;
        }
        if self.cells[i] == Cell::Mine {
            return;
        }
        revealed[i] = true;
        if let Cell::Empty(0) = self.cells[i] {
            for p in self.neighbors(x, y) {
                self.simulate_opening(p.x, p.y, revealed);
            }
        }
    }

    /// Reveal a cell. First reveal lays out mines; revealing a mine loses;
    /// revealing every non-mine cell wins.
    pub fn reveal(&mut self, x: usize, y: usize) {
        if self.status == Status::Won || self.status == Status::Lost {
            return;
        }
        let i = self.idx(x, y);
        if self.states[i] != CellState::Hidden {
            return;
        }

        if self.status == Status::Ready {
            self.place_mines(x, y);
            self.status = Status::Playing;
        }

        self.reveal_from(x, y);

        if self.cells[i] == Cell::Mine {
            self.status = Status::Lost;
            self.lost_at = Some(Pos { x, y });
            self.reveal_all_mines();
            return;
        }

        if self.revealed_count == self.width * self.height - self.mine_count {
            self.status = Status::Won;
            // Flag any remaining mines for the classic "solved" look.
            for j in 0..self.cells.len() {
                if self.cells[j] == Cell::Mine && self.states[j] == CellState::Hidden {
                    self.states[j] = CellState::Flagged;
                }
            }
        }
    }

    /// Recursively reveal, flood-filling through zero-adjacency cells.
    fn reveal_from(&mut self, x: usize, y: usize) {
        let i = self.idx(x, y);
        if self.states[i] != CellState::Hidden {
            return;
        }
        self.states[i] = CellState::Revealed;
        self.revealed_count += 1;

        if let Cell::Empty(0) = self.cells[i] {
            for p in self.neighbors(x, y) {
                self.reveal_from(p.x, p.y);
            }
        }
    }

    /// Chord: when a revealed number has exactly that many adjacent flags,
    /// reveal every remaining hidden neighbour.
    pub fn chord(&mut self, x: usize, y: usize) {
        if self.status != Status::Playing {
            return;
        }
        let i = self.idx(x, y);
        if self.states[i] != CellState::Revealed {
            return;
        }
        let Cell::Empty(n) = self.cells[i] else {
            return;
        };
        if n == 0 {
            return;
        }
        let neighbors = self.neighbors(x, y);
        let flags = neighbors
            .iter()
            .filter(|p| self.states[self.idx(p.x, p.y)] == CellState::Flagged)
            .count();
        if flags == n as usize {
            for p in neighbors {
                if self.states[self.idx(p.x, p.y)] == CellState::Hidden {
                    self.reveal(p.x, p.y);
                }
            }
        }
    }

    /// Toggle a flag on a hidden cell (hidden <-> flagged).
    pub fn toggle_flag(&mut self, x: usize, y: usize) {
        if self.status == Status::Won || self.status == Status::Lost {
            return;
        }
        let i = self.idx(x, y);
        self.states[i] = match self.states[i] {
            CellState::Hidden => CellState::Flagged,
            CellState::Flagged => CellState::Hidden,
            CellState::Revealed => CellState::Revealed,
        };
    }

    pub fn flag_count(&self) -> usize {
        self.states
            .iter()
            .filter(|&&s| s == CellState::Flagged)
            .count()
    }

    /// Mines not yet flagged; negative when over-flagged.
    pub fn mines_remaining(&self) -> isize {
        self.mine_count as isize - self.flag_count() as isize
    }

    /// On loss, reveal every unflagged mine so the player sees the board.
    fn reveal_all_mines(&mut self) {
        for i in 0..self.cells.len() {
            if self.cells[i] == Cell::Mine && self.states[i] != CellState::Flagged {
                self.states[i] = CellState::Revealed;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_click_is_always_safe_and_flood_fills() {
        let mut g = Game::new(9, 9, 10, 42);
        g.reveal(4, 4);
        assert_ne!(g.status, Status::Lost);
        // The clicked cell is revealed and not a mine.
        assert_eq!(g.state(4, 4), CellState::Revealed);
        assert_ne!(g.cell(4, 4), Cell::Mine);
        // The centre opening must have flood-filled beyond the single cell.
        assert!(g.revealed_count > 1, "expected a flood-fill opening");
    }

    #[test]
    fn mine_count_is_respected() {
        let mut g = Game::new(9, 9, 10, 7);
        g.reveal(0, 0);
        let mines = (0..g.width * g.height)
            .filter(|&i| g.cells[i] == Cell::Mine)
            .count();
        assert_eq!(mines, 10);
    }

    #[test]
    fn flag_toggles_between_hidden_and_flagged() {
        let mut g = Game::new(5, 5, 3, 1);
        assert_eq!(g.state(0, 0), CellState::Hidden);
        g.toggle_flag(0, 0);
        assert_eq!(g.state(0, 0), CellState::Flagged);
        g.toggle_flag(0, 0);
        assert_eq!(g.state(0, 0), CellState::Hidden);
        assert_eq!(g.flag_count(), 0);
    }

    #[test]
    fn mines_remaining_tracks_flags() {
        let mut g = Game::new(9, 9, 10, 3);
        assert_eq!(g.mines_remaining(), 10);
        g.toggle_flag(0, 0);
        assert_eq!(g.mines_remaining(), 9);
        g.toggle_flag(1, 0);
        g.toggle_flag(1, 0);
        assert_eq!(g.mines_remaining(), 9);
    }

    #[test]
    fn reveal_number_does_not_flood() {
        // Deterministic seed chosen for a board with a numbered first click.
        let mut g = Game::new(3, 3, 8, 12345);
        g.reveal(1, 1);
        assert_ne!(g.status, Status::Lost);
        assert_eq!(g.cell(1, 1), Cell::Empty(8));
        assert_eq!(g.revealed_count, 1);
    }

    #[test]
    fn winning_reveals_and_flags_all_mines() {
        // 2x2 with one mine: revealing every safe cell wins.
        let mut g = Game::new(2, 2, 1, 99);
        g.reveal(0, 0);
        for y in 0..2 {
            for x in 0..2 {
                if g.cell(x, y) != Cell::Mine {
                    g.reveal(x, y);
                }
            }
        }
        assert_eq!(g.status, Status::Won);
        assert_eq!(g.mines_remaining(), 0);
    }

    #[test]
    fn losing_sets_lost_at_and_reveals_mines() {
        let mut g = Game::new(9, 9, 10, 5);
        g.reveal(4, 4); // first click, always safe

        // Locate a still-hidden mine deterministically.
        let mut mine = None;
        'scan: for y in 0..9 {
            for x in 0..9 {
                if g.cell(x, y) == Cell::Mine && g.state(x, y) == CellState::Hidden {
                    mine = Some((x, y));
                    break 'scan;
                }
            }
        }
        let (mx, my) = mine.expect("board must contain a hidden mine");
        g.reveal(mx, my);

        assert_eq!(g.status, Status::Lost);
        assert_eq!(g.lost_at, Some(Pos { x: mx, y: my }));
        // Every mine must now be revealed or flagged.
        for i in 0..g.width * g.height {
            if g.cells[i] == Cell::Mine {
                assert!(matches!(
                    g.states[i],
                    CellState::Revealed | CellState::Flagged
                ));
            }
        }
    }

    #[test]
    fn mine_count_clamped_below_cell_count() {
        let g = Game::new(3, 3, 100, 0);
        assert_eq!(g.mine_count, 8);
    }

    #[test]
    fn generating_first_click_yields_no_guess_board_when_budget_allows() {
        let mut g = Game::new(9, 9, 10, 42).with_search_budget(200);
        g.reveal(4, 4);
        assert!(!g.is_fallback, "beginner board should find a no-guess layout");
        assert!(
            g.is_no_guess_from_here(),
            "accepted non-fallback board must be no-guess from the opening"
        );
    }

    #[test]
    fn intermediate_generating_click_finds_no_guess_in_tiered_budget() {
        let mut g = Game::new(16, 16, 40, 99).with_search_budget(512);
        g.reveal(8, 8);
        assert!(!g.is_fallback);
        assert!(g.is_no_guess_from_here());
    }

    #[test]
    fn same_seed_and_generating_click_replay_accepted_layout() {
        let mut a = Game::new(9, 9, 10, 12345).with_search_budget(100);
        let mut b = Game::new(9, 9, 10, 12345).with_search_budget(100);
        a.reveal(2, 3);
        b.reveal(2, 3);
        assert_eq!(a.is_fallback, b.is_fallback);
        for y in 0..9 {
            for x in 0..9 {
                assert_eq!(a.cell(x, y), b.cell(x, y));
            }
        }
    }


    #[test]
    fn expert_generating_click_completes_within_budget() {
        let mut g = Game::new(30, 16, 99, 42).with_search_budget(256);
        g.reveal(15, 8);
        assert_ne!(g.status, Status::Lost);
        if !g.is_fallback {
            assert!(g.is_no_guess_from_here());
        }
    }

    #[test]
    fn exhausted_search_budget_marks_fallback_board() {
        // Dense custom: first candidate is rarely no-guess; budget 1 forces Fallback often.
        let mut found = false;
        for seed in 0..200 {
            let mut g = Game::new(5, 5, 12, seed).with_search_budget(1);
            g.reveal(2, 2);
            if g.is_fallback {
                assert!(
                    !g.is_no_guess_from_here(),
                    "fallback board must still require a Forced guess"
                );
                found = true;
                break;
            }
        }
        assert!(found, "expected at least one Fallback board in seed scan");
    }
}
