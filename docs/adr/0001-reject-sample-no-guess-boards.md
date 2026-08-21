# Reject-sample No-guess boards on first reveal

After the generating first click we keep the opening safe zone and propose mine layouts from the Seed RNG stream until a full-information No-guess board is found (tiered attempt budget by Difficulty) or we accept a Fallback board and notify once. Reject sampling won over constructive generation because it preserves deterministic Seed replay and fits deferred placement; the acceptance engine is generation-only (no in-game hints), and search time is not play time.

## Considered Options

- **Reject sampling** (chosen) — place, test, resample from Seed
- **Constructive** — solver-guided placement; harder to keep Seed semantics identical to today’s stream
- **Hybrid** — more machinery than we need for standard densities

## Consequences

- Dense Custom may Fallback more often; presets should Fallback only exceptionally
- Docs must state the no-guess contract and that Fallback can happen
