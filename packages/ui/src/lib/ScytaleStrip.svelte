<script lang="ts">
  // The brand's one signature: a leather strip wound around a rod, as on a Spartan scytale.
  // It stays still, except at unlock, when it "unwinds" and the ciphertext reads as plain text.
  let {
    cipher = 'QETRYASLXMONPK',
    plain = 'UNLOCKED',
    unwound = false,
    height = 26,
    letters = true,
  }: { cipher?: string; plain?: string; unwound?: boolean; height?: number; letters?: boolean } = $props();

  // The plain word starts at the second segment, so it stays visible when narrow rods clip the end.
  const segments = $derived([...cipher].map((c, i) => ({ cipher: c, plain: plain[i - 1] ?? '' })));
</script>

<div class="rod" class:unwound style:--h="{height}px" aria-hidden="true">
  {#each segments as seg, i (i)}
    <span class="seg" style:--i={i}>
      {#if letters}
        <span class="glyph cipher">{seg.cipher}</span>
        <span class="glyph plain">{seg.plain}</span>
      {/if}
    </span>
  {/each}
</div>

<style>
  .rod {
    height: var(--h);
    border-radius: var(--r-pill);
    background: var(--rod);
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 12px;
    overflow: hidden;
  }
  .seg {
    position: relative;
    /* Grow to fill wide rods; never shrink below legibility. Extra segments are clipped. */
    flex: 1 0 calc(var(--h) * 0.85);
    height: 100%;
    background: var(--strip);
    transform: skewX(-28deg);
    transition: transform 0.5s var(--ease) calc(var(--i) * 35ms);
  }
  .glyph {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    transform: skewX(28deg);
    font-family: var(--font-display);
    font-size: calc(var(--h) * 0.5);
    color: var(--strip-ink);
    transition:
      opacity 0.3s var(--ease) calc(var(--i) * 35ms + 0.2s),
      transform 0.5s var(--ease) calc(var(--i) * 35ms);
  }
  .plain {
    opacity: 0;
  }
  .unwound .seg {
    transform: skewX(0deg);
  }
  .unwound .glyph {
    transform: skewX(0deg);
  }
  .unwound .cipher {
    opacity: 0;
  }
  .unwound .plain {
    opacity: 1;
  }
</style>
