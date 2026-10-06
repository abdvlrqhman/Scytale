<script lang="ts">
  import type { ItemKind } from '../types';
  import Icon, { type IconName } from './Icon.svelte';

  let { title, kind = 'login', size = 'md' }: { title: string; kind?: ItemKind; size?: 'sm' | 'md' | 'lg' } = $props();

  const kindIcon: Record<ItemKind, IconName | null> = { login: null, note: 'note', card: 'card', identity: 'identity' };

  // Stable tint per title, so an item keeps its color on every device.
  const tone = $derived([...title].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7) % 6 + 1);
  const letter = $derived(title.trim().charAt(0).toUpperCase() || '?');
  const icon = $derived(kindIcon[kind]);
</script>

<span class="mono {size}" style:--tone="var(--tint-{tone})" aria-hidden="true">
  {#if icon}
    <Icon name={icon} size={size === 'lg' ? 24 : 17} />
  {:else}
    {letter}
  {/if}
</span>

<style>
  .mono {
    display: grid;
    place-items: center;
    flex-shrink: 0;
    border-radius: 50%;
    background: var(--tint-bg);
    color: var(--tone);
    font-family: var(--font-display);
    line-height: 1;
  }
  .sm {
    width: 30px;
    height: 30px;
    font-size: 14px;
  }
  .md {
    width: 36px;
    height: 36px;
    font-size: 17px;
  }
  .lg {
    width: 60px;
    height: 60px;
    font-size: 28px;
  }
</style>
