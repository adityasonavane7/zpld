#!/usr/bin/env bash
# Reproducible release build with all checks.
set -euo pipefail

echo "==> Formatting check"
cargo fmt --all -- --check

echo "==> Clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo "==> Tests"
cargo test --all

echo "==> Release build"
cargo build --release --all

echo ""
echo "Artifacts in target/release/:"
ls -lh target/release/zpld-supervisor target/release/zpldctl target/release/udp-counter
