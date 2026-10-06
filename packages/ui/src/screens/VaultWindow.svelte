<script lang="ts" module>
  export type Section = 'all' | 'favorites' | 'login' | 'card' | 'identity' | 'note' | 'generator' | 'settings';
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon, { type IconName } from '../lib/Icon.svelte';
  import IconButton from '../lib/IconButton.svelte';
  import ItemRow from '../lib/ItemRow.svelte';
  import ScytaleStrip from '../lib/ScytaleStrip.svelte';
  import SearchField from '../lib/SearchField.svelte';
  import SyncBadge from '../lib/SyncBadge.svelte';
  import Wordmark from '../lib/Wordmark.svelte';
  import type { ItemSummary, SyncInfo } from '../types';

  let {
    items,
    section = $bindable('all'),
    selectedId,
    query = $bindable(''),
    sync,
    onselect,
    onadd,
    onlock,
    children,
  }: {
    items: ItemSummary[];
    section?: Section;
    selectedId?: string;
    query?: string;
    sync: SyncInfo;
    onselect: (id: string) => void;
    onadd: () => void;
    onlock: () => void;
    /** The right-hand pane: item, editor, generator or settings. */
    children: Snippet;
  } = $props();

  const nav: { id: Section; label: string; icon: IconName }[] = [
    { id: 'all', label: 'All items', icon: 'grid' },
    { id: 'favorites', label: 'Favorites', icon: 'star' },
    { id: 'login', label: 'Logins', icon: 'key' },
    { id: 'card', label: 'Cards', icon: 'card' },
    { id: 'identity', label: 'Identities', icon: 'identity' },
    { id: 'note', label: 'Secure notes', icon: 'note' },
  ];
  const tools: { id: Section; label: string; icon: IconName }[] = [
    { id: 'generator', label: 'Password generator', icon: 'dice' },
    { id: 'settings', label: 'Settings', icon: 'settings' },
  ];

  const inSection = (i: ItemSummary, s: Section) =>
    s === 'all' || s === 'generator' || s === 'settings' ? true : s === 'favorites' ? !!i.favorite : i.kind === s;
  const count = (s: Section) => items.filter((i) => inSection(i, s)).length;

  const q = $derived(query.trim().toLowerCase());
  const visible = $derived(
    items.filter(
      (i) =>
        inSection(i, section) && (!q || i.title.toLowerCase().includes(q) || i.subtitle.toLowerCase().includes(q)),
    ),
  );
</script>

<div class="window">
  <nav aria-label="Vault sections">
    <div class="brand">
      <Wordmark size={26} />
      <ScytaleStrip height={20} letters={false} />
    </div>
    {#each nav as n (n.id)}
      <button type="button" class="nav" class:active={section === n.id} aria-current={section === n.id || undefined} onclick={() => (section = n.id)}>
        <Icon name={n.icon} />
        <span class="nav-label">{n.label}</span>
        <span class="count">{count(n.id)}</span>
      </button>
    {/each}
    <hr />
    {#each tools as n (n.id)}
      <button type="button" class="nav" class:active={section === n.id} aria-current={section === n.id || undefined} onclick={() => (section = n.id)}>
        <Icon name={n.icon} />
        <span class="nav-label">{n.label}</span>
      </button>
    {/each}
    <div class="grow"></div>
    <div class="sync"><SyncBadge {sync} /></div>
  </nav>

  <section class="list" aria-label="Items">
    <div class="list-top">
      <SearchField id="win-search" label="Search vault" placeholder={`Search ${items.length} items`} bind:value={query} />
      <IconButton icon="plus" label="Add item" tone="accent" onclick={onadd} />
      <IconButton icon="lock" label="Lock vault" onclick={onlock} />
    </div>
    <div class="rows">
      {#each visible as item (item.id)}
        <ItemRow {item} selected={item.id === selectedId} onclick={onselect} />
      {:else}
        <p class="empty">{q ? `Nothing matches “${query}”.` : 'Nothing here yet.'}</p>
      {/each}
    </div>
  </section>

  <main>
    {@render children()}
  </main>
</div>

<style>
  .window {
    display: flex;
    height: 100%;
    min-height: 0;
    background: var(--bg);
  }
  nav {
    width: 240px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 22px 14px 16px;
    background: var(--bg-sunken);
    border-right: 1px solid var(--line-soft);
  }
  .brand {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 0 10px 20px;
  }
  .nav {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 42px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--ink-soft);
    font-size: var(--text-md);
    text-align: left;
  }
  .nav :global(svg) {
    color: var(--muted);
  }
  .nav:hover {
    background: var(--surface-hover);
  }
  .nav.active {
    background: var(--surface-raised);
    color: var(--ink);
    font-weight: 600;
  }
  .nav.active :global(svg),
  .nav.active .count {
    color: var(--accent-text);
  }
  .nav-label {
    flex: 1;
  }
  .count {
    font-size: var(--text-sm);
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  hr {
    width: calc(100% - 20px);
    border: 0;
    border-top: 1px solid var(--line-soft);
    margin: 12px 10px;
  }
  .grow {
    flex: 1;
  }
  .sync {
    padding: 0 12px;
  }
  .list {
    width: 340px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--line-soft);
  }
  .list-top {
    display: flex;
    gap: var(--space-2);
    padding: 18px 14px 10px;
  }
  .list-top :global(.search) {
    flex: 1;
  }
  .rows {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 8px 12px;
  }
  .empty {
    margin: 0;
    padding: 16px 12px;
    color: var(--muted);
  }
  main {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: 34px 40px;
  }
</style>
