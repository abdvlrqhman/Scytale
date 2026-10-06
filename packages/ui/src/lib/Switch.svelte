<script lang="ts">
  let {
    id,
    label,
    description,
    checked = $bindable(false),
    onchange,
  }: { id: string; label: string; description?: string; checked?: boolean; onchange?: (checked: boolean) => void } =
    $props();
</script>

<div class="switch-row">
  <span class="text">
    <label for={id}>{label}</label>
    {#if description}<span class="desc" id="{id}-desc">{description}</span>{/if}
  </span>
  <input
    {id}
    type="checkbox"
    role="switch"
    bind:checked
    aria-describedby={description ? `${id}-desc` : undefined}
    onchange={() => onchange?.(checked)}
  />
</div>

<style>
  .switch-row {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    min-height: var(--hit);
  }
  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  label {
    font-weight: 600;
  }
  .desc {
    font-size: var(--text-sm);
    color: var(--muted);
  }
  input {
    appearance: none;
    flex-shrink: 0;
    width: 46px;
    height: 28px;
    margin: 0;
    border-radius: var(--r-pill);
    background: var(--line);
    position: relative;
    cursor: pointer;
    transition: background-color 0.2s var(--ease);
  }
  input::after {
    content: '';
    position: absolute;
    top: 3px;
    left: 3px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--ink);
    transition: transform 0.2s var(--ease);
  }
  input:checked {
    background: var(--accent);
  }
  input:checked::after {
    transform: translateX(18px);
    background: var(--on-accent);
  }
</style>
