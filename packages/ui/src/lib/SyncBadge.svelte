<script lang="ts">
  import type { SyncInfo } from '../types';

  let { sync }: { sync: SyncInfo } = $props();
</script>

<span class="sync {sync.status}" role="status">
  <span class="dot" aria-hidden="true"></span>
  {sync.text}
</span>

<style>
  .sync {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: var(--text-xs);
    color: var(--muted);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ok);
    flex-shrink: 0;
  }
  .syncing .dot {
    background: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .offline .dot {
    background: var(--placeholder);
  }
  .error .dot {
    background: var(--danger);
  }
  .error {
    color: var(--danger);
  }
  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
</style>
