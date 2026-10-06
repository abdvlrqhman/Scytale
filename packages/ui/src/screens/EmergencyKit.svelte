<script lang="ts">
  // A printable sheet. The app opens it in its own page and calls print(); it is never uploaded.
  import ScytaleStrip from '../lib/ScytaleStrip.svelte';
  import Wordmark from '../lib/Wordmark.svelte';

  let {
    secretKey,
    storage,
    created,
  }: {
    secretKey: string;
    /** Where the vault syncs, e.g. "Dropbox (sam.rivera@gmail.com)", or "This device only". */
    storage: string;
    /** e.g. "6 October 2026" */
    created: string;
  } = $props();
</script>

<article class="kit">
  <header>
    <Wordmark size={30} />
    <span class="title">Emergency Kit</span>
  </header>
  <ScytaleStrip height={18} letters={false} />

  <p class="lead">
    Keep this sheet somewhere safe, like with your passport. With it and your master password you can open your vault on a
    new device, even if every other device is lost. Nobody else can recover your vault for you.
  </p>

  <dl>
    <div class="entry">
      <dt>Secret Key</dt>
      <dd class="key">{secretKey}</dd>
    </div>
    <div class="entry">
      <dt>Master password</dt>
      <dd class="blank">Write it here only if this sheet is locked away</dd>
    </div>
    <div class="entry">
      <dt>Vault storage</dt>
      <dd>{storage}</dd>
    </div>
    <div class="entry">
      <dt>Created</dt>
      <dd>{created}</dd>
    </div>
  </dl>

  <section>
    <h2>To sign in on a new device</h2>
    <ol>
      <li>Install Scytale and choose “I already use Scytale on another device”.</li>
      <li>Connect the same storage as above.</li>
      <li>Type the Secret Key and your master password.</li>
    </ol>
  </section>
</article>

<style>
  .kit {
    max-width: 680px;
    margin: 0 auto;
    padding: 40px 36px;
    display: flex;
    flex-direction: column;
    gap: 22px;
    background: var(--surface-raised);
    color: var(--ink);
    border-radius: var(--r-xl);
  }
  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
  }
  .title {
    font-family: var(--font-display);
    font-size: var(--text-2xl);
    color: var(--accent-text);
  }
  .lead {
    margin: 0;
    font-size: var(--text-lg);
    line-height: 1.6;
    color: var(--ink-soft);
  }
  dl {
    margin: 0;
    display: flex;
    flex-direction: column;
  }
  .entry {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 20px;
    padding: 16px 0;
    border-top: 1px solid var(--line);
  }
  dt {
    width: 160px;
    font-weight: 600;
  }
  dd {
    margin: 0;
    flex: 1 1 260px;
    min-width: 0;
  }
  .key {
    font-family: ui-monospace, 'Cascadia Mono', 'SF Mono', Menlo, Consolas, monospace;
    font-size: 20px;
    letter-spacing: 0.06em;
    overflow-wrap: anywhere;
  }
  .blank {
    min-height: 40px;
    border-bottom: 1px dashed var(--muted);
    color: var(--muted);
    font-size: var(--text-sm);
  }
  h2 {
    margin: 0 0 8px;
    font-family: var(--font-display);
    font-weight: 400;
    font-size: var(--text-xl);
  }
  ol {
    margin: 0;
    padding-left: 20px;
    line-height: 1.7;
  }
  @media print {
    .kit {
      background: #fff;
      color: #000;
      border-radius: 0;
      padding: 0;
      max-width: none;
    }
    .lead,
    .title {
      color: #000;
    }
    .entry {
      border-color: #999;
    }
  }
</style>
