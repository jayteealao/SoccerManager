<!-- The positions pitch of the player panel, ported from the sketch's profile: a pitch with the
     goal on the left and one chip per position, each the position code and his whole rating
     1 to 20 in its band colour; a position he is not rated at shows "—". His natural position
     is the highest and carries a heavier outline. A screen reader reads the list beneath it,
     not the drawing. `chips` is `player-panel.js` `positionChips(...).chips`. -->
<script>
  import { BAND_WORDS } from '../lib/scale.js';

  let { chips = [] } = $props();
</script>

<div class="pitch" data-pitch>
  <svg class="lines" viewBox="0 0 200 120" preserveAspectRatio="none" aria-hidden="true">
    <rect x="1" y="1" width="198" height="118" />
    <line x1="100" y1="1" x2="100" y2="119" />
    <circle cx="100" cy="60" r="16" />
    <rect x="1" y="30" width="28" height="60" />
    <rect x="171" y="30" width="28" height="60" />
  </svg>
  {#each chips as chip (chip.code)}
    <span
      class="chip {chip.band ?? 'none'}"
      class:natural={chip.natural}
      style:left="{chip.x}%"
      style:top="{chip.y}%"
      data-chip={chip.code}
      data-rating={chip.whole ?? 'none'}
      data-natural={chip.natural ? 'true' : null}
      aria-hidden="true"
    >
      {chip.code}<b class="num">{chip.text}</b>
    </span>
  {/each}
</div>
<ul class="sr" data-pitch-list>
  {#each chips as chip (chip.code)}
    <li>{chip.label}{#if chip.band}, {BAND_WORDS[chip.band]}{/if}</li>
  {/each}
</ul>

<style>
  .pitch {
    position: relative;
    height: 120px;
    margin-bottom: 8px;
    border-radius: var(--radius-sm);
    background: var(--ground-2);
  }

  .lines {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    fill: none;
    stroke: var(--seg-edge);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }

  .lines * {
    vector-effect: non-scaling-stroke;
  }

  .chip {
    position: absolute;
    transform: translate(-50%, -50%);
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 0 4px;
    font: 700 9px/15px var(--fd);
    color: var(--position-chip-ink);
    white-space: nowrap;
    background: var(--position-chip);
    border: 1px solid var(--rule-2);
    border-radius: var(--radius-sm);
  }

  .chip b {
    font-weight: 700;
  }

  .chip.none b {
    color: var(--ink-2);
  }

  .chip.natural {
    border-width: 2px;
    padding: 0 3px;
  }

  .v4 {
    border-color: var(--good);
  }

  .v4 b {
    color: var(--good);
  }

  .v3 {
    border-color: var(--mid);
  }

  .v3 b {
    color: var(--mid);
  }

  .v2 {
    border-color: var(--warn);
  }

  .v2 b {
    color: var(--warn);
  }

  .v1 {
    border-color: var(--bad);
  }

  .v1 b {
    color: var(--bad);
  }

  .num {
    font-variant-numeric: tabular-nums;
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: 0;
    padding: 0;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
