import { AuthError, ConflictError, type Fetch, type RemoteEntry, type RemoteStore } from './remote';

// Dropbox, "App folder" access: the app sees only Apps/Scytale. OAuth 2 with PKCE and an offline
// refresh token; no client secret exists anywhere. Docs: https://developers.dropbox.com/oauth-guide
const API = 'https://api.dropboxapi.com';
const CONTENT = 'https://content.dropboxapi.com';

export function dropboxAuthorizeUrl(clientId: string, redirectUri: string, codeChallenge: string, state: string) {
  const u = new URL('https://www.dropbox.com/oauth2/authorize');
  u.search = new URLSearchParams({
    client_id: clientId,
    response_type: 'code',
    code_challenge: codeChallenge,
    code_challenge_method: 'S256',
    token_access_type: 'offline',
    redirect_uri: redirectUri,
    state,
  }).toString();
  return u.toString();
}

/** Exchanges the authorization code (PKCE) for a long-lived refresh token. */
export async function dropboxExchange(
  clientId: string,
  code: string,
  verifier: string,
  redirectUri: string,
  fetchFn: Fetch = (input, init) => fetch(input, init),
): Promise<string> {
  const res = await fetchFn(`${API}/oauth2/token`, {
    method: 'POST',
    body: new URLSearchParams({ code, grant_type: 'authorization_code', client_id: clientId, code_verifier: verifier, redirect_uri: redirectUri }),
  });
  if (!res.ok) throw new AuthError('Dropbox did not accept the sign-in. Try connecting again.');
  const json = (await res.json()) as { refresh_token?: string };
  if (!json.refresh_token) throw new AuthError('Dropbox did not return offline access.');
  return json.refresh_token;
}

/** PKCE verifier and S256 challenge (RFC 7636). */
export async function pkcePair(): Promise<{ verifier: string; challenge: string }> {
  const b64url = (bytes: Uint8Array) => btoa(String.fromCharCode(...bytes)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  const verifier = b64url(crypto.getRandomValues(new Uint8Array(32)));
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier)));
  return { verifier, challenge: b64url(digest) };
}

export class DropboxStore implements RemoteStore {
  readonly name = 'Dropbox';
  #access: { token: string; expires: number } | null = null;

  constructor(
    private readonly clientId: string,
    private readonly refreshToken: string,
    private readonly fetchFn: Fetch = (input, init) => fetch(input, init),
  ) {}

  async #token(): Promise<string> {
    if (this.#access && Date.now() < this.#access.expires - 60_000) return this.#access.token;
    const res = await this.fetchFn(`${API}/oauth2/token`, {
      method: 'POST',
      body: new URLSearchParams({ grant_type: 'refresh_token', refresh_token: this.refreshToken, client_id: this.clientId }),
    });
    if (!res.ok) throw new AuthError('Dropbox access was revoked or expired. Connect Dropbox again.');
    const json = (await res.json()) as { access_token: string; expires_in: number };
    this.#access = { token: json.access_token, expires: Date.now() + json.expires_in * 1000 };
    return json.access_token;
  }

  async #rpc(endpoint: string, body: unknown) {
    const res = await this.fetchFn(`${API}/2/${endpoint}`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${await this.#token()}`, 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (res.status === 401) throw new AuthError('Dropbox access was revoked. Connect Dropbox again.');
    return res;
  }

  async list(dir: string): Promise<RemoteEntry[]> {
    let res = await this.#rpc('files/list_folder', { path: `/${dir}`, recursive: true });
    if (res.status === 409) return []; // path/not_found
    const out: RemoteEntry[] = [];
    for (;;) {
      if (!res.ok) throw new Error(`Dropbox listing failed (${res.status}).`);
      const page = (await res.json()) as {
        entries: { '.tag': string; path_display: string; rev?: string }[];
        has_more: boolean;
        cursor: string;
      };
      for (const e of page.entries) {
        if (e['.tag'] === 'file' && e.rev) out.push({ path: e.path_display.replace(/^\//, ''), etag: e.rev });
      }
      if (!page.has_more) return out;
      res = await this.#rpc('files/list_folder/continue', { cursor: page.cursor });
    }
  }

  async get(path: string) {
    const res = await this.fetchFn(`${CONTENT}/2/files/download`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${await this.#token()}`, 'Dropbox-API-Arg': JSON.stringify({ path: `/${path}` }) },
    });
    if (res.status === 409) return null;
    if (res.status === 401) throw new AuthError('Dropbox access was revoked. Connect Dropbox again.');
    if (!res.ok) throw new Error(`Dropbox download failed (${res.status}).`);
    const meta = JSON.parse(res.headers.get('Dropbox-API-Result') ?? '{}') as { rev?: string };
    return { bytes: new Uint8Array(await res.arrayBuffer()), etag: meta.rev ?? '' };
  }

  async put(path: string, bytes: Uint8Array, ifMatch?: string | null): Promise<string> {
    // update+rev with strict_conflict: fail instead of creating "conflicted copy" files.
    const mode = ifMatch === null ? 'add' : ifMatch ? { '.tag': 'update', update: ifMatch } : 'overwrite';
    const res = await this.fetchFn(`${CONTENT}/2/files/upload`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${await this.#token()}`,
        'Content-Type': 'application/octet-stream',
        'Dropbox-API-Arg': JSON.stringify({ path: `/${path}`, mode, autorename: false, strict_conflict: true, mute: true }),
      },
      body: new Uint8Array(bytes),
    });
    if (res.status === 409) throw new ConflictError('The file changed in Dropbox.');
    if (res.status === 401) throw new AuthError('Dropbox access was revoked. Connect Dropbox again.');
    if (!res.ok) throw new Error(`Dropbox upload failed (${res.status}).`);
    return ((await res.json()) as { rev: string }).rev;
  }
}
