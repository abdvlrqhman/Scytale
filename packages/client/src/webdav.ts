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
 * Minimal DAV:multistatus reader. Regex, not DOMParser, because Chrome's extension service worker
 * has no DOMParser. Namespace prefixes vary by server (d:, D:, none), so they are ignored.
 */
export function parseMultistatus(xml: string): { href: string; etag: string; collection: boolean }[] {
  const tag = (name: string) => new RegExp(`<(?:[\\w-]+:)?${name}(?:\\s[^>]*)?>([\\s\\S]*?)</(?:[\\w-]+:)?${name}>`, 'i');
  const responses = xml.match(/<(?:[\w-]+:)?response(?:\s[^>]*)?>[\s\S]*?<\/(?:[\w-]+:)?response>/gi) ?? [];
  return responses.map((r) => {
    const xmlDecode = (s: string) => s.replace(/&quot;/g, '"').replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>');
    const href = xmlDecode(tag('href').exec(r)?.[1]?.trim() ?? '');
    const etag = xmlDecode(tag('getetag').exec(r)?.[1]?.trim() ?? '');
    const collection = /<(?:[\w-]+:)?collection\s*\/?>/i.test(tag('resourcetype').exec(r)?.[1] ?? '');
    return { href, etag, collection };
  });
}
