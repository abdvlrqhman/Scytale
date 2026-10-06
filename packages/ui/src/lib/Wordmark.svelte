<script lang="ts">
  let { size = 22 }: { size?: number } = $props();
</script>

<span class="wordmark" style:font-size="{size}px">Scytale</span>

<style>
  .wordmark {
    font-family: var(--font-display);
    letter-spacing: 0.04em;
    color: var(--ink);
    line-height: 1;
  }
</style>
