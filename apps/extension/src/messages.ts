// The popup and vault page talk to the background worker with these messages. Only extension pages
// may send them (the background checks the sender).
import type { Settings, Status } from '@scytale/client';
import type { GeneratorOptions, ItemDraft, ItemKind, ItemSummary, ItemView } from '@scytale/ui/types';

export type Request =
  | { type: 'status' }
  | { type: 'create'; password: string }
  | { type: 'unlock'; password: string; secretKey?: string }
  | { type: 'lock' }
  | { type: 'list' }
  | { type: 'tabMatches' }
  | { type: 'view'; id: string; revealed: Record<string, string> }
  | { type: 'reveal'; id: string; field: string }
  | { type: 'draft'; id: string }
  | { type: 'newDraft'; kind: ItemKind; forTab: boolean }
  | { type: 'save'; id: string | null; draft: ItemDraft }
  | { type: 'remove'; id: string }
  | { type: 'favorite'; id: string; favorite: boolean }
  | { type: 'fill'; id: string }
  | { type: 'totp'; id: string }
  | { type: 'generate'; options: GeneratorOptions }
  | { type: 'importText'; text: string }
  | { type: 'importEncrypted'; b64: string; password: string }
  | { type: 'exportCsv' }
  | { type: 'exportEncrypted'; password: string }
  | { type: 'settings' }
  | { type: 'updateSettings'; patch: Partial<Settings> }
  | { type: 'secretKey'; password: string }
  | { type: 'changePassword'; current: string; next: string }
  | { type: 'copied' }
  | { type: 'openVaultPage'; hash: string };

export interface StatusReply {
  status: Status;
  /** True when this device has never opened the vault (the Secret Key must be typed). */
  needsSecretKey: boolean;
  settings: Settings;
}

export interface TabMatches {
  site?: string;
  matches: ItemSummary[];
}

/** What each request resolves to. */
export interface Replies {
  status: StatusReply;
  create: { secretKey: string };
  unlock: void;
  lock: void;
  list: ItemSummary[];
  tabMatches: TabMatches;
  view: ItemView;
  reveal: string;
  draft: ItemDraft;
  newDraft: ItemDraft;
  save: string;
  remove: void;
  favorite: void;
  fill: void;
  totp: { code: string; remaining: number; period: number };
  generate: { value: string; bits: number };
  importText: { added: number; skipped: number };
  importEncrypted: { added: number; skipped: number };
  exportCsv: { csv: string; omitted: number };
  exportEncrypted: string;
  settings: Settings;
  updateSettings: Settings;
  secretKey: string;
  changePassword: void;
  copied: void;
  openVaultPage: void;
}

export type Reply<T> = { ok: true; data: T } | { ok: false; error: string };

/** Sends a request to the background worker; throws its user-facing error message. */
export async function send<R extends Request>(req: R): Promise<Replies[R['type']]> {
  const reply = (await browser.runtime.sendMessage(req)) as Reply<Replies[R['type']]> | undefined;
  if (!reply) throw new Error('Scytale is starting. Try again in a moment.');
  if (!reply.ok) throw new Error(reply.error);
  return reply.data;
}
