<!-- The season block of the player panel, ported from the sketch's form block: his last six
     match ratings as a line over a dashed 7.0 line, then So far, Story, Goal and Form. Before
     his first match it reads "No matches this season yet." with his goal. The line is drawn
     for the eye only; a screen reader reads the ratings as text. `season` is
     `player-panel.js` `seasonModel(...)`. -->
<script>
  import { ratingText } from '../lib/scale.js';

  let { season } = $props();

  const W = 200;
  const H = 40;
  /// Where a rating 1.0 to 10.0 sits on the line's height.
  const y = (r) => H - 3 - ((Math.max(1, Math.min(10, r)) - 1) / 9) * (H - 6);
  let points = $derived(
    (season?.points ?? []).map((r, i, all) => ({
      x: all.length === 1 ? W / 2 : 4 + (i * (W - 8)) / (all.length - 1),
      y: y(r),
    }))
  );
</script>

<div class="season" data-season={season?.none ? 'none' : season?.empty ? 'empty' : 'some'}>
  {#if season?.none}
    <p class="g">{season.text}</p>
  {:else}
    {#if season?.empty}
      <p class="g" data-season-empty>{season.text}</p>
    {:else}
      <svg class="line" viewBox="0 0 {W} {H}" preserveAspectRatio="none" aria-hidden="true" data-season-line={points.length}>
        <line class="ref" x1="0" y1={y(7)} x2={W} y2={y(7)} />
        {#if points.length > 1}
          <polyline points={points.map((p) => `${p.x},${p.y}`).join(' ')} />
        {/if}
        {#each points as p, i (i)}<circle cx={p.x} cy={p.y} r="2.5" />{/each}
      </svg>
      <p class="sr">Last ratings: {season.points.map((r) => ratingText(r)).join(', ')}.</p>
    {/if}
    {#each season?.rows ?? [] as row (row.key)}
      <p class="kv" data-season-row={row.key}>
        <span>{row.label}</span>
        <b class:g={row.known === false}>
          {#if row.mark}<i class="mark {row.token === '--good' ? 'up' : row.token === '--warn' ? 'down' : 'flat'}" aria-hidden="true">{row.mark}</i>{/if}{row.text}
        </b>
      </p>
    {/each}
  {/if}
</div>

<style>
  .season {
    margin-bottom: 6px;
  }

  .line {
    display: block;
    width: 100%;
    height: 40px;
    margin: 2px 0 6px;
    overflow: visible;
  }

  .line .ref {
    stroke: var(--rule);
    stroke-dasharray: 3 3;
    vector-effect: non-scaling-stroke;
  }

  .line polyline {
    fill: none;
    stroke: var(--cyan);
    stroke-width: 1.5;
    vector-effect: non-scaling-stroke;
  }

  .line circle {
    fill: var(--cyan);
  }

  .kv {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    margin: 0;
    padding: 3px 0;
    font-size: 10px;
    color: var(--ink-2);
    border-top: 1px solid var(--rule-2);
  }

  .kv > span {
    flex: none;
    white-space: nowrap;
  }

  .kv b {
    color: var(--ink);
    font-weight: 600;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .kv b.g {
    color: var(--ink-3);
    font-weight: 400;
  }

  .mark {
    font-style: normal;
    margin-right: 3px;
  }

  .mark.up {
    color: var(--good);
  }

  .mark.down {
    color: var(--warn);
  }

  .mark.flat {
    color: var(--ink-2);
  }

  .g {
    margin: 0 0 6px;
    color: var(--ink-3);
    font-size: 10px;
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: 0;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
