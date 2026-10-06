<script lang="ts">
  import Button from '../lib/Button.svelte';
  import PasswordField from '../lib/PasswordField.svelte';
  import ScytaleStrip from '../lib/ScytaleStrip.svelte';
  import TextField from '../lib/TextField.svelte';
  import Wordmark from '../lib/Wordmark.svelte';

  let {
    needsSecretKey = false,
    busy = false,
    error,
    unlocked = false,
    onunlock,
    onforgot,
  }: {
    /** First unlock on a new device: ask for the Secret Key too. */
    needsSecretKey?: boolean;
    busy?: boolean;
    error?: string;
    /** Set once the vault opened: plays the strip's unwind before the vault appears. */
    unlocked?: boolean;
    onunlock: (password: string, secretKey?: string) => void;
    onforgot: () => void;
  } = $props();

  let password = $state('');
  let secretKey = $state('');

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!password || busy) return;
    onunlock(password, needsSecretKey ? secretKey : undefined);
  }
</script>

<form class="unlock" onsubmit={submit}>
  <div class="brand">
    <Wordmark size={34} />
    <ScytaleStrip unwound={unlocked} height={30} />
  </div>

  <div class="fields">
    {#if needsSecretKey}
      <TextField
        id="unlock-secret-key"
        label="Secret Key"
        placeholder="S1-XXXXX-XXXXX-…"
        hint="It is on your Emergency Kit, or in Settings on a device that is already signed in."
        mono
        autocomplete="off"
        spellcheck="false"
        bind:value={secretKey}
      />
    {/if}
    <PasswordField id="unlock-password" label="Master password" {error} autofocus bind:value={password} />
  </div>

  <Button type="submit" variant="primary" size="lg" full disabled={!password || busy}>
    {busy ? 'Unlocking…' : 'Unlock'}
  </Button>
  <Button variant="ghost" onclick={onforgot}>Forgot your master password?</Button>
</form>

<style>
  .unlock {
    width: 100%;
    max-width: 360px;
    margin: 0 auto;
    padding: 40px 20px 24px;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .brand {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
  }
  .brand :global(.rod) {
    align-self: stretch;
  }
  .fields {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
</style>
