<!-- A section's cyan label with its grey note. Sections are split by 1 px rules, never
     boxed: pass `rule` to draw the rule above the label. -->
<script>
  let { label, note = '', rule = false, level = 3 } = $props();
</script>

{#if rule}<div class="hr" role="presentation"></div>{/if}
<svelte:element this={`h${level}`} class="sl">
  <span>{label}</span>
  {#if note}<em>{note}</em>{/if}
</svelte:element>

<style>
  .hr {
    border-top: 1px solid var(--rule);
    margin: 14px 0;
  }

  .sl {
    font: 700 10.5px var(--fd);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--cyan);
    margin: 0 0 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  em {
    font-style: normal;
    color: var(--ink-3);
    font: 500 9.5px var(--fb);
    letter-spacing: 0;
    text-transform: none;
  }
</style>
