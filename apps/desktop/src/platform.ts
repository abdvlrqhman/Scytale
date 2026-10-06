import {
  DropboxStore,
  type KeyValueStore,
  type RemoteConfig,
  type RemoteEntry,
  type RemoteStore,
  ConflictError,
  WebDavStore,
} from '@scytale/client';
import { invoke } from '@tauri-apps/api/core';
import { fetch as tauriFetch } from '@tauri-apps/plugin-http';
import { LazyStore } from '@tauri-apps/plugin-store';

/**
 * Settings and the encrypted vault in the app's data folder; the Secret Key in the OS credential
 * store (Keychain, Credential Manager, Secret Service) instead of a plain file.
 */
export class DesktopStore implements KeyValueStore {
  readonly #file = new LazyStore('scytale.json', { defaults: {}, autoSave: 100 });

  async get<T>(key: string) {
    if (key === 'secretKey') return ((await invoke<string | null>('keyring_get', { account: 'secret-key' })) ?? undefined) as T | undefined;
    return this.#file.get<T>(key);
  }
  async set(key: string, value: unknown) {
    if (key === 'secretKey') return invoke<void>('keyring_set', { account: 'secret-key', value: String(value) });
    await this.#file.set(key, value);
  }
  async remove(key: string) {
    if (key === 'secretKey') return invoke<void>('keyring_delete', { account: 'secret-key' });
    await this.#file.delete(key);
  }
}

/** Storage in a folder another app keeps in sync. All file access is done (and confined) in Rust. */
export class FolderStore implements RemoteStore {
  readonly name: string;
  constructor(private readonly root: string) {
    this.name = root.split(/[\\/]/).filter(Boolean).pop() ?? root;
  }
  list(dir: string): Promise<RemoteEntry[]> {
    return invoke('folder_list', { root: this.root, dir });
  }
  async get(path: string) {
    const buf = await invoke<number[] | null>('folder_get', { root: this.root, path });
    if (!buf) return null;
    const entry = (await this.list(path.split('/').slice(0, -1).join('/'))).find((e) => e.path === path);
    return { bytes: new Uint8Array(buf), etag: entry?.etag ?? '' };
  }
  async put(path: string, bytes: Uint8Array, ifMatch?: string | null) {
    try {
      return await invoke<string>('folder_put', {
        root: this.root,
        path,
        bytes: Array.from(bytes),
        ifMatch: ifMatch === null ? '' : ifMatch,
      });
    } catch (e) {
      throw String(e) === 'conflict' ? new ConflictError('The file changed in the folder.') : e;
    }
  }
}

/** WebDAV and Dropbox go through Rust's HTTP client, so servers need no CORS setup. */
export function remoteFor(c: RemoteConfig): RemoteStore {
  if (c.kind === 'folder') return new FolderStore(c.path);
  if (c.kind === 'webdav') return new WebDavStore(c.url, c.username, c.password, tauriFetch);
  return new DropboxStore(import.meta.env.VITE_DROPBOX_CLIENT_ID ?? '', c.refreshToken, tauriFetch);
}

export function copyText(text: string, clearAfterSecs: number) {
  return invoke<void>('copy_text', { text, clearAfterSecs });
}
