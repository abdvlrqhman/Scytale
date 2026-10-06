<script lang="ts">
  import type { Settings as SettingsData } from '@scytale/client';
  import {
    Button,
    Generator,
    IconButton,
    ItemDetail,
    ItemEdit,
    Popup,
    ScytaleStrip,
    Settings,
    Toast,
    Unlock,
    Wordmark,
  } from '@scytale/ui';
  import type { GeneratorOptions, ItemDraft, ItemSummary, ItemView, SyncInfo } from '@scytale/ui/types';
  import { onDestroy, onMount } from 'svelte';
  import { deviceName } from '@/src/device';
  import { send } from '@/src/messages';
  import { applyTheme, copyText } from '@/src/ui-helpers';

  type View =
    | { name: 'loading' }
    | { name: 'new' }
    | { name: 'locked' }
    | { name: 'list' }
    | { name: 'item'; id: string }
    | { name: 'edit'; id: string | null; draft: ItemDraft }
    | { name: 'generator' }
    | { name: 'settings' };

  let view: View = $state({ name: 'loading' });
  let needsSecretKey = $state(false);
  let settings: SettingsData | undefined = $state();

  // List
  let site: string | undefined = $state();
  let matches: ItemSummary[] = $state([]);
  let items: ItemSummary[] = $state([]);
  let query = $state('');
  const sync: SyncInfo = { status: 'offline', text: 'Sync is off. Your vault is saved on this device.' };

  // Item
  let item: ItemView | undefined = $state();
  let revealed: Record<string, string> = $state({});
  let totpTimer: ReturnType<typeof setInterval> | undefined;

  // Unlock
  let unlockError: string | undefined = $state();
  let unlocking = $state(false);
  let unlocked = $state(false);

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

  // Toast
  let toast: string | null = $state(null);
  let toastTimer: ReturnType<typeof setTimeout> | undefined;
  function say(message: string) {
    toast = message;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), 3000);
  }
  const fail = (e: unknown) => say(e instanceof Error ? e.message : String(e));

  async function start() {
    const s = await send({ type: 'status' });
    settings = s.settings;
    needsSecretKey = s.needsSecretKey;
    applyTheme(s.settings.theme);
    if (s.status === 'new') view = { name: 'new' };
    else if (s.status === 'locked') view = { name: 'locked' };
    else await showList();
  }

  async function showList() {
    stopTotp();
    const [tab, all] = await Promise.all([send({ type: 'tabMatches' }), send({ type: 'list' })]);
    site = tab.site;
    matches = tab.matches;
    items = all;
    view = { name: 'list' };
  }

  async function onUnlock(password: string, secretKey?: string) {
    unlocking = true;
    unlockError = undefined;
    try {
      await send({ type: 'unlock', password, secretKey });
      unlocked = true; // plays the strip's unwind
      await new Promise((r) => setTimeout(r, 650));
      await showList();
    } catch (e) {
      unlockError = e instanceof Error ? e.message : String(e);
    } finally {
      unlocking = false;
    }
  }

  async function lock() {
    await send({ type: 'lock' });
    unlocked = false;
    view = { name: 'locked' };
  }

  async function fill(id: string) {
    try {
      await send({ type: 'fill', id });
      window.close();
    } catch (e) {
      fail(e);
    }
  }

  async function openItem(id: string) {
    revealed = {};
    item = await send({ type: 'view', id, revealed });
    view = { name: 'item', id };
    stopTotp();
    if (item.totp) {
      totpTimer = setInterval(async () => {
        if (!item?.totp) return;
        item.totp = await send({ type: 'totp', id });
      }, 1000);
    }
  }

  function stopTotp() {
    clearInterval(totpTimer);
    totpTimer = undefined;
  }

  async function refreshItem() {
    if (view.name === 'item') item = await send({ type: 'view', id: view.id, revealed });
  }

  async function copyField(key: string) {
    if (!item) return;
    const field = item.fields.find((f) => f.key === key);
    try {
      if (field?.secret) {
        const value = revealed[key] ?? (await send({ type: 'reveal', id: item.id, field: key }));
        await copyText(value, true);
        say(`${field.label} copied. ${clearNote()}`);
      } else if (field?.value) {
        await copyText(field.value, false);
        say(`${field.label} copied.`);
      }
    } catch (e) {
      fail(e);
    }
  }

  async function copyTotp() {
    if (!item?.totp) return;
    await copyText(item.totp.code, true);
    say(`Code copied. ${clearNote()}`);
  }

  const clearNote = () =>
    settings?.clipboardSeconds ? `Clipboard clears in ${settings.clipboardSeconds} s.` : '';

  async function edit(id: string | null) {
    stopTotp();
    const draft = id ? await send({ type: 'draft', id }) : await send({ type: 'newDraft', kind: 'login', forTab: true });
    suggestion = (await send({ type: 'generate', options: { ...genOptions, mode: 'password' } })).value;
    view = { name: 'edit', id, draft };
  }

  function suggest(): string {
    const value = suggestion;
    send({ type: 'generate', options: { ...genOptions, mode: 'password' } }).then((g) => (suggestion = g.value));
    return value;
  }

  async function save(id: string | null, draft: ItemDraft) {
    try {
      const saved = await send({ type: 'save', id, draft });
      say(id ? 'Saved.' : 'Added.');
      await openItem(saved);
    } catch (e) {
      fail(e);
    }
  }

  async function remove(id: string) {
    await send({ type: 'remove', id });
    say('Deleted.');
    await showList();
  }

  async function regenerate() {
    generated = await send({ type: 'generate', options: $state.snapshot(genOptions) });
  }

  // Regenerate whenever an option changes while the generator is open.
  $effect(() => {
    if (view.name === 'generator') {
      JSON.stringify(genOptions);
      regenerate().catch(fail);
    }
  });

  // Persist settings edits.
  let settingsReady = false;
  $effect(() => {
    if (!settings) return;
    const snapshot = $state.snapshot(settings);
    applyTheme(snapshot.theme);
    if (!settingsReady) {
      settingsReady = true;
      return;
    }
    send({ type: 'updateSettings', patch: snapshot }).catch(fail);
  });

  const openVault = (hash: string) => send({ type: 'openVaultPage', hash }).then(() => window.close());

  onMount(() => {
    start().catch(fail);
  });
  onDestroy(stopTotp);
</script>

<div class="popup-root">
  {#if view.name === 'loading'}
    <div class="center"><Wordmark size={30} /></div>
  {:else if view.name === 'new'}
    <div class="welcome">
      <Wordmark size={36} />
      <ScytaleStrip height={30} />
      <p>Your passwords, locked with a key only you hold. Setting up takes about two minutes.</p>
      <Button variant="primary" size="lg" full onclick={() => openVault('setup')}>Set up Scytale</Button>
      <Button size="lg" full onclick={() => openVault('join')}>I already use Scytale</Button>
    </div>
  {:else if view.name === 'locked'}
    <Unlock {needsSecretKey} busy={unlocking} error={unlockError} {unlocked} onunlock={onUnlock} onforgot={() => openVault('forgot')} />
  {:else if view.name === 'list'}
    <Popup
      {site}
      {matches}
      {items}
      {sync}
      bind:query
      onfill={fill}
      onopen={openItem}
      onlock={lock}
      ongenerate={() => (view = { name: 'generator' })}
      onadd={() => edit(null)}
      onsettings={() => (view = { name: 'settings' })}
      onimport={() => openVault('import')}
    />
  {:else if view.name === 'item' && item}
    <div class="page">
      <ItemDetail
        {item}
        onback={showList}
        onreveal={async (key) => {
          revealed[key] = await send({ type: 'reveal', id: item!.id, field: key });
          await refreshItem();
        }}
        onhide={async (key) => {
          delete revealed[key];
          await refreshItem();
        }}
        oncopy={copyField}
        oncopytotp={copyTotp}
        onedit={() => edit(item!.id)}
        onfavorite={async () => {
          await send({ type: 'favorite', id: item!.id, favorite: !item!.favorite });
          await refreshItem();
        }}
      />
    </div>
  {:else if view.name === 'edit'}
    <div class="page">
      <ItemEdit
        initial={view.draft}
        isNew={view.id === null}
        {suggest}
        onsave={(d) => save(view.name === 'edit' ? view.id : null, d)}
        oncancel={() => (view.name === 'edit' && view.id ? openItem(view.id) : showList())}
        ondelete={view.id ? () => remove((view as { id: string }).id) : undefined}
      />
    </div>
  {:else if view.name === 'generator'}
    <div class="page">
      <div class="back"><IconButton icon="back" label="Back to list" onclick={showList} /></div>
      <Generator
        bind:options={genOptions}
        value={generated.value}
        bits={generated.bits}
        onregenerate={() => regenerate().catch(fail)}
        oncopy={async () => {
          await copyText(generated.value, true);
          say(`Password copied. ${clearNote()}`);
        }}
      />
    </div>
  {:else if view.name === 'settings' && settings}
    <div class="page">
      <div class="back"><IconButton icon="back" label="Back to list" onclick={showList} /></div>
      <Settings
        bind:theme={settings.theme}
        bind:autoLockMinutes={settings.autoLockMinutes}
        bind:clipboardSeconds={settings.clipboardSeconds}
        {sync}
        devices={[{ id: 'this', name: deviceName(), lastSeen: 'Active now', current: true }]}
        version={browser.runtime.getManifest().version}
        onsyncnow={() => {}}
        onchangestorage={() => openVault('sync')}
        onremovedevice={() => {}}
        onimport={() => openVault('import')}
        onexportencrypted={() => openVault('export')}
        onexportcsv={() => openVault('export')}
        onchangepassword={() => openVault('password')}
        onshowkit={() => openVault('kit')}
      />
    </div>
  {/if}
  <Toast message={toast} />
</div>

<style>
  :global(html),
  :global(body) {
    width: 380px;
    height: 600px;
    overflow: hidden;
  }
  .popup-root {
    position: relative;
    width: 380px;
    height: 600px;
    overflow: hidden;
    background: var(--bg);
  }
  .center {
    height: 100%;
    display: grid;
    place-items: center;
  }
  .welcome {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: var(--space-4);
    padding: 24px;
    text-align: center;
  }
  .welcome :global(.rod) {
    align-self: stretch;
  }
  .welcome p {
    margin: 0 0 var(--space-2);
    color: var(--ink-soft);
    line-height: 1.55;
  }
  .page {
    height: 100%;
    overflow-y: auto;
    padding: 16px 18px 72px;
  }
  .page :global(h1) {
    font-size: var(--text-2xl);
  }
  .back {
    margin: -6px 0 6px -10px;
  }
</style>
