<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Props = HTMLButtonAttributes & {
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    size?: 'sm' | 'md' | 'lg';
    full?: boolean;
    children: Snippet;
  };

  let { variant = 'secondary', size = 'md', full = false, type = 'button', children, ...rest }: Props = $props();
</script>

<button {type} class="btn {variant} {size}" class:full {...rest}>
  {@render children()}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    min-height: var(--hit);
    padding: 0 18px;
    border: 1px solid transparent;
    border-radius: var(--r-md);
    font-size: var(--text-md);
    font-weight: 600;
    white-space: nowrap;
    transition:
      background-color 0.15s var(--ease),
      border-color 0.15s var(--ease),
      filter 0.15s var(--ease);
  }
  .primary {
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 700;
  }
  .primary:hover {
    filter: brightness(1.07);
  }
  .secondary {
    background: transparent;
    border-color: var(--line);
    color: var(--ink);
  }
  .secondary:hover {
    background: var(--surface-hover);
  }
  .ghost {
    background: transparent;
    color: var(--accent-text);
  }
  .ghost:hover {
    background: var(--surface-hover);
  }
  .danger {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .sm {
    min-height: 40px;
    padding: 0 14px;
    font-size: var(--text-sm);
  }
  .lg {
    min-height: 52px;
    padding: 0 24px;
    font-size: var(--text-lg);
  }
  .full {
    width: 100%;
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
