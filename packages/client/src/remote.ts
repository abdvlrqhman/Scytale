// Storage the user already owns. Only ciphertext ever goes through these adapters.

export interface RemoteEntry {
  /** Relative to the store root, e.g. "scytale/<vault id>/devices/<device id>.scyv". */
  path: string;
  etag: string;
}

/** A conditional write lost a race; re-read and try again. */
export class ConflictError extends Error {}

/** The provider rejected the credentials (password changed, access revoked): reconnect. */
export class AuthError extends Error {}

export interface RemoteStore {
  /** Human name for Settings, e.g. "Dropbox" or "cloud.example.com". */
  readonly name: string;
  /** Files (not folders) under `dir`, recursively. An absent folder is an empty list. */
  list(dir: string): Promise<RemoteEntry[]>;
  get(path: string): Promise<{ bytes: Uint8Array; etag: string } | null>;
  /**
   * `ifMatch`: undefined overwrites, `null` writes only if absent, a string only if unchanged.
   * Creates missing parent folders. Returns the new etag.
   */
  put(path: string, bytes: Uint8Array, ifMatch?: string | null): Promise<string>;
}

export type RemoteConfig =
  | { kind: 'webdav'; url: string; username: string; password: string }
  | { kind: 'dropbox'; refreshToken: string };

/** `fetch`, injectable: the desktop app routes requests through Rust to avoid CORS. */
export type Fetch = typeof fetch;
