# Scytale

*(SKIT-uh-lee)* The Spartans' cipher rod: a message wound around it could only be read by someone holding
a rod of the same thickness. Scytale works the same way — your vault can only be opened with your master
password **and** the Secret Key held by your own devices.

**A free, open-source password manager with no server.**
Your vault is encrypted on your device and synced through storage you already own. Nobody else —
not us, not your cloud provider — can read it.

> ⚠️ **Pre-alpha.** Not audited. Do not store real passwords in it yet.

## How it works

- **Two secrets.** Your master password plus a random 128-bit Secret Key that never leaves your devices.
  A stolen vault file can't be cracked, even if your password is weak.
- **No server.** Sync goes through your own WebDAV, Dropbox, OneDrive, Google Drive or synced folder.
  Each device writes only its own encrypted file, so edits are never lost.
- **One audited core.** All cryptography lives in one Rust crate, used natively by the desktop app and
  as WASM by the browser extension.

Read more: [Architecture](docs/ARCHITECTURE.md) · [Cryptography](docs/CRYPTO.md) ·
[Sync](docs/SYNC.md) · [Threat model](docs/THREAT_MODEL.md)

## Platforms

| Platform | Status |
|---|---|
| Chrome, Edge, Brave, Opera, Vivaldi | planned (v1) |
| Firefox 140+ | planned (v1) |
| Windows, macOS, Linux desktop (Tauri) | planned (v1) |
| Android, iOS, Safari | later (v2) |

## Develop

Requirements: Rust (the version is pinned in `rust-toolchain.toml`, so rustup installs it automatically),
Node 24+, pnpm 11 (`npm i -g pnpm@11`).

```sh
cargo test --workspace
```

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[GPL-3.0-or-later](LICENSE). Forks of a security tool should stay open source so users can verify them.
