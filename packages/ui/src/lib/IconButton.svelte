<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Icon, { type IconName } from './Icon.svelte';

  type Props = HTMLButtonAttributes & {
    icon: IconName;
    /** Required: icon-only buttons need an accessible name. */
    label: string;
    tone?: 'plain' | 'accent';
  };

  let { icon, label, tone = 'plain', type = 'button', ...rest }: Props = $props();
</script>

<button {type} class="icon-btn {tone}" aria-label={label} title={label} {...rest}>
  <Icon name={icon} size={20} />
</button>

<style>
  .icon-btn {
    display: grid;
    place-items: center;
    width: var(--hit);
    height: var(--hit);
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--muted);
    transition: background-color 0.15s var(--ease);
  }
  .icon-btn:hover {
    background: var(--surface-hover);
    color: var(--ink);
  }
  .accent {
    background: var(--accent);
    color: var(--on-accent);
  }
  .accent:hover {
    background: var(--accent);
    color: var(--on-accent);
    filter: brightness(1.07);
  }
</style>
