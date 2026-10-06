<script lang="ts">
  import Button from '../lib/Button.svelte';
  import PasswordField from '../lib/PasswordField.svelte';
  import SegmentedControl from '../lib/SegmentedControl.svelte';
  import Switch from '../lib/Switch.svelte';
  import TextField from '../lib/TextField.svelte';
  import type { ItemDraft, ItemKind } from '../types';

  let {
    initial,
    isNew = false,
    suggest,
    onsave,
    oncancel,
    ondelete,
  }: {
    initial: ItemDraft;
    isNew?: boolean;
    /** Returns a freshly generated password for the password field. */
    suggest: () => string;
    onsave: (draft: ItemDraft) => void;
    oncancel: () => void;
    ondelete?: () => void;
  } = $props();

  // A local copy: nothing changes until Save.
  // svelte-ignore state_referenced_locally
  let d: ItemDraft = $state({ ...initial });
  let triedSave = $state(false);
  let confirmingDelete = $state(false);

  const titleError = $derived(triedSave && !d.title.trim() ? 'Give it a name, for example the website.' : undefined);

  const kinds: { value: ItemKind; label: string }[] = [
    { value: 'login', label: 'Login' },
    { value: 'note', label: 'Note' },
    { value: 'card', label: 'Card' },
    { value: 'identity', label: 'Identity' },
  ];

  function save(e: SubmitEvent) {
    e.preventDefault();
    triedSave = true;
    if (!d.title.trim()) return;
    onsave($state.snapshot(d));
  }
</script>

<form class="edit" onsubmit={save} novalidate>
  <h1>{isNew ? 'New item' : `Edit ${initial.title}`}</h1>

  {#if isNew}
    <SegmentedControl legend="Item type" name="kind" options={kinds} bind:value={d.kind} />
  {/if}

  <TextField id="e-title" label="Name" error={titleError} autocomplete="off" bind:value={d.title} />

  {#if d.kind === 'login'}
    <TextField id="e-username" label="Username or email" autocomplete="off" spellcheck="false" bind:value={d.username} />
    <PasswordField id="e-password" label="Password" ongenerate={() => (d.password = suggest())} bind:value={d.password} />
    <TextField
      id="e-urls"
      label="Websites"
      hint="One per line. Scytale only fills on these sites."
      autocomplete="off"
      spellcheck="false"
      bind:value={d.urls}
    />
    <Switch
      id="e-exact"
      label="Only fill on the exact address"
      description="Off: also fill on other parts of the same site, like login.example.com."
      bind:checked={d.exactHost}
    />
    <TextField
      id="e-totp"
      label="One-time code secret"
      hint="Paste the setup key or otpauth:// link from the site's 2-step settings."
      mono
      autocomplete="off"
      spellcheck="false"
      bind:value={d.totp}
    />
  {:else if d.kind === 'card'}
    <TextField id="e-holder" label="Name on card" autocomplete="off" bind:value={d.holder} />
    <TextField id="e-number" label="Card number" inputmode="numeric" mono autocomplete="off" bind:value={d.number} />
    <div class="pair">
      <TextField id="e-expiry" label="Expiry" placeholder="MM/YY" autocomplete="off" bind:value={d.expiry} />
      <PasswordField id="e-cvv" label="Security code" bind:value={d.cvv} />
    </div>
  {:else if d.kind === 'identity'}
    <TextField id="e-name" label="Full name" autocomplete="off" bind:value={d.fullName} />
    <TextField id="e-email" label="Email" type="email" autocomplete="off" bind:value={d.email} />
    <TextField id="e-phone" label="Phone" type="tel" autocomplete="off" bind:value={d.phone} />
    <TextField id="e-address" label="Address" autocomplete="off" bind:value={d.address} />
  {/if}

  <div class="notes">
    <label for="e-notes">Notes</label>
    <textarea id="e-notes" rows="4" bind:value={d.notes}></textarea>
  </div>

  <div class="bar">
    {#if ondelete && !isNew}
      {#if confirmingDelete}
        <div class="confirm" role="alert">
          <span>Delete “{initial.title}” from all your devices?</span>
          <Button variant="danger" onclick={ondelete}>Delete</Button>
          <Button variant="ghost" onclick={() => (confirmingDelete = false)}>Keep it</Button>
        </div>
      {:else}
        <Button variant="danger" onclick={() => (confirmingDelete = true)}>Delete</Button>
      {/if}
    {/if}
    <span class="spacer"></span>
    <Button variant="ghost" onclick={oncancel}>Cancel</Button>
    <Button type="submit" variant="primary">Save</Button>
  </div>
</form>

<style>
  .edit {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-width: 560px;
  }
  h1 {
    margin: 0 0 var(--space-2);
    font-family: var(--font-display);
    font-weight: 400;
    font-size: var(--text-3xl);
  }
  .pair {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-3);
  }
  .notes {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .notes label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--ink-soft);
  }
  textarea {
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    font-size: var(--text-lg);
    line-height: 1.5;
    resize: vertical;
  }
  textarea:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    padding-top: var(--space-2);
  }
  .spacer {
    flex: 1;
  }
  .confirm {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    color: var(--danger);
  }
</style>
