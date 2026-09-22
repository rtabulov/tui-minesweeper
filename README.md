# TUI Minesweeper

Terminal Minesweeper with no-guess boards after the opening click.

## Run

```bash
npx @rassul/minesweeper
```

Same binary name after a global install: `tui-minesweeper`.

Supported `npx` platforms: Linux x64/arm64, macOS x64/arm64, Windows x64.

## Develop

```bash
cargo run --release
```

## Release (maintainers)

Versions are lockstep across `Cargo.toml`, npm packages, and tag `vX.Y.Z`.

1. Bump `version` in `Cargo.toml` (and keep `npm/minesweeper/package.json` in sync, or let CI `set-version` rewrite it on the tag job).
2. Commit, then `git tag vX.Y.Z && git push origin vX.Y.Z`.
3. GitHub Actions (`.github/workflows/release.yml`) builds five natives, publishes platform packages + `@rassul/minesweeper`, publishes crates.io, and attaches binaries + `SHA256SUMS` to the Release.

### First publish bootstrap

Trusted publishing needs the package/crate to exist and the workflow filename `release.yml` registered:

1. **npm:** either set a short-lived `NPM_TOKEN` repo secret for the first tag, or pack/publish once locally after a green build; then on each of the six packages add a GitHub Actions trusted publisher for `rtabulov/tui-minesweeper` / `release.yml`. Remove `NPM_TOKEN` afterward.
2. **crates.io:** `cargo publish` once with your account (or temporary `CARGO_REGISTRY_TOKEN`), then enable trusted publishing for the same workflow. Remove the secret afterward.

## License

MIT
