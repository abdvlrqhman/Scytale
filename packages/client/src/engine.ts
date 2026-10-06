// The vault core, wherever it runs: WASM in the extension, Rust behind IPC in the desktop app.
// Everything is async so both can implement it.

export interface Kdf {
  m_kib: number;
  t: number;
  p: number;
}

export type CoreItemData =
  | { type: 'login'; username: string; password: string; urls: string[]; totp: string; exact_host: boolean }
  | { type: 'note' }
  | { type: 'card'; holder: string; number: string; expiry: string; cvv: string; pin: string }
  | {
      type: 'identity';
      full_name: string;
      email: string;
      phone: string;
      company: string;
      address1: string;
      address2: string;
      city: string;
      region: string;
      postal_code: string;
      country: string;
    };

/** Mirrors `scytale_core::Item` (serde JSON). */
export interface CoreItem {
  title: string;
  notes: string;
  favorite: boolean;
  data: CoreItemData;
}

/** An item with its secrets blanked; `secrets` names the ones it holds. */
export interface CoreView {
  id: string;
  item: CoreItem;
  secrets: string[];
  edited_ms: number;
  edited_device: string;
  /** When each previous password was last used, newest first (ms). */
  history: number[];
}

export interface CoreSummary {
  id: string;
  title: string;
  subtitle: string;
  kind: CoreItemData['type'];
  favorite: boolean;
}

export interface MergeResult {
  device: string;
  seq: number;
  device_name: string;
  changed: boolean;
}

export interface Created {
  header: Uint8Array;
  vaultId: string;
  /**
   * What lets a restarted extension service worker reopen the vault without the password
   * (the vault key, kept in memory-only session storage). `null` where the core keeps the vault
   * open itself (desktop).
   */
  resume: Uint8Array | null;
}

export interface VaultEngine {
  defaultKdf(): Promise<Kdf>;
  randomId(): Promise<string>;
  generateSecretKey(): Promise<string>;
  /** Throws a readable message on a typo. */
  normalizeSecretKey(input: string): Promise<string>;
  headerVaultId(header: Uint8Array): Promise<string>;
  headerSupersedes(a: Uint8Array, b: Uint8Array): Promise<boolean>;
  generate(options: object): Promise<{ value: string; bits: number }>;
  importCsv(text: string): Promise<{ items: CoreItem[]; skipped: number }>;
  importBitwardenJson(text: string): Promise<{ items: CoreItem[]; skipped: number }>;
  openEncryptedExport(bytes: Uint8Array, password: string): Promise<CoreItem[]>;

  create(password: string, secretKey: string, kdf: Kdf, nowMs: number, deviceId: string): Promise<Created>;
  /** Opens the vault. Returns the resume token (see {@link Created.resume}). */
  unlock(header: Uint8Array, password: string, secretKey: string, deviceId: string): Promise<Uint8Array | null>;
  resume(token: Uint8Array, vaultId: string, deviceId: string): Promise<void>;
  changePassword(
    header: Uint8Array,
    newPassword: string,
    secretKey: string,
    kdf: Kdf,
    nowMs: number,
    deviceId: string,
  ): Promise<Uint8Array>;
  lock(): Promise<void>;
  isOpen(): Promise<boolean>;

  // Requires an open vault.
  setGuard(json: string): Promise<void>;
  guardJson(): Promise<string>;
  mergeSnapshot(bytes: Uint8Array): Promise<MergeResult>;
  seal(seq: number, deviceName: string): Promise<Uint8Array>;
  list(): Promise<CoreSummary[]>;
  view(id: string): Promise<CoreView>;
  reveal(id: string, field: string): Promise<string>;
  revealHistory(id: string, index: number): Promise<string>;
  draft(id: string): Promise<CoreItem>;
  upsert(id: string | null, item: CoreItem, nowMs: number): Promise<string>;
  addItems(items: CoreItem[], nowMs: number): Promise<number>;
  remove(id: string, nowMs: number): Promise<boolean>;
  matches(pageUrl: string): Promise<{ id: string; exact: boolean }[]>;
  credentialsFor(id: string, pageUrl: string): Promise<{ username: string; password: string }>;
  totp(id: string, unixSecs: number): Promise<{ code: string; remaining: number; period: number }>;
  compact(nowMs: number, maxAgeMs: number): Promise<void>;
  exportCsv(): Promise<{ csv: string; omitted: number }>;
  exportEncrypted(password: string, kdf: Kdf): Promise<Uint8Array>;
}
