# TUI Minesweeper

Terminal Minesweeper: board generation, play rules, and the player-facing contract for fair layouts.

## Language

**No-guess board**:
A mine layout that, after its generating first click, can be finished using only Deductions — never a Forced guess. The player can still lose by opening a cell that was not Deduced.
_Avoid_: Solvable (vague), winnable

**Generating first click**:
The first reveal that places mines and opens the board; the no-guess guarantee applies to that opening, not to every possible first cell.
_Avoid_: First click (when meaning any click before mines exist without the guarantee)

**Opening safe zone**:
The generating first click and its neighbours, excluded from mine placement when the board is large enough (otherwise only the clicked cell), so the first reveal flood-fills.
_Avoid_: Safe cell (for a single Deduced cell mid-game)

**Difficulty**:
A named board size and mine count (Beginner, Intermediate, Expert) or a Custom width/height/mines triple. The no-guess guarantee covers all of them. On presets, Fallback boards should be exceptional; Custom may see them more often.
_Avoid_: Level, mode (for board shape)

**Deduction**:
A judgment that a hidden cell is mine or safe because every completion consistent with revealed adjacency agrees on that cell.
_Avoid_: Heuristic, guess, “basic” / pattern-only logic (as the product standard)

**Forced guess**:
A position where the board is not finished and no hidden cell can be Deduced.
_Avoid_: Optional guess, fifty-fifty (as the only case)

**No-guess search**:
After the generating first click, proposing first-click-safe layouts from the Seed stream until one is a No-guess board or the budget is exhausted. Search time is not play time; the game clock starts only after a layout is accepted. The same engine is not used for in-game hints.
_Avoid_: Solving (when meaning in-game assistance)

**Fallback board**:
A first-click-safe layout kept after No-guess search gives up; it may contain Forced guesses. The player is told once when this happens.
_Avoid_: Failed board, unsolvable board (when meaning this accepted compromise)

**Seed**:
The value that fixes the RNG stream used to propose candidate layouts so the same seed and generating first click replay the same accepted board.
_Avoid_: Board id (unless we introduce one separately)

**Finished game**:
A play session that ended in Won or Lost. Counted in Games whether or not the player won.
_Avoid_: Game (when meaning an abandoned session)

**Win time**:
Elapsed play time for a Finished game that ended in Won. Loss durations are not accumulated for time averages.
_Avoid_: Clear time (ambiguous with a single fast win)

**Best time**:
The shortest Win time recorded for a Difficulty.
_Avoid_: Personal best (fine in casual speech, not the column label)

**Average win time**:
Mean Win time across wins only; shown in statistics as Avg (W). Losses affect Games and win rate, not this average.
_Avoid_: Average time, Avg (when losses are excluded)
