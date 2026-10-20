<!-- When each game plan can switch on: a lane per plan with a time condition, from its minute
     to the end of the 90, and a line for the plans that can switch on at any time. A lane that
     starts late has its words before the bar, so none runs off the edge; a list under the drawing
     says the same for a screen reader. Drawn with theme tokens only. -->
<script>
  import { timeline } from '../lib/game-plans.js';

  let { state, schema } = $props();

  let t = $derived(timeline(state, schema));
  const W = 600;
  const LANE = 16;
  const x = (minute) => 8 + (minute / 90) * (W - 16);
  /// From this minute on a lane's words sit before its bar instead of on it.
  const LATE = 55;
</script>

<div class="tl" data-plan-timeline>
  <svg viewBox="0 0 {W} {24 + Math.max(1, t.lanes.length) * LANE}" aria-hidden="true">
    {#each [0, 15, 30, 45, 60, 75, 90] as m (m)}
      <line class="tick" x1={x(m)} x2={x(m)} y1="12" y2={20 + Math.max(1, t.lanes.length) * LANE} />
      <text class="lab" x={x(m)} y="9">{m}'</text>
    {/each}
    {#each t.lanes as lane, i (lane.n)}
      <rect class="bar {lane.tone}" x={x(lane.from)} y={16 + i * LANE} width={x(90) - x(lane.from)} height={LANE - 5} rx="2" />
      {#if lane.from > LATE}
        <text class="txt" x={x(lane.from) - 4} y={16 + i * LANE + 8} text-anchor="end">{lane.text}</text>
      {:else}
        <text class="txt" x={x(lane.from) + 4} y={16 + i * LANE + 8}>{lane.text}</text>
      {/if}
    {/each}
    {#if t.lanes.length === 0}
      <text class="txt" x="8" y="28">No plan has a time condition.</text>
    {/if}
  </svg>
  <ul class="sr-only" aria-label="When the game plans can switch on">
    {#each t.lanes as lane (lane.n)}<li>{lane.text}, from minute {lane.from} to the end</li>{/each}
    {#if t.lanes.length === 0}<li>No plan has a time condition.</li>{/if}
  </ul>
  {#if t.anyTime.length}
    <div class="any">Any time: {t.anyTime.map((a) => a.text).join(' · ')}</div>
  {/if}
</div>

<style>
  .tl svg {
    display: block;
    width: 100%;
    height: auto;
  }

  .tick {
    stroke: var(--rule-2);
    stroke-width: 1;
  }

  .lab {
    font: 600 8px var(--fd);
    fill: var(--ink-3);
    text-anchor: middle;
  }

  .bar {
    fill: var(--cyan);
    opacity: 0.35;
  }

  .bar.red {
    fill: var(--bad);
  }

  .bar.en {
    fill: var(--warn);
  }

  .txt {
    font: 500 8.5px var(--fb);
    fill: var(--ink);
  }

  .any {
    font-size: 9.5px;
    color: var(--ink-3);
    margin-top: 4px;
  }
</style>
