# Contributing

## Setup

- Rust: install [rustup](https://rustup.rs). `rust-toolchain.toml` pins the version.
- Node 24+ and pnpm 11: `npm i -g pnpm@11`.

## Before opening a PR

```sh
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
```

## Rules

- **Crypto changes** update [docs/CRYPTO.md](docs/CRYPTO.md) in the same PR and add known-answer test
  vectors. No new primitives without discussion in an issue first.
- **New dependencies** need a reason in the PR description. Fewer dependencies means less attack surface.
- **Inner layers never import outer ones.** `crates/core` has no I/O and no platform code. See
  [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
- **Security issues** go through [SECURITY.md](SECURITY.md), never public issues.
