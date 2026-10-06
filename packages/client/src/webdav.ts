import { AuthError, ConflictError, type Fetch, type RemoteEntry, type RemoteStore } from './remote';

/**
 * WebDAV (RFC 4918): Nextcloud, ownCloud, Synology, Apache mod_dav, rclone serve webdav…
 * Uses PROPFIND Depth: 1 recursively (many servers refuse Depth: infinity) and If-Match /
 * If-None-Match for conditional writes. Weak ETags are never used as preconditions.
 */
export class WebDavStore implements RemoteStore {
  readonly name: string;
  readonly #base: URL;
  readonly #auth: string;

  constructor(
    baseUrl: string,
    username: string,
    password: string,
    private readonly fetchFn: Fetch = (input, init) => fetch(input, init),
  ) {
    this.#base = new URL(baseUrl.endsWith('/') ? baseUrl : `${baseUrl}/`);
    if (this.#base.protocol !== 'https:' && !['localhost', '127.0.0.1'].includes(this.#base.hostname)) {
      throw new Error('Use an https:// address. Scytale only sends encrypted files, but your login must not travel in clear text.');
    }
    this.name = this.#base.hostname;
    this.#auth = `Basic ${btoa(String.fromCharCode(...new TextEncoder().encode(`${username}:${password}`)))}`;
  }

  #url(path: string) {
    return new URL(path.split('/').map(encodeURIComponent).join('/'), this.#base);
  }

  async #req(method: string, path: string, init: RequestInit = {}) {
    const res = await this.fetchFn(this.#url(path), {
      ...init,
      method,
      headers: { Authorization: this.#auth, ...(init.headers as Record<string, string>) },
    });
    if (res.status === 401 || res.status === 403) throw new AuthError('The WebDAV server rejected the username or app password.');
    return res;
  }

  async list(dir: string): Promise<RemoteEntry[]> {
    const res = await this.#req('PROPFIND', `${dir}/`, {
      headers: { Depth: '1', 'Content-Type': 'application/xml' },
      body: '<?xml version="1.0"?><d:propfind xmlns:d="DAV:"><d:prop><d:getetag/><d:resourcetype/></d:prop></d:propfind>',
    });
    if (res.status === 404) return [];
    if (res.status !== 207) throw new Error(`WebDAV listing failed (${res.status}).`);
    const self = this.#url(`${dir}/`).pathname;
    const out: RemoteEntry[] = [];
    for (const r of parseMultistatus(await res.text())) {
      const pathname = new URL(r.href, this.#base).pathname;
      if (pathname === self || pathname === self.slice(0, -1)) continue;
      const rel = decodeURIComponent(pathname.slice(this.#base.pathname.length)).replace(/\/$/, '');
      if (r.collection) out.push(...(await this.list(rel)));
      else out.push({ path: rel, etag: r.etag });
    }
    return out;
  }

  async get(path: string) {
    const res = await this.#req('GET', path);
    if (res.status === 404) return null;
    if (!res.ok) throw new Error(`WebDAV download failed (${res.status}).`);
    return { bytes: new Uint8Array(await res.arrayBuffer()), etag: res.headers.get('ETag') ?? '' };
  }

  async put(path: string, bytes: Uint8Array, ifMatch?: string | null): Promise<string> {
    const headers: Record<string, string> = { 'Content-Type': 'application/octet-stream' };
    if (ifMatch === null) headers['If-None-Match'] = '*';
    else if (ifMatch && !ifMatch.startsWith('W/')) headers['If-Match'] = ifMatch;
    let res = await this.#req('PUT', path, { headers, body: new Uint8Array(bytes) });
    if (res.status === 409 || res.status === 404) {
      // Parent folder missing: create each level, then retry once.
      const parts = path.split('/').slice(0, -1);
      for (let i = 1; i <= parts.length; i++) await this.#req('MKCOL', `${parts.slice(0, i).join('/')}/`);
      res = await this.#req('PUT', path, { headers, body: new Uint8Array(bytes) });
    }
    if (res.status === 412) throw new ConflictError('The file changed on the server.');
    if (!res.ok) throw new Error(`WebDAV upload failed (${res.status}).`);
    return res.headers.get('ETag') ?? res.headers.get('OC-ETag') ?? (await this.get(path))?.etag ?? '';
  }
}

/**
 * Minimal DAV:multistatus reader. Not DOMParser, because Chrome's extension service worker has none.
 * One linear pass over the tags (no backtracking regex), since the reply comes from a server the
 * user chose but we do not trust. Namespace prefixes vary by server (d:, D:, none) and are ignored.
 */
export function parseMultistatus(xml: string): { href: string; etag: string; collection: boolean }[] {
  const out: { href: string; etag: string; collection: boolean }[] = [];
  let current: { href: string; etag: string; collection: boolean } | null = null;
  let capture: 'href' | 'getetag' | null = null;
  let textStart = 0;
  let inResourceType = false;
  // indexOf scanning: linear in the input by construction, whatever the server sends.
  for (let lt = xml.indexOf('<'); lt !== -1; ) {
    const gt = xml.indexOf('>', lt + 1);
    if (gt === -1) break;
    const raw = xml.slice(lt + 1, gt);
    const closing = raw.startsWith('/');
    const body = closing ? raw.slice(1) : raw;
    const selfClosing = body.endsWith('/');
    const qname = body.split(/[\s/]/, 1)[0] ?? '';
    const name = qname.slice(qname.indexOf(':') + 1).toLowerCase();
    if (!closing) {
      if (name === 'response') current = { href: '', etag: '', collection: false };
      else if (current && (name === 'href' || name === 'getetag') && !selfClosing) {
        capture = name;
        textStart = gt + 1;
      } else if (current && name === 'resourcetype') inResourceType = !selfClosing;
      else if (current && name === 'collection' && inResourceType) current.collection = true;
    } else if (current && capture === name) {
      const text = xmlDecode(xml.slice(textStart, lt).trim());
      if (name === 'href') current.href = text;
      else current.etag = text;
      capture = null;
    } else if (name === 'resourcetype') inResourceType = false;
    else if (name === 'response' && current) {
      out.push(current);
      current = null;
    }
    lt = xml.indexOf('<', gt + 1);
  }
  return out;
}

/** Decodes XML entities in one pass, so "&amp;lt;" becomes "&lt;", not "<". */
function xmlDecode(s: string): string {
  const named: Record<string, string> = { quot: '"', amp: '&', lt: '<', gt: '>', apos: "'" };
  return s.replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (whole, ent: string) => {
    if (ent[0] !== '#') return named[ent.toLowerCase()] ?? whole;
    const code = ent[1] === 'x' || ent[1] === 'X' ? parseInt(ent.slice(2), 16) : parseInt(ent.slice(1), 10);
    return Number.isFinite(code) && code <= 0x10ffff ? String.fromCodePoint(code) : whole;
  });
}
