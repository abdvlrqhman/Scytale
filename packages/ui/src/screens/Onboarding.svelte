<script lang="ts">
  import Button from '../lib/Button.svelte';
  import Icon from '../lib/Icon.svelte';
  import PasswordField from '../lib/PasswordField.svelte';
  import ScytaleStrip from '../lib/ScytaleStrip.svelte';
  import StrengthMeter from '../lib/StrengthMeter.svelte';
  import Wordmark from '../lib/Wordmark.svelte';
  import type { StorageProvider, Strength } from '../types';

  let {
    secretKey,
    allowFolder = false,
    available,
    rate,
    suggest,
    oncreate,
    onsavekit,
    oncopykey,
    onstorage,
    onimport,
    onfinish,
    onjoin,
  }: {
    /** Arrives after `oncreate`; the kit step waits for it. */
    secretKey?: string;
    /** Desktop only: sync through a folder another app already syncs. */
    allowFolder?: boolean;
    /** Providers this build can connect; others are not offered. */
    available?: StorageProvider[];
    rate: (password: string) => Strength;
    suggest: () => string;
    oncreate: (password: string) => void;
    onsavekit: () => void;
    oncopykey: () => void;
    onstorage: (provider: StorageProvider) => void;
    onimport: () => void;
    onfinish: () => void;
    onjoin: () => void;
  } = $props();

  type Step = 'welcome' | 'password' | 'kit' | 'storage' | 'done';
  const order: Step[] = ['password', 'kit', 'storage'];

  let step: Step = $state('welcome');
  let password = $state('');
  let confirm = $state('');
  let kitSaved = $state(false);
  let provider: StorageProvider = $state('dropbox');

  const strength = $derived(password ? rate(password) : null);
  const mismatch = $derived(confirm.length > 0 && confirm !== password);
  const canCreate = $derived(!!strength && strength.score >= 2 && confirm === password);
  const stepNo = $derived(order.indexOf(step) + 1);

  type ProviderOption = { id: StorageProvider; name: string; detail: string };
  const providers: ProviderOption[] = $derived(([
    { id: 'dropbox', name: 'Dropbox', detail: 'Sign in once; works everywhere.' },
    { id: 'gdrive', name: 'Google Drive', detail: 'Stored in a private app folder.' },
    { id: 'onedrive', name: 'OneDrive', detail: 'Stored in Apps › Scytale.' },
    { id: 'webdav', name: 'WebDAV', detail: 'Nextcloud, ownCloud or your own server.' },
    ...(allowFolder
      ? [{ id: 'folder' as const, name: 'A folder', detail: 'Any folder Syncthing or iCloud Drive keeps in sync.' }]
      : []),
    { id: 'none', name: 'Only this device for now', detail: 'You can turn on sync later in Settings.' },
  ] as ProviderOption[]).filter((p) => p.id === 'none' || !available || available.includes(p.id)));
  $effect(() => {
    if (!providers.some((p) => p.id === provider)) provider = providers[0]?.id ?? 'none';
  });

  function createVault(e: SubmitEvent) {
    e.preventDefault();
    if (!canCreate) return;
    oncreate(password);
    password = confirm = '';
    step = 'kit';
  }
</script>

<div class="onboarding">
  {#if step !== 'welcome' && step !== 'done'}
    <p class="progress">Step {stepNo} of {order.length}</p>
  {/if}

  {#if step === 'welcome'}
    <div class="hero">
      <Wordmark size={40} />
      <ScytaleStrip height={32} />
      <p class="lede">
        Your passwords, locked with a key only you hold. Nothing is stored on our servers, because there are none.
      </p>
    </div>
    <div class="stack">
      <Button variant="primary" size="lg" full onclick={() => (step = 'password')}>Create a new vault</Button>
      <Button size="lg" full onclick={onjoin}>I already use Scytale on another device</Button>
    </div>
  {:else if step === 'password'}
    <form class="stack" onsubmit={createVault}>
      <h1>Choose a master password</h1>
      <p class="body">
        It opens your vault on every device. Nobody can reset it for you, so pick something you will remember: a short
        sentence works well.
      </p>
      <PasswordField id="ob-password" label="Master password" autofocus bind:value={password} />
      {#if strength}<StrengthMeter {...strength} />{/if}
      <PasswordField
        id="ob-confirm"
        label="Type it again"
        error={mismatch ? 'The two passwords are different.' : undefined}
        bind:value={confirm}
      />
      <Button variant="ghost" onclick={() => (password = confirm = suggest())}>Suggest a memorable one</Button>
      <Button type="submit" variant="primary" size="lg" full disabled={!canCreate}>Create my vault</Button>
    </form>
  {:else if step === 'kit'}
    <div class="stack">
      <h1>Save your Emergency Kit</h1>
      <p class="body">
        Your vault opens with your master password <strong>and</strong> this Secret Key. Your devices remember the key,
        but if you lose them all, the kit is the only way back in.
      </p>
      <div class="key" aria-live="polite">
        {#if secretKey}
          <span class="key-label">Secret Key</span>
          <code>{secretKey}</code>
        {:else}
          <span class="key-label">Creating your vault…</span>
        {/if}
      </div>
      <div class="row">
        <Button variant="primary" disabled={!secretKey} onclick={onsavekit}><Icon name="download" />Save Emergency Kit</Button>
        <Button disabled={!secretKey} onclick={oncopykey}><Icon name="copy" />Copy key</Button>
      </div>
      <label class="check">
        <input type="checkbox" bind:checked={kitSaved} />
        I saved my Emergency Kit somewhere safe, like a printed copy at home.
      </label>
      <Button variant="primary" size="lg" full disabled={!kitSaved || !secretKey} onclick={() => (step = 'storage')}>
        Continue
      </Button>
    </div>
  {:else if step === 'storage'}
    <form
      class="stack"
      onsubmit={(e) => {
        e.preventDefault();
        onstorage(provider);
        step = 'done';
      }}
    >
      <h1>Where should your devices sync?</h1>
      <p class="body">Pick storage you already have. It only ever receives encrypted files.</p>
      <fieldset class="providers">
        <legend class="sr-only">Sync storage</legend>
        {#each providers as p (p.id)}
          <label class="provider" class:checked={provider === p.id}>
            <input type="radio" name="provider" value={p.id} bind:group={provider} />
            <span class="name">{p.name}</span>
            <span class="detail">{p.detail}</span>
          </label>
        {/each}
      </fieldset>
      <Button type="submit" variant="primary" size="lg" full>
        {provider === 'none' ? 'Continue without sync' : 'Connect'}
      </Button>
    </form>
  {:else}
    <div class="hero">
      <ScytaleStrip unwound height={32} />
      <h1>Your vault is ready</h1>
      <p class="lede">Bring your passwords over from your browser or another password manager, or start fresh.</p>
    </div>
    <div class="stack">
      <Button variant="primary" size="lg" full onclick={onimport}><Icon name="upload" />Import passwords</Button>
      <Button size="lg" full onclick={onfinish}>Open my vault</Button>
    </div>
  {/if}
</div>

<style>
  .onboarding {
    width: 100%;
    max-width: 440px;
    margin: 0 auto;
    padding: 36px 20px 28px;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
  .progress {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--muted);
  }
  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-5);
    text-align: center;
  }
  .hero :global(.rod) {
    align-self: stretch;
  }
  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-weight: 400;
    font-size: var(--text-3xl);
    line-height: 1.15;
    text-wrap: balance;
  }
  .lede,
  .body {
    margin: 0;
    color: var(--ink-soft);
    font-size: var(--text-lg);
    line-height: 1.55;
  }
  .lede {
    max-width: 34ch;
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .key {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 18px 20px;
    border-radius: var(--r-lg);
    background: var(--surface-raised);
    border: 1px dashed var(--accent);
  }
  .key-label {
    font-size: var(--text-sm);
    color: var(--muted);
  }
  code {
    font-family: ui-monospace, 'Cascadia Mono', 'SF Mono', Menlo, Consolas, monospace;
    font-size: 17px;
    letter-spacing: 0.04em;
    color: var(--accent-text);
    word-break: break-all;
  }
  .check {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    font-size: var(--text-md);
    color: var(--ink-soft);
    cursor: pointer;
  }
  .check input {
    width: 20px;
    height: 20px;
    margin: 0;
    accent-color: var(--accent);
    flex-shrink: 0;
  }
  .providers {
    margin: 0;
    padding: 0;
    border: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .provider {
    position: relative;
    display: flex;
    flex-direction: column;
    padding: 14px 18px;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    cursor: pointer;
  }
  .provider.checked {
    border-color: var(--accent);
    background: var(--surface-raised);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .provider input {
    position: absolute;
    opacity: 0;
    inset: 0;
    margin: 0;
    cursor: pointer;
  }
  .provider:has(input:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .name {
    font-weight: 600;
  }
  .detail {
    font-size: var(--text-sm);
    color: var(--muted);
  }
</style>
