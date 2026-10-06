<script lang="ts">
  // Connects the user's own storage. Permissions are requested first thing in each click handler,
  // because browsers only grant them during the user's gesture.
  import { type RemoteConfig, dropboxAuthorizeUrl, dropboxExchange, pkcePair } from '@scytale/client';
  import { Button, PasswordField, SegmentedControl, TextField } from '@scytale/ui';
  import { send } from './messages';
  import { DROPBOX_CLIENT_ID } from './providers';

  let {
    mode,
    ondone,
    onerror,
  }: {
    /** "connect" uploads this vault; "join" downloads one onto this new device. */
    mode: 'connect' | 'join';
    ondone: () => void;
    onerror: (message: string) => void;
  } = $props();

  type Kind = 'webdav' | 'dropbox';
  const kinds: { value: Kind; label: string }[] = [
    { value: 'webdav', label: 'WebDAV' },
    ...(DROPBOX_CLIENT_ID ? [{ value: 'dropbox' as const, label: 'Dropbox' }] : []),
  ];
  let kind: Kind = $state('webdav');
  let url = $state('');
  let username = $state('');
  let password = $state('');
  let busy = $state(false);

  function connectWebdav(e: SubmitEvent) {
    e.preventDefault();
    let origin: string;
    try {
      origin = new URL(url.trim()).origin;
    } catch {
      return onerror('Enter the full address, starting with https://');
    }
    // Synchronous up to here: the permission prompt must open inside the click.
    browser.permissions.request({ origins: [`${origin}/*`] }).then(async (granted) => {
      if (!granted) return onerror(`Scytale needs permission to reach ${origin} to sync with it.`);
      await run({ kind: 'webdav', url: url.trim(), username: username.trim(), password });
    });
  }

  function connectDropbox() {
    browser.permissions.request({ permissions: ['identity'] }).then(async (granted) => {
      if (!granted) return onerror('Scytale needs permission to open the Dropbox sign-in window.');
      busy = true;
      try {
        const { verifier, challenge } = await pkcePair();
        const state = crypto.randomUUID();
        const redirect = browser.identity.getRedirectURL('dropbox');
        const result = await browser.identity.launchWebAuthFlow({
          url: dropboxAuthorizeUrl(DROPBOX_CLIENT_ID, redirect, challenge, state),
          interactive: true,
        });
        const back = new URL(result ?? '');
        const code = back.searchParams.get('code');
        if (!code || back.searchParams.get('state') !== state) throw new Error('Dropbox sign-in was cancelled.');
        await run({ kind: 'dropbox', refreshToken: await dropboxExchange(DROPBOX_CLIENT_ID, code, verifier, redirect) });
      } catch (err) {
        onerror(err instanceof Error ? err.message : String(err));
      } finally {
        busy = false;
      }
    });
  }

  async function run(config: RemoteConfig) {
    busy = true;
    try {
      if (mode === 'join') await send({ type: 'join', config });
      else {
        const state = await send({ type: 'connect', config });
        if (state.info.status === 'error') throw new Error(state.info.text);
      }
      password = '';
      ondone();
    } catch (err) {
      onerror(err instanceof Error ? err.message : String(err));
    } finally {
      busy = false;
    }
  }
</script>

<div class="setup">
  {#if kinds.length > 1}
    <SegmentedControl legend="Storage" name="sync-kind" options={kinds} bind:value={kind} />
  {/if}

  {#if kind === 'webdav'}
    <form class="stack" onsubmit={connectWebdav}>
      <p class="body">
        Works with Nextcloud, ownCloud, Synology and any WebDAV server. In Nextcloud, create an app password under
        Settings, Security; the address is shown under Files, Settings, WebDAV.
      </p>
      <TextField
        id="dav-url"
        label="Server address"
        placeholder="https://cloud.example.com/remote.php/dav/files/you/"
        type="url"
        autocomplete="url"
        spellcheck="false"
        bind:value={url}
      />
      <TextField id="dav-user" label="Username" autocomplete="username" spellcheck="false" bind:value={username} />
      <PasswordField id="dav-pass" label="App password" bind:value={password} />
      <div>
        <Button type="submit" variant="primary" disabled={busy || !url || !username || !password}>
          {busy ? 'Connecting…' : mode === 'join' ? 'Find my vault' : 'Connect and upload'}
        </Button>
      </div>
    </form>
  {:else}
    <div class="stack">
      <p class="body">Scytale gets its own folder (Apps, Scytale) and nothing else in your Dropbox.</p>
      <div>
        <Button variant="primary" disabled={busy} onclick={connectDropbox}>{busy ? 'Connecting…' : 'Sign in with Dropbox'}</Button>
      </div>
    </div>
  {/if}
</div>

<style>
  .setup,
  .stack {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .body {
    margin: 0;
    color: var(--ink-soft);
    line-height: 1.6;
  }
</style>
