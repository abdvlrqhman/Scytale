import type { KeyValueStore } from '@scytale/client';

/**
 * `local`: on disk, survives restarts. `session`: memory only, cleared when the browser closes,
 * and hidden from content scripts (Chrome's default access level; Firefox never exposes it).
 */
export class BrowserStore implements KeyValueStore {
  constructor(private readonly area: 'local' | 'session') {}

  async get<T>(key: string) {
    const r = await browser.storage[this.area].get(key);
    return r[key] as T | undefined;
  }
  async set(key: string, value: unknown) {
    await browser.storage[this.area].set({ [key]: value });
  }
  async remove(key: string) {
    await browser.storage[this.area].remove(key);
  }
}
