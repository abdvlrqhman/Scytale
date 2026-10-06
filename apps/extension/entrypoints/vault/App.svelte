<script lang="ts">
  // Full-tab page for what does not fit a popup: setup, the Emergency Kit, import, export and
  // password changes. Routed by the URL hash (vault.html#import).
  import {
    Button,
    EmergencyKit,
    Onboarding,
    PasswordField,
    ScytaleStrip,
    StrengthMeter,
    Unlock,
    Wordmark,
  } from '@scytale/ui';
  import type { StorageProvider } from '@scytale/ui/types';
  import { onMount } from 'svelte';
  import { send } from '@/src/messages';
  import { applyTheme, copyText, download, loadStrength, rate, today } from '@/src/ui-helpers';

  type Route = 'setup' | 'join' | 'import' | 'export' | 'kit' | 'password' | 'forgot' | 'sync';
  const routes: Route[] = ['setup', 'join', 'import', 'export', 'kit', 'password', 'forgot', 'sync'];

  let route: Route = $state('setup');
  let status: 'new' | 'locked' | 'unlocked' | undefined = $state();
  let needsSecretKey = $state(false);
  let message: { text: string; tone: 'ok' | 'error' } | null = $state(null);
  let ratingReady = $state(0); // bumps when zxcvbn loads, so meters re-rate

  // Setup
  let secretKey: string | undefined = $state();
  let showKit = $state(false);
  let storage = 'This device only (sync is off)';
  let setupDone = $state(false);

  // Unlock (for pages that need an open vault)
  let unlockError: string | undefined = $state();
  let unlocking = $state(false);

  // Import / export / kit / password
  let importPassword = $state('');
  let pendingExport: File | null = $state(null);
  let exportPassword = $state('');
  let exportConfirm = $state('');
  let csvConfirm = $state(false);
  let kitPassword = $state('');
  let kitKey: string | undefined = $state();
  let current = $state('');
  let next = $state('');
  let nextConfirm = $state('');

  const say = (text: string, tone: 'ok' | 'error' = 'ok') => (message = { text, tone });
  const fail = (e: unknown) => say(e instanceof Error ? e.message : String(e), 'error');

  async function refresh() {
    const s = await send({ type: 'status' });
    status = s.status;
    needsSecretKey = s.needsSecretKey;
    applyTheme(s.settings.theme);
  }

  function readRoute() {
    const h = location.hash.slice(1) as Route;
    route = routes.includes(h) ? h : 'setup';
    message = null;
  }

  onMount(() => {
    readRoute();
    window.addEventListener('hashchange', readRoute);
    refresh().catch(fail);
    loadStrength().then(() => ratingReady++);
    return () => window.removeEventListener('hashchange', readRoute);
  });

  const rateNow = (pw: string) => {
    void ratingReady; // re-rate once zxcvbn has loaded
    return rate(pw);
  };

  // --- Setup ----------------------------------------------------------------------------------
  async function create(password: string) {
    try {
      secretKey = (await send({ type: 'create', password })).secretKey;
      status = 'unlocked';
    } catch (e) {
      fail(e);
    }
  }

  async function suggestPassphrase() {
    return (
      await send({
        type: 'generate',
        options: {
          mode: 'passphrase',
          words: 4,
          separator: '-',
          capitalize: true,
          includeNumber: true,
          length: 20,
          lower: true,
          upper: true,
          digits: true,
          symbols: true,
          avoidAmbiguous: false,
        },
      })
    ).value;
  }
  let suggestion = $state('');
  onMount(() => {
    suggestPassphrase().then((s) => (suggestion = s));
  });

  function chooseStorage(p: StorageProvider) {
    if (p !== 'none') say('Sync arrives in the next update. Your vault is saved on this device for now.');
  }

  // --- Unlock gate --------------------------------------------------------------------------
  async function unlock(password: string, sk?: string) {
    unlocking = true;
    unlockError = undefined;
    try {
      await send({ type: 'unlock', password, secretKey: sk });
      await refresh();
    } catch (e) {
      unlockError = e instanceof Error ? e.message : String(e);
    } finally {
      unlocking = false;
    }
  }

  // --- Import --------------------------------------------------------------------------------
  async function onFile(e: Event) {
    const file = (e.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    message = null;
    if (file.name.endsWith('.scyx')) {
      pendingExport = file;
      return;
    }
    try {
      const r = await send({ type: 'importText', text: await file.text() });
      report(r);
    } catch (err) {
      fail(err);
    }
  }

  async function importEncrypted() {
    if (!pendingExport) return;
    try {
      const bytes = new Uint8Array(await pendingExport.arrayBuffer());
      let s = '';
      for (const b of bytes) s += String.fromCharCode(b);
      report(await send({ type: 'importEncrypted', b64: btoa(s), password: importPassword }));
      pendingExport = null;
      importPassword = '';
    } catch (err) {
      fail(err);
    }
  }

  function report(r: { added: number; skipped: number }) {
    const skipped = r.skipped ? ` Skipped ${r.skipped} that had nothing to import.` : '';
    say(`Added ${r.added} item${r.added === 1 ? '' : 's'} to your vault.${skipped}`);
  }

  // --- Export --------------------------------------------------------------------------------
  async function exportEncrypted() {
    if (exportPassword.length < 8 || exportPassword !== exportConfirm) return;
    try {
      const b64 = await send({ type: 'exportEncrypted', password: exportPassword });
      const bin = atob(b64);
      const bytes = Uint8Array.from(bin, (c) => c.charCodeAt(0));
      download(`scytale-export-${today()}.scyx`, bytes, 'application/octet-stream');
      exportPassword = exportConfirm = '';
      say('Encrypted export saved to your downloads.');
    } catch (e) {
      fail(e);
    }
  }

  async function exportCsv() {
    try {
      const r = await send({ type: 'exportCsv' });
      download(`scytale-export-${today()}.csv`, r.csv, 'text/csv');
      csvConfirm = false;
      say(
        r.omitted
          ? `CSV saved. ${r.omitted} cards and identities are not in it; use the encrypted export for those.`
          : 'CSV saved. Delete it once you have imported it elsewhere.',
      );
    } catch (e) {
      fail(e);
    }
  }

  // --- Kit -------------------------------------------------------------------------------------
  async function revealKit() {
    try {
      kitKey = await send({ type: 'secretKey', password: kitPassword });
      kitPassword = '';
    } catch (e) {
      fail(e);
    }
  }

  // --- Password --------------------------------------------------------------------------------
  const nextStrength = $derived(next ? rateNow(next) : null);
  async function changePassword(e: SubmitEvent) {
    e.preventDefault();
    if (next !== nextConfirm || !nextStrength || nextStrength.score < 2) return;
    try {
      await send({ type: 'changePassword', current, next });
      current = next = nextConfirm = '';
      say('Master password changed. Use the new one from now on, on every device.');
    } catch (err) {
      fail(err);
    }
  }

  const needsUnlock = $derived(
    status === 'locked' && ['import', 'export', 'kit', 'password'].includes(route),
  );
</script>

<div class="shell">
  <header class="top">
    <Wordmark size={26} />
  </header>

  {#if message}
    <p class="message {message.tone}" role={message.tone === 'error' ? 'alert' : 'status'}>{message.text}</p>
  {/if}

  {#if needsUnlock}
    <section class="card narrow">
      <Unlock {needsSecretKey} busy={unlocking} error={unlockError} onunlock={unlock} onforgot={() => (location.hash = 'forgot')} />
    </section>
  {:else if route === 'setup'}
    {#if status === 'unlocked' && !secretKey && !setupDone}
      <section class="card narrow">
        <h1>Scytale is set up</h1>
        <p class="body">Open Scytale from your browser's toolbar. Tip: pin it so it is one click away.</p>
      </section>
    {:else if showKit && secretKey}
      <section class="kit-wrap">
        <EmergencyKit {secretKey} {storage} created={new Date().toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' })} />
        <div class="actions no-print">
          <Button variant="primary" onclick={() => window.print()}>Print or save as PDF</Button>
          <Button onclick={() => (showKit = false)}>Back</Button>
        </div>
      </section>
    {:else if setupDone}
      <section class="card narrow center">
        <ScytaleStrip unwound height={30} />
        <h1>You're all set</h1>
        <p class="body">Open Scytale from your browser's toolbar. Pin it so it is one click away.</p>
      </section>
    {:else}
      <section class="card narrow">
        <Onboarding
          {secretKey}
          rate={rateNow}
          suggest={() => {
            const s = suggestion;
            suggestPassphrase().then((v) => (suggestion = v));
            return s;
          }}
          oncreate={create}
          onsavekit={() => (showKit = true)}
          oncopykey={() => secretKey && copyText(secretKey, false).then(() => say('Secret Key copied. Paste it somewhere safe, then clear it.'))}
          onstorage={chooseStorage}
          onimport={() => {
            setupDone = true;
            location.hash = 'import';
          }}
          onfinish={() => (setupDone = true)}
          onjoin={() => (location.hash = 'join')}
        />
      </section>
    {/if}
  {:else if route === 'join'}
    <section class="card narrow">
      <h1>Add this device</h1>
      <p class="body">
        Your vault reaches new devices through sync, which arrives in the next update. Until then, export from your other
        device (Settings, Export encrypted file) and import that file here after setting up.
      </p>
      <Button variant="primary" onclick={() => (location.hash = 'setup')}>Set up this device</Button>
    </section>
  {:else if route === 'import'}
    <section class="card">
      <h1>Import passwords</h1>
      <p class="body">
        Choose an export from Chrome, Edge, Brave, Firefox, Safari, Bitwarden (CSV or unencrypted JSON), 1Password,
        LastPass or KeePassXC, or a Scytale encrypted export. Delete plain exports once imported.
      </p>
      <label class="file">
        <span>Choose a file</span>
        <input type="file" accept=".csv,.json,.scyx,text/csv,application/json" onchange={onFile} />
      </label>
      {#if pendingExport}
        <PasswordField id="imp-pw" label="Password of this Scytale export" bind:value={importPassword} />
        <div class="actions"><Button variant="primary" disabled={!importPassword} onclick={importEncrypted}>Import</Button></div>
      {/if}
    </section>
  {:else if route === 'export'}
    <section class="card">
      <h1>Export</h1>
      <h2>Encrypted file</h2>
      <p class="body">Everything, including cards and identities, locked with a password you choose now.</p>
      <PasswordField id="exp-pw" label="Export password" hint="At least 8 characters." bind:value={exportPassword} />
      <PasswordField
        id="exp-pw2"
        label="Type it again"
        error={exportConfirm && exportConfirm !== exportPassword ? 'The two passwords are different.' : undefined}
        bind:value={exportConfirm}
      />
      <div class="actions">
        <Button variant="primary" disabled={exportPassword.length < 8 || exportPassword !== exportConfirm} onclick={exportEncrypted}>
          Save encrypted export
        </Button>
      </div>
      <h2>Plain CSV</h2>
      <p class="body">For moving to another password manager. Anyone who opens this file can read every password in it.</p>
      <label class="check"><input type="checkbox" bind:checked={csvConfirm} /> I understand the CSV is not protected.</label>
      <div class="actions"><Button variant="danger" disabled={!csvConfirm} onclick={exportCsv}>Save readable CSV</Button></div>
    </section>
  {:else if route === 'kit'}
    {#if kitKey}
      <section class="kit-wrap">
        <EmergencyKit secretKey={kitKey} {storage} created={today()} />
        <div class="actions no-print">
          <Button variant="primary" onclick={() => window.print()}>Print or save as PDF</Button>
          <Button onclick={() => (kitKey = undefined)}>Hide</Button>
        </div>
      </section>
    {:else}
      <section class="card narrow">
        <h1>Emergency Kit</h1>
        <p class="body">Enter your master password to show your Secret Key.</p>
        <PasswordField id="kit-pw" label="Master password" bind:value={kitPassword} />
        <div class="actions"><Button variant="primary" disabled={!kitPassword} onclick={revealKit}>Show my kit</Button></div>
      </section>
    {/if}
  {:else if route === 'password'}
    <section class="card narrow">
      <form class="stack" onsubmit={changePassword}>
        <h1>Change master password</h1>
        <PasswordField id="cp-current" label="Current password" bind:value={current} />
        <PasswordField id="cp-next" label="New password" bind:value={next} />
        {#if nextStrength}<StrengthMeter {...nextStrength} />{/if}
        <PasswordField
          id="cp-confirm"
          label="Type the new one again"
          error={nextConfirm && nextConfirm !== next ? 'The two passwords are different.' : undefined}
          bind:value={nextConfirm}
        />
        <Button type="submit" variant="primary" disabled={!current || !nextStrength || nextStrength.score < 2 || next !== nextConfirm}>
          Change password
        </Button>
      </form>
    </section>
  {:else if route === 'forgot'}
    <section class="card narrow">
      <h1>Forgot your master password?</h1>
      <p class="body">
        Nobody can reset it: not us, not your cloud provider. That is what keeps your vault private. If a device is still
        unlocked, export an encrypted file now, then set up again with a new password and import it. Otherwise the vault
        cannot be opened.
      </p>
    </section>
  {:else if route === 'sync'}
    <section class="card narrow">
      <h1>Sync</h1>
      <p class="body">Sync through your own Dropbox, Google Drive, OneDrive or WebDAV arrives in the next update.</p>
    </section>
  {/if}
</div>

<style>
  .shell {
    max-width: 760px;
    margin: 0 auto;
    padding-block: 32px 64px;
    padding-inline: 16px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .top {
    padding: 0 4px;
  }
  .card {
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--r-xl);
    padding: 28px;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .narrow {
    max-width: 520px;
    width: 100%;
  }
  .center {
    align-items: center;
    text-align: center;
  }
  .center :global(.rod) {
    align-self: stretch;
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
  .body {
    margin: 0;
    color: var(--ink-soft);
    line-height: 1.6;
    max-width: 62ch;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .message {
    margin: 0;
    padding: 14px 18px;
    border-radius: var(--r-md);
    background: var(--surface-raised);
    border: 1px solid var(--line);
  }
  .message.error {
    background: var(--danger-soft);
    color: var(--danger);
    border-color: transparent;
  }
  .file {
    display: inline-flex;
    flex-direction: column;
    gap: 8px;
    font-weight: 600;
  }
  .file input {
    padding: 12px;
    border: 1px dashed var(--line);
    border-radius: var(--r-md);
    background: var(--surface-raised);
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
  .kit-wrap {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  @media print {
    .no-print,
    .top,
    .message {
      display: none !important;
    }
    .shell {
      padding: 0;
      max-width: none;
    }
  }
</style>
