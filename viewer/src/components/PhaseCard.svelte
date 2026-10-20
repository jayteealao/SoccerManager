<!-- One phase of play on the Phases board: its title and where the ball is, the mini pitch,
     and one row per instruction that sits in it. A transition card lists the players who have
     their own instruction for it and links to the Roles tab. The share of the last match and
     the phase's result are faded stubs: they need match reports the board does not read yet. -->
<script>
  import BoardRow from './BoardRow.svelte';
  import PhasePitch from './PhasePitch.svelte';
  import StubSection from './StubSection.svelte';

  let { card, rows = [], slots = [], players = null, disabled = false, onset = () => {}, onroles = () => {} } = $props();
</script>

<section class="card" data-card={card.id} aria-labelledby="card-{card.id}">
  <div class="head">
    <h4 id="card-{card.id}">{card.title}</h4>
    <span>{card.sub}</span>
    <StubSection note="share of the last match: {card.id}" inline><em>—%</em></StubSection>
  </div>
  <div class="in">
    <PhasePitch {slots} zone={card.zone} side={card.side} label="{card.title}: {card.sub}" />
    <div class="rows">
      {#each card.rows as placed, i (placed.primitive)}
        {@const row = rows[i]}
        {#if placed.sub}<div class="sub">{placed.sub}</div>{/if}
        {#if row}
          <BoardRow
            label={row.label}
            full={row.full}
            words={row.words}
            control={placed.control}
            value={row.value}
            def={row.def}
            changed={row.changed}
            error={row.error}
            styleWord={row.styleWord}
            primitive={row.primitive}
            {disabled}
            onchange={(level) => onset(row.index, level)}
          />
        {/if}
      {/each}
      {#if players}
        <div class="sub">Player instructions</div>
        {#each players as p (p.slot)}
          <div class="own"><span class="tag">OWN</span><span><b>{p.name}</b> · {p.word}</span></div>
        {:else}
          <div class="own none">None</div>
        {/each}
        <button type="button" class="lnk" onclick={onroles}>Edit in Roles ›</button>
      {/if}
    </div>
  </div>
  <StubSection note="phase result: {card.id}">
    <div class="foot">Result against the last opponent</div>
  </StubSection>
</section>

<style>
  .card {
    min-width: 0;
    background: var(--ground-2);
    border-radius: 3px;
    padding: 6px 8px 5px;
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 6px;
    margin-bottom: 4px;
  }

  h4 {
    margin: 0;
    font: 700 11.5px var(--fd);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ink);
  }

  .head > span {
    font-size: 9px;
    color: var(--ink-3);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .head em {
    font: 600 9px var(--fd);
    font-style: normal;
    color: var(--ink-2);
  }

  .in {
    display: grid;
    grid-template-columns: 84px minmax(0, 1fr);
    gap: 8px;
    align-items: start;
  }

  .rows {
    min-width: 0;
  }

  .sub {
    font: 700 8px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-3);
    padding: 4px 0 1px;
  }

  .own {
    display: flex;
    gap: 5px;
    align-items: center;
    font-size: 9.5px;
    padding: 2px 0;
    border-bottom: 1px solid var(--rule-2);
    color: var(--ink-2);
  }

  .own b {
    color: var(--ink);
  }

  .own.none {
    color: var(--ink-3);
  }

  .tag {
    font: 700 7.5px/11px var(--fd);
    letter-spacing: 0.05em;
    padding: 0 3px;
    border-radius: var(--radius-sm);
    background: var(--cyan);
    color: var(--cyan-ink);
  }

  .lnk {
    min-height: var(--hit);
    margin: 0;
    padding: 0;
    border: 0;
    background: none;
    font: 600 9px var(--fd);
    color: var(--cyan);
    cursor: pointer;
  }

  .lnk:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: 1px;
  }

  .foot {
    margin-top: 4px;
    padding-top: 3px;
    border-top: 1px solid var(--rule-2);
    font-size: 9px;
    color: var(--ink-3);
  }

  @media (min-width: 1600px) {
    .in {
      grid-template-columns: 108px minmax(0, 1fr);
    }
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .in {
      grid-template-columns: 108px minmax(0, 1fr);
    }
  }
</style>
