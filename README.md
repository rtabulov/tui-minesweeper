# TUI Minesweeper

Terminal Minesweeper with no-guess boards after the opening click.

## Run

```bash
npx @rassul/minesweeper
```

Same binary name after a global install: `tui-minesweeper`.

Supported `npx` platforms: Linux x64/arm64, macOS x64/arm64, Windows x64.

Stats and settings live in the platform config dir (`…/tui-minesweeper/state.json`). An old `~/.tui-minesweeper.json` is migrated once automatically.

## Develop

```bash
cargo run --release
```

## Release (maintainers)

Versions are lockstep across `Cargo.toml`, npm packages, and tag `vX.Y.Z`.

```bash
./scripts/release.sh 0.1.2           # bump, test, commit, tag, push
./scripts/release.sh 0.1.2 --dry-run # print steps only
./scripts/release.sh 0.1.2 --no-push # commit + tag locally
```

That syncs `Cargo.toml` + `Cargo.lock` + `npm/minesweeper/package.json`, runs tests, then pushes `vX.Y.Z` so `.github/workflows/release.yml` builds natives, publishes npm + crates.io, and creates the GitHub Release.

### First publish bootstrap

Trusted publishing needs the package/crate to exist and the workflow filename `release.yml` registered:

1. **npm:** either set a short-lived `NPM_TOKEN` repo secret for the first tag, or pack/publish once locally after a green build; then on each of the six packages add a GitHub Actions trusted publisher for `rtabulov/tui-minesweeper` / `release.yml`. Remove `NPM_TOKEN` afterward.
2. **crates.io:** `cargo publish` once with your account (or temporary `CARGO_REGISTRY_TOKEN`), then enable trusted publishing for the same workflow. Remove the secret afterward.

## License

MIT
