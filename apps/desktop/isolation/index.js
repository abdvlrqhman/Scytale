// Tauri isolation pattern: every IPC message from the webview passes through this sandboxed frame
// before reaching Rust. Commands only accept fixed shapes, so messages pass through unchanged.
window.__TAURI_ISOLATION_HOOK__ = (payload) => payload;
