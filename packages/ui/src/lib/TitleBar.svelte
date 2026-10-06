<script lang="ts">
  // The desktop window has no OS title bar. This is its replacement: a drag region plus window
  // controls drawn in the Bronze style, placed where each platform's users expect them.
  import Icon from './Icon.svelte';
  import Wordmark from './Wordmark.svelte';

  let {
    platform,
    maximized = false,
    onminimize,
    ontogglemaximize,
    onclose,
  }: {
    platform: 'macos' | 'windows' | 'linux';
    maximized?: boolean;
    onminimize: () => void;
    ontogglemaximize: () => void;
    /** Hides the window to the tray; quitting lives in the tray menu. */
    onclose: () => void;
  } = $props();

  const mac = $derived(platform === 'macos');
</script>

<!-- data-tauri-drag-region makes the bar move the window; buttons opt out automatically.
     Double-click to maximize is a mouse shortcut; keyboard users have the maximize button. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<header class="titlebar" class:mac data-tauri-drag-region ondblclick={ontogglemaximize}>
  {#if mac}
    <div class="lights">
      <button type="button" class="light close" aria-label="Hide to tray" title="Hide to tray" onclick={onclose}>
        <Icon name="close" size={10} stroke={2.5} />
      </button>
      <button type="button" class="light min" aria-label="Minimize" title="Minimize" onclick={onminimize}>
        <Icon name="minimize" size={10} stroke={2.5} />
      </button>
      <button
        type="button"
        class="light max"
        aria-label={maximized ? 'Restore' : 'Maximize'}
        title={maximized ? 'Restore' : 'Maximize'}
        onclick={ontogglemaximize}
      >
        <Icon name={maximized ? 'restore' : 'maximize'} size={10} stroke={2.5} />
      </button>
    </div>
  {/if}

  <span class="name" data-tauri-drag-region><Wordmark size={15} /></span>

  {#if !mac}
    <div class="controls">
      <button type="button" class="ctl" aria-label="Minimize" title="Minimize" onclick={onminimize}>
        <Icon name="minimize" size={16} />
      </button>
      <button
        type="button"
        class="ctl"
        aria-label={maximized ? 'Restore' : 'Maximize'}
        title={maximized ? 'Restore' : 'Maximize'}
        onclick={ontogglemaximize}
      >
        <Icon name={maximized ? 'restore' : 'maximize'} size={16} />
      </button>
      <button type="button" class="ctl close" aria-label="Hide to tray" title="Hide to tray" onclick={onclose}>
        <Icon name="close" size={16} />
      </button>
    </div>
  {/if}
</header>

<style>
  .titlebar {
    display: flex;
    align-items: center;
    height: 40px;
    flex-shrink: 0;
    background: var(--bg-sunken);
    border-bottom: 1px solid var(--line-soft);
    user-select: none;
    -webkit-user-select: none;
  }
  .name {
    flex: 1;
    padding: 0 16px;
    opacity: 0.85;
  }
  .mac .name {
    text-align: center;
    padding-right: 76px; /* balance the lights so the name is truly centered */
  }
  .controls {
    display: flex;
    align-self: stretch;
  }
  .ctl {
    display: grid;
    place-items: center;
    width: 48px;
    border: 0;
    background: transparent;
    color: var(--muted);
    transition: background-color 0.12s var(--ease);
  }
  .ctl:hover {
    background: var(--surface-hover);
    color: var(--ink);
  }
  .ctl.close:hover {
    background: var(--danger);
    color: var(--bg);
  }
  .lights {
    display: flex;
    gap: 8px;
    padding: 0 14px;
  }
  .light {
    display: grid;
    place-items: center;
    width: 14px;
    height: 14px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    color: transparent;
  }
  /* Bronze-family lights; the glyphs appear on hover, like the platform's own. */
  .light.close {
    background: var(--danger);
  }
  .light.min {
    background: var(--accent);
  }
  .light.max {
    background: var(--ok);
  }
  .lights:hover .light,
  .light:focus-visible {
    color: var(--on-accent);
  }
</style>
