<script lang="ts">
  import Button from '../lib/Button.svelte';
  import Icon from '../lib/Icon.svelte';
  import IconButton from '../lib/IconButton.svelte';
  import ItemRow from '../lib/ItemRow.svelte';
  import ScytaleStrip from '../lib/ScytaleStrip.svelte';
  import SearchField from '../lib/SearchField.svelte';
  import SyncBadge from '../lib/SyncBadge.svelte';
  import Wordmark from '../lib/Wordmark.svelte';
  import type { ItemSummary, SyncInfo } from '../types';

  let {
    site,
    matches,
    items,
    sync,
    query = $bindable(''),
    onfill,
    onopen,
    onlock,
    ongenerate,
    onadd,
    onsettings,
    onimport,
  }: {
    /** Host of the current tab, if it is a normal web page. */
    site?: string;
    matches: ItemSummary[];
    items: ItemSummary[];
    sync: SyncInfo;
    query?: string;
    onfill: (id: string) => void;
    onopen: (id: string) => void;
    onlock: () => void;
    ongenerate: () => void;
    onadd: () => void;
    onsettings: () => void;
    onimport: () => void;
  } = $props();

  const q = $derived(query.trim().toLowerCase());
  const results = $derived(
    q ? items.filter((i) => i.title.toLowerCase().includes(q) || i.subtitle.toLowerCase().includes(q)) : items,
  );
  const showMatches = $derived(!q && matches.length > 0);

  function onSearchKey(e: KeyboardEvent) {
    // Enter fills the best match, so the common case is: open popup, press Enter.
    if (e.key !== 'Enter') return;
    const first = showMatches ? matches[0] : results[0];
    if (first && (showMatches || first.kind === 'login')) onfill(first.id);
  }
</script>

<div class="popup">
  <header>
    <Wordmark />
    <IconButton icon="lock" label="Lock vault" onclick={onlock} />
  </header>
  <div class="strip"><ScytaleStrip /></div>

  <div class="search">
    <SearchField
      id="popup-search"
      label="Search vault"
      placeholder={`Search ${items.length} items`}
      bind:value={query}
      onkeydown={onSearchKey}
    />
  </div>

  {#if showMatches}
    <section class="matches" aria-labelledby="matches-h">
      <h2 id="matches-h">On {site}</h2>
      <div class="match-card">
        {#each matches as m, i (m.id)}
          <ItemRow item={m} onclick={onopen}>
            {#snippet trailing()}
              <Button variant={i === 0 ? 'primary' : 'secondary'} onclick={() => onfill(m.id)}>Fill</Button>
            {/snippet}
          </ItemRow>
        {/each}
      </div>
    </section>
  {/if}

  <section class="list" aria-labelledby="list-h">
    <h2 id="list-h">{q ? `${results.length} found` : 'All items'}</h2>
    {#if items.length === 0}
      <div class="empty">
        <p>Your vault is empty. Add your first login, or bring your passwords over from another app.</p>
        <div class="empty-actions">
          <Button variant="primary" onclick={onadd}>Add a login</Button>
          <Button onclick={onimport}>Import</Button>
        </div>
      </div>
    {:else if results.length === 0}
      <p class="empty">Nothing matches “{query}”. Check the spelling, or search by website or username.</p>
    {:else}
      <div class="rows">
        {#each results as item (item.id)}
          <ItemRow {item} size="sm" onclick={onopen} />
        {/each}
      </div>
    {/if}
  </section>

  <footer>
    <div class="actions">
      <button type="button" class="act" onclick={ongenerate}><Icon name="dice" />Generate</button>
      <button type="button" class="act" onclick={onadd}><Icon name="plus" />Add item</button>
      <button type="button" class="act" onclick={onsettings}><Icon name="settings" />Settings</button>
    </div>
    <div class="sync"><SyncBadge {sync} /></div>
  </footer>
</div>

<style>
  .popup {
    width: 380px;
    height: 600px;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--ink);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 8px 0 20px;
  }
  .strip {
    padding: 4px 18px 14px;
  }
  .search {
    padding: 0 18px 14px;
  }
  h2 {
    margin: 0 0 8px;
    font-size: var(--text-sm);
    font-weight: 500;
    color: var(--muted);
  }
  .matches {
    padding: 0 14px 12px;
  }
  .matches h2 {
    padding: 0 4px;
  }
  .match-card {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px;
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
  }
  .list {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 0 10px;
  }
  .list h2 {
    padding: 0 8px;
  }
  .rows {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-bottom: 8px;
  }
  .empty {
    margin: 0;
    padding: 12px 8px;
    color: var(--muted);
    font-size: var(--text-md);
  }
  .empty p {
    margin: 0 0 12px;
  }
  .empty-actions {
    display: flex;
    gap: var(--space-2);
  }
  footer {
    border-top: 1px solid var(--line-soft);
    padding: 4px 10px 10px;
  }
  .actions {
    display: flex;
    justify-content: space-between;
  }
  .act {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: var(--hit);
    padding: 0 12px;
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    font-size: var(--text-sm);
    font-weight: 600;
  }
  .act :global(svg) {
    color: var(--accent-text);
  }
  .act:hover {
    background: var(--surface-hover);
  }
  .sync {
    padding: 2px 12px 0;
  }
</style>
