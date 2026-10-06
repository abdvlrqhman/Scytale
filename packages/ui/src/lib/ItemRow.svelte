<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { ItemSummary } from '../types';
  import Monogram from './Monogram.svelte';

  let {
    item,
    selected = false,
    size = 'md',
    onclick,
    trailing,
  }: {
    item: ItemSummary;
    selected?: boolean;
    size?: 'sm' | 'md';
    onclick?: (id: string) => void;
    /** Actions after the row, e.g. a Fill button. Kept outside the main button: no nested buttons. */
    trailing?: Snippet;
  } = $props();
</script>

<div class="row" class:selected>
  <button type="button" class="main" aria-current={selected || undefined} onclick={() => onclick?.(item.id)}>
    <Monogram title={item.title} kind={item.kind} {size} />
    <span class="text">
      <span class="title">{item.title}</span>
      <span class="subtitle">{item.subtitle}</span>
    </span>
  </button>
  {#if trailing}
    <div class="trailing">{@render trailing()}</div>
  {/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    border-radius: var(--r-md);
    transition: background-color 0.15s var(--ease);
  }
  .row:hover {
    background: var(--surface-hover);
  }
  .selected,
  .selected:hover {
    background: var(--surface-raised);
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-height: 54px;
    padding: 6px 10px;
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    text-align: left;
  }
  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .title,
  .subtitle {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .title {
    font-weight: 600;
    font-size: var(--text-md);
  }
  .subtitle {
    font-size: var(--text-sm);
    color: var(--muted);
  }
  .trailing {
    padding-right: 6px;
    display: flex;
    gap: var(--space-1);
  }
</style>
