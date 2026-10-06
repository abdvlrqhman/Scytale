<script lang="ts">
  import Button from '../src/lib/Button.svelte';
  import ScytaleStrip from '../src/lib/ScytaleStrip.svelte';
  import SegmentedControl from '../src/lib/SegmentedControl.svelte';
  import TitleBar from '../src/lib/TitleBar.svelte';
  import Toast from '../src/lib/Toast.svelte';
  import Generator from '../src/screens/Generator.svelte';
  import ItemDetail from '../src/screens/ItemDetail.svelte';
  import ItemEdit from '../src/screens/ItemEdit.svelte';
  import Onboarding from '../src/screens/Onboarding.svelte';
  import Popup from '../src/screens/Popup.svelte';
  import Settings from '../src/screens/Settings.svelte';
  import Unlock from '../src/screens/Unlock.svelte';
  import VaultWindow, { type Section } from '../src/screens/VaultWindow.svelte';
  import type { GeneratorOptions, ItemView, Theme } from '../src/types';
  import * as mock from './mock';

  type Screen = 'popup' | 'unlock' | 'onboarding' | 'desktop' | 'tokens';
  const screens: { value: Screen; label: string }[] = [
    { value: 'popup', label: 'Popup' },
    { value: 'unlock', label: 'Unlock' },
    { value: 'onboarding', label: 'First run' },
    { value: 'desktop', label: 'Desktop' },
    { value: 'tokens', label: 'Brand' },
  ];
  const initialHash = location.hash.slice(1);
  const hashParts = initialHash.split('-');

  let theme: Theme = $state(hashParts.includes('light') ? 'light' : 'dark');
  // "frameonly": just the screen, edge to edge (used to compose the README hero image).
  const frameOnly = hashParts.includes('frameonly');
  let screen: Screen = $state(screens.some((s) => s.value === hashParts[0]) ? (hashParts[0] as Screen) : 'popup');
  let toast: string | null = $state(null);
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  // Desktop state
  let section: Section = $state(
    hashParts.includes('settings') ? 'settings' : hashParts.includes('generator') ? 'generator' : 'all',
  );
  let editing = $state(hashParts.includes('edit'));
  let item: ItemView = $state(structuredClone(mock.github));
  let autoLockMinutes = $state(15);
  let clipboardSeconds = $state(30);
  let lockOnClose = $state(true);
  let startAtLogin = $state(true);
  let maximized = $state(false);
  let platform: 'windows' | 'macos' | 'linux' = $state(hashParts.includes('mac') ? 'macos' : 'windows');

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
  let genSeed = $state(0);
  const generated = $derived.by(() => {
    void genSeed; // "New" bumps the seed to regenerate with the same options
    return mock.generate(genOptions);
  });

  // Unlock demo
  let unlockError: string | undefined = $state();
  let unlocked = $state(false);

  // Onboarding demo
  let secretKey: string | undefined = $state();

  // Strip demo
  let unwound = $state(false);

  $effect(() => {
    document.documentElement.dataset.theme = theme;
  });

  function say(message: string) {
    toast = message;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), 2600);
  }

  const copied = (what: string) => say(`${what} copied. Clipboard clears in ${clipboardSeconds || 30} s.`);

  function setSecret(key: string, value: string | undefined) {
    item.fields = item.fields.map((f) => (f.key === key ? { ...f, value } : f));
  }

  const radii = [
    { name: 'Small', token: '--r-sm', use: 'Keycaps, tags' },
    { name: 'Medium', token: '--r-md', use: 'Buttons, inputs, rows' },
    { name: 'Large', token: '--r-lg', use: 'Cards' },
    { name: 'Extra large', token: '--r-xl', use: 'Panels, windows' },
    { name: 'Pill', token: '--r-pill', use: 'Switches, chips' },
  ];
</script>

<div class="page" class:bare={frameOnly}>
  {#if !frameOnly}
  <header class="bar">
    <div class="titles">
      <h1>Scytale: Bronze</h1>
      <p>Live components with sample data. Everything is clickable.</p>
    </div>
    <div class="controls">
      <SegmentedControl legend="Screen" name="screen" options={screens} bind:value={screen} />
      <SegmentedControl
        legend="Theme"
        name="theme"
        options={[
          { value: 'dark', label: 'Dark' },
          { value: 'light', label: 'Light' },
        ]}
        bind:value={theme}
      />
    </div>
  </header>
  {/if}

  {#if screen === 'popup'}
    <div class="frame popup">
      <Popup
        site="github.com"
        matches={mock.matches}
        items={mock.items}
        sync={mock.sync}
        onfill={(id) => say(`Filled ${mock.items.find((i) => i.id === id)?.title} on github.com`)}
        onopen={(id) => say(`Open ${id}`)}
        onlock={() => say('Vault locked')}
        ongenerate={() => say('Generator opens here')}
        onadd={() => say('New item opens here')}
        onsettings={() => say('Settings open here')}
        onimport={() => say('Import opens here')}
      />
      <Toast message={toast} />
    </div>
  {:else if screen === 'unlock'}
    <div class="frame popup unlock">
      <Unlock
        error={unlockError}
        {unlocked}
        onunlock={(pw) => {
          if (pw === 'scytale') {
            unlockError = undefined;
            unlocked = true;
          } else {
            unlockError = 'That password does not open this vault. Check Caps Lock and try again.';
            unlocked = false;
          }
        }}
        onforgot={() =>
          say('Nobody can reset it. Use your Emergency Kit on a device that is still signed in.')}
      />
      <p class="hint-line">Sample password: <code>scytale</code></p>
      <Toast message={toast} />
    </div>
  {:else if screen === 'onboarding'}
    <div class="frame onboarding">
      <Onboarding
        {secretKey}
        allowFolder
        rate={mock.rate}
        suggest={mock.suggestPassphrase}
        oncreate={() => setTimeout(() => (secretKey = 'S1-7KQ2M-X9TRD-4VHNP-0ZCWE-8FJ3A-B6Y'), 600)}
        onsavekit={() => say('The Emergency Kit PDF saves here')}
        oncopykey={() => say('Secret Key copied')}
        onstorage={(p) => say(`Connect ${p}`)}
        onimport={() => say('Import opens here')}
        onfinish={() => (screen = 'desktop')}
        onjoin={() => (screen = 'unlock')}
      />
      <Toast message={toast} />
    </div>
  {:else if screen === 'desktop'}
    <div class="desk-scroll">
      <div class="frame desktop">
        <TitleBar
          {platform}
          {maximized}
          onminimize={() => say('Minimized')}
          ontogglemaximize={() => (maximized = !maximized)}
          onclose={() => say('Hidden to the tray. Scytale keeps running there.')}
        />
        <VaultWindow
          items={mock.items}
          bind:section
          selectedId={section === 'generator' || section === 'settings' ? undefined : 'gh'}
          sync={mock.sync}
          onselect={(id) => {
            editing = false;
            if (section === 'generator' || section === 'settings') section = 'all';
            say(id === 'gh' ? 'GitHub is open' : `${id} opens here`);
          }}
          onadd={() => say('New item opens here')}
          onlock={() => (screen = 'unlock')}
        >
          {#if section === 'generator'}
            <Generator
              bind:options={genOptions}
              value={generated.value}
              bits={generated.bits}
              onregenerate={() => genSeed++}
              oncopy={() => copied('Password')}
            />
          {:else if section === 'settings'}
            <Settings
              bind:theme
              bind:autoLockMinutes
              bind:clipboardSeconds
              bind:lockOnClose
              bind:startAtLogin
              storageName="Dropbox"
              sync={mock.sync}
              devices={mock.devices}
              version="0.1.0"
              onsyncnow={() => say('Synced')}
              onchangestorage={() => say('Storage picker opens here')}
              onremovedevice={() => say('Device removed')}
              onimport={() => say('Import opens here')}
              onexportencrypted={() => say('Encrypted export saved')}
              onexportcsv={() => say('CSV exported')}
              onchangepassword={() => say('Change password opens here')}
              onshowkit={() => say('Asks for your master password first')}
            />
          {:else if editing}
            <ItemEdit
              initial={mock.githubDraft}
              suggest={() => mock.generate({ ...genOptions, mode: 'password' }).value}
              onsave={() => {
                editing = false;
                say('Saved');
              }}
              oncancel={() => (editing = false)}
              ondelete={() => {
                editing = false;
                say('Deleted from all devices');
              }}
            />
          {:else}
            <ItemDetail
              {item}
              onreveal={(key) => setSecret(key, mock.githubPassword)}
              onhide={(key) => setSecret(key, undefined)}
              oncopy={(key) => copied(key === 'password' ? 'Password' : 'Username')}
              oncopytotp={() => copied('Code')}
              onedit={() => (editing = true)}
              onfavorite={() => (item.favorite = !item.favorite)}
            />
          {/if}
        </VaultWindow>
        <Toast message={toast} />
      </div>
    </div>
  {:else}
    <div class="brand-page">
      <section>
        <h2>The strip</h2>
        <p>The one moving part of the brand. It stays still, and unwinds only when the vault unlocks.</p>
        <ScytaleStrip {unwound} height={34} />
        <Button onclick={() => (unwound = !unwound)}>{unwound ? 'Wind it back' : 'Play the unlock'}</Button>
      </section>
      <section>
        <h2>Corner radius</h2>
        <ul class="radii">
          {#each radii as r (r.token)}
            <li>
              <span class="swatch" style:border-radius="var({r.token})"></span>
              <span><strong>{r.name}</strong><br /><small>{r.use}</small></span>
            </li>
          {/each}
        </ul>
      </section>
    </div>
  {/if}
</div>

<style>
  .bare {
    padding: 0 !important;
    max-width: none !important;
  }
  .bare .frame {
    border: 0;
    border-radius: 0;
    box-shadow: none;
  }
  .bare .desktop {
    height: 100vh;
  }
  .bare .desk-scroll {
    padding: 0;
  }
  .page {
    min-height: 100vh;
    padding-block: 24px 48px;
    padding-inline: 16px;
    max-width: 1280px;
    margin: 0 auto;
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 28px;
  }
  .titles {
    min-width: 0;
  }
  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-weight: 400;
    font-size: var(--text-3xl);
  }
  .titles p {
    margin: 4px 0 0;
    color: var(--muted);
  }
  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .frame {
    position: relative;
    border-radius: var(--r-xl);
    overflow: hidden;
    border: 1px solid var(--line);
    box-shadow: var(--shadow-pop);
    background: var(--bg);
  }
  .popup {
    width: 380px;
    max-width: 100%;
  }
  .unlock {
    min-height: 600px;
  }
  .hint-line {
    margin: 0;
    padding: 0 20px 20px;
    text-align: center;
    color: var(--muted);
    font-size: var(--text-sm);
  }
  .onboarding {
    width: 480px;
    max-width: 100%;
    min-height: 640px;
  }
  .desk-scroll {
    overflow-x: auto;
    padding-bottom: 16px;
  }
  .desktop {
    width: 1200px;
    height: 820px;
    display: flex;
    flex-direction: column;
  }
  .desktop > :global(.window) {
    flex: 1;
    min-height: 0;
  }
  .brand-page {
    display: flex;
    flex-wrap: wrap;
    gap: 40px;
  }
  .brand-page section {
    flex: 1 1 320px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: flex-start;
  }
  .brand-page :global(.rod) {
    align-self: stretch;
  }
  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-weight: 400;
    font-size: var(--text-2xl);
  }
  .brand-page p {
    margin: 0;
    color: var(--ink-soft);
    max-width: 52ch;
  }
  .radii {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 12px;
  }
  .radii li {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .swatch {
    width: 72px;
    height: 44px;
    background: var(--surface-raised);
    border: 1px solid var(--accent);
  }
  small {
    color: var(--muted);
  }
</style>
