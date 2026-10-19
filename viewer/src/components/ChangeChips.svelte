<!-- The live strip during play: the newest changes, each a chip with its word. In effect
     (green) since its minute; players adjusting (cyan) for about the take-up time; a
     formation at the next stoppage (orange), which can be cancelled. Each chip carries a
     mark and words, never colour alone; chips fade in over 200 ms, and not at all under
     reduced motion. -->
<script>
  let { chips = [], takeUp = 30, oncancel = () => {} } = $props();

  const MARK = { ok: '✓', run: '◔', wa: '◷' };
</script>

<div class="live" role="status" aria-live="polite">
  <b class="cap">Changes</b>
  {#each chips as chip (chip.queue_id)}
    <span class="chip {chip.tone}" data-board-chip={chip.queue_id}>
      {MARK[chip.tone]} {chip.label} <em>{chip.note}</em>
      {#if chip.cancellable}
        <button type="button" class="x" aria-label="Cancel: {chip.label}" onclick={() => oncancel(chip.queue_id)}>
          <svg width="9" height="9" viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9" /></svg>
        </button>
      {/if}
    </span>
  {/each}
  <span class="note"
    >Changes start at once; players take about {takeUp} s to adjust. A formation change waits for the next stoppage.</span
  >
</div>

<style>
  .live {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 0 2px;
    padding: 5px 8px;
    background: var(--ground-2);
    border-radius: 3px;
    flex-wrap: wrap;
  }

  .cap {
    font: 700 11px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font: 600 9.5px var(--fd);
    padding: 0 6px;
    line-height: 18px;
    border-radius: var(--radius-sm);
    border: 1px solid currentColor;
    white-space: nowrap;
  }

  .chip em {
    font-style: normal;
    font-weight: 500;
    color: var(--ink-2);
  }

  .chip.ok {
    color: var(--good);
  }

  .chip.run {
    color: var(--cyan);
  }

  .chip.wa {
    color: var(--warn);
  }

  .note {
    font-size: 9px;
    color: var(--ink-3);
  }

  .x {
    display: inline-grid;
    place-items: center;
    width: 18px;
    height: 18px;
    margin: 0;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--ink-2);
    cursor: pointer;
  }

  .x path {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }

  .x:hover {
    color: var(--ink);
    background: var(--rule);
  }

  .x:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: 1px;
  }

  @media (prefers-reduced-motion: no-preference) {
    .chip {
      animation: chip-in 200ms var(--ease);
    }
  }

  @keyframes chip-in {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }

    to {
      opacity: 1;
      transform: none;
    }
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .x {
      min-width: 44px;
      min-height: 44px;
    }
  }
</style>
