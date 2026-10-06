# Architecture

## Shape

**Functional core, imperative shell.** All cryptography and vault logic lives in one Rust crate with no
I/O. Every app is a thin shell around it.

```
crates/core        Rust. domain · crypto · vault format · merge · generator · TOTP · importers
crates/core-wasm   wasm-bindgen bindings → used by the extension
packages/client    TypeScript application layer: use-cases + ports & adapters (sync providers, storage)
packages/ui        Svelte 5 design system + screens, shared by the extension and the desktop app
apps/extension     WXT, Manifest V3: Chrome, Edge, Brave, Opera, Vivaldi, Firefox
apps/desktop       Tauri v2: links crates/core natively
```

## Dependency rule

```
apps ──► ui, client ──► core-wasm / Tauri commands ──► core
```

Arrows point inward only. `core` depends on nothing platform-specific. Package boundaries enforce this:
a layer cannot import what it does not list as a dependency.

## Ports

A port exists only where at least two real adapters exist.

| Port (`packages/client`) | Adapters |
|---|---|
| `CoreBridge` | `WasmCore` (extension) · `TauriCore` (desktop) |
| `LocalStore` | `chrome.storage` (extension) · Tauri app-data dir (desktop) |
| `RemoteStore` | WebDAV · Dropbox · OneDrive · Google Drive · Folder (desktop) |

Everything else is concrete code: no factories, no DI container.

## Key custody

- **Desktop:** the vault key never leaves Rust memory and is zeroized on lock. The webview receives item
  summaries and only the individual fields the user reveals or copies. Only ciphertext crosses into
  TypeScript for upload and download.
- **Extension:** keys live in WASM memory in the background context, plus `storage.session`
  (memory-only, hidden from content scripts) so the MV3 service worker can restart without re-prompting.
  JavaScript cannot guarantee zeroization; see [THREAT_MODEL.md](THREAT_MODEL.md).

## Decisions

| Decision | Why |
|---|---|
| **No server** | Nothing to breach, nothing to pay for, nobody to subpoena. Sync goes through the user's own storage. See [SYNC.md](SYNC.md). |
| **Rust core everywhere** | One crypto implementation to audit. Native in Tauri, WASM in the extension, UniFFI → Kotlin/Swift for the mobile autofill services in v2. Cost: contributors need Rust + Node. |
| **One file per device** | Google Drive has no conditional writes, and synced folders have none at all. If no device ever writes another's file, no write can be lost. |
| **No injected in-page UI (v1)** | DOM-based extension clickjacking (DEF CON 33, 2025) attacks UI that extensions inject into pages. Filling only from the popup, keyboard shortcut or context menu removes that attack class. |
| **Least-privilege extension** | No `<all_urls>` at install. Filling uses `activeTab`. "Offer to save logins" is opt-in and asks for host access only when enabled. |
| **Strength meter in the UI, not the core** | The Rust `zxcvbn` crate pulls in wasm-bindgen, chrono and two regex engines, which would break "core has no platform code". The UI uses `@zxcvbn-ts`; mobile (v2) can use a native port. |
