<!-- The commentary: one row per released event, newest first as the sketch lists it, each a
     minute and a line; goals, cards, substitutions and injuries lead with a word, so colour is
     never the only signal. Only goals and cards are read out, through a polite live region.
     `state` is `rows`, `skeleton` (the wait before the match), or `idle` (no match to play). -->
<script>
  import SectionLabel from './SectionLabel.svelte';
  import StubSection from './StubSection.svelte';
  import { EMPTY_TEXT } from '../lib/feed.js';

  let { rows = [], spoken = '', state = 'rows' } = $props();

  // Newest first. Rows only ever append, or are replaced whole on a rewind, so a row's
  // place from the start is a stable key.
  let newest = $derived(rows.map((row, i) => ({ row, key: i })).reverse());
</script>

<section class="commentary" aria-label="Commentary">
  <div class="head">
    <SectionLabel label="Commentary" />
    {#if state === 'rows'}
      <!-- STUB: commentary filters (crowd, whistle and ball). Drawn for layout and feel only. -->
      <StubSection note="commentary filters" inline>
        <span class="tg on">Crowd</span>
        <span class="tg on">Whistle &amp; ball</span>
      </StubSection>
    {/if}
  </div>
  {#if state === 'skeleton'}
    <div class="skels" aria-hidden="true">
      {#each [0, 1, 2, 3, 4] as i (i)}<div class="skel"></div>{/each}
    </div>
  {:else if state === 'idle'}
    <p class="empty quiet">The feed fills when a match plays.</p>
  {:else if rows.length === 0}
    <p class="empty">{EMPTY_TEXT}</p>
  {:else}
    <ol class="rows">
      {#each newest as { row, key } (key)}
        <li class:hl={row.word} data-kind={row.kind} data-tick={row.tick}>
          <b class="num">{row.minute}</b>
          <span>{#if row.word}<em>{row.word}</em> {/if}{row.text}</span>
        </li>
      {/each}
    </ol>
  {/if}
  <p class="live" aria-live="polite">{spoken}</p>
</section>

<style>
  .commentary {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }

  .head {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  .head > :global(h3) {
    flex: 1;
  }

  .head :global(.stub) {
    gap: 4px;
  }

  .tg {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 19px;
    padding: 0 7px;
    border-radius: var(--radius-sm);
    font-size: 9.5px;
    background: var(--toggle-ground);
    box-shadow: inset 0 0 0 1px var(--toggle-edge);
    color: var(--toggle-ink);
    white-space: nowrap;
  }

  .tg.on {
    background: var(--toggle-on-ground);
    box-shadow: inset 0 0 0 1px var(--cyan);
    color: var(--ink);
  }

  .tg.on::before {
    content: '✓';
    color: var(--cyan);
    font-weight: 800;
  }

  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    min-height: 0;
    flex: 1;
  }

  li {
    display: flex;
    gap: 8px;
    padding: 3px 0;
    border-bottom: 1px solid var(--rule-2);
    font-size: 10.5px;
    color: var(--ink-2);
  }

  li b {
    width: 24px;
    flex: none;
    font: 700 10.5px/1.35 var(--fd);
    font-variant-numeric: tabular-nums;
    color: var(--ink);
  }

  li.hl span {
    color: var(--ink);
  }

  em {
    font: 700 9.5px var(--fd);
    font-style: normal;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    margin-right: 2px;
  }

  .empty {
    margin: 0;
    font-size: 10.5px;
    color: var(--ink);
  }

  .empty.quiet {
    color: var(--ink-3);
  }

  .skel {
    height: 14px;
    margin-bottom: 8px;
    background: var(--skeleton);
    border-radius: var(--radius-sm);
  }

  .live {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
