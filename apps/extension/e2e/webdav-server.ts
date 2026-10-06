// A tiny in-memory WebDAV server for end-to-end tests (PROPFIND depth 1, GET, PUT, MKCOL).
import { createServer, type Server } from 'node:http';
import type { AddressInfo } from 'node:net';

export async function startWebDav(user: string, pass: string) {
  const files = new Map<string, { bytes: Buffer; etag: string }>();
  const dirs = new Set<string>(['/dav/']);
  let n = 0;
  const auth = `Basic ${Buffer.from(`${user}:${pass}`).toString('base64')}`;
  const entry = (href: string, etag?: string) =>
    `<d:response><d:href>${encodeURI(href)}</d:href><d:propstat><d:prop>${etag ? `<d:getetag>${etag.replace(/"/g, '&quot;')}</d:getetag><d:resourcetype/>` : '<d:resourcetype><d:collection/></d:resourcetype>'}</d:prop></d:propstat></d:response>`;

  const server: Server = createServer(async (req, res) => {
    const chunks: Buffer[] = [];
    for await (const c of req) chunks.push(c as Buffer);
    // Extensions call from their own origin: answer CORS preflights like a real server would.
    res.setHeader('Access-Control-Allow-Origin', '*');
    if (req.method === 'OPTIONS') return void res.writeHead(204).end();
    if (req.headers.authorization !== auth) return void res.writeHead(401).end();
    const path = decodeURIComponent(new URL(req.url!, 'http://x').pathname);
    const parent = path.replace(/[^/]+\/?$/, '');
    if (req.method === 'PROPFIND') {
      if (!dirs.has(path)) return void res.writeHead(404).end();
      const rows = [entry(path)];
      for (const d of dirs) if (d !== path && d.startsWith(path) && !d.slice(path.length, -1).includes('/')) rows.push(entry(d));
      for (const [f, v] of files) if (f.startsWith(path) && !f.slice(path.length).includes('/')) rows.push(entry(f, v.etag));
      return void res.writeHead(207).end(`<?xml version="1.0"?><d:multistatus xmlns:d="DAV:">${rows.join('')}</d:multistatus>`);
    }
    if (req.method === 'GET') {
      const f = files.get(path);
      return f ? void res.writeHead(200, { ETag: f.etag }).end(f.bytes) : void res.writeHead(404).end();
    }
    if (req.method === 'PUT') {
      if (!dirs.has(parent)) return void res.writeHead(409).end();
      const existing = files.get(path);
      if (req.headers['if-none-match'] === '*' && existing) return void res.writeHead(412).end();
      if (req.headers['if-match'] && req.headers['if-match'] !== existing?.etag) return void res.writeHead(412).end();
      const etag = `"e${++n}"`;
      files.set(path, { bytes: Buffer.concat(chunks), etag });
      return void res.writeHead(201, { ETag: etag }).end();
    }
    if (req.method === 'MKCOL') {
      if (dirs.has(path)) return void res.writeHead(405).end();
      dirs.add(path);
      return void res.writeHead(201).end();
    }
    res.writeHead(405).end();
  });
  await new Promise<void>((r) => server.listen(0, r));
  const port = (server.address() as AddressInfo).port;
  return { server, files, url: `http://localhost:${port}/dav/` };
}
