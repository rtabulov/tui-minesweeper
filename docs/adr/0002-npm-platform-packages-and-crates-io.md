# Distribute via npm platform packages + crates.io

Players launch with `npx @rassul/minesweeper` (bin `tui-minesweeper`). The game stays a Rust binary; npm is a thin public launcher, not a rewrite. We ship a scoped meta package plus per-OS/arch optionalDependencies (`@rassul/minesweeper-<os>-<arch>` for linux-x64, linux-arm64, darwin-x64, darwin-arm64, win32-x64), publish lockstep versions from tag-driven CI, also publish the crate to crates.io, and attach the same binaries to the GitHub Release. Unsupported platforms fail fast in the Node stub.

## Considered Options

- **Platform optionalDependencies** (chosen) — offline after install; standard for native CLIs; more CI packages per release
- **GitHub Releases fetch on first run** — smaller meta package; needs network at launch; weaker offline story
- **Fat multi-binary npm tarball** — simple mentally; wasteful downloads
- **crates.io / brew only** — misses the stated `npx` UX

## Consequences

- First npm and crates.io publishes need a bootstrap (manual or one-time token) before trusted publishing (OIDC) can be wired per package/crate
- Release workflow filename is part of the trust config — renaming it breaks publishes until npm/crates.io settings are updated
- musl and Windows arm64 are out of day-one scope; stub must list supported targets explicitly
