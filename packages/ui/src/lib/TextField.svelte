<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLInputAttributes } from 'svelte/elements';

  type Props = Omit<HTMLInputAttributes, 'value'> & {
    id: string;
    label: string;
    value?: string;
    hint?: string;
    error?: string;
    mono?: boolean;
    /** Buttons inside the field's right edge (reveal, generate…). */
    trailing?: Snippet;
  };

  let { id, label, value = $bindable(''), hint, error, mono = false, trailing, ...rest }: Props = $props();
  const msgId = $derived(error || hint ? `${id}-msg` : undefined);
</script>

<div class="field">
  <label for={id}>{label}</label>
  <div class="control" class:invalid={!!error}>
    <input {id} class:mono bind:value aria-invalid={error ? true : undefined} aria-describedby={msgId} {...rest} />
    {#if trailing}
      <div class="trailing">{@render trailing()}</div>
    {/if}
  </div>
  {#if error}
    <p id={msgId} class="msg error">{error}</p>
  {:else if hint}
    <p id={msgId} class="msg">{hint}</p>
  {/if}
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--ink-soft);
  }
  .control {
    display: flex;
    align-items: center;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    transition: border-color 0.15s var(--ease);
  }
  .control:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .invalid {
    border-color: var(--danger);
  }
  input {
    flex: 1;
    min-width: 0;
    height: var(--hit);
    padding: 0 14px;
    border: 0;
    background: transparent;
    font-size: var(--text-lg);
  }
  input:focus-visible {
    outline: none;
  }
  .mono {
    font-family: ui-monospace, 'Cascadia Mono', 'SF Mono', Menlo, Consolas, monospace;
    letter-spacing: 0.02em;
  }
  .trailing {
    display: flex;
    padding-right: 2px;
  }
  .msg {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--muted);
  }
  .error {
    color: var(--danger);
  }
</style>
