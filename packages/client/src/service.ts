// The use cases. Runs where the engine runs: the extension's background worker, or the desktop
// webview. Every public method is safe to call right after a service-worker restart.
import type { GeneratorOptions, ItemDraft, ItemKind, ItemSummary, ItemView, Theme } from '@scytale/ui/types';
import type { CoreItem, VaultEngine } from './engine';
import { draftFromItem, emptyDraft, itemFromDraft, itemView } from './mapping';
import { type KeyValueStore, fromB64, toB64 } from './store';

export interface Settings {
  theme: Theme;
  autoLockMinutes: number;
  clipboardSeconds: number;
  lockOnClose: boolean;
}

export const DEFAULT_SETTINGS: Settings = { theme: 'dark', autoLockMinutes: 15, clipboardSeconds: 30, lockOnClose: true };

export type Status = 'new' | 'locked' | 'unlocked';

/** Tombstones are purged after this long (see docs/SYNC.md). */
const TOMBSTONE_MAX_AGE_MS = 90 * 24 * 3600 * 1000;

// Local-store keys.
const K = {
  header: 'header',
  vaultId: 'vaultId',
  deviceId: 'deviceId',
  secretKey: 'secretKey',
  snapshot: 'snapshot',
  seq: 'seq',
  guard: 'guard',
  settings: 'settings',
  devices: 'devices',
} as const;

/** A message the UI can show as-is. */
export class UserError extends Error {}

function friendly(e: unknown, newDevice: boolean): Error {
  const msg = e instanceof Error ? e.message : String(e);
  if (msg.includes('wrong master password')) {
    return new UserError(
      newDevice
        ? 'That master password and Secret Key do not open this vault.'
        : 'That password does not open this vault. Check Caps Lock and try again.',
    );
  }
  if (msg.includes('invalid Secret Key')) return new UserError(`That Secret Key looks wrong: ${msg.split(': ').pop()}.`);
  return e instanceof Error ? e : new Error(msg);
}

export class VaultService {
  constructor(
    private readonly engine: VaultEngine,
    /** Survives restarts: encrypted vault, header, Secret Key, settings. */
    private readonly local: KeyValueStore,
    /** Memory-only: the resume token. Empty after a browser restart, so the vault is locked. */
    private readonly session: KeyValueStore,
    private readonly deviceName: string,
    private readonly now: () => number = Date.now,
  ) {}

  async status(): Promise<Status> {
    if (!(await this.local.get(K.header))) return 'new';
    return (await this.ensureOpen()) ? 'unlocked' : 'locked';
  }

  /** Reopens the vault after a service-worker restart, if the session is still alive. */
  async ensureOpen(): Promise<boolean> {
    if (await this.engine.isOpen()) return true;
    const token = await this.session.get<string>('resume');
    const vaultId = await this.local.get<string>(K.vaultId);
    if (!token || !vaultId) return false;
    await this.engine.resume(fromB64(token), vaultId, await this.#deviceId());
    await this.#loadLocal();
    return true;
  }

  /** Creates the vault and returns its Secret Key, to show once on the Emergency Kit. */
  async create(password: string): Promise<string> {
    if (await this.local.get(K.header)) throw new UserError('A vault already exists on this device.');
    const secretKey = await this.engine.generateSecretKey();
    const created = await this.engine.create(
      password,
      secretKey,
      await this.engine.defaultKdf(),
      this.now(),
      await this.#deviceId(),
    );
    await this.local.set(K.header, toB64(created.header));
    await this.local.set(K.vaultId, created.vaultId);
    await this.local.set(K.secretKey, secretKey);
    if (created.resume) await this.session.set('resume', toB64(created.resume));
    await this.#persist();
    return secretKey;
  }

  /** `secretKey` is needed only on a device that has never opened this vault. */
  async unlock(password: string, secretKey?: string): Promise<void> {
    const header = await this.local.get<string>(K.header);
    if (!header) throw new UserError('There is no vault on this device yet.');
    let sk = await this.local.get<string>(K.secretKey);
    try {
      if (secretKey) sk = await this.engine.normalizeSecretKey(secretKey);
      if (!sk) throw new UserError('Enter your Secret Key. It is on your Emergency Kit.');
      const token = await this.engine.unlock(fromB64(header), password, sk, await this.#deviceId());
      if (token) await this.session.set('resume', toB64(token));
    } catch (e) {
      throw friendly(e, !!secretKey);
    }
    if (secretKey) await this.local.set(K.secretKey, sk);
    await this.#loadLocal();
  }

  async lock(): Promise<void> {
    await this.engine.lock();
    await this.session.remove('resume');
  }

  /** Checks the master password without changing state (e.g. before showing the Secret Key). */
  async verifyPassword(password: string): Promise<boolean> {
    const header = await this.local.get<string>(K.header);
    const sk = await this.local.get<string>(K.secretKey);
    if (!header || !sk) return false;
    try {
      // Unlocking again re-opens the same vault; reload its contents afterwards.
      const token = await this.engine.unlock(fromB64(header), password, sk, await this.#deviceId());
      if (token) await this.session.set('resume', toB64(token));
      await this.#loadLocal();
      return true;
    } catch {
      return false;
    }
  }

  async secretKey(password: string): Promise<string> {
    if (!(await this.verifyPassword(password))) throw new UserError('That password is not right.');
    return (await this.local.get<string>(K.secretKey))!;
  }

  async changePassword(current: string, next: string): Promise<void> {
    if (!(await this.verifyPassword(current))) throw new UserError('Your current password is not right.');
    const header = fromB64((await this.local.get<string>(K.header))!);
    const sk = (await this.local.get<string>(K.secretKey))!;
    const updated = await this.engine.changePassword(header, next, sk, await this.engine.defaultKdf(), this.now(), await this.#deviceId());
    await this.local.set(K.header, toB64(updated));
  }

  // --- Items -------------------------------------------------------------------------------

  async list(): Promise<ItemSummary[]> {
    await this.#open();
    return (await this.engine.list()).sort((a, b) => a.title.localeCompare(b.title, undefined, { sensitivity: 'base' }));
  }

  async view(id: string, revealed: Record<string, string> = {}): Promise<ItemView> {
    await this.#open();
    const v = await this.engine.view(id);
    const me = await this.#deviceId();
    const devices = (await this.local.get<Record<string, string>>(K.devices)) ?? {};
    const deviceName = v.edited_device === me ? 'on this device' : `on ${devices[v.edited_device] ?? 'another device'}`;
    const totp = v.secrets.includes('totp') ? await this.totp(id).catch(() => undefined) : undefined;
    return itemView(v, { now: this.now(), deviceName, revealed, totp });
  }

  async reveal(id: string, field: string): Promise<string> {
    await this.#open();
    return this.engine.reveal(id, field);
  }

  async revealHistory(id: string, index: number): Promise<string> {
    await this.#open();
    return this.engine.revealHistory(id, index);
  }

  async draft(id: string): Promise<ItemDraft> {
    await this.#open();
    return draftFromItem(await this.engine.draft(id));
  }

  newDraft(kind: ItemKind = 'login', pageUrl?: string): ItemDraft {
    const d = emptyDraft(kind);
    if (pageUrl) {
      try {
        const u = new URL(pageUrl);
        if (u.protocol === 'https:' || u.protocol === 'http:') {
          d.urls = u.origin;
          d.title = u.hostname.replace(/^www\./, '');
        }
      } catch {
        // not a web page; leave blank
      }
    }
    return d;
  }

  /** Creates (`id` null) or updates an item. Returns its id. */
  async save(id: string | null, draft: ItemDraft): Promise<string> {
    await this.#open();
    const original = id ? await this.engine.draft(id) : undefined;
    const saved = await this.engine.upsert(id, itemFromDraft(draft, original), this.now());
    await this.#persist();
    return saved;
  }

  async setFavorite(id: string, favorite: boolean): Promise<void> {
    await this.#open();
    const item = await this.engine.draft(id);
    await this.engine.upsert(id, { ...item, favorite }, this.now());
    await this.#persist();
  }

  async remove(id: string): Promise<void> {
    await this.#open();
    await this.engine.remove(id, this.now());
    await this.#persist();
  }

  /** Logins for this page, exact host first. */
  async matches(pageUrl: string): Promise<ItemSummary[]> {
    await this.#open();
    const ids = await this.engine.matches(pageUrl);
    const all = new Map((await this.engine.list()).map((s) => [s.id, s]));
    return ids.flatMap((m) => all.get(m.id) ?? []);
  }

  async credentialsFor(id: string, pageUrl: string) {
    await this.#open();
    return this.engine.credentialsFor(id, pageUrl);
  }

  async totp(id: string) {
    await this.#open();
    return this.engine.totp(id, Math.floor(this.now() / 1000));
  }

  generate(options: GeneratorOptions) {
    return this.engine.generate(options);
  }

  // --- Import / export -------------------------------------------------------------------

  /** Bitwarden JSON or any supported CSV, detected from the content. */
  async importText(text: string): Promise<{ added: number; skipped: number }> {
    await this.#open();
    const trimmed = text.trimStart();
    const parsed = trimmed.startsWith('{')
      ? await this.engine.importBitwardenJson(text)
      : await this.engine.importCsv(text);
    return { added: await this.#add(parsed.items), skipped: parsed.skipped };
  }

  async importEncrypted(bytes: Uint8Array, password: string): Promise<{ added: number; skipped: number }> {
    await this.#open();
    let items: CoreItem[];
    try {
      items = await this.engine.openEncryptedExport(bytes, password);
    } catch (e) {
      throw e instanceof Error && e.message.includes('wrong') ? new UserError('That export password is not right.') : e;
    }
    return { added: await this.#add(items), skipped: 0 };
  }

  async exportCsv() {
    await this.#open();
    return this.engine.exportCsv();
  }

  async exportEncrypted(password: string): Promise<Uint8Array> {
    await this.#open();
    return this.engine.exportEncrypted(password, await this.engine.defaultKdf());
  }

  // --- Settings ----------------------------------------------------------------------------

  async settings(): Promise<Settings> {
    return { ...DEFAULT_SETTINGS, ...(await this.local.get<Partial<Settings>>(K.settings)) };
  }

  async updateSettings(patch: Partial<Settings>): Promise<Settings> {
    const next = { ...(await this.settings()), ...patch };
    await this.local.set(K.settings, next);
    return next;
  }

  // --- Internals ---------------------------------------------------------------------------

  async #open() {
    if (!(await this.ensureOpen())) throw new UserError('The vault is locked.');
  }

  async #add(items: CoreItem[]) {
    if (items.length === 0) return 0;
    const n = await this.engine.addItems(items, this.now());
    await this.#persist();
    return n;
  }

  async #deviceId(): Promise<string> {
    let id = await this.local.get<string>(K.deviceId);
    if (!id) {
      id = await this.engine.randomId();
      await this.local.set(K.deviceId, id);
    }
    return id;
  }

  async #loadLocal() {
    const guard = await this.local.get<string>(K.guard);
    if (guard) await this.engine.setGuard(guard);
    const snapshot = await this.local.get<string>(K.snapshot);
    if (snapshot) await this.engine.mergeSnapshot(fromB64(snapshot));
  }

  /** Seals this device's vault into local storage (and, from Phase 4, uploads it). */
  async #persist() {
    await this.engine.compact(this.now(), TOMBSTONE_MAX_AGE_MS);
    const seq = ((await this.local.get<number>(K.seq)) ?? 0) + 1;
    const bytes = await this.engine.seal(seq, this.deviceName);
    await this.local.set(K.snapshot, toB64(bytes));
    await this.local.set(K.seq, seq);
    await this.local.set(K.guard, await this.engine.guardJson());
  }
}
