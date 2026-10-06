<script lang="ts">
  import { MemoryStore, type RemoteConfig, type Settings as SettingsData, UserError, VaultService } from '@scytale/client';
  import {
    Button,
    EmergencyKit,
    Generator,
    ItemDetail,
    ItemEdit,
    Onboarding,
    PasswordField,
    ScytaleStrip,
    Settings,
    StrengthMeter,
    TitleBar,
    Toast,
    Unlock,
    VaultWindow,
    Wordmark,
    type Section,
  } from '@scytale/ui';
  import type {
    DeviceView,
    GeneratorOptions,
    ItemDraft,
    ItemSummary,
    ItemView,
    Strength,
    SyncInfo,
  } from '@scytale/ui/types';
  import { getVersion } from '@tauri-apps/api/app';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { check, type Update } from '@tauri-apps/plugin-updater';
  import { onMount } from 'svelte';
  import DesktopSync from './DesktopSync.svelte';
  import { DesktopStore, copyText, remoteFor } from './platform';
  import { TauriEngine } from './tauri-engine';

  type Platform = 'windows' | 'macos' | 'linux';
  type Pane =
    | { kind: 'empty' }
    | { kind: 'item'; id: string }
    | { kind: 'edit'; id: string | null; draft: ItemDraft }
    | { kind: 'import' }
    | { kind: 'export' }
    | { kind: 'kit' }
    | { kind: 'password' }
    | { kind: 'sync' };

  const appWindow = getCurrentWindow();
  let platform: Platform = $state('windows');
  let appVersion = $state('');
  let maximized = $state(false);
  let svc: VaultService | undefined;

  let status: 'loading' | 'new' | 'joining' | 'locked' | 'unlocked' = $state('loading');
  let needsSecretKey = $state(false);
  let settings: SettingsData | undefined = $state();
  let startAtLogin = $state(false);

  // Vault
  let items: ItemSummary[] = $state([]);
  let section: Section = $state('all');
  let query = $state('');
  let pane: Pane = $state({ kind: 'empty' });
  let item: ItemView | undefined = $state();
  let revealed: Record<string, string> = $state({});
  let sync: SyncInfo = $state({ status: 'offline', text: 'Sync is off. Your vault is saved on this device.' });
  let storageName: string | undefined = $state();
  let devices: DeviceView[] = $state([]);

  // Unlock / setup
  let unlockError: string | undefined = $state();
  let unlocking = $state(false);
  let unlocked = $state(false);
  let secretKey: string | undefined = $state();
  let showKit = $state(false);

  // Panels
  let pendingImport: { name: string; bytes: number[] } | null = $state(null);
  let importPassword = $state('');
  let exportPassword = $state('');
  let exportConfirm = $state('');
  let csvConfirm = $state(false);
  let kitPassword = $state('');
  let kitKey: string | undefined = $state();
  let current = $state('');
  let next = $state('');
  let nextConfirm = $state('');

  // Generator
  let genOptions: GeneratorOptions = $state({
    mode: 'passphrase',
    length: 20,
    lower: true,
    upper: true,
    digits: true,
    symbols: true,
    avoidAmbiguous: false,
    words: 5,
    separator: '-',
    capitalize: true,
    includeNumber: true,
  });
  let generated = $state({ value: '', bits: 0 });
  let suggestion = '';

  // Toast and update banner
  let toast: string | null = $state(null);
  let toastTimer: ReturnType<typeof setTimeout> | undefined;
  let update: Update | null = $state(null);

  function say(message: string) {
    toast = message;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), 3200);
  }
  const fail = (e: unknown) => say(e instanceof Error ? e.message : String(e));
  const platformName = () => ({ windows: 'Windows', macos: 'macOS', linux: 'Linux' })[platform];

  // --- Boot ------------------------------------------------------------------------------------
  let syncTimer: ReturnType<typeof setTimeout> | undefined;
  function scheduleSync(ms: number) {
    clearTimeout(syncTimer);
    syncTimer = setTimeout(() => runSync().catch(() => {}), ms);
  }

  async function runSync() {
    if (!svc || status !== 'unlocked') return;
    const before = JSON.stringify(items);
    const info = await svc.sync();
    await loadSyncState(info);
    await refreshItems();
    if (JSON.stringify(items) !== before && pane.kind === 'item') await openItem(pane.id);
  }

  async function loadSyncState(info?: SyncInfo & { storage?: string }) {
    if (!svc) return;
    const s = info ?? (await svc.syncInfo());
    sync = { status: s.status, text: s.text };
    storageName = s.storage;
    devices = await svc.devices();
  }

  async function boot() {
    platform = await invoke<Platform>('platform');
    appVersion = await getVersion();
    svc = new VaultService(new TauriEngine(), new DesktopStore(), new MemoryStore(), `Scytale for ${platformName()}`, {
      remoteFor,
      onChange: () => scheduleSync(2000),
    });
    settings = await svc.settings();
    applyTheme();
    startAtLogin = await isEnabled().catch(() => false);
    await refreshStatus();
    if (import.meta.env.PROD) check().then((u) => (update = u)).catch(() => {});
  }

  async function refreshStatus() {
    if (!svc) return;
    const s = await svc.status();
    needsSecretKey = !(await svc.hasSecretKey());
    if (s === 'new') status = 'new';
    else if (s === 'locked') status = 'locked';
    else await enterVault();
  }

  async function enterVault() {
    status = 'unlocked';
    await refreshItems();
    await loadSyncState();
    scheduleSync(0);
  }

  async function refreshItems() {
    if (svc) items = await svc.list();
  }

  function applyTheme() {
    if (settings) document.documentElement.dataset.theme = settings.theme;
  }

  // --- Auto-lock: idle time in the app, and (optionally) when hidden to the tray --------------
  let lastActive = Date.now();
  const touch = () => (lastActive = Date.now());

  async function lock() {
    await svc?.lock();
    unlocked = false;
    item = undefined;
    revealed = {};
    pane = { kind: 'empty' };
    status = 'locked';
  }

  onMount(() => {
    boot().catch(fail);
    const idle = setInterval(() => {
      if (status === 'unlocked' && settings && Date.now() - lastActive > settings.autoLockMinutes * 60_000) lock();
    }, 15_000);
    const periodic = setInterval(() => scheduleSync(0), 5 * 60_000);
    const unlisteners = [
      listen('vault-locked', () => lock()),
      listen('window-hidden', () => {
        if (settings?.lockOnClose) lock();
      }),
      appWindow.onResized(async () => (maximized = await appWindow.isMaximized())),
    ];
    // Website links open in the default browser, never inside the app.
    const onClick = (e: MouseEvent) => {
      const a = (e.target as HTMLElement).closest('a');
      if (a && /^https?:/.test(a.href)) {
        e.preventDefault();
        openUrl(a.href).catch(fail);
      }
    };
    document.addEventListener('click', onClick);
    return () => {
      clearInterval(idle);
      clearInterval(periodic);
      unlisteners.forEach((u) => u.then((f) => f()));
      document.removeEventListener('click', onClick);
    };
  });

  // --- Unlock & setup ----------------------------------------------------------------------------
  async function onUnlock(password: string, sk?: string) {
    if (!svc) return;
    unlocking = true;
    unlockError = undefined;
    try {
      await svc.unlock(password, sk);
      unlocked = true;
      await new Promise((r) => setTimeout(r, 650));
      await enterVault();
    } catch (e) {
      unlockError = e instanceof Error ? e.message : String(e);
    } finally {
      unlocking = false;
    }
  }

  async function create(password: string) {
    try {
      secretKey = await svc!.create(password);
    } catch (e) {
      fail(e);
    }
  }

  async function connect(config: RemoteConfig, join: boolean) {
    try {
      if (join) {
        await svc!.join(config);
        needsSecretKey = true;
        status = 'locked';
      } else {
        const info = await svc!.connect(config);
        if (info.status === 'error') throw new UserError(info.text);
        await loadSyncState(info);
        say('Connected. Your vault is synced.');
        if (status !== 'unlocked') await enterVault();
      }
    } catch (e) {
      fail(e);
      throw e;
    }
  }

  // --- Items -----------------------------------------------------------------------------------
  async function openItem(id: string) {
    if (!svc) return;
    if (section === 'generator' || section === 'settings') section = 'all';
    revealed = {};
    item = await svc.view(id);
    pane = { kind: 'item', id };
  }

  async function edit(id: string | null) {
    if (!svc) return;
    if (section === 'generator' || section === 'settings') section = 'all';
    const draft = id ? await svc.draft(id) : svc.newDraft('login');
    suggestion = (await svc.generate({ ...genOptions, mode: 'password' })).value;
    pane = { kind: 'edit', id, draft };
  }

  function suggest() {
    const v = suggestion;
    svc?.generate({ ...genOptions, mode: 'password' }).then((g) => (suggestion = g.value));
    return v;
  }

  async function copyField(key: string) {
    if (!svc || !item) return;
    const field = item.fields.find((f) => f.key === key);
    if (!field) return;
    if (field.secret) {
      await copyText(revealed[key] ?? (await svc.reveal(item.id, key)), settings?.clipboardSeconds ?? 30);
      say(`${field.label} copied. ${clearNote()}`);
    } else if (field.value) {
      await copyText(field.value, 0);
      say(`${field.label} copied.`);
    }
  }
  const clearNote = () => (settings?.clipboardSeconds ? `Clipboard clears in ${settings.clipboardSeconds} s.` : '');

  // TOTP ticker
  $effect(() => {
    if (pane.kind !== 'item' || !item?.totp) return;
    const id = pane.id;
    const t = setInterval(async () => {
      if (item?.totp && svc) item.totp = await svc.totp(id);
    }, 1000);
    return () => clearInterval(t);
  });

  // Generator
  $effect(() => {
    if (section === 'generator' && svc) {
      const opts = $state.snapshot(genOptions);
      svc.generate(opts).then((g) => (generated = g)).catch(fail);
    }
  });

  // Settings persistence
  let settingsLoaded = false;
  $effect(() => {
    if (!settings || !svc) return;
    const snap = $state.snapshot(settings);
    applyTheme();
    if (!settingsLoaded) {
      settingsLoaded = true;
      return;
    }
    svc.updateSettings(snap).catch(fail);
  });
  let autostartLoaded = false;
  $effect(() => {
    const want = startAtLogin;
    if (!autostartLoaded) {
      autostartLoaded = true;
      return;
    }
    (want ? enable() : disable()).catch(fail);
  });

  // --- Panels ----------------------------------------------------------------------------------
  async function pickImport() {
    const file = await invoke<{ name: string; bytes: number[] } | null>('pick_import_file');
    if (!file || !svc) return;
    if (file.name.endsWith('.scyx')) {
      pendingImport = file;
      return;
    }
    try {
      report(await svc.importText(new TextDecoder().decode(new Uint8Array(file.bytes))));
      await refreshItems();
    } catch (e) {
      fail(e);
    }
  }

  async function importEncrypted() {
    if (!pendingImport || !svc) return;
    try {
      report(await svc.importEncrypted(new Uint8Array(pendingImport.bytes), importPassword));
      pendingImport = null;
      importPassword = '';
      await refreshItems();
    } catch (e) {
      fail(e);
    }
  }

  function report(r: { added: number; skipped: number }) {
    say(`Added ${r.added} item${r.added === 1 ? '' : 's'}.${r.skipped ? ` Skipped ${r.skipped} with nothing to import.` : ''}`);
  }

  const today = () => new Date().toISOString().slice(0, 10);

  async function exportEncrypted() {
    if (!svc) return;
    const bytes = await svc.exportEncrypted(exportPassword);
    if (await invoke<boolean>('save_file', { suggestedName: `scytale-export-${today()}.scyx`, bytes: Array.from(bytes) })) {
      exportPassword = exportConfirm = '';
      say('Encrypted export saved.');
    }
  }

  async function exportCsv() {
    if (!svc) return;
    const r = await svc.exportCsv();
    const bytes = Array.from(new TextEncoder().encode(r.csv));
    if (await invoke<boolean>('save_file', { suggestedName: `scytale-export-${today()}.csv`, bytes })) {
      csvConfirm = false;
      say(r.omitted ? `CSV saved. ${r.omitted} cards and identities are only in the encrypted export.` : 'CSV saved.');
    }
  }

  async function revealKit() {
    try {
      kitKey = await svc!.secretKey(kitPassword);
      kitPassword = '';
    } catch (e) {
      fail(e);
    }
  }

  // Strength: a length-based estimate here; the desktop has no zxcvbn bundle yet.
  function rate(pw: string): Strength {
    const classes = [/[a-z]/, /[A-Z]/, /\d/, /[^\w]/].filter((r) => r.test(pw)).length;
    const score = Math.min(4, Math.floor(pw.length / 6) + (classes >= 3 && pw.length >= 10 ? 1 : 0)) as Strength['score'];
    const labels = ['Too easy to guess', 'Weak', 'Fair', 'Strong', 'Very strong'];
    return { score, label: labels[score] ?? 'Strong', hint: score < 3 ? 'Longer is stronger: try a short sentence.' : undefined };
  }
  const nextStrength = $derived(next ? rate(next) : null);

  async function changePassword(e: SubmitEvent) {
    e.preventDefault();
    try {
      await svc!.changePassword(current, next);
      current = next = nextConfirm = '';
      say('Master password changed.');
      scheduleSync(0);
    } catch (err) {
      fail(err);
    }
  }

  const created = () => new Date().toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' });
</script>

<svelte:window onpointerdown={touch} onkeydown={touch} />

<div class="app">
  <TitleBar
    {platform}
    {maximized}
    onminimize={() => appWindow.minimize()}
    ontogglemaximize={() => appWindow.toggleMaximize()}
    onclose={() => appWindow.hide()}
  />

  {#if update}
    <div class="update" role="status">
      Scytale {update.version} is ready.
      <Button
        size="sm"
        variant="primary"
        onclick={async () => {
          await update?.downloadAndInstall();
          await relaunch();
        }}>Restart to update</Button
      >
      <Button size="sm" variant="ghost" onclick={() => (update = null)}>Later</Button>
    </div>
  {/if}

  <div class="body">
    {#if status === 'loading'}
      <div class="center"><Wordmark size={34} /></div>
    {:else if status === 'new'}
      <div class="center scroll">
        {#if showKit && secretKey}
          <div class="kit">
            <EmergencyKit {secretKey} storage={storageName ?? 'This device only (sync is off)'} created={created()} />
            <div class="row no-print">
              <Button variant="primary" onclick={() => window.print()}>Print or save as PDF</Button>
              <Button onclick={() => (showKit = false)}>Back</Button>
            </div>
          </div>
        {:else}
          <Onboarding
            {secretKey}
            allowFolder
            available={['webdav', 'folder']}
            {rate}
            suggest={() => {
              const s = suggestion;
              svc?.generate({ ...genOptions, mode: 'passphrase', words: 4 }).then((g) => (suggestion = g.value));
              return s;
            }}
            oncreate={create}
            onsavekit={() => (showKit = true)}
            oncopykey={async () => {
              if (secretKey) await copyText(secretKey, 60);
              say('Secret Key copied. It clears from the clipboard in 60 s.');
            }}
            onstorage={async (p) => {
              // "Only this device" continues to the last onboarding step; a provider opens its form.
              if (p === 'none') return;
              await enterVault();
              pane = { kind: 'sync' };
            }}
            onimport={async () => {
              await enterVault();
              pane = { kind: 'import' };
            }}
            onfinish={enterVault}
            onjoin={() => (status = 'joining')}
          />
        {/if}
      </div>
    {:else if status === 'joining'}
      <div class="center scroll">
        <section class="panel narrow">
          <h1>Add this device</h1>
          <p class="text">Connect the storage your other device syncs to. Then sign in with your master password and Secret Key.</p>
          <DesktopSync mode="join" onconnect={(c) => connect(c, true)} />
          <Button variant="ghost" onclick={() => (status = 'new')}>Back</Button>
        </section>
      </div>
    {:else if status === 'locked'}
      <div class="center">
        <Unlock
          {needsSecretKey}
          busy={unlocking}
          error={unlockError}
          {unlocked}
          onunlock={onUnlock}
          onforgot={() => say('Nobody can reset it. A device that is still unlocked can export an encrypted file.')}
        />
      </div>
    {:else}
      <VaultWindow {items} bind:section bind:query selectedId={pane.kind === 'item' ? pane.id : undefined} {sync} onselect={openItem} onadd={() => edit(null)} onlock={lock}>
        {#if section === 'generator'}
          <Generator
            bind:options={genOptions}
            value={generated.value}
            bits={generated.bits}
            onregenerate={() => svc?.generate($state.snapshot(genOptions)).then((g) => (generated = g))}
            oncopy={async () => {
              await copyText(generated.value, settings?.clipboardSeconds ?? 30);
              say(`Password copied. ${clearNote()}`);
            }}
          />
        {:else if section === 'settings' && settings}
          <Settings
            bind:theme={settings.theme}
            bind:autoLockMinutes={settings.autoLockMinutes}
            bind:clipboardSeconds={settings.clipboardSeconds}
            bind:lockOnClose={settings.lockOnClose}
            bind:startAtLogin
            {storageName}
            {sync}
            {devices}
            version={appVersion}
            onsyncnow={() => runSync().catch(fail)}
            onchangestorage={() => {
              section = 'all';
              pane = { kind: 'sync' };
            }}
            onremovedevice={() => say('Removing devices arrives in a later version.')}
            onimport={() => {
              section = 'all';
              pane = { kind: 'import' };
            }}
            onexportencrypted={() => {
              section = 'all';
              pane = { kind: 'export' };
            }}
            onexportcsv={() => {
              section = 'all';
              pane = { kind: 'export' };
            }}
            onchangepassword={() => {
              section = 'all';
              pane = { kind: 'password' };
            }}
            onshowkit={() => {
              section = 'all';
              pane = { kind: 'kit' };
            }}
          />
        {:else if pane.kind === 'item' && item}
          <ItemDetail
            {item}
            onreveal={async (key) => {
              revealed[key] = await svc!.reveal(item!.id, key);
              item = await svc!.view(item!.id, revealed);
            }}
            onhide={async (key) => {
              delete revealed[key];
              item = await svc!.view(item!.id, revealed);
            }}
            oncopy={(k) => copyField(k).catch(fail)}
            oncopytotp={async () => {
              if (!item?.totp) return;
              await copyText(item.totp.code, settings?.clipboardSeconds ?? 30);
              say(`Code copied. ${clearNote()}`);
            }}
            onedit={() => edit(item!.id)}
            onfavorite={async () => {
              await svc!.setFavorite(item!.id, !item!.favorite);
              await refreshItems();
              item = await svc!.view(item!.id, revealed);
            }}
          />
        {:else if pane.kind === 'edit'}
          {#key pane}
            <ItemEdit
              initial={pane.draft}
              isNew={pane.id === null}
              {suggest}
              onsave={async (d) => {
                try {
                  const id = await svc!.save(pane.kind === 'edit' ? pane.id : null, d);
                  await refreshItems();
                  await openItem(id);
                  say('Saved.');
                } catch (e) {
                  fail(e);
                }
              }}
              oncancel={() => (pane.kind === 'edit' && pane.id ? openItem(pane.id) : (pane = { kind: 'empty' }))}
              ondelete={pane.id
                ? async () => {
                    await svc!.remove((pane as { id: string }).id);
                    await refreshItems();
                    pane = { kind: 'empty' };
                    say('Deleted.');
                  }
                : undefined}
            />
          {/key}
        {:else if pane.kind === 'import'}
          <section class="panel">
            <h1>Import passwords</h1>
            <p class="text">
              Exports from Chrome, Edge, Brave, Firefox, Safari, Bitwarden (CSV or unencrypted JSON), 1Password, LastPass,
              KeePassXC, or a Scytale encrypted export. Delete plain exports once imported.
            </p>
            <div><Button variant="primary" onclick={() => pickImport().catch(fail)}>Choose a file</Button></div>
            {#if pendingImport}
              <PasswordField id="d-imp" label="Password of this Scytale export" bind:value={importPassword} />
              <div><Button variant="primary" disabled={!importPassword} onclick={importEncrypted}>Import</Button></div>
            {/if}
          </section>
        {:else if pane.kind === 'export'}
          <section class="panel">
            <h1>Export</h1>
            <h2>Encrypted file</h2>
            <p class="text">Everything, including cards and identities, locked with a password you choose now.</p>
            <PasswordField id="d-exp" label="Export password" hint="At least 8 characters." bind:value={exportPassword} />
            <PasswordField
              id="d-exp2"
              label="Type it again"
              error={exportConfirm && exportConfirm !== exportPassword ? 'The two passwords are different.' : undefined}
              bind:value={exportConfirm}
            />
            <div>
              <Button variant="primary" disabled={exportPassword.length < 8 || exportPassword !== exportConfirm} onclick={() => exportEncrypted().catch(fail)}>
                Save encrypted export
              </Button>
            </div>
            <h2>Plain CSV</h2>
            <p class="text">For moving to another password manager. Anyone who opens this file can read every password in it.</p>
            <label class="check"><input type="checkbox" bind:checked={csvConfirm} /> I understand the CSV is not protected.</label>
            <div><Button variant="danger" disabled={!csvConfirm} onclick={() => exportCsv().catch(fail)}>Save readable CSV</Button></div>
          </section>
        {:else if pane.kind === 'kit'}
          {#if kitKey}
            <div class="kit">
              <EmergencyKit secretKey={kitKey} storage={storageName ?? 'This device only (sync is off)'} created={created()} />
              <div class="row no-print">
                <Button variant="primary" onclick={() => window.print()}>Print or save as PDF</Button>
                <Button onclick={() => (kitKey = undefined)}>Hide</Button>
              </div>
            </div>
          {:else}
            <section class="panel narrow">
              <h1>Emergency Kit</h1>
              <p class="text">Enter your master password to show your Secret Key.</p>
              <PasswordField id="d-kit" label="Master password" bind:value={kitPassword} />
              <div><Button variant="primary" disabled={!kitPassword} onclick={revealKit}>Show my kit</Button></div>
            </section>
          {/if}
        {:else if pane.kind === 'password'}
          <form class="panel narrow" onsubmit={changePassword}>
            <h1>Change master password</h1>
            <PasswordField id="d-cur" label="Current password" bind:value={current} />
            <PasswordField id="d-next" label="New password" bind:value={next} />
            {#if nextStrength}<StrengthMeter {...nextStrength} />{/if}
            <PasswordField
              id="d-conf"
              label="Type the new one again"
              error={nextConfirm && nextConfirm !== next ? 'The two passwords are different.' : undefined}
              bind:value={nextConfirm}
            />
            <div>
              <Button type="submit" variant="primary" disabled={!current || !nextStrength || nextStrength.score < 2 || next !== nextConfirm}>
                Change password
              </Button>
            </div>
          </form>
        {:else if pane.kind === 'sync'}
          <section class="panel narrow">
            <h1>Sync</h1>
            {#if storageName}
              <p class="text">Your encrypted vault syncs through <strong>{storageName}</strong>. {sync.text}.</p>
              <div class="row">
                <Button variant="primary" onclick={() => runSync().catch(fail)}>Sync now</Button>
                <Button
                  variant="danger"
                  onclick={async () => {
                    await svc!.disconnect();
                    await loadSyncState();
                    say('Sync is off. The copy in your storage was left as it was.');
                  }}>Turn off sync</Button
                >
              </div>
            {:else}
              <p class="text">Pick storage you already have. It only ever receives encrypted files.</p>
              <DesktopSync mode="connect" onconnect={(c) => connect(c, false)} />
            {/if}
          </section>
        {:else}
          <div class="empty">
            <ScytaleStrip height={22} letters={false} />
            <p>{items.length ? 'Choose an item on the left.' : 'Your vault is empty. Add your first login, or import from another app.'}</p>
            {#if !items.length}
              <div class="row">
                <Button variant="primary" onclick={() => edit(null)}>Add a login</Button>
                <Button onclick={() => (pane = { kind: 'import' })}>Import</Button>
              </div>
            {/if}
          </div>
        {/if}
      </VaultWindow>
    {/if}
  </div>
  <Toast message={toast} />
</div>

<style>
  :global(html),
  :global(body),
  :global(#app) {
    height: 100%;
    overflow: hidden;
  }
  .app {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .body > :global(.window) {
    flex: 1;
  }
  .center {
    flex: 1;
    display: grid;
    place-items: center;
    padding: 24px;
  }
  .scroll {
    overflow-y: auto;
    place-items: start center;
  }
  .update {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    background: var(--accent-soft);
    color: var(--ink);
    font-size: var(--text-sm);
  }
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-width: 640px;
  }
  .narrow {
    max-width: 480px;
    width: 100%;
  }
  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-weight: 400;
    font-size: var(--text-3xl);
  }
  h2 {
    margin: var(--space-4) 0 0;
    font-family: var(--font-display);
    font-weight: 400;
    font-size: var(--text-xl);
  }
  .text {
    margin: 0;
    color: var(--ink-soft);
    line-height: 1.6;
    max-width: 62ch;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .check {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .check input {
    width: 20px;
    height: 20px;
    accent-color: var(--accent);
  }
  .kit {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    width: 100%;
    max-width: 720px;
  }
  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
    color: var(--muted);
    text-align: center;
  }
  .empty :global(.rod) {
    width: 220px;
  }
  .empty p {
    margin: 0;
    max-width: 40ch;
  }
  @media print {
    .no-print {
      display: none !important;
    }
  }
</style>
