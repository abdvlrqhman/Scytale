<script lang="ts">
  import Button from '../lib/Button.svelte';
  import Icon from '../lib/Icon.svelte';
  import SegmentedControl from '../lib/SegmentedControl.svelte';
  import Switch from '../lib/Switch.svelte';
  import SyncBadge from '../lib/SyncBadge.svelte';
  import type { DeviceView, SyncInfo, Theme } from '../types';

  let {
    theme = $bindable(),
    autoLockMinutes = $bindable(),
    clipboardSeconds = $bindable(),
    lockOnClose = $bindable(),
    startAtLogin = $bindable(),
    storageName,
    sync,
    devices,
    version,
    onsyncnow,
    onchangestorage,
    onremovedevice,
    onimport,
    onexportencrypted,
    onexportcsv,
    onchangepassword,
    onshowkit,
  }: {
    theme: Theme;
    autoLockMinutes: number;
    clipboardSeconds: number;
    /** Leave undefined where it is not a choice (the extension always locks when the browser closes). */
    lockOnClose?: boolean;
    /** Desktop app only; leave undefined in the extension to hide the option. */
    startAtLogin?: boolean;
    /** "Dropbox", or undefined when sync is off. */
    storageName?: string;
    sync: SyncInfo;
    devices: DeviceView[];
    version: string;
    onsyncnow: () => void;
    onchangestorage: () => void;
    onremovedevice: (id: string) => void;
    onimport: () => void;
    onexportencrypted: () => void;
    onexportcsv: () => void;
    onchangepassword: () => void;
    onshowkit: () => void;
  } = $props();

  let csvWarning = $state(false);
</script>

<div class="settings">
  <h1>Settings</h1>

  <section aria-labelledby="s-look">
    <h2 id="s-look">Appearance</h2>
    <div class="row">
      <span class="label">Theme</span>
      <SegmentedControl
        legend="Theme"
        name="s-theme"
        options={[
          { value: 'dark', label: 'Dark' },
          { value: 'light', label: 'Light' },
        ]}
        bind:value={theme}
      />
    </div>
  </section>

  {#if startAtLogin !== undefined}
    <section aria-labelledby="s-desktop">
      <h2 id="s-desktop">Desktop app</h2>
      <Switch
        id="s-startup"
        label="Open Scytale when I sign in to my computer"
        description="It starts locked and waits in the tray. Closing the window also keeps it in the tray."
        bind:checked={startAtLogin}
      />
    </section>
  {/if}

  <section aria-labelledby="s-security">
    <h2 id="s-security">Security</h2>
    <div class="row">
      <label class="label" for="s-autolock">Lock after</label>
      <select id="s-autolock" bind:value={autoLockMinutes}>
        <option value={1}>1 minute idle</option>
        <option value={5}>5 minutes idle</option>
        <option value={15}>15 minutes idle</option>
        <option value={60}>1 hour idle</option>
        <option value={240}>4 hours idle</option>
      </select>
    </div>
    <div class="row">
      <label class="label" for="s-clip">Clear copied passwords after</label>
      <select id="s-clip" bind:value={clipboardSeconds}>
        <option value={15}>15 seconds</option>
        <option value={30}>30 seconds</option>
        <option value={60}>1 minute</option>
        <option value={0}>Never</option>
      </select>
    </div>
    {#if lockOnClose !== undefined}
      <Switch id="s-close" label="Lock when the window is hidden to the tray" bind:checked={lockOnClose} />
    {/if}
    <div class="buttons">
      <Button onclick={onchangepassword}>Change master password</Button>
      <Button onclick={onshowkit}>Show Emergency Kit</Button>
    </div>
  </section>

  <section aria-labelledby="s-sync">
    <h2 id="s-sync">Sync</h2>
    {#if storageName}
      <p class="body">Your encrypted vault syncs through <strong>{storageName}</strong>.</p>
      <SyncBadge {sync} />
      <div class="buttons">
        <Button onclick={onsyncnow}><Icon name="refresh" />Sync now</Button>
        <Button variant="ghost" onclick={onchangestorage}>Change storage</Button>
      </div>
    {:else}
      <p class="body">Sync is off. Your vault lives only on this device.</p>
      <div class="buttons"><Button variant="primary" onclick={onchangestorage}>Turn on sync</Button></div>
    {/if}
  </section>

  <section aria-labelledby="s-devices">
    <h2 id="s-devices">Devices</h2>
    <ul class="devices">
      {#each devices as dev (dev.id)}
        <li>
          <Icon name="laptop" />
          <span class="dev-text">
            <span class="dev-name">{dev.name}{#if dev.current}<span class="this"> This device</span>{/if}</span>
            <span class="dev-seen">{dev.lastSeen}</span>
          </span>
          {#if !dev.current}
            <Button variant="ghost" size="sm" onclick={() => onremovedevice(dev.id)}>Remove</Button>
          {/if}
        </li>
      {/each}
    </ul>
  </section>

  <section aria-labelledby="s-data">
    <h2 id="s-data">Import and export</h2>
    <div class="buttons">
      <Button onclick={onimport}><Icon name="upload" />Import</Button>
      <Button onclick={onexportencrypted}><Icon name="download" />Export encrypted file</Button>
      <Button variant="ghost" onclick={() => (csvWarning = true)}>Export as plain CSV</Button>
    </div>
    {#if csvWarning}
      <div class="warning" role="alert">
        <p>
          A CSV file holds every password in readable text. Anyone who opens it can read them. Delete it as soon as you
          have imported it elsewhere.
        </p>
        <div class="buttons">
          <Button variant="danger" onclick={onexportcsv}>Export readable CSV</Button>
          <Button variant="ghost" onclick={() => (csvWarning = false)}>Cancel</Button>
        </div>
      </div>
    {/if}
  </section>

  <p class="version">Scytale {version}. Open source under the GPL.</p>
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    max-width: 620px;
  }
  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-weight: 400;
    font-size: var(--text-3xl);
  }
  section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: 20px 22px;
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
  }
  h2 {
    margin: 0 0 var(--space-1);
    font-family: var(--font-display);
    font-weight: 400;
    font-size: var(--text-xl);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    min-height: var(--hit);
  }
  .label {
    font-weight: 600;
  }
  select {
    min-height: var(--hit);
    padding: 0 14px;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface-raised);
    color: var(--ink);
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .body {
    margin: 0;
    color: var(--ink-soft);
  }
  .devices {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .devices li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-height: 56px;
    color: var(--muted);
  }
  .devices li + li {
    border-top: 1px solid var(--line-soft);
  }
  .dev-text {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .dev-name {
    color: var(--ink);
    font-weight: 600;
  }
  .this {
    margin-left: 8px;
    padding: 2px 8px;
    border-radius: var(--r-pill);
    background: var(--accent-soft);
    color: var(--accent-text);
    font-size: var(--text-xs);
  }
  .dev-seen {
    font-size: var(--text-sm);
  }
  .warning {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: 14px 16px;
    border-radius: var(--r-md);
    background: var(--danger-soft);
    color: var(--danger);
  }
  .warning p {
    margin: 0;
  }
  .version {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--muted);
  }
</style>
