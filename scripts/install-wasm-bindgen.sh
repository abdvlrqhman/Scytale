#!/usr/bin/env sh
# Installs the wasm-bindgen CLI pinned in crates/core-wasm/Cargo.toml, verified by checksum.
# Linux x86_64 (CI and the reproducible-build container). Elsewhere:
#   cargo install wasm-bindgen-cli --version <version> --locked
set -eu
V=0.2.129
SHA=82d12bb940e2d4e72e0d5605387fc1b8ca179044e012b620f0ce4e7440e8320e
F="wasm-bindgen-$V-x86_64-unknown-linux-musl.tar.gz"
DIR="${RUNNER_TEMP:-/tmp}"
curl -fsSL -o "$DIR/$F" "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$V/$F"
echo "$SHA  $DIR/$F" | sha256sum -c -
tar -xzf "$DIR/$F" -C "$DIR"
BIN="$DIR/wasm-bindgen-$V-x86_64-unknown-linux-musl"
if [ -n "${GITHUB_PATH:-}" ]; then echo "$BIN" >> "$GITHUB_PATH"; else install -m 755 "$BIN/wasm-bindgen" /usr/local/bin/; fi
