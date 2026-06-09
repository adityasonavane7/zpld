# Contributing to zpld

## Getting Started

```bash
# Install Rust via rustup (https://rustup.rs)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/your-org/zpld
cd zpld
cargo build
```

Run `scripts/dev-setup.sh` to install additional tools (clippy, rustfmt, cargo-audit).

## Development Workflow

- **One logical change per PR.** If you are fixing a bug and refactoring nearby
  code, split them into separate PRs.
- **All CI checks must pass.** This includes `cargo fmt`, `cargo clippy`, and
  `cargo test`. Run them locally before pushing.
- **No `unsafe` without a comment.** Every `unsafe` block must have a comment
  explaining why it is sound.
- **Keep the worker contract stable.** Changes to traits in `zpld-framework`
  that break existing worker implementations require a major version bump and
  a migration guide.

## Code Style

Formatting is enforced by `rustfmt` with the project config in `rustfmt.toml`.
Run `cargo fmt --all` before committing. CI will reject unformatted code.

Linting is enforced by `clippy`. Run `cargo clippy --all-targets -- -D warnings`
before pushing. Clippy warnings are treated as errors in CI.

## Writing a Worker

See [docs/worker-contract.md](docs/worker-contract.md) for the full
specification. The `workers/udp-counter` implementation is the reference —
read it before writing a new worker.

## Submitting a Pull Request

1. Fork the repository and create a branch from `main`.
2. Make your changes. Add tests where applicable.
3. Run `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test --all`.
4. Open a PR against `main`. Fill in the PR template.
5. A maintainer will review within a few days.

## Reporting Security Issues

Do not open a public issue for security vulnerabilities. See [SECURITY.md](SECURITY.md).
