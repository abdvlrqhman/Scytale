# Building the Scytale extension from source

The extension bundles the vault core, written in Rust and compiled to WebAssembly. The source zip
uploaded to the stores is the whole repository for that reason.

## With Docker (identical to the release)

```sh
docker build -f scripts/extension.Dockerfile -o out .
```

`out/firefox-mv3` and `out/chrome-mv3` are the unpacked builds.

## Without Docker

Tools: Node 24, pnpm 11.15, Rust via [rustup](https://rustup.rs) (`rust-toolchain.toml` pins the
version), and the wasm-bindgen CLI at the version pinned in `crates/core-wasm/Cargo.toml`
(`cargo install wasm-bindgen-cli --version 0.2.129 --locked`, or `scripts/install-wasm-bindgen.sh`
on Linux x86_64).

```sh
pnpm install --frozen-lockfile
pnpm run build:wasm
pnpm --filter @scytale/extension run build:firefox   # -> apps/extension/.output/firefox-mv3
pnpm --filter @scytale/extension run build           # -> apps/extension/.output/chrome-mv3
```

Builds are deterministic: CI builds twice from scratch and compares every file.

## Notes for reviewers

- No remote code. The only CSP exception is `'wasm-unsafe-eval'`, for the bundled WebAssembly core.
- No host permissions at install. `optional_host_permissions` is requested only when the user
  connects their own WebDAV server; `identity` only when they sign in to Dropbox.
- The `innerHTML` lint warning comes from the Svelte runtime inserting its own static templates.
