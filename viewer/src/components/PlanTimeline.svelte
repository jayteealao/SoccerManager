<!-- When each game plan can switch on: a lane per plan with a time condition, from its minute
     to the end of the 90, and a line for the plans that can switch on at any time. Drawn with
     theme tokens only. -->
<script>
  import { timeline } from '../lib/game-plans.js';

  let { state, schema } = $props();

  let t = $derived(timeline(state, schema));
  const W = 600;
  const LANE = 16;
  const x = (minute) => 8 + (minute / 90) * (W - 16);
</script>

<div class="tl" data-plan-timeline>
  <svg viewBox="0 0 {W} {24 + Math.max(1, t.lanes.length) * LANE}" role="img" aria-label="When the game plans can switch on">
    {#each [0, 15, 30, 45, 60, 75, 90] as m (m)}
      <line class="tick" x1={x(m)} x2={x(m)} y1="12" y2={20 + Math.max(1, t.lanes.length) * LANE} />
      <text class="lab" x={x(m)} y="9">{m}'</text>
    {/each}
    {#each t.lanes as lane, i (lane.n)}
      <rect class="bar {lane.tone}" x={x(lane.from)} y={16 + i * LANE} width={x(90) - x(lane.from)} height={LANE - 5} rx="2" />
      <text class="txt" x={x(lane.from) + 4} y={16 + i * LANE + 8}>{lane.text}</text>
    {/each}
    {#if t.lanes.length === 0}
      <text class="txt" x="8" y="28">No plan has a time condition.</text>
    {/if}
  </svg>
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
