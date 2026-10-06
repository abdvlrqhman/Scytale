<script lang="ts">
  import type { GeneratorOptions } from '../types';
  import Button from '../lib/Button.svelte';
  import Icon from '../lib/Icon.svelte';
  import SegmentedControl from '../lib/SegmentedControl.svelte';
  import Switch from '../lib/Switch.svelte';

  let {
    options = $bindable(),
    value,
    bits,
    onregenerate,
    oncopy,
    onuse,
  }: {
    options: GeneratorOptions;
    /** The current result; the caller regenerates when `options` change. */
    value: string;
    /** Estimated strength in bits. */
    bits: number;
    onregenerate: () => void;
    oncopy: () => void;
    /** Present when opened from a password field: put the result there. */
    onuse?: () => void;
  } = $props();

  const verdict = $derived(bits >= 80 ? 'Very strong' : bits >= 60 ? 'Strong' : bits >= 40 ? 'Fair' : 'Weak');
</script>

<section class="gen" aria-labelledby="gen-h">
  <h1 id="gen-h">Generate a password</h1>

  <div class="result">
    <output class="value" aria-live="polite">{value}</output>
    <div class="result-bar">
      <span class="bits"><strong>{verdict}</strong> about {Math.round(bits)} bits</span>
      <Button variant="ghost" size="sm" onclick={onregenerate}><Icon name="refresh" />New</Button>
      <Button variant={onuse ? 'secondary' : 'primary'} size="sm" onclick={oncopy}><Icon name="copy" />Copy</Button>
      {#if onuse}<Button variant="primary" size="sm" onclick={onuse}>Use this</Button>{/if}
    </div>
  </div>

  <SegmentedControl
    legend="Kind"
    name="gen-mode"
    options={[
      { value: 'passphrase', label: 'Words' },
      { value: 'password', label: 'Characters' },
    ]}
    bind:value={options.mode}
  />

  {#if options.mode === 'password'}
    <div class="slider">
      <label for="gen-length">Length <strong>{options.length}</strong></label>
      <input
        id="gen-length"
        type="range"
        min="8"
        max="64"
        style:--pct="{((options.length - 8) / 56) * 100}%"
        bind:value={options.length}
      />
    </div>
    <Switch id="gen-upper" label="Capital letters" bind:checked={options.upper} />
    <Switch id="gen-digits" label="Numbers" bind:checked={options.digits} />
    <Switch id="gen-symbols" label="Symbols" bind:checked={options.symbols} />
    <Switch
      id="gen-ambiguous"
      label="Avoid look-alike characters"
      description="Leaves out I, l, 1, O, 0 and o. Handy for passwords you type by hand."
      bind:checked={options.avoidAmbiguous}
    />
  {:else}
    <div class="slider">
      <label for="gen-words">Words <strong>{options.words}</strong></label>
      <input
        id="gen-words"
        type="range"
        min="3"
        max="10"
        style:--pct="{((options.words - 3) / 7) * 100}%"
        bind:value={options.words}
      />
    </div>
    <Switch id="gen-cap" label="Capitalize words" bind:checked={options.capitalize} />
    <Switch id="gen-num" label="Add a number" bind:checked={options.includeNumber} />
  {/if}
</section>

<style>
  .gen {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-width: 520px;
  }
  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-weight: 400;
    font-size: var(--text-3xl);
  }
  .result {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: 18px 18px 12px 20px;
    border-radius: var(--r-lg);
    background: var(--surface-raised);
    border: 1px solid var(--line);
  }
  .value {
    font-family: ui-monospace, 'Cascadia Mono', 'SF Mono', Menlo, Consolas, monospace;
    font-size: 20px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }
  .result-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1);
  }
  .bits {
    flex: 1;
    font-size: var(--text-sm);
    color: var(--muted);
  }
  .bits strong {
    color: var(--ok);
  }
  .slider {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .slider label {
    font-weight: 600;
  }
  .slider strong {
    color: var(--accent-text);
    font-variant-numeric: tabular-nums;
  }
</style>
