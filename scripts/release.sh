#!/usr/bin/env bash
# Tag a release. Bumps version, commits, tags, and pushes.
# Usage: ./scripts/release.sh 0.2.0
set -euo pipefail

VERSION=${1:?Usage: $0 <version>}

echo "==> Releasing v${VERSION}"

# Ensure working tree is clean
if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "Working tree is dirty. Commit or stash changes first."
    exit 1
fi

# Update version in workspace Cargo.toml
sed -i "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml

# Rebuild to update Cargo.lock
cargo build --all --quiet

git add Cargo.toml Cargo.lock
git commit -m "chore: release v${VERSION}"
git tag -a "v${VERSION}" -m "Release v${VERSION}"

echo ""
echo "Tagged v${VERSION}. Push with:"
echo "  git push origin main v${VERSION}"
