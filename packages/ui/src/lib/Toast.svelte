<script lang="ts">
  import Icon from './Icon.svelte';

  // e.g. "Password copied. Clipboard clears in 30 s."
  let { message }: { message: string | null } = $props();
</script>

<div class="toast-region" role="status" aria-live="polite">
  {#if message}
    <div class="toast">
      <Icon name="check" size={18} />
      {message}
    </div>
  {/if}
</div>

<style>
  .toast-region {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 16px;
    display: flex;
    justify-content: center;
    pointer-events: none;
    padding: 0 16px;
  }
  .toast {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 18px;
    border-radius: var(--r-pill);
    background: var(--ink);
    color: var(--bg);
    font-size: var(--text-sm);
    font-weight: 600;
    box-shadow: var(--shadow-pop);
    animation: rise 0.25s var(--ease);
  }
  @keyframes rise {
    from {
      transform: translateY(8px);
      opacity: 0;
    }
  }
</style>
