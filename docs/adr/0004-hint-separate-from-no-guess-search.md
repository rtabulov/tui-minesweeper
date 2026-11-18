# Hint Deduction is separate from No-guess search

Player-facing Hints deduce from revealed adjacency and flags only; they must not consult the true mine layout. No-guess search (ADR 0001) stays generation-only. Reusing the acceptance path with `is_mine` for Hints would silently cheat and blur Assisted play with board generation.
