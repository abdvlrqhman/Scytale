// VaultEngine over Tauri IPC. The vault lives in Rust; nothing here ever sees the vault key.
import type { CoreItem, Created, Kdf, VaultEngine } from '@scytale/client';
import { invoke } from '@tauri-apps/api/core';

// Tauri serializes arguments as JSON, where a Uint8Array would become an object: send plain arrays.
const arr = (b: Uint8Array) => Array.from(b);
const bytes = (v: ArrayBuffer | number[]) => new Uint8Array(v);

export class TauriEngine implements VaultEngine {
  defaultKdf(): Promise<Kdf> {
    return invoke('default_kdf');
  }
  randomId(): Promise<string> {
    return invoke('random_id');
  }
  generateSecretKey(): Promise<string> {
    return invoke('generate_secret_key');
  }
  normalizeSecretKey(input: string): Promise<string> {
    return invoke('normalize_secret_key', { input });
  }
  headerVaultId(header: Uint8Array): Promise<string> {
    return invoke('header_vault_id', { header: arr(header) });
  }
  headerSupersedes(a: Uint8Array, b: Uint8Array): Promise<boolean> {
    return invoke('header_supersedes', { a: arr(a), b: arr(b) });
  }
  generate(options: object): Promise<{ value: string; bits: number }> {
    return invoke('generate', { options });
  }
  importCsv(text: string): Promise<{ items: CoreItem[]; skipped: number }> {
    return invoke('import_csv', { text });
  }
  importBitwardenJson(text: string): Promise<{ items: CoreItem[]; skipped: number }> {
    return invoke('import_bitwarden_json', { text });
  }
  openEncryptedExport(b: Uint8Array, password: string): Promise<CoreItem[]> {
    return invoke('open_encrypted_export', { bytes: arr(b), password });
  }

  async create(password: string, secretKey: string, kdf: Kdf, nowMs: number, device: string): Promise<Created> {
    const r = await invoke<{ header: number[]; vaultId: string }>('create_vault', { password, secretKey, kdf, nowMs, device });
    return { header: bytes(r.header), vaultId: r.vaultId, resume: null };
  }
  async unlock(header: Uint8Array, password: string, secretKey: string, device: string) {
    await invoke('unlock', { header: arr(header), password, secretKey, device });
    return null; // the session stays in Rust
  }
  async resume() {
    // Nothing to restore: a webview reload does not lose the Rust session.
  }
  async changePassword(header: Uint8Array, newPassword: string, secretKey: string, kdf: Kdf, nowMs: number, device: string) {
    return bytes(await invoke<number[]>('change_password', { header: arr(header), newPassword, secretKey, kdf, nowMs, device }));
  }
  lock(): Promise<void> {
    return invoke('lock');
  }
  isOpen(): Promise<boolean> {
    return invoke('is_open');
  }

  setGuard(json: string): Promise<void> {
    return invoke('set_guard', { json });
  }
  guardJson(): Promise<string> {
    return invoke('guard_json');
  }
  mergeSnapshot(b: Uint8Array) {
    return invoke<{ device: string; seq: number; device_name: string; changed: boolean }>('merge_snapshot', { bytes: arr(b) });
  }
  async seal(seq: number, deviceName: string) {
    return bytes(await invoke<ArrayBuffer>('seal', { seq, deviceName }));
  }
  list() {
    return invoke<Awaited<ReturnType<VaultEngine['list']>>>('list');
  }
  view(id: string) {
    return invoke<Awaited<ReturnType<VaultEngine['view']>>>('view', { id });
  }
  reveal(id: string, field: string): Promise<string> {
    return invoke('reveal', { id, field });
  }
  revealHistory(id: string, index: number): Promise<string> {
    return invoke('reveal_history', { id, index });
  }
  draft(id: string): Promise<CoreItem> {
    return invoke('draft', { id });
  }
  upsert(id: string | null, item: CoreItem, nowMs: number): Promise<string> {
    return invoke('upsert', { id, item, nowMs });
  }
  addItems(items: CoreItem[], nowMs: number): Promise<number> {
    return invoke('add_items', { items, nowMs });
  }
  remove(id: string, nowMs: number): Promise<boolean> {
    return invoke('remove', { id, nowMs });
  }
  matches(pageUrl: string): Promise<{ id: string; exact: boolean }[]> {
    return invoke('matches', { pageUrl });
  }
  credentialsFor(id: string, pageUrl: string): Promise<{ username: string; password: string }> {
    return invoke('credentials_for', { id, pageUrl });
  }
  totp(id: string, unixSecs: number): Promise<{ code: string; remaining: number; period: number }> {
    return invoke('totp', { id, unixSecs });
  }
  compact(nowMs: number, maxAgeMs: number): Promise<void> {
    return invoke('compact', { nowMs, maxAgeMs });
  }
  exportCsv(): Promise<{ csv: string; omitted: number }> {
    return invoke('export_csv');
  }
  async exportEncrypted(password: string, kdf: Kdf) {
    return bytes(await invoke<ArrayBuffer>('export_encrypted', { password, kdf }));
  }
}
