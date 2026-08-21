//! Full-information no-guess acceptance for mine layouts (generation only).

/// Whether the board can be finished from the current revealed cells using only
/// Deductions (every consistent completion agrees) — never a Forced guess.
pub fn is_no_guess(
    width: usize,
    height: usize,
    mine_count: usize,
    is_mine: &[bool],
    revealed: &[bool],
) -> bool {
    let n = width * height;
    debug_assert_eq!(is_mine.len(), n);
    debug_assert_eq!(revealed.len(), n);
    debug_assert_eq!(is_mine.iter().filter(|&&m| m).count(), mine_count);

    let mut revealed = revealed.to_vec();
    let mut flagged = vec![false; n];

    loop {
        let safe_count = revealed.iter().filter(|&&r| r).count();
        if safe_count == n - mine_count {
            return true;
        }

        let flagged_count = flagged.iter().filter(|&&f| f).count();
        let remaining_mines = mine_count - flagged_count;
        let unknowns: Vec<usize> = (0..n).filter(|&i| !revealed[i] && !flagged[i]).collect();
        if unknowns.is_empty() {
            return safe_count == n - mine_count;
        }

        let mut progress = false;

        // Local adjacency rules + global mine-count rules.
        if remaining_mines == 0 {
            for &i in &unknowns {
                reveal_safe(i, width, height, is_mine, &mut revealed);
                progress = true;
            }
        } else if remaining_mines == unknowns.len() {
            for &i in &unknowns {
                flagged[i] = true;
                progress = true;
            }
        }

        for i in 0..n {
            if !revealed[i] || is_mine[i] {
                continue;
            }
            let need = adjacency(width, height, is_mine, i) as usize;
            let neighbors = neighbors(width, height, i);
            let flagged_n = neighbors.iter().filter(|&&j| flagged[j]).count();
            let hidden: Vec<usize> = neighbors
                .iter()
                .copied()
                .filter(|&j| !revealed[j] && !flagged[j])
                .collect();
            if flagged_n == need && !hidden.is_empty() {
                for j in hidden {
                    reveal_safe(j, width, height, is_mine, &mut revealed);
                    progress = true;
                }
            } else if flagged_n + hidden.len() == need && !hidden.is_empty() {
                for j in hidden {
                    flagged[j] = true;
                    progress = true;
                }
            }
        }

        if progress {
            continue;
        }

        // Full-information: cells fixed across every consistent completion.
        let Some((always_mine, always_safe)) =
            deduce_from_completions(width, height, is_mine, &revealed, &flagged, remaining_mines)
        else {
            return false;
        };

        for i in 0..n {
            if always_mine[i] && !flagged[i] {
                flagged[i] = true;
                progress = true;
            }
            if always_safe[i] && !revealed[i] && !flagged[i] {
                reveal_safe(i, width, height, is_mine, &mut revealed);
                progress = true;
            }
        }

        if !progress {
            return false;
        }
    }
}

fn reveal_safe(
    i: usize,
    width: usize,
    height: usize,
    is_mine: &[bool],
    revealed: &mut [bool],
) {
    debug_assert!(!is_mine[i]);
    if revealed[i] {
        return;
    }
    revealed[i] = true;
    if adjacency(width, height, is_mine, i) == 0 {
        for j in neighbors(width, height, i) {
            if !is_mine[j] {
                reveal_safe(j, width, height, is_mine, revealed);
            }
        }
    }
}

fn adjacency(width: usize, height: usize, is_mine: &[bool], i: usize) -> u8 {
    neighbors(width, height, i)
        .into_iter()
        .filter(|&j| is_mine[j])
        .count() as u8
}

fn neighbors(width: usize, height: usize, i: usize) -> Vec<usize> {
    let x = (i % width) as isize;
    let y = (i / width) as isize;
    let mut out = Vec::with_capacity(8);
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x + dx;
            let ny = y + dy;
            if nx >= 0 && ny >= 0 && (nx as usize) < width && (ny as usize) < height {
                out.push(ny as usize * width + nx as usize);
            }
        }
    }
    out
}

/// Enumerate consistent mine placements on the constraint frontier; return
/// per-cell agreement. Sea cells (no adjacent numbers) use residual mine count.
/// `None` if there are no consistent completions (should not happen for a real board).
fn deduce_from_completions(
    width: usize,
    height: usize,
    is_mine: &[bool],
    revealed: &[bool],
    flagged: &[bool],
    remaining_mines: usize,
) -> Option<(Vec<bool>, Vec<bool>)> {
    let n = width * height;
    let unknowns: Vec<usize> = (0..n).filter(|&i| !revealed[i] && !flagged[i]).collect();
    if unknowns.is_empty() {
        return Some((vec![false; n], vec![false; n]));
    }

    let mut constraints: Vec<(usize, Vec<usize>)> = Vec::new();
    for i in 0..n {
        if !revealed[i] || is_mine[i] {
            continue;
        }
        let need = adjacency(width, height, is_mine, i) as usize;
        let nbrs = neighbors(width, height, i);
        let flagged_n = nbrs.iter().filter(|&&j| flagged[j]).count();
        let vars: Vec<usize> = nbrs
            .iter()
            .copied()
            .filter(|&j| !revealed[j] && !flagged[j])
            .collect();
        constraints.push((need.saturating_sub(flagged_n), vars));
    }

    let mut on_frontier = vec![false; n];
    for (_, vars) in &constraints {
        for &v in vars {
            on_frontier[v] = true;
        }
    }
    let frontier: Vec<usize> = unknowns
        .iter()
        .copied()
        .filter(|&i| on_frontier[i])
        .collect();
    let sea_len = unknowns.iter().filter(|&&i| !on_frontier[i]).count();

    // Frontier too large for one CSP pass: no progress this step (conservative —
    // may increase Fallback rate; never marks a Forced-guess board as No-guess).
    if frontier.len() > 22 {
        return Some((vec![false; n], vec![false; n]));
    }

    let mut mine_in_all = vec![true; frontier.len()];
    let mut safe_in_all = vec![true; frontier.len()];
    let mut sea_always_mine = true;
    let mut sea_always_safe = true;
    let mut found = 0usize;
    let mut assign = vec![false; frontier.len()];
    let mut partial = vec![None; frontier.len()];

    fn consistent(frontier: &[usize], assign: &[bool], constraints: &[(usize, Vec<usize>)]) -> bool {
        for &(need, ref vars) in constraints {
            let mut mines = 0usize;
            for &v in vars {
                let Some(pos) = frontier.iter().position(|&u| u == v) else {
                    continue;
                };
                if assign[pos] {
                    mines += 1;
                }
            }
            if mines != need {
                return false;
            }
        }
        true
    }

    fn prune_ok(
        frontier: &[usize],
        assign: &[Option<bool>],
        constraints: &[(usize, Vec<usize>)],
        mines_used: usize,
        remaining_mines: usize,
        sea_len: usize,
        idx: usize,
    ) -> bool {
        let slots_left = frontier.len() - idx;
        let min_total = mines_used;
        let max_total = mines_used + slots_left;
        // Residual for sea must be placeable.
        if min_total > remaining_mines || max_total + sea_len < remaining_mines {
            return false;
        }
        for &(need, ref vars) in constraints {
            let mut mines = 0usize;
            let mut open = 0usize;
            for &v in vars {
                let Some(pos) = frontier.iter().position(|&u| u == v) else {
                    continue;
                };
                match assign[pos] {
                    Some(true) => mines += 1,
                    Some(false) => {}
                    None => open += 1,
                }
            }
            if mines > need || mines + open < need {
                return false;
            }
        }
        true
    }

    fn search(
        idx: usize,
        mines_used: usize,
        remaining_mines: usize,
        sea_len: usize,
        frontier: &[usize],
        assign: &mut [bool],
        partial: &mut [Option<bool>],
        constraints: &[(usize, Vec<usize>)],
        mine_in_all: &mut [bool],
        safe_in_all: &mut [bool],
        sea_always_mine: &mut bool,
        sea_always_safe: &mut bool,
        found: &mut usize,
    ) {
        if idx == frontier.len() {
            let sea_mines = remaining_mines.saturating_sub(mines_used);
            if mines_used > remaining_mines || sea_mines > sea_len {
                return;
            }
            if !consistent(frontier, assign, constraints) {
                return;
            }
            *found += 1;
            for (i, &a) in assign.iter().enumerate() {
                if a {
                    safe_in_all[i] = false;
                } else {
                    mine_in_all[i] = false;
                }
            }
            if sea_mines != sea_len {
                *sea_always_mine = false;
            }
            if sea_mines != 0 {
                *sea_always_safe = false;
            }
            return;
        }
        if !prune_ok(
            frontier,
            partial,
            constraints,
            mines_used,
            remaining_mines,
            sea_len,
            idx,
        ) {
            return;
        }

        assign[idx] = false;
        partial[idx] = Some(false);
        search(
            idx + 1,
            mines_used,
            remaining_mines,
            sea_len,
            frontier,
            assign,
            partial,
            constraints,
            mine_in_all,
            safe_in_all,
            sea_always_mine,
            sea_always_safe,
            found,
        );
        assign[idx] = true;
        partial[idx] = Some(true);
        search(
            idx + 1,
            mines_used + 1,
            remaining_mines,
            sea_len,
            frontier,
            assign,
            partial,
            constraints,
            mine_in_all,
            safe_in_all,
            sea_always_mine,
            sea_always_safe,
            found,
        );
        partial[idx] = None;
    }

    search(
        0,
        0,
        remaining_mines,
        sea_len,
        &frontier,
        &mut assign,
        &mut partial,
        &constraints,
        &mut mine_in_all,
        &mut safe_in_all,
        &mut sea_always_mine,
        &mut sea_always_safe,
        &mut found,
    );

    if found == 0 {
        return None;
    }

    let mut always_mine = vec![false; n];
    let mut always_safe = vec![false; n];
    for (k, &cell) in frontier.iter().enumerate() {
        if mine_in_all[k] {
            always_mine[cell] = true;
        }
        if safe_in_all[k] {
            always_safe[cell] = true;
        }
    }
    if sea_len > 0 {
        for &cell in &unknowns {
            if on_frontier[cell] {
                continue;
            }
            if sea_always_mine {
                always_mine[cell] = true;
            }
            if sea_always_safe {
                always_safe[cell] = true;
            }
        }
    }
    Some((always_mine, always_safe))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_hidden_cell_deduced_by_mine_count_is_no_guess() {
        // 2×1: one mine. Left revealed safe ⇒ right must be the mine.
        let is_mine = [false, true];
        let revealed = [true, false];
        assert!(is_no_guess(2, 1, 1, &is_mine, &revealed));
    }

    #[test]
    fn symmetric_two_hidden_one_mine_is_forced_guess() {
        // 2×1: both hidden, one mine — neither cell is fixed across completions.
        let is_mine = [true, false];
        let revealed = [false, false];
        assert!(!is_no_guess(2, 1, 1, &is_mine, &revealed));
    }

    #[test]
    fn numbered_cell_with_one_hidden_neighbour_deduces_mine() {
        // 3×1: mine in the middle. Left revealed (count 1) ⇒ middle is the mine ⇒ right safe.
        let is_mine = [false, true, false];
        let revealed = [true, false, false];
        assert!(is_no_guess(3, 1, 1, &is_mine, &revealed));
    }

    #[test]
    fn overlapping_constraints_deduce_without_guess() {
        // 3×2: mines at bottom corners; top row revealed. Only CSP pins the centre safe.
        let is_mine = [
            false, false, false, //
            true, false, true,
        ];
        let revealed = [
            true, true, true, //
            false, false, false,
        ];
        assert!(is_no_guess(3, 2, 2, &is_mine, &revealed));
    }
}
