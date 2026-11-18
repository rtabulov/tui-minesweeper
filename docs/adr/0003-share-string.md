# Share string: versioned opening payload

Players share an opaque, versioned, unpadded base64url string that pins Difficulty identity (preset vs Custom), Seed, and generating first click. Launch auto-opens that click so recipients land on the same accepted board. The payload is opening-only (no later flags, reveals, cursor, timer, or outcome). Seed remains the RNG `u64`; the Share string is the portable envelope. CLI `--share` is mutually exclusive with `--seed` and difficulty/dims flags (wired in a follow-on ticket).

## Format (version 1)

Byte layout before base64url (URL-safe alphabet, no padding):

| Field | Encoding |
|-------|----------|
| version | `u8` = `1` |
| difficulty tag | `u8`: `0` Beginner, `1` Intermediate, `2` Expert, `3` Custom |
| custom dims | if tag `3`: `width`, `height`, `mines` as little-endian `u16` each |
| seed | little-endian `u64` |
| click | `x`, `y` as little-endian `u16` each |

Unknown version bytes fail closed. Clicks outside the encoded Difficulty board fail closed. Difficulty identity is required so Beginner and Custom with the same dims keep distinct No-guess search budgets (ADR-0001).

## Considered Options

- **Opening-only Share string** (chosen) — short, unambiguous “this puzzle”
- **Mid-game / “at position” snapshot** — deferred; larger and not needed for first share
- **Dims-only difficulty** — rejected; would collapse preset vs Custom budgets

## Consequences

- Encode is available only after the generating first click exists
- Launch path must call the same reveal / after-move orchestration as a normal opening (clock, Fallback notice)
- Future major formats bump the version byte; old clients reject unknown versions
