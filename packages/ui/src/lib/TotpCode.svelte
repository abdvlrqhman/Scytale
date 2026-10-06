<script lang="ts">
  let { code, remaining, period = 30 }: { code: string; remaining: number; period?: number } = $props();

  const circumference = 2 * Math.PI * 10;
  const offset = $derived(circumference * (1 - remaining / period));
  const pretty = $derived(code.length === 6 ? `${code.slice(0, 3)} ${code.slice(3)}` : code);
  const ending = $derived(remaining <= 5);
</script>

<span class="totp" class:ending>
  <span class="code">{pretty}</span>
  <svg width="24" height="24" viewBox="0 0 24 24" role="img" aria-label="{remaining} seconds left">
    <circle cx="12" cy="12" r="10" class="track" />
    <circle
      cx="12"
      cy="12"
      r="10"
      class="left"
      stroke-dasharray={circumference}
      stroke-dashoffset={offset}
      transform="rotate(-90 12 12)"
    />
  </svg>
  <span class="secs">{remaining}s</span>
</span>

<style>
  .totp {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }
  .code {
    font-size: var(--text-2xl);
    font-weight: 600;
    letter-spacing: 0.08em;
    font-variant-numeric: tabular-nums;
  }
  circle {
    fill: none;
    stroke-width: 2.5;
  }
  .track {
    stroke: var(--line);
  }
  .left {
    stroke: var(--accent);
    stroke-linecap: round;
    transition: stroke-dashoffset 1s linear;
  }
  .secs {
    font-size: var(--text-sm);
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .ending .left {
    stroke: var(--danger);
  }
</style>
