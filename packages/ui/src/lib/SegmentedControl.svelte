<script lang="ts" generics="T extends string">
  // Radio group styled as a segmented switch: native keyboard and screen-reader behavior for free.
  let {
    legend,
    name,
    options,
    value = $bindable(),
  }: { legend: string; name: string; options: { value: T; label: string }[]; value: T } = $props();
</script>

<fieldset class="seg">
  <legend class="sr-only">{legend}</legend>
  {#each options as opt (opt.value)}
    <label class:checked={value === opt.value}>
      <input type="radio" {name} value={opt.value} bind:group={value} />
      {opt.label}
    </label>
  {/each}
</fieldset>

<style>
  .seg {
    display: inline-flex;
    margin: 0;
    padding: 4px;
    gap: 2px;
    border: 1px solid var(--line);
    border-radius: var(--r-pill);
    background: var(--surface);
  }
  label {
    position: relative;
    display: grid;
    place-items: center;
    min-height: 36px;
    padding: 0 16px;
    border-radius: var(--r-pill);
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--muted);
    cursor: pointer;
  }
  .checked {
    background: var(--accent);
    color: var(--on-accent);
  }
  input {
    position: absolute;
    opacity: 0;
    inset: 0;
    margin: 0;
    cursor: pointer;
  }
  label:has(input:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
