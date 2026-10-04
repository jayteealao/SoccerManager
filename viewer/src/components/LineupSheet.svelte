<!-- One club's line-up sheet on the Pre-match line-ups, ported from the sketch's broadcast
     graphic: a 3 px rule in the club's kit colour over the navy head (crest, name, shape), the
     eleven in slot order with each shirt number in a kit-coloured box, then the substitutes
     in three columns. Our sheet carries the injury-risk word per player; condition, sharpness
     and the workload note are LATER stubs. The other club's sheet mirrors ours and carries no
     figures: the engine sends none for it. Read only: nothing here takes focus. -->
<script>
  import Crest from './Crest.svelte';
  import StubSection from './StubSection.svelte';
  import { teamKit } from '../lib/team-kit.js';

  let { team, sheet, shape = '', side = 0 } = $props();

  let kit = $derived(teamKit(team));
  let name = $derived(team?.['team.name'] ?? '');
  let away = $derived(side === 1);
</script>

<section class="sheet" class:away aria-label="{name} line-up">
  <div class="head" style:border-top-color={kit?.fill}>
    {#if !away}<Crest {team} width={24} height={26} />{/if}
    <h3 data-may-truncate>{name}</h3>
    <span class="shape">{shape}</span>
    {#if away}<Crest {team} width={24} height={26} />{/if}
  </div>

  <table>
    <caption class="sr">{name}: the eleven{away ? '' : ', with the injury risk of each'}</caption>
    <thead>
      <tr>
        <th scope="col" class="n"><span class="sr">Shirt</span></th>
        <th scope="col" class="nm"><span class="sr">Player</span></th>
        <th scope="col" class="pos"><span class="sr">Position</span></th>
        {#if !away}
          <th scope="col" class="fig" aria-hidden="true">
            <!-- STUB: condition and sharpness between matches need a model the engine does not have. -->
            <StubSection note="condition and sharpness" inline><span class="h">Cond</span><span class="h">Sharp</span></StubSection>
          </th>
          <th scope="col" class="risk"><span class="h">Risk</span></th>
        {/if}
      </tr>
    </thead>
    <tbody>
      {#each sheet.eleven as row (row.id)}
        <tr>
          <td class="n"><span class="box" style:background={kit?.fill} style:color={kit?.number}>{row.shirt}</span></td>
          <td class="nm">{row.surname}</td>
          <td class="pos">{row.position}</td>
          {#if !away}
            <td class="fig" aria-hidden="true">
              <StubSection note="condition and sharpness: {row.surname}" inline><span class="v">—</span><span class="v">—</span></StubSection>
            </td>
            <td class="risk {row.risk.tone}">{row.risk.word}</td>
          {/if}
        </tr>
      {/each}
    </tbody>
  </table>

  {#if away}
    <p class="note">Their eleven as named at kick-off</p>
  {:else}
    <!-- STUB: the workload note needs the match calendar, which the engine does not have. -->
    <StubSection note="workload note" later><p class="note">Workload over recent matches</p></StubSection>
  {/if}

  <h4>Substitutes</h4>
  <ul class="subs">
    {#each sheet.bench as row (row.id)}
      <li data-may-truncate><span class="num">{row.shirt}</span> {row.surname}</li>
    {/each}
  </ul>
</section>

<style>
  .sheet {
    min-width: 0;
  }

  .head {
    height: 40px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px;
    background: var(--sheet-head);
    border-top: 3px solid var(--club-sample);
    color: var(--band-ink);
  }

  h3 {
    margin: 0;
    flex: 1;
    font: 800 15px var(--fd);
    letter-spacing: 0.02em;
    text-transform: uppercase;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .shape {
    font: 700 11px var(--fd);
    letter-spacing: 0.04em;
    color: var(--navy-sub);
    white-space: nowrap;
  }

  .away h3 {
    text-align: right;
    order: 1;
  }

  .away .shape {
    order: 0;
  }

  .away :global(.crest) {
    order: 2;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 8px;
  }

  th {
    height: 16px;
    padding: 0;
    text-align: left;
  }

  .h {
    display: inline-block;
    width: 36px;
    text-align: right;
    font: 600 8.5px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-3);
  }

  th.risk .h {
    width: auto;
    text-align: left;
    padding-left: 6px;
  }

  td {
    height: 20px;
    padding: 0;
    font-size: 11.5px;
    color: var(--ink);
    white-space: nowrap;
  }

  td.n {
    width: 26px;
  }

  .box {
    display: inline-grid;
    place-items: center;
    width: 20px;
    height: 16px;
    border-radius: var(--radius-sm);
    font: 700 10.5px var(--fd);
    font-variant-numeric: tabular-nums;
    background: var(--club-sample);
    color: var(--band-ink);
  }

  td.nm {
    font: 600 12px var(--fb);
    padding-left: 4px;
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  td.pos {
    font: 600 8.5px var(--fd);
    letter-spacing: 0.05em;
    color: var(--ink-3);
    width: 34px;
  }

  .away td.pos {
    text-align: right;
  }

  td.fig {
    width: 76px;
  }

  .v {
    display: inline-block;
    width: 36px;
    text-align: right;
    color: var(--ink-2);
  }

  td.risk {
    font-size: 10px;
    padding-left: 6px;
    width: 44px;
  }

  .risk.good {
    color: var(--good);
  }

  .risk.warn {
    color: var(--warn);
  }

  .risk.bad {
    color: var(--bad);
  }

  .note {
    margin: 6px 0 0;
    font-size: 9.5px;
    color: var(--ink-3);
  }

  h4 {
    margin: 12px 0 4px;
    font: 700 9.5px var(--fd);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--ink-3);
  }

  .subs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    column-gap: 8px;
    row-gap: 1px;
    font-size: 11px;
    color: var(--ink);
  }

  .subs li {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .subs .num {
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
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
