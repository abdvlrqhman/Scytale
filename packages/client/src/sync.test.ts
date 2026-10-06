// Two devices syncing through a real (in-process) WebDAV server, plus Dropbox request shapes.
import { readFileSync } from 'node:fs';
import { createServer, type Server } from 'node:http';
import type { AddressInfo } from 'node:net';
import * as wasm from '@scytale/core-wasm';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { DropboxStore } from './dropbox';
import { emptyDraft } from './mapping';
import { AuthError, ConflictError, type RemoteConfig } from './remote';
import { VaultService } from './service';
import { MemoryStore } from './store';
import { WasmEngine } from './wasm-engine';
import { WebDavStore, parseMultistatus } from './webdav';

// --- A minimal WebDAV server: PROPFIND (Depth 1), GET, PUT (+preconditions), MKCOL ----------
let server: Server;
let base: string;
const files = new Map<string, { bytes: Buffer; etag: string }>();
const dirs = new Set<string>(['/dav/']);
let etagCounter = 0;
const AUTH = `Basic ${Buffer.from('sam:app-password').toString('base64')}`;

beforeAll(async () => {
  wasm.initSync({ module: readFileSync(new URL('../../core-wasm/pkg/scytale_core_wasm_bg.wasm', import.meta.url)) });
  server = createServer(async (req, res) => {
    const chunks: Buffer[] = [];
    for await (const c of req) chunks.push(c as Buffer);
    const body = Buffer.concat(chunks);
    const path = decodeURIComponent(new URL(req.url!, 'http://x').pathname);
    if (req.headers.authorization !== AUTH) return void res.writeHead(401).end();
    const parent = path.replace(/[^/]+\/?$/, '');
    switch (req.method) {
      case 'PROPFIND': {
        if (!dirs.has(path)) return void res.writeHead(404).end();
        const rows = [`<d:response><d:href>${encodeURI(path)}</d:href><d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop></d:propstat></d:response>`];
        for (const d of dirs) if (d !== path && d.startsWith(path) && !d.slice(path.length, -1).includes('/'))
          rows.push(`<d:response><d:href>${encodeURI(d)}</d:href><d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop></d:propstat></d:response>`);
        for (const [f, v] of files) if (f.startsWith(path) && !f.slice(path.length).includes('/'))
          rows.push(`<d:response><d:href>${encodeURI(f)}</d:href><d:propstat><d:prop><d:getetag>${v.etag.replace(/"/g, '&quot;')}</d:getetag><d:resourcetype/></d:prop></d:propstat></d:response>`);
        return void res.writeHead(207, { 'Content-Type': 'application/xml' }).end(`<?xml version="1.0"?><d:multistatus xmlns:d="DAV:">${rows.join('')}</d:multistatus>`);
      }
      case 'GET': {
        const f = files.get(path);
        return f ? void res.writeHead(200, { ETag: f.etag }).end(f.bytes) : void res.writeHead(404).end();
      }
      case 'PUT': {
        if (!dirs.has(parent)) return void res.writeHead(409).end();
        const existing = files.get(path);
        if (req.headers['if-none-match'] === '*' && existing) return void res.writeHead(412).end();
        if (req.headers['if-match'] && req.headers['if-match'] !== existing?.etag) return void res.writeHead(412).end();
        const etag = `"e${++etagCounter}"`;
        files.set(path, { bytes: body, etag });
        return void res.writeHead(existing ? 204 : 201, { ETag: etag }).end();
      }
      case 'MKCOL':
        if (dirs.has(path)) return void res.writeHead(405).end();
        if (!dirs.has(parent)) return void res.writeHead(409).end();
        dirs.add(path);
        return void res.writeHead(201).end();
    }
    res.writeHead(405).end();
  });
  await new Promise<void>((r) => server.listen(0, '127.0.0.1', r));
  base = `http://127.0.0.1:${(server.address() as AddressInfo).port}/dav/`;
});

afterAll(() => server?.close());

let clock = Date.UTC(2026, 9, 6, 12);
const now = () => (clock += 1000);
const remoteFor = (c: RemoteConfig) => {
  if (c.kind !== 'webdav') throw new Error('test only uses WebDAV');
  return new WebDavStore(c.url, c.username, c.password);
};
const device = (name: string) => new VaultService(new WasmEngine(wasm), new MemoryStore(), new MemoryStore(), name, { now, remoteFor });
const webdav = (): RemoteConfig => ({ kind: 'webdav', url: base, username: 'sam', password: 'app-password' });
const login = (title: string, password: string) => ({ ...emptyDraft('login'), title, password, urls: `https://${title.toLowerCase()}.example` });
const titles = async (svc: VaultService) => (await svc.list()).map((i) => i.title).sort();

describe('WebDAV sync between two devices', () => {
  it('joins, converges after concurrent edits, and spreads a password change', async () => {
    const a = device('Brave on Windows');
    const sk = await a.create('shared master password');
    const mail = await a.save(null, login('Mail', 'p1'));
    expect((await a.connect(webdav())).status).toBe('ok');
    expect([...files.keys()].some((f) => f.endsWith('header.scyh'))).toBe(true);

    const b = device('Firefox on Linux');
    await b.join(webdav());
    expect(await b.status()).toBe('locked');
    expect(await b.hasSecretKey()).toBe(false);
    await b.unlock('shared master password', sk);
    await b.sync();
    expect(await titles(b)).toEqual(['Mail']);

    // Concurrent, offline edits on both devices.
    await a.save(mail, { ...(await a.draft(mail)), password: 'p2-from-a' });
    await b.save(null, login('Bank', 'b1'));
    await a.sync();
    await b.sync();
    await a.sync();
    expect(await titles(a)).toEqual(['Bank', 'Mail']);
    expect(await titles(b)).toEqual(['Bank', 'Mail']);
    expect(await b.reveal(mail, 'password')).toBe('p2-from-a');
    expect((await a.devices()).map((d) => d.name)).toEqual(['Brave on Windows', 'Firefox on Linux']);

    // A master-password change reaches B through the header; the vault key is unchanged.
    await a.changePassword('shared master password', 'brand new master password');
    await a.sync();
    await b.sync();
    await b.lock();
    await expect(b.unlock('shared master password')).rejects.toThrow();
    await b.unlock('brand new master password');
    expect(await titles(b)).toEqual(['Bank', 'Mail']);
  });

  it('rejects wrong WebDAV credentials before saving anything', async () => {
    const c = device('Edge on Windows');
    await c.create('another vault password');
    await expect(c.connect({ ...webdav(), password: 'wrong' } as RemoteConfig)).rejects.toBeInstanceOf(AuthError);
    expect(await c.remoteConfig()).toBeUndefined();
  });

  it('refuses plain http to anything but localhost', () => {
    expect(() => new WebDavStore('http://cloud.example.com/dav', 'a', 'b')).toThrow('https');
  });
});

describe('parseMultistatus', () => {
  it('reads Nextcloud-style responses with any namespace prefix', () => {
    const xml = `<?xml version="1.0"?><d:multistatus xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns">
      <d:response><d:href>/remote.php/dav/files/sam/scytale/</d:href><d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop></d:propstat></d:response>
      <d:response><d:href>/remote.php/dav/files/sam/scytale/a%20b.scyv</d:href><d:propstat><d:prop><d:getetag>&quot;5f1&quot;</d:getetag><d:resourcetype/></d:prop></d:propstat></d:response>
    </d:multistatus>`;
    expect(parseMultistatus(xml)).toEqual([
      { href: '/remote.php/dav/files/sam/scytale/', etag: '', collection: true },
      { href: '/remote.php/dav/files/sam/scytale/a%20b.scyv', etag: '"5f1"', collection: false },
    ]);
    // Entities decode exactly once.
    expect(parseMultistatus('<d:response><d:href>/a&amp;lt;b</d:href></d:response>')[0]?.href).toBe('/a&lt;b');
    // Hostile input stays linear: a long run of unclosed tags parses instantly.
    const t0 = performance.now();
    parseMultistatus('<d:response><d:href>' + '<x:'.repeat(50_000));
    expect(performance.now() - t0).toBeLessThan(500);
    expect(parseMultistatus('<D:multistatus xmlns:D="DAV:"><D:response><D:href>/x</D:href><D:getetag>W/"1"</D:getetag></D:response></D:multistatus>')).toEqual([
      { href: '/x', etag: 'W/"1"', collection: false },
    ]);
  });
});

describe('DropboxStore', () => {
  it('uploads with update+rev and strict_conflict, and maps 409 to a conflict', async () => {
    const calls: { url: string; init: RequestInit }[] = [];
    const fake = (async (url: string, init: RequestInit) => {
      calls.push({ url, init });
      if (url.endsWith('/oauth2/token')) return Response.json({ access_token: 'at', expires_in: 14400 });
      if (url.endsWith('/files/upload') && calls.length > 3) return new Response('{}', { status: 409 });
      return Response.json({ rev: 'r2' });
    }) as typeof fetch;
    const store = new DropboxStore('client-id', 'refresh', fake);
    expect(await store.put('scytale/v/header.scyh', new Uint8Array([1]), 'r1')).toBe('r2');
    const arg = JSON.parse((calls[1]!.init.headers as Record<string, string>)['Dropbox-API-Arg']!);
    expect(arg).toEqual({ path: '/scytale/v/header.scyh', mode: { '.tag': 'update', update: 'r1' }, autorename: false, strict_conflict: true, mute: true });
    expect(String(calls[0]!.init.body)).toContain('grant_type=refresh_token');
    await store.put('x', new Uint8Array([1]), null);
    await expect(store.put('x', new Uint8Array([1]), null)).rejects.toBeInstanceOf(ConflictError);
  });
});
