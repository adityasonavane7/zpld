#!/usr/bin/env bash
set -euo pipefail

echo "==> Checking Rust toolchain"
if ! command -v rustup &>/dev/null; then
    echo "rustup not found. Install from https://rustup.rs"
    exit 1
fi

rustup component add rustfmt clippy

echo "==> Installing cargo-audit"
cargo install cargo-audit --quiet

echo "==> Installing cargo-watch (optional, for auto-rebuild)"
cargo install cargo-watch --quiet || true

echo "==> Verifying build"
cargo build --all

echo ""
echo "Dev environment ready."
echo "  cargo build --all        Build everything"
echo "  cargo test --all         Run all tests"
echo "  cargo fmt --all          Format code"
echo "  cargo clippy --all-targets -- -D warnings   Lint"
echo "  cargo audit              Check for known vulnerabilities"
