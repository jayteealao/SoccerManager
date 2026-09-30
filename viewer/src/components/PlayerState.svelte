<!-- The player-state table of the Touchline, ported from the sketch: our eleven at the rendered
     tick, each with the slow reserve (the engine's one energy figure) as a bar, a number and
     its band word, and the last card with its minute. The sprint reserve, the mind and whether
     the player has taken up the last shout need models the engine does not have: those columns
     are LATER stubs. The word, never the colour, carries each state. -->
<script>
  import SectionLabel from './SectionLabel.svelte';
  import StubSection from './StubSection.svelte';

  let { rows = [], clock = '' } = $props();
</script>

<section aria-label="Player state">
  <SectionLabel label="Player state" note="true figures · {clock}" />
  <table>
    <caption class="sr">Our eleven at {clock}: the slow reserve and cards of each player</caption>
    <thead>
      <tr>
        <th scope="col" class="n">#</th>
        <th scope="col" class="nm">Player</th>
        <th scope="col" class="stubcol" aria-hidden="true">
          <!-- STUB: the sprint reserve needs a model the engine does not have. -->
          <StubSection note="sprint reserve" inline>Sprint reserve</StubSection>
        </th>
        <th scope="col" class="res">Slow reserve</th>
        <th scope="col" class="stubcol mind" aria-hidden="true">
          <StubSection note="mind" inline>Mind</StubSection>
        </th>
        <th scope="col" class="card">Card</th>
        <th scope="col" class="stubcol taken" aria-hidden="true">
          <StubSection note="taken up" inline>Taken up</StubSection>
        </th>
      </tr>
    </thead>
    <tbody>
      {#each rows as row, i (row.wire)}
        <tr class:even={i % 2 === 1} class:off={row.sentOff}>
          <td class="n num">{row.shirt}</td>
          <td class="nm">{row.surname}</td>
          <td class="stubcol" aria-hidden="true">
            <StubSection note="sprint reserve: {row.surname}" inline><span class="bar"><i></i></span></StubSection>
          </td>
          <td class="res">
            <span class="bar" aria-hidden="true"><i style:width="{row.reserve}%" style:background="var({row.token})"></i></span>
            <b class="num">{row.reserve}</b>
            <span class="band" style:color="var({row.token})">{row.band}</span>
          </td>
          <td class="stubcol mind" aria-hidden="true">
            <StubSection note="mind: {row.surname}" inline>—</StubSection>
          </td>
          <td class="card">
            {#if row.card}
              <span class="cm {row.card.kind}" aria-hidden="true"></span><span class="num">{row.card.minute}</span>
              <span class="sr">{row.card.word}</span>
            {:else if row.injured}
              <span class="inj">Injured</span>
            {/if}
          </td>
          <td class="stubcol taken" aria-hidden="true">
            <StubSection note="taken up: {row.surname}" inline>—</StubSection>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
  <p class="foot">
    The slow reserve is each player's energy: it drains through the match. A tired player is
    the usual reason the assistant proposes a substitution.
  </p>
</section>

<style>
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 11px;
  }

  th {
    height: 22px;
    text-align: left;
    font: 600 8.5px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-3);
    white-space: nowrap;
    padding: 0 4px;
  }

  td {
    height: 23px;
    padding: 0 4px;
    color: var(--ink);
    white-space: nowrap;
  }

  tr.even td {
    background: var(--ground-3);
  }

  tr.off td {
    color: var(--ink-3);
  }

  .n {
    width: 16px;
    color: var(--cyan);
  }

  td.nm {
    font-weight: 600;
  }

  .bar {
    display: inline-block;
    vertical-align: middle;
    width: 40px;
    height: 5px;
    background: var(--bar-track);
    border-radius: 1px;
    overflow: hidden;
  }

  .bar i {
    display: block;
    height: 100%;
    background: var(--good);
  }

  .res b {
    display: inline-block;
    width: 22px;
    text-align: right;
    font-weight: 600;
  }

  .band {
    margin-left: 4px;
    font-size: 10px;
  }

  .mind,
  .taken {
    text-align: center;
  }

  .cm {
    display: inline-block;
    width: 6px;
    height: 9px;
    margin-right: 3px;
    vertical-align: -1px;
    border-radius: 1px;
  }

  .cm.yellow {
    background: var(--mid);
  }

  .cm.red {
    background: var(--bad);
  }

  .inj {
    color: var(--bad);
    font-size: 10px;
  }

  .num {
    font-variant-numeric: tabular-nums;
  }

  .foot {
    margin: 8px 0 0;
    font-size: 9.5px;
    line-height: 1.45;
    color: var(--ink-3);
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
