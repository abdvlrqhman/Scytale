// Runs the real WASM core in Node: these tests exercise the same code the extension ships.
import { readFileSync } from 'node:fs';
import * as wasm from '@scytale/core-wasm';
import { beforeAll, describe, expect, it } from 'vitest';
import { emptyDraft } from './mapping';
import { UserError, VaultService } from './service';
import { MemoryStore } from './store';
import { WasmEngine } from './wasm-engine';

beforeAll(() => {
  wasm.initSync({ module: readFileSync(new URL('../../core-wasm/pkg/scytale_core_wasm_bg.wasm', import.meta.url)) });
});

let clock = Date.UTC(2026, 9, 6, 12);
const now = () => (clock += 1000);

function fresh(local = new MemoryStore(), session = new MemoryStore()) {
  return { local, session, svc: new VaultService(new WasmEngine(wasm), local, session, 'Brave on Windows', { now }) };
}

const github = () => ({
  ...emptyDraft('login'),
  title: 'GitHub',
  username: 'sam-rivera',
  password: 'x7#Qm2-vLp9!Tr4@Kc8wZe',
  urls: 'https://github.com',
  totp: 'JBSWY3DPEHPK3PXP',
});

describe('VaultService', () => {
  it('creates, locks and unlocks a vault', async () => {
    const { svc } = fresh();
    expect(await svc.status()).toBe('new');
    const secretKey = await svc.create('correct horse battery staple');
    expect(secretKey).toMatch(/^S1-[0-9A-Z]{5}(-[0-9A-Z]{5}){4}-[0-9A-Z]{3}$/);
    expect(await svc.status()).toBe('unlocked');

    const id = await svc.save(null, github());
    await svc.lock();
    expect(await svc.status()).toBe('locked');
    await expect(svc.list()).rejects.toThrow('locked');

    await expect(svc.unlock('wrong')).rejects.toBeInstanceOf(UserError);
    await svc.unlock('correct horse battery staple');
    expect((await svc.list()).map((i) => i.id)).toEqual([id]);
  });

  it('survives a service-worker restart via the session token, and not a browser restart', async () => {
    const a = fresh();
    await a.svc.create('pw pw pw pw');
    const id = await a.svc.save(null, github());

    // Same stores, brand-new engine and service: what a restarted worker sees.
    const restarted = fresh(a.local, a.session);
    expect(await restarted.svc.status()).toBe('unlocked');
    expect(await restarted.svc.reveal(id, 'password')).toBe('x7#Qm2-vLp9!Tr4@Kc8wZe');

    // Browser restart: session storage is gone, so the vault is locked.
    const rebooted = fresh(a.local, new MemoryStore());
    expect(await rebooted.svc.status()).toBe('locked');
  });

  it('never returns secrets from list or view until revealed', async () => {
    const { svc } = fresh();
    await svc.create('pw pw pw pw');
    const id = await svc.save(null, github());
    expect(JSON.stringify(await svc.list())).not.toContain('x7#Qm2');
    const view = await svc.view(id);
    expect(JSON.stringify(view)).not.toContain('x7#Qm2');
    expect(JSON.stringify(view)).not.toContain('JBSWY3DP');
    expect(view.fields.find((f) => f.key === 'password')).toMatchObject({ secret: true, value: undefined });
    expect(view.totp?.code).toMatch(/^\d{6}$/);
    expect(view.edited).toMatch(/on this device$/);
  });

  it('matches and fills only on the saved site', async () => {
    const { svc } = fresh();
    await svc.create('pw pw pw pw');
    const id = await svc.save(null, github());
    expect((await svc.matches('https://github.com/login')).map((m) => m.id)).toEqual([id]);
    expect(await svc.matches('https://github.com.evil.io/login')).toEqual([]);
    await expect(svc.credentialsFor(id, 'https://gitlab.com')).rejects.toThrow();
    expect(await svc.credentialsFor(id, 'https://github.com/x')).toEqual({
      username: 'sam-rivera',
      password: 'x7#Qm2-vLp9!Tr4@Kc8wZe',
    });
  });

  it('keeps the old password in history when it changes', async () => {
    const { svc } = fresh();
    await svc.create('pw pw pw pw');
    const id = await svc.save(null, github());
    await svc.save(id, { ...(await svc.draft(id)), password: 'new-one' });
    const view = await svc.view(id);
    expect(view.history).toHaveLength(1);
    expect(await svc.revealHistory(id, 0)).toBe('x7#Qm2-vLp9!Tr4@Kc8wZe');
  });

  it('imports CSV and round-trips an encrypted export', async () => {
    const { svc } = fresh();
    await svc.create('pw pw pw pw');
    const r = await svc.importText('name,url,username,password,note\nMail,https://mail.example,me,p1,\n');
    expect(r).toEqual({ added: 1, skipped: 0 });

    const bytes = await svc.exportEncrypted('export password');
    const other = fresh();
    await other.svc.create('another one');
    await expect(other.svc.importEncrypted(bytes, 'nope')).rejects.toBeInstanceOf(UserError);
    expect(await other.svc.importEncrypted(bytes, 'export password')).toEqual({ added: 1, skipped: 0 });
    expect((await other.svc.list())[0]?.title).toBe('Mail');
  });

  it('changes the master password without touching items', async () => {
    const { svc } = fresh();
    await svc.create('old password here');
    await svc.save(null, github());
    await svc.changePassword('old password here', 'new password here');
    await svc.lock();
    await expect(svc.unlock('old password here')).rejects.toThrow();
    await svc.unlock('new password here');
    expect(await svc.list()).toHaveLength(1);
  });

  it('opens on a new device with the Secret Key, and catches typos in it', async () => {
    const a = fresh();
    const sk = await a.svc.create('shared password');
    await a.svc.save(null, github());
    // A second install gets the header + snapshot (sync does this in Phase 4) but no Secret Key.
    const local = new MemoryStore();
    for (const k of ['header', 'vaultId', 'snapshot', 'seq', 'guard']) local.data.set(k, a.local.data.get(k));
    const b = fresh(local);
    await expect(b.svc.unlock('shared password')).rejects.toThrow('Secret Key');
    await expect(b.svc.unlock('shared password', sk.replace(/.$/, sk.endsWith('0') ? '1' : '0'))).rejects.toThrow(
      'Secret Key',
    );
    await b.svc.unlock('shared password', sk.toLowerCase());
    expect(await b.svc.list()).toHaveLength(1);
  });
});
