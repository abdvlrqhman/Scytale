<script lang="ts">
  import Button from '../lib/Button.svelte';
  import IconButton from '../lib/IconButton.svelte';
  import Monogram from '../lib/Monogram.svelte';
  import TotpCode from '../lib/TotpCode.svelte';
  import type { ItemView } from '../types';

  let {
    item,
    onreveal,
    onhide,
    oncopy,
    oncopytotp,
    onedit,
    onfavorite,
    onback,
  }: {
    item: ItemView;
    onreveal: (key: string) => void;
    onhide: (key: string) => void;
    oncopy: (key: string) => void;
    oncopytotp: () => void;
    onedit: () => void;
    onfavorite: () => void;
    /** Shown in the narrow (popup) layout. */
    onback?: () => void;
  } = $props();

  const strengthLabel = { weak: 'Weak', fair: 'Fair', strong: 'Strong' } as const;
</script>

<article class="detail" aria-labelledby="detail-title">
  <header>
    {#if onback}<IconButton icon="back" label="Back to list" onclick={onback} />{/if}
    <Monogram title={item.title} kind={item.kind} size="lg" />
    <div class="heading">
      <h1 id="detail-title">{item.title}</h1>
      <p>{item.edited}</p>
    </div>
    <div class="head-actions">
      <IconButton
        icon="star"
        label={item.favorite ? 'Remove from favorites' : 'Add to favorites'}
        aria-pressed={item.favorite}
        tone={item.favorite ? 'accent' : 'plain'}
        onclick={onfavorite}
      />
      <Button onclick={onedit}>Edit</Button>
    </div>
  </header>

  <dl class="fields">
    {#each item.fields as f (f.key)}
      <div class="field">
        <dt>{f.label}</dt>
        <dd class="value" class:masked={f.secret && f.value === undefined} class:mono={f.secret && f.value !== undefined}>
          {#if f.secret && f.value === undefined}
            <span aria-label="Hidden">••••••••••••••••</span>
          {:else if f.href}
            <a href={f.href} target="_blank" rel="noreferrer noopener">{f.value}</a>
          {:else}
            {f.value}
          {/if}
        </dd>
        <dd class="actions">
          {#if f.secret && f.key === 'password' && item.strength}
            <span class="chip {item.strength}">{strengthLabel[item.strength]}</span>
          {/if}
          {#if f.secret}
            {#if f.value === undefined}
              <Button variant="ghost" size="sm" onclick={() => onreveal(f.key)}>Show</Button>
            {:else}
              <Button variant="ghost" size="sm" onclick={() => onhide(f.key)}>Hide</Button>
            {/if}
          {/if}
          {#if f.copyable}
            <Button variant={f.key === 'password' ? 'primary' : 'ghost'} size="sm" onclick={() => oncopy(f.key)}>Copy</Button>
          {/if}
        </dd>
      </div>
    {/each}
    {#if item.totp}
      <div class="field">
        <dt>One-time code</dt>
        <dd class="value"><TotpCode {...item.totp} /></dd>
        <dd class="actions"><Button variant="ghost" size="sm" onclick={oncopytotp}>Copy</Button></dd>
      </div>
    {/if}
  </dl>

  <div class="extra">
    {#if item.history.length}
      <section>
        <h2>Previous passwords</h2>
        <ul class="history">
          {#each item.history as h, i (i)}
            <li><span class="dots">{h.label}</span><span>{h.until}</span></li>
          {/each}
        </ul>
      </section>
    {/if}
    {#if item.notes}
      <section>
        <h2>Notes</h2>
        <p class="notes">{item.notes}</p>
      </section>
    {/if}
  </div>
</article>

<style>
  .detail {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    min-width: 0;
  }
  header {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-4);
  }
  .heading {
    flex: 1 1 200px;
    min-width: 0;
  }
  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-weight: 400;
    font-size: var(--text-4xl);
    line-height: 1.1;
    overflow-wrap: anywhere;
  }
  .heading p {
    margin: 4px 0 0;
    color: var(--muted);
  }
  .head-actions {
    display: flex;
    gap: var(--space-2);
  }
  .fields {
    margin: 0;
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
  }
  .field {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px var(--space-4);
    padding: 12px 12px 12px 22px;
  }
  .field + .field {
    border-top: 1px solid var(--line-soft);
  }
  dt {
    width: 120px;
    font-size: var(--text-sm);
    color: var(--accent-text);
  }
  dd {
    margin: 0;
  }
  .value {
    flex: 1 1 130px;
    min-width: 0;
    font-size: var(--text-lg);
    overflow-wrap: anywhere;
  }
  .masked {
    letter-spacing: 0.14em;
  }
  .mono {
    font-family: ui-monospace, 'Cascadia Mono', 'SF Mono', Menlo, Consolas, monospace;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }
  .chip {
    padding: 4px 10px;
    border-radius: var(--r-pill);
    font-size: var(--text-xs);
    font-weight: 700;
  }
  .chip.strong {
    color: var(--ok);
    background: color-mix(in srgb, var(--ok) 16%, transparent);
  }
  .chip.fair {
    color: var(--warn);
    background: color-mix(in srgb, var(--warn) 16%, transparent);
  }
  .chip.weak {
    color: var(--danger);
    background: var(--danger-soft);
  }
  .extra {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-6);
  }
  .extra section {
    flex: 1 1 240px;
    min-width: 0;
  }
  h2 {
    margin: 0 0 10px;
    font-family: var(--font-display);
    font-weight: 400;
    font-size: var(--text-xl);
  }
  .history {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .history li {
    display: flex;
    justify-content: space-between;
    gap: var(--space-4);
    padding: 8px 0;
    font-size: var(--text-sm);
    color: var(--muted);
  }
  .history li + li {
    border-top: 1px solid var(--line-soft);
  }
  .dots {
    color: var(--ink);
    letter-spacing: 0.14em;
  }
  .notes {
    margin: 0;
    color: var(--ink-soft);
    line-height: 1.55;
    white-space: pre-wrap;
    max-width: 60ch;
  }
</style>
