import type * as Wasm from '@scytale/core-wasm';
import type { CoreItem, Created, Kdf, VaultEngine } from './engine';

/** {@link VaultEngine} over the WASM build. The module must already be initialized. */
export class WasmEngine implements VaultEngine {
  #handle: Wasm.VaultHandle | null = null;
  // The vault key as JS bytes. The extension needs it anyway (session storage, to survive service
  // worker restarts); it is zero-filled on lock.
  #key: Uint8Array | null = null;

  constructor(private readonly wasm: typeof Wasm) {}

  get #open(): Wasm.VaultHandle {
    if (!this.#handle) throw new Error('The vault is locked.');
    return this.#handle;
  }

  async defaultKdf(): Promise<Kdf> {
    return JSON.parse(this.wasm.default_kdf());
  }
  async randomId() {
    return this.wasm.random_id();
  }
  async generateSecretKey() {
    return this.wasm.generate_secret_key();
  }
  async normalizeSecretKey(input: string) {
    return this.wasm.normalize_secret_key(input);
  }
  async headerVaultId(header: Uint8Array) {
    return JSON.parse(this.wasm.header_info(header)).vault_id as string;
  }
  async headerSupersedes(a: Uint8Array, b: Uint8Array) {
    return this.wasm.header_supersedes(a, b);
  }
  async generate(options: object) {
    return JSON.parse(this.wasm.generate(JSON.stringify(options)));
  }
  async importCsv(text: string) {
    return JSON.parse(this.wasm.import_csv(text));
  }
  async importBitwardenJson(text: string) {
    return JSON.parse(this.wasm.import_bitwarden_json(text));
  }
  async openEncryptedExport(bytes: Uint8Array, password: string): Promise<CoreItem[]> {
    return JSON.parse(this.wasm.open_encrypted_export(bytes, password));
  }

  async create(password: string, secretKey: string, kdf: Kdf, nowMs: number, deviceId: string): Promise<Created> {
    const c = this.wasm.create_vault(password, secretKey, kdf.m_kib, kdf.t, kdf.p, nowMs, deviceId);
    try {
      this.#replace(new this.wasm.VaultHandle(c.vault_key, c.vault_id, deviceId), c.vault_key);
      return { header: c.header, vaultId: c.vault_id, resume: c.vault_key };
    } finally {
      c.free();
    }
  }

  async unlock(header: Uint8Array, password: string, secretKey: string, deviceId: string) {
    const key = this.wasm.unlock(header, password, secretKey);
    const vaultId = JSON.parse(this.wasm.header_info(header)).vault_id as string;
    this.#replace(new this.wasm.VaultHandle(key, vaultId, deviceId), key.slice());
    return key;
  }

  async resume(token: Uint8Array, vaultId: string, deviceId: string) {
    this.#replace(new this.wasm.VaultHandle(token, vaultId, deviceId), token.slice());
  }

  async changePassword(header: Uint8Array, newPassword: string, secretKey: string, kdf: Kdf, nowMs: number, deviceId: string) {
    if (!this.#key) throw new Error('The vault is locked.');
    return this.wasm.change_password(header, this.#key, newPassword, secretKey, kdf.m_kib, kdf.t, kdf.p, nowMs, deviceId);
  }

  async lock() {
    this.#replace(null, null);
  }
  async isOpen() {
    return this.#handle !== null;
  }

  async setGuard(json: string) {
    this.#open.set_guard(json);
  }
  async guardJson() {
    return this.#open.guard_json();
  }
  async mergeSnapshot(bytes: Uint8Array) {
    return JSON.parse(this.#open.merge_snapshot(bytes));
  }
  async seal(seq: number, deviceName: string) {
    return this.#open.seal(seq, deviceName);
  }
  async list() {
    return JSON.parse(this.#open.list());
  }
  async view(id: string) {
    return JSON.parse(this.#open.view(id));
  }
  async reveal(id: string, field: string) {
    return this.#open.reveal(id, field);
  }
  async revealHistory(id: string, index: number) {
    return this.#open.reveal_history(id, index);
  }
  async draft(id: string) {
    return JSON.parse(this.#open.draft(id));
  }
  async upsert(id: string | null, item: CoreItem, nowMs: number) {
    return this.#open.upsert(id, JSON.stringify(item), nowMs);
  }
  async addItems(items: CoreItem[], nowMs: number) {
    return this.#open.add_items(JSON.stringify(items), nowMs);
  }
  async remove(id: string, nowMs: number) {
    return this.#open.remove(id, nowMs);
  }
  async matches(pageUrl: string) {
    return JSON.parse(this.#open.matches(pageUrl));
  }
  async credentialsFor(id: string, pageUrl: string) {
    return JSON.parse(this.#open.credentials_for(id, pageUrl));
  }
  async totp(id: string, unixSecs: number) {
    return JSON.parse(this.#open.totp(id, unixSecs));
  }
  async compact(nowMs: number, maxAgeMs: number) {
    this.#open.compact(nowMs, maxAgeMs);
  }
  async exportCsv() {
    return JSON.parse(this.#open.export_csv());
  }
  async exportEncrypted(password: string, kdf: Kdf) {
    return this.#open.export_encrypted(password, kdf.m_kib, kdf.t, kdf.p);
  }

  /** Frees the old handle, which wipes its keys and items from WASM memory. */
  #replace(next: Wasm.VaultHandle | null, key: Uint8Array | null) {
    this.#handle?.free();
    this.#key?.fill(0);
    this.#handle = next;
    this.#key = key;
  }
}
