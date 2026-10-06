<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';
  import Icon from './Icon.svelte';

  type Props = Omit<HTMLInputAttributes, 'value'> & { id: string; label: string; value?: string };

  let { id, label, value = $bindable(''), ...rest }: Props = $props();
</script>

<div class="search">
  <label for={id} class="sr-only">{label}</label>
  <Icon name="search" />
  <input {id} type="search" autocomplete="off" spellcheck="false" bind:value {...rest} />
</div>

<style>
  .search {
    position: relative;
    display: flex;
    align-items: center;
    color: var(--placeholder);
  }
  .search :global(svg) {
    position: absolute;
    left: 14px;
    pointer-events: none;
  }
  input {
    width: 100%;
    height: var(--hit);
    padding: 0 14px 0 42px;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--ink);
    font-size: var(--text-md);
  }
  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 0;
    border-color: transparent;
  }
</style>
