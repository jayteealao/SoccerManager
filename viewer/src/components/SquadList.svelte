<!-- The one squad list of the sketch's pre-match Tactics board: every player once, grouped as
     In the eleven, On the bench, Not picked and Cannot play, with a slot chip, the shirt
     number, the name with position chips, and the Fit column. Each row is a button: click one
     row then another to swap their places, or drag a row onto another row or onto a pitch
     slot. The picked row is named in the hint above the list, beside Empty the picked slot.
     The Tactic and Sharp columns and the Cannot play group are LATER stubs: the engine has no
     familiarity, sharpness or availability model yet. -->
<script>
  import SectionLabel from './SectionLabel.svelte';
  import StubSection from './StubSection.svelte';

  let {
    list,
    total = 0,
    shape = '',
    benchSize = 7,
    pickedText = null,
    canEmpty = false,
    onpickrow = () => {},
    onpickplace = () => {},
    ondrop = () => {},
    onempty = () => {},
  } = $props();

  let filled = $derived(list.eleven.filter((r) => !r.empty).length);
  let clash = $derived([...list.eleven, ...list.bench].find((r) => r.clash) ?? null);

  function dragstart(event, index) {
    event.dataTransfer?.setData('text/plain', String(index));
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
    }
  }

  function dragover(event) {
    event.preventDefault();
  }

  function drop(event, to) {
    event.preventDefault();
    const from = Number(event.dataTransfer?.getData('text/plain'));
    if (Number.isInteger(from)) {
      ondrop(from, to);
    }
  }
</script>

{#snippet grip()}
  <svg class="grip" width="8" height="12" viewBox="0 0 8 12" aria-hidden="true" focusable="false">
    <circle cx="2" cy="2" r="1.1" /><circle cx="6" cy="2" r="1.1" />
    <circle cx="2" cy="6" r="1.1" /><circle cx="6" cy="6" r="1.1" />
    <circle cx="2" cy="10" r="1.1" /><circle cx="6" cy="10" r="1.1" />
  </svg>
{/snippet}

{#snippet rowsOf(rows)}
  {#each rows as row (row.empty ? row.key : row.index)}
    {#if row.empty}
      <button
        type="button"
        class="row empty"
        class:picked={row.picked}
        aria-pressed={row.picked}
        aria-label={row.label}
        onclick={() => onpickplace(row.place)}
        ondragover={dragover}
        ondrop={(e) => drop(e, row.place)}
      >
        <span></span>
        <span class="chip {row.place.kind === 'slot' ? 'eleven' : 'bench'}">{row.chip}</span>
        <span></span>
        <span class="who g">Empty place</span>
        <span></span><span></span><span></span>
      </button>
    {:else}
      <button
        type="button"
        class="row"
        class:eleven={row.group === 'eleven'}
        class:picked={row.picked}
        draggable="true"
        aria-pressed={row.picked}
        aria-label={row.label}
        data-squad={row.index}
        onclick={() => onpickrow(row.index)}
        ondragstart={(e) => dragstart(e, row.index)}
        ondragover={dragover}
        ondrop={(e) => drop(e, row.index)}
      >
        <span class="g">{@render grip()}</span>
        <span class="chip {row.chipKind}">{row.chip}</span>
        <b class="num g shirt">{row.shirt}</b>
        <span class="who">
          <b class="name">{row.name}</b>
          {#each row.positions as p (p)}<span class="pc">{p}</span>{/each}
          {#if row.clash}<span class="bd">named twice</span>{/if}
        </span>
        <!-- STUB: the player's grasp of the tactic. -->
        <StubSection note="tactic column" inline><i class="track"></i></StubSection>
        <b class="num r">{row.fitness}</b>
        <!-- STUB: match sharpness. -->
        <StubSection note="sharpness column" inline><b class="num r">—</b></StubSection>
      </button>
    {/if}
  {/each}
{/snippet}

<SectionLabel label="Squad" note="{total} players · drag a row onto another to swap" />

<div class="legend" aria-hidden="true">
  <span class="chip eleven">RB</span><span>In the eleven</span>
  <span class="chip bench">S1</span><span>On the bench</span>
  <span class="chip none">—</span><span>Not picked</span>
  <svg class="icon" width="12" height="12" viewBox="0 0 16 16"><rect class="hurt" x="1" y="1" width="14" height="14" rx="2" /><path class="cross" d="M8 4v8M4 8h8" /></svg>
  <svg class="icon" width="12" height="12" viewBox="0 0 16 16"><rect class="hurt" x="4" y="1.5" width="8.5" height="13" rx="1.2" transform="rotate(8 8 8)" /></svg>
  <span>Cannot play</span>
</div>

<div class="hint" role="status">
  {#if pickedText}
    <span><b>{pickedText}</b> <span class="g">Click another row to swap, or</span></span>
  {:else}
    <span class="g">Click a row, then another row or a pitch slot, to swap.</span>
  {/if}
  <button class="btn gh" type="button" disabled={!canEmpty} onclick={onempty}>Empty the picked slot</button>
</div>

<div class="head" aria-hidden="true">
  <span></span><span class="c">Slot</span><span class="r">No</span><span>Player · positions</span>
  <span>Tactic</span><span class="r">Fit</span><span class="r">Sharp</span>
</div>

<div class="list">
  <div class="group"><span>In the eleven</span><em>{shape} · {filled} of 11</em></div>
  {@render rowsOf(list.eleven)}
  <div class="group">
    <span>On the bench</span><em>{benchSize} places{#if clash} · {clash.name} named twice{/if}</em>
  </div>
  {@render rowsOf(list.bench)}
  <div class="group"><span>Not picked</span><em>{list.out.length} players</em></div>
  {@render rowsOf(list.out)}
  <!-- STUB: availability (injuries and bans) needs a model the engine does not have yet. -->
  <StubSection note="cannot play group" later>
    <div class="group"><span>Cannot play</span><em>injuries and bans</em></div>
    <div class="row off">
      <span></span><span class="chip none">—</span><span></span>
      <span class="who">
        <svg class="icon" width="12" height="12" viewBox="0 0 16 16" role="img" aria-label="Injured"><rect class="hurt" x="1" y="1" width="14" height="14" rx="2" /><path class="cross" d="M8 4v8M4 8h8" /></svg>
        <span class="bd">Injured · the reason in words</span>
      </span>
    </div>
    <div class="row off">
      <span></span><span class="chip none">—</span><span></span>
      <span class="who">
        <svg class="icon" width="12" height="12" viewBox="0 0 16 16" role="img" aria-label="Suspended"><rect class="hurt" x="4" y="1.5" width="8.5" height="13" rx="1.2" transform="rotate(8 8 8)" /></svg>
        <span class="bd">Suspended · the reason in words</span>
      </span>
    </div>
  </StubSection>
</div>

<style>
  .legend {
    display: flex;
    gap: 10px;
    align-items: center;
    font-size: 9.5px;
    color: var(--ink-2);
    margin: -4px 0 6px;
  }

  .hint {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    background: var(--picked-ground);
    border: 1px solid var(--cyan);
    padding: 4px 8px;
    margin-bottom: 4px;
    font-size: 10px;
    color: var(--ink);
    min-height: 32px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-sm);
    font: 600 10px var(--fb);
    cursor: pointer;
    white-space: nowrap;
  }

  .btn.gh {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--control-edge);
    color: var(--ghost-ink);
  }

  .btn:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .head,
  .row {
    display: grid;
    grid-template-columns: 10px 44px 20px 1fr 66px 36px 36px;
    gap: 6px;
    align-items: center;
    padding: 0 4px;
  }

  .head {
    height: 18px;
    border-bottom: 1px solid var(--rule);
    font: 600 9px var(--fd);
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--ink-3);
  }

  .list {
    max-height: 560px;
    overflow-y: auto;
  }

  .group {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    height: 17px;
    padding: 4px 4px 0;
    font: 700 9.5px var(--fd);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--ink-2);
  }

  .group em {
    font: 500 9px var(--fb);
    font-style: normal;
    letter-spacing: 0;
    text-transform: none;
    color: var(--ink-3);
  }

  .row {
    width: 100%;
    height: 20px;
    border: 0;
    border-bottom: 1px solid var(--rule-2);
    background: transparent;
    color: var(--ink);
    font: inherit;
    font-size: 10.5px;
    text-align: left;
    cursor: pointer;
  }

  .row.eleven {
    background: var(--eleven-tint);
  }

  .row.picked {
    background: var(--picked-ground);
    box-shadow: inset 0 0 0 1px var(--cyan);
  }

  .row:focus-visible {
    outline-offset: -2px;
  }

  .row.off {
    cursor: default;
  }

  .chip {
    font: 700 9px var(--fd);
    text-align: center;
    border-radius: var(--radius-sm);
    height: 15px;
    line-height: 13px;
    border: 1px solid transparent;
    white-space: nowrap;
    overflow: hidden;
  }

  .chip.eleven {
    background: var(--good);
    color: var(--on-slot);
    line-height: 15px;
    border: 0;
  }

  .chip.bench {
    border-color: var(--cyan);
    color: var(--cyan);
  }

  .chip.clash {
    border-color: var(--bad);
    color: var(--bad);
  }

  .chip.none {
    font-weight: 600;
    color: var(--ink-3);
  }

  .legend .chip {
    padding: 0 4px;
  }

  .who {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    overflow: hidden;
  }

  .name {
    font-weight: 600;
    white-space: nowrap;
  }

  .pc {
    display: inline-flex;
    align-items: center;
    height: 13px;
    padding: 0 4px;
    border-radius: var(--radius-sm);
    background: var(--position-chip);
    color: var(--position-chip-ink);
    font: 600 8.5px var(--fd);
  }

  .shirt {
    font-weight: 600;
    text-align: right;
  }

  .track {
    display: block;
    height: 5px;
    width: 34px;
    background: var(--bar-track);
    border-radius: 1px;
  }

  .num {
    font-weight: 600;
  }

  .r {
    text-align: right;
  }

  .c {
    text-align: center;
  }

  .g {
    color: var(--ink-3);
  }

  .bd {
    color: var(--bad);
    font-size: 9px;
    white-space: nowrap;
  }

  .hurt {
    fill: var(--bad);
  }

  .cross {
    stroke: var(--ink);
    stroke-width: 2;
  }

  .icon {
    flex: none;
  }
</style>
