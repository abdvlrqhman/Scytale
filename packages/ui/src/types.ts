export type ItemKind = 'login' | 'note' | 'card' | 'identity';

/** What a list needs to show an item. Never contains a secret. */
export interface ItemSummary {
  id: string;
  title: string;
  /** Username, "Ending in 4242", "Secure note"… */
  subtitle: string;
  kind: ItemKind;
  favorite?: boolean;
}

export type SyncStatus = 'ok' | 'syncing' | 'offline' | 'error';

export interface SyncInfo {
  status: SyncStatus;
  /** Plain sentence, e.g. "Synced with Dropbox 2 minutes ago". */
  text: string;
}

export type Theme = 'dark' | 'light';

/** One labelled value on an item page. Secret fields arrive without `value` until revealed. */
export interface FieldView {
  key: string;
  label: string;
  value?: string;
  secret?: boolean;
  copyable?: boolean;
  /** Render as a link (websites). */
  href?: string;
}

export interface ItemView {
  id: string;
  title: string;
  kind: ItemKind;
  /** "Edited 3 days ago on Firefox on Linux" */
  edited: string;
  favorite: boolean;
  fields: FieldView[];
  totp?: { code: string; remaining: number; period: number };
  strength?: 'weak' | 'fair' | 'strong';
  history: { label: string; until: string }[];
  notes: string;
}

/** Editable copy of an item. Strings only, so forms bind directly. */
export interface ItemDraft {
  kind: ItemKind;
  title: string;
  notes: string;
  favorite: boolean;
  username: string;
  password: string;
  urls: string;
  totp: string;
  exactHost: boolean;
  holder: string;
  number: string;
  expiry: string;
  cvv: string;
  fullName: string;
  email: string;
  phone: string;
  address: string;
}

export interface DeviceView {
  id: string;
  name: string;
  lastSeen: string;
  current: boolean;
}

export type StorageProvider = 'webdav' | 'dropbox' | 'onedrive' | 'gdrive' | 'folder' | 'none';

export interface GeneratorOptions {
  mode: 'password' | 'passphrase';
  length: number;
  lower: boolean;
  upper: boolean;
  digits: boolean;
  symbols: boolean;
  avoidAmbiguous: boolean;
  words: number;
  separator: string;
  capitalize: boolean;
  includeNumber: boolean;
}

export interface Strength {
  score: 0 | 1 | 2 | 3 | 4;
  label: string;
  hint?: string;
}
