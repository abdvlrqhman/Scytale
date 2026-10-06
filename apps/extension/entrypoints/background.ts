import {
  DropboxStore,
  type RemoteConfig,
  UserError,
  VaultService,
  WasmEngine,
  WebDavStore,
  fromB64,
  toB64,
} from '@scytale/client';
import init, * as wasm from '@scytale/core-wasm';
import wasmUrl from '@scytale/core-wasm/wasm?url';
import { BrowserStore } from '@/src/browser-store';
import { deviceName } from '@/src/device';
import { type FillResult, fillPage } from '@/src/fill';
import type { Reply, Request, StatusReply, SyncState, TabMatches } from '@/src/messages';
import { DROPBOX_CLIENT_ID } from '@/src/providers';

export default defineBackground({
  type: 'module',
  main() {
    const local = new BrowserStore('local');
    const session = new BrowserStore('session');
    const remoteFor = (c: RemoteConfig) => {
      if (c.kind === 'webdav') return new WebDavStore(c.url, c.username, c.password);
      if (c.kind === 'dropbox') return new DropboxStore(DROPBOX_CLIENT_ID, c.refreshToken);
      throw new UserError('Folder sync is only available in the desktop app.');
    };
    const ready: Promise<VaultService> = init({ module_or_path: wasmUrl }).then(
      () =>
        new VaultService(new WasmEngine(wasm), local, session, deviceName(), {
          remoteFor,
          onChange: () => scheduleSync(2000),
        }),
    );

    // --- Sync: 2 s after a change, every 5 minutes, and after unlocking -------------------------
    let syncTimer: ReturnType<typeof setTimeout> | undefined;
    function scheduleSync(ms: number) {
      clearTimeout(syncTimer);
      syncTimer = setTimeout(() => ready.then((s) => s.sync()).catch(() => {}), ms);
    }
    browser.alarms.create('sync', { periodInMinutes: 5 });

    async function syncState(svc: VaultService): Promise<SyncState> {
      const { storage, ...info } = await svc.syncInfo();
      return { info, storage, devices: await svc.devices() };
    }

    // --- Messages from our own pages only -------------------------------------------------
    browser.runtime.onMessage.addListener((msg: unknown, sender) => {
      const ownPage = sender.id === browser.runtime.id && sender.url?.startsWith(browser.runtime.getURL('/'));
      if (!ownPage || !isRequest(msg)) return undefined;
      return handle(msg).then(
        (data): Reply<unknown> => ({ ok: true, data }),
        (e: unknown): Reply<unknown> => ({ ok: false, error: userMessage(e) }),
      );
    });

    async function handle(req: Request): Promise<unknown> {
      const svc = await ready;
      if (req.type !== 'status') await session.set('lastActive', Date.now());
      switch (req.type) {
        case 'status':
          return {
            status: await svc.status(),
            needsSecretKey: !(await svc.hasSecretKey()),
            settings: await svc.settings(),
          } satisfies StatusReply;
        case 'create':
          return { secretKey: await svc.create(req.password) };
        case 'unlock':
          await svc.unlock(req.password, req.secretKey);
          scheduleSync(0);
          return undefined;
        case 'lock':
          return svc.lock();
        case 'list':
          return svc.list();
        case 'tabMatches': {
          const tab = await activeTab();
          const url = tab?.url;
          if (!url || !/^https?:/.test(url)) return { matches: [] } satisfies TabMatches;
          return { site: new URL(url).hostname.replace(/^www\./, ''), matches: await svc.matches(url) } satisfies TabMatches;
        }
        case 'view':
          return svc.view(req.id, req.revealed);
        case 'reveal':
          return svc.reveal(req.id, req.field);
        case 'draft':
          return svc.draft(req.id);
        case 'newDraft':
          return svc.newDraft(req.kind, req.forTab ? (await activeTab())?.url : undefined);
        case 'save':
          return svc.save(req.id, req.draft);
        case 'remove':
          return svc.remove(req.id);
        case 'favorite':
          return svc.setFavorite(req.id, req.favorite);
        case 'fill':
          return fillActiveTab(svc, req.id);
        case 'totp':
          return svc.totp(req.id);
        case 'generate':
          return svc.generate(req.options);
        case 'importText':
          return svc.importText(req.text);
        case 'importEncrypted':
          return svc.importEncrypted(fromB64(req.b64), req.password);
        case 'exportCsv':
          return svc.exportCsv();
        case 'exportEncrypted':
          return toB64(await svc.exportEncrypted(req.password));
        case 'settings':
          return svc.settings();
        case 'updateSettings':
          return svc.updateSettings(req.patch);
        case 'secretKey':
          return svc.secretKey(req.password);
        case 'changePassword':
          return svc.changePassword(req.current, req.next);
        case 'copied':
          return scheduleClipboardClear((await svc.settings()).clipboardSeconds);
        case 'syncInfo':
          return syncState(svc);
        case 'syncNow':
          await svc.sync();
          return syncState(svc);
        case 'connect':
          await svc.connect(req.config);
          return syncState(svc);
        case 'join':
          return svc.join(req.config);
        case 'disconnect':
          return svc.disconnect();
        case 'openVaultPage':
          await browser.tabs.create({ url: browser.runtime.getURL(`/vault.html#${req.hash}`) });
          return undefined;
      }
    }

    // --- Filling ---------------------------------------------------------------------------
    async function activeTab() {
      const [tab] = await browser.tabs.query({ active: true, currentWindow: true });
      return tab;
    }

    async function fillActiveTab(svc: VaultService, id: string) {
      const tab = await activeTab();
      if (!tab?.id || !tab.url) throw new UserError('Open the page you want to sign in to first.');
      // Throws unless this login is saved for this exact site (Public Suffix List rules, no http downgrade).
      const creds = await svc.credentialsFor(id, tab.url);
      const [res] = await browser.scripting.executeScript({
        target: { tabId: tab.id, frameIds: [0] }, // top frame only: never fill a cross-origin iframe
        func: fillPage,
        args: [new URL(tab.url).origin, creds.username, creds.password],
      });
      const result = res?.result as FillResult | undefined;
      if (result === 'origin-changed') throw new UserError('The page changed before filling. Try again.');
      if (result !== 'filled') throw new UserError('There is no sign-in form on this page that Scytale can see.');
    }

    // --- Keyboard shortcut: fill when exactly one login matches, otherwise ask ---------------
    browser.commands.onCommand.addListener(async (command) => {
      if (command !== 'fill-login') return;
      const svc = await ready;
      const tab = await activeTab();
      const matches = svc && (await svc.status()) === 'unlocked' && tab?.url ? await svc.matches(tab.url) : [];
      if (matches.length === 1 && matches[0]) {
        await fillActiveTab(svc, matches[0].id).catch(() => openPopup());
      } else {
        await openPopup();
      }
    });

    async function openPopup() {
      try {
        await browser.action.openPopup();
      } catch {
        await browser.action.setBadgeText({ text: '•' });
        setTimeout(() => browser.action.setBadgeText({ text: '' }), 4000);
      }
    }

    // --- Auto-lock ---------------------------------------------------------------------------
    browser.alarms.create('autolock', { periodInMinutes: 1 });
    browser.alarms.onAlarm.addListener(async (alarm) => {
      if (alarm.name === 'sync') return void (await ready).sync();
      if (alarm.name !== 'autolock') return;
      const svc = await ready;
      const last = (await session.get<number>('lastActive')) ?? 0;
      const { autoLockMinutes } = await svc.settings();
      if (Date.now() - last > autoLockMinutes * 60_000 && (await svc.status()) === 'unlocked') await svc.lock();
    });
    browser.idle.onStateChanged.addListener(async (state) => {
      if (state === 'locked') await (await ready).lock(); // the screen was locked
    });

    // --- Clipboard clearing -------------------------------------------------------------------
    async function scheduleClipboardClear(seconds: number) {
      if (!seconds) return;
      const ms = seconds * 1000;
      if (import.meta.env.FIREFOX) {
        // Firefox's background page has a DOM and the clipboardWrite permission.
        setTimeout(() => navigator.clipboard.writeText('').catch(() => {}), ms);
        return;
      }
      // Chrome's service worker cannot touch the clipboard, and may sleep before a timer fires;
      // the offscreen document owns the whole delay-then-clear.
      const contexts = await browser.runtime.getContexts({ contextTypes: [browser.runtime.ContextType.OFFSCREEN_DOCUMENT] });
      if (contexts.length === 0) {
        await browser.offscreen.createDocument({
          url: 'offscreen.html',
          reasons: [browser.offscreen.Reason.CLIPBOARD],
          justification: 'Clear copied passwords from the clipboard after the delay chosen in Settings.',
        });
      }
      await browser.runtime.sendMessage({ target: 'offscreen', clearAfterMs: ms });
    }

    // End-to-end tests drive filling directly; release builds never expose this.
    if (import.meta.env.MODE === 'e2e') {
      (globalThis as Record<string, unknown>).__scytale = { ready, fillActiveTab };
    }
  },
});

function isRequest(msg: unknown): msg is Request {
  return typeof msg === 'object' && msg !== null && typeof (msg as { type?: unknown }).type === 'string';
}

function userMessage(e: unknown): string {
  if (e instanceof UserError) return e.message;
  const msg = e instanceof Error ? e.message : String(e);
  // Core errors are already written for people (see scytale-core Error); keep anything else short.
  return msg || 'Something went wrong.';
}
