#!/usr/bin/env bash
# Bump lockstep versions, commit, tag, and push a release.
#
# Usage:
#   ./scripts/release.sh 0.1.2
#   ./scripts/release.sh 0.1.2 --dry-run
#   ./scripts/release.sh 0.1.2 --no-push
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

VERSION=""
DRY_RUN=0
PUSH=1

usage() {
  cat <<'EOF'
Usage: ./scripts/release.sh <semver> [--dry-run] [--no-push]

  <semver>    X.Y.Z (e.g. 0.1.2)
  --dry-run   Print steps; do not write files, commit, tag, or push
  --no-push   Commit and tag locally only
EOF
}

for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    --no-push) PUSH=0 ;;
    -h | --help)
      usage
      exit 0
      ;;
    -*)
      echo "Unknown option: $arg" >&2
      usage >&2
      exit 2
      ;;
    *)
      if [[ -n "$VERSION" ]]; then
        echo "Unexpected argument: $arg" >&2
        usage >&2
        exit 2
      fi
      VERSION="$arg"
      ;;
  esac
done

if [[ -z "$VERSION" || ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  usage >&2
  exit 2
fi

TAG="v${VERSION}"
CURRENT="$(
  cargo metadata --no-deps --format-version 1 \
    | node -e '
        let s = "";
        process.stdin.on("data", (d) => (s += d));
        process.stdin.on("end", () => {
          const j = JSON.parse(s);
          const pkg = j.packages.find((p) => p.name === "tui-minesweeper");
          if (!pkg) process.exit(1);
          process.stdout.write(pkg.version);
        });
      '
)"

if [[ "$CURRENT" == "$VERSION" ]]; then
  echo "Already at ${VERSION}; nothing to bump." >&2
  exit 1
fi

run() {
  if [[ "$DRY_RUN" -eq 1 ]]; then
    printf '[dry-run]'
    printf ' %q' "$@"
    printf '\n'
  else
    "$@"
  fi
}

echo "Release ${CURRENT} → ${VERSION} (tag ${TAG})"

if [[ "$DRY_RUN" -eq 0 ]]; then
  if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "Working tree is dirty; commit or stash first." >&2
    exit 1
  fi
  if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "Tag ${TAG} already exists locally." >&2
    exit 1
  fi
  if git ls-remote --tags origin "refs/tags/${TAG}" | grep -q .; then
    echo "Tag ${TAG} already exists on origin." >&2
    exit 1
  fi
fi

run node npm/scripts/bump-cargo-version.cjs "$VERSION"
run cargo update -p tui-minesweeper
run node npm/scripts/set-version.cjs "$VERSION"
run cargo test -q
run cargo build --release --locked

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "[dry-run] git add Cargo.toml Cargo.lock npm/minesweeper/package.json"
  echo "[dry-run] git commit -m \"chore: bump version to ${VERSION}\""
  echo "[dry-run] git tag -a ${TAG} -m ${TAG}"
  if [[ "$PUSH" -eq 1 ]]; then
    echo "[dry-run] git push origin HEAD"
    echo "[dry-run] git push origin ${TAG}"
  else
    echo "[dry-run] skip push (--no-push)"
  fi
  exit 0
fi

git add Cargo.toml Cargo.lock npm/minesweeper/package.json
git commit -m "chore: bump version to ${VERSION}"
git tag -a "$TAG" -m "$TAG"

if [[ "$PUSH" -eq 1 ]]; then
  git push origin HEAD
  git push origin "$TAG"
  echo "Pushed ${TAG}. Watch: gh run list --workflow=release.yml --limit 1"
else
  echo "Created local commit + ${TAG}. Push when ready:"
  echo "  git push origin HEAD && git push origin ${TAG}"
fi
