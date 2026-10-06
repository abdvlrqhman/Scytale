<script lang="ts">
  import type { RemoteConfig } from '@scytale/client';
  import { Button, PasswordField, SegmentedControl, TextField } from '@scytale/ui';
  import { invoke } from '@tauri-apps/api/core';

  let {
    mode,
    onconnect,
  }: {
    /** "connect" uploads this vault; "join" downloads one onto this device. */
    mode: 'connect' | 'join';
    onconnect: (config: RemoteConfig) => Promise<void>;
  } = $props();

  type Kind = 'webdav' | 'folder';
  let kind: Kind = $state('webdav');
  let url = $state('');
  let username = $state('');
  let password = $state('');
  let busy = $state(false);

  async function run(config: RemoteConfig) {
    busy = true;
    try {
      await onconnect(config);
      password = '';
    } finally {
      busy = false;
    }
  }

  async function chooseFolder() {
    const path = await invoke<string | null>('pick_folder');
    if (path) await run({ kind: 'folder', path });
  }
</script>

<div class="stack">
  <SegmentedControl
    legend="Storage"
    name="desk-sync-kind"
    options={[
      { value: 'webdav', label: 'WebDAV' },
      { value: 'folder', label: 'A folder' },
    ]}
    bind:value={kind}
  />
  {#if kind === 'webdav'}
    <form
      class="stack"
      onsubmit={(e) => {
        e.preventDefault();
        run({ kind: 'webdav', url: url.trim(), username: username.trim(), password });
      }}
    >
      <p class="body">
        Nextcloud, ownCloud, Synology or any WebDAV server. In Nextcloud, create an app password under Settings, Security;
        the address is under Files, Settings, WebDAV.
      </p>
      <TextField
        id="dd-url"
        label="Server address"
        placeholder="https://cloud.example.com/remote.php/dav/files/you/"
        type="url"
        spellcheck="false"
        bind:value={url}
      />
      <TextField id="dd-user" label="Username" spellcheck="false" bind:value={username} />
      <PasswordField id="dd-pass" label="App password" bind:value={password} />
      <div>
        <Button type="submit" variant="primary" disabled={busy || !url || !username || !password}>
          {busy ? 'Connecting…' : mode === 'join' ? 'Find my vault' : 'Connect and upload'}
        </Button>
      </div>
    </form>
  {:else}
    <p class="body">
      Pick a folder that Syncthing, iCloud Drive, Dropbox, Google Drive or OneDrive already keeps in sync. Scytale writes
      only encrypted files into a "scytale" folder inside it.
    </p>
    <div><Button variant="primary" disabled={busy} onclick={chooseFolder}>{busy ? 'Connecting…' : 'Choose a folder'}</Button></div>
  {/if}
</div>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .body {
    margin: 0;
    color: var(--ink-soft);
    line-height: 1.6;
    max-width: 62ch;
  }
</style>
