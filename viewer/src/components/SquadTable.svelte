<!-- The squad table of the sketch's players-squad screen, driven by the column model
     (lib/columns.js): No and Player, then the active view's columns in order, then the
     Morale and Contract columns as LATER stubs. Each built header is a button that sorts by its
     column (again to reverse), with `aria-sort`; the sorted header is cyan with an underline and
     ▼ or ▲, so the arrow is never the only cue. Numbers 1 to 20 are whole, in the four band
     colours; hidden values are words; match ratings sit in `.rt` chips with one decimal. A
     cell's spoken text (the number with its band word, the chips' averages) is real text for
     a screen reader, kept out of sight, and the drawn text is hidden from it. A row's player
     name opens his panel; the row of the player last opened is marked current. -->
<script>
  import StubSection from './StubSection.svelte';
  import { LATER_COLUMNS, TRAILING_LATER } from '../lib/columns.js';

  let { rows = [], columns = [], sort = null, ctx = {}, selected = null, separate = false, onsort = () => {}, onopen = () => {} } = $props();

  const later = LATER_COLUMNS.filter((c) => TRAILING_LATER.includes(c.id));

  function ariaSort(id) {
    if (sort?.column !== id) {
      return 'none';
    }
    return sort.direction === 'down' ? 'descending' : 'ascending';
  }
</script>

{#snippet said(text, label)}
  {#if label}<span aria-hidden="true">{text}</span><span class="sr">{label}</span>{:else}{text}{/if}
{/snippet}

<table class="tbl" data-squad-table data-sort={sort ? `${sort.column}:${sort.direction}` : 'none'}>
  <caption class="sr">The squad: one row per player; select a column header to sort by it</caption>
  <thead>
    <tr>
      {#each columns as column (column.id)}
        <th
          scope="col"
          class="{column.align} {column.wide ? 'wide' : 'narrow'}"
          class:sorted={sort?.column === column.id}
          aria-sort={ariaSort(column.id)}
          data-col={column.id}
        >
          <button type="button" class="sorter" onclick={() => onsort(column.id)}>
            <span aria-hidden="true">{column.label}</span>{#if sort?.column === column.id}<span class="arrow" aria-hidden="true">{sort.direction === 'down' ? ' ▼' : ' ▲'}</span>{/if}
            <span class="sr">{column.name}{sort?.column === column.id ? (sort.direction === 'down' ? ', sorted high to low' : ', sorted low to high') : ', select to sort'}</span>
          </button>
        </th>
      {/each}
      {#each later as column (column.id)}
        <th scope="col" class="g narrow" aria-hidden="true">
          <StubSection note="column: {column.name}" inline later>{column.label}</StubSection>
        </th>
      {/each}
    </tr>
  </thead>
  <tbody>
    {#each rows as row, i (row.index)}
      {#if separate && i === 11}
        <tr class="sep" aria-hidden="true"><td colspan={columns.length + later.length}></td></tr>
      {/if}
      <tr class:me={selected === row.index} aria-current={selected === row.index ? 'true' : undefined} class:odd={i % 2 === 0} data-row={row.index} data-player={row.player['player.id']}>
        {#each columns as column (column.id)}
          {@const cell = column.cell(row.player, ctx)}
          <td class="{column.align}" class:wide={column.wide} data-cell={column.id} data-hidden={cell.hidden ?? undefined}>
            {#if column.id === 'player'}
              <button type="button" class="who" onclick={() => onopen(row.index)} aria-label="Open the panel of {cell.text}">{cell.text}</button>
            {:else if cell.chips}
              <span class="chips">
                {#if cell.label}<span class="sr">{cell.label}</span>{/if}
                {#each cell.chips as chip, c (c)}<span class="rt {chip.band}" aria-hidden="true">{chip.text}</span>{/each}
              </span>
            {:else if cell.band}
              <b class="num {cell.band}">{@render said(cell.text, cell.label)}</b>
            {:else if cell.ring}
              <span class="pc {cell.ring}">{cell.text}</span>
            {:else if cell.strong}
              <b class="num">{cell.text}</b>
            {:else if cell.muted}
              <span class="g">{@render said(cell.text, cell.label)}</span>
            {:else}
              <span class:disp={cell.display}>{@render said(cell.text, cell.label)}</span>{#if cell.note}<span class="g note"> · {cell.note}</span>{/if}
            {/if}
          </td>
        {/each}
        {#each later as column (column.id)}
          <td class="g" aria-hidden="true">
            <StubSection note="cell: {column.name}" inline>—</StubSection>
          </td>
        {/each}
      </tr>
    {/each}
  </tbody>
</table>

<style>
  .tbl {
    width: 100%;
    border-collapse: collapse;
    font-size: 10px;
  }

  th {
    font: 600 9.5px var(--fd);
    color: var(--ink-2);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    text-align: left;
    padding: 0 4px;
    white-space: nowrap;
  }

  th.narrow {
    width: 1%;
  }

  th.c {
    text-align: center;
  }

  th.r {
    text-align: right;
  }

  /* Every header and name button is at least --hit high (24 px, 44 px at the compact step) and
     a header at least 24 px wide, so neighbouring controls never share a pointer target. */
  .sorter {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    min-height: var(--hit);
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    cursor: pointer;
  }

  .sorter:hover {
    color: var(--ink);
  }

  .sorter:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: 1px;
  }

  th.sorted {
    color: var(--cyan);
    box-shadow: inset 0 -2px 0 var(--cyan);
  }

  td {
    height: var(--hit);
    padding: 0 4px;
    white-space: nowrap;
    background: var(--ground-3);
    font-variant-numeric: tabular-nums;
  }

  tr.odd td {
    background: var(--ground-2);
  }

  td.wide {
    width: auto;
    font-size: 9.5px;
  }

  td.c {
    text-align: center;
  }

  td.r {
    text-align: right;
  }

  tr.me td {
    background: var(--cyan);
    color: var(--cyan-ink);
    font-weight: 600;
  }

  /* The selected row is cyan: every number, ring and word in it reads in the cyan's own ink,
     so no band colour sits on the cyan. The match-rating chips keep their own grounds. */
  tr.me .g,
  tr.me .who,
  tr.me .v1,
  tr.me .v2,
  tr.me .v3,
  tr.me .v4,
  tr.me .pc {
    color: var(--cyan-ink);
  }

  tr.me .pc {
    background: transparent;
  }

  .sep td {
    height: 4px;
    background: transparent;
  }

  .who {
    min-height: var(--hit);
    padding: 0;
    border: 0;
    background: none;
    color: var(--ink);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .who:hover {
    text-decoration: underline;
  }

  .who:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: 1px;
  }

  .disp {
    font: 600 9.5px var(--fd);
  }

  .g {
    color: var(--ink-3);
  }

  .note {
    font-size: 9px;
  }

  .num {
    font-variant-numeric: tabular-nums;
  }

  b.num {
    font-weight: 700;
  }

  .v4 {
    color: var(--good);
  }

  .v3 {
    color: var(--mid);
  }

  .v2 {
    color: var(--warn);
  }

  .v1 {
    color: var(--bad);
  }

  .pc {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    height: 15px;
    padding: 0 4px;
    border-radius: 2px;
    background: var(--chip);
    font: 600 9px var(--fd);
    color: var(--good);
  }

  .pc::before {
    content: '';
    width: 7px;
    height: 7px;
    border-radius: 50%;
    border: 1.5px solid currentColor;
    border-right-color: transparent;
  }

  .pc.lo {
    color: var(--warn);
  }

  .pc.vl {
    color: var(--bad);
  }

  .chips {
    display: inline-flex;
    gap: 4px;
  }

  .rt {
    display: inline-block;
    min-width: 30px;
    text-align: center;
    font: 700 10px/15px var(--fd);
    border-radius: 2px;
    color: var(--on-good);
  }

  .rt.a {
    background: var(--good);
  }

  .rt.b {
    background: var(--mid);
    color: var(--on-mid);
  }

  .rt.c {
    background: var(--warn);
    color: var(--on-warn);
  }

  .rt.d {
    background: var(--bad);
    color: var(--on-bad);
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  /* The compact step: tighter cell padding, and every header keeps the 44 px hit area across
     as well as down. The default view's columns are then wider than a 768 px window, so the
     table scrolls sideways in its box (a data table may) and the Player column stays put. */
  @media (max-width: 1023px), (max-height: 599px) {
    th,
    td {
      padding: 0 2px;
    }

    .sorter {
      min-width: var(--hit);
    }

    th[data-col='player'],
    td[data-cell='player'] {
      position: sticky;
      left: 0;
      z-index: 1;
      background: var(--ground-3);
    }

    th[data-col='player'] {
      background: var(--ground);
    }

    tr.odd td[data-cell='player'] {
      background: var(--ground-2);
    }

    tr.me td[data-cell='player'] {
      background: var(--cyan);
    }
  }
</style>
