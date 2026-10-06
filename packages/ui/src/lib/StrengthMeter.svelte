<script lang="ts">
  // Presentational: the caller scores the password (zxcvbn in the app layer) and passes the result.
  let { score, label, hint }: { score: 0 | 1 | 2 | 3 | 4; label: string; hint?: string } = $props();
  const level = $derived(score <= 1 ? 'weak' : score === 2 ? 'fair' : 'strong');
</script>

<div class="meter {level}">
  <div class="bars" aria-hidden="true">
    {#each [1, 2, 3, 4] as n (n)}
      <span class:on={score >= n}></span>
    {/each}
  </div>
  <p><strong>{label}</strong>{#if hint}<span> {hint}</span>{/if}</p>
</div>

<style>
  .meter {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .bars {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 4px;
  }
  .bars span {
    height: 6px;
    border-radius: var(--r-pill);
    background: var(--line);
    transition: background-color 0.2s var(--ease);
  }
  .weak .on {
    background: var(--danger);
  }
  .fair .on {
    background: var(--warn);
  }
  .strong .on {
    background: var(--ok);
  }
  p {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--muted);
  }
  strong {
    color: var(--ink);
    font-weight: 600;
  }
</style>
