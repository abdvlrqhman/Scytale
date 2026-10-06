<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';
  import IconButton from './IconButton.svelte';
  import TextField from './TextField.svelte';

  type Props = Omit<HTMLInputAttributes, 'value' | 'type'> & {
    id: string;
    label: string;
    value?: string;
    hint?: string;
    error?: string;
    /** Shows a "generate" button; the caller fills `value`. */
    ongenerate?: () => void;
  };

  let { value = $bindable(''), ongenerate, ...rest }: Props = $props();
  let visible = $state(false);
</script>

<TextField
  {...rest}
  type={visible ? 'text' : 'password'}
  mono={visible}
  autocomplete="off"
  spellcheck="false"
  bind:value
>
  {#snippet trailing()}
    {#if ongenerate}
      <IconButton icon="dice" label="Generate a password" onclick={ongenerate} />
    {/if}
    <IconButton
      icon={visible ? 'eyeOff' : 'eye'}
      label={visible ? 'Hide password' : 'Show password'}
      aria-pressed={visible}
      onclick={() => (visible = !visible)}
    />
  {/snippet}
</TextField>
