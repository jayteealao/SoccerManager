<!-- The parts of the sketch's Match live screen that this version does not build, kept in
     place for the look: the pitch overlay toggles, the highlight modes and pause rules, the
     momentum chart and the win probability figure. Each renders static
     sample content inside StubSection, so it is faded, inert, hidden from assistive
     technology, and takes no prop from the match: it can never look live. -->
<script>
  import SectionLabel from './SectionLabel.svelte';
  import StubSection from './StubSection.svelte';

  let { part } = $props();

  // The momentum chart's sample spells: threat per three minutes, home above the line and
  // away below it, as the board draws them.
  const SPELLS = [
    [6, 2], [9, 3], [5, 6], [11, 2], [8, 4], [4, 9], [7, 5], [12, 3], [10, 2], [6, 7],
    [3, 10], [5, 8], [9, 4], [13, 2], [8, 3], [4, 11], [3, 12], [2, 9], [6, 5], [9, 3],
    [11, 4], [7, 6],
  ];
  const WIN = '0,34 40,33 80,30 120,18 160,16 200,20 240,31 280,36 320,33 384,34';
</script>

{#if part === 'overlays'}
  <!-- STUB: pitch overlays, zoom and follow. Not built yet; drawn for layout and
       feel only. -->
  <StubSection note="pitch overlays, zoom and follow">
    <div class="line overlays">
      <span class="g">Overlays</span>
      <span class="tg on">Passing lanes</span>
      <span class="tg on">Pressure</span>
      <span class="tg">Pitch control</span>
      <span class="tg on">Shape</span>
      <span class="tg">Player state</span>
      <span class="tg">Heat zones</span>
      <span class="gap"></span>
      <span class="tg">− 1.5× +</span>
      <span class="tg">Follow: Ferraz</span>
    </div>
  </StubSection>
{:else if part === 'highlights'}
  <!-- STUB: highlight modes, dynamic mode, custom rules and pause-on events. Not built
       yet; drawn for layout and feel only. -->
  <StubSection note="highlight modes and pause rules">
    <div class="line">
      <span class="g w54">Highlights</span>
      <span class="seg"
        ><span>Key</span><span class="on">Extended</span><span>Comprehensive</span><span
          >Full match</span
        ><span>Commentary only</span></span
      >
      <span class="tg on">Dynamic</span>
      <span class="tg on">Build-up 6 s</span>
    </div>
    <div class="line">
      <span class="g w54">Rules</span>
      <span class="tg on">All our set pieces</span>
      <span class="tg on">Every Ferraz shot</span>
      <span class="tg">+ Rule</span>
      <span class="g pause">Pause on</span>
      <span class="tg on">Goal</span>
      <span class="tg on">Injury</span>
      <span class="tg on">Card</span>
      <span class="tg on">Staff alert</span>
    </div>
  </StubSection>
{:else if part === 'momentum'}
  <!-- STUB: momentum chart. Not built yet; drawn for layout and feel only. -->
  <StubSection note="momentum chart">
    <div class="momentum">
      <SectionLabel label="Momentum" note="threat per 3 minutes · cyan home, orange away" />
      <svg width="742" height="76" viewBox="0 0 742 76" aria-hidden="true">
        {#each [0, 15, 30, 45, 60, 75] as m (m)}
          <line class="rule" x1={m * 8.24} y1="2" x2={m * 8.24} y2="66" />
          <text x={m * 8.24 + 2} y="75">{m}'</text>
        {/each}
        <line class="mid" x1="0" y1="34" x2="742" y2="34" />
        {#each SPELLS as [home, away], i (i)}
          <rect class="home" x={i * 24.7 + 2} y={34 - home * 2.2} width="20" height={home * 2.2} />
          <rect class="away" x={i * 24.7 + 2} y="34" width="20" height={away * 2.2} />
        {/each}
      </svg>
    </div>
  </StubSection>
{:else if part === 'win-probability'}
  <!-- STUB: win probability model figure. Not built yet; drawn for layout and
       feel only. -->
  <StubSection note="win probability">
    <div class="win">
      <SectionLabel label="Win probability" note="a model figure · if level: extra time" />
      <div class="bar">
        <i class="h" style:flex="44">ROVERS 44%</i><i class="d" style:flex="31">EXTRA TIME 31%</i><i
          class="a"
          style:flex="25">25%</i
        >
      </div>
      <svg width="384" height="60" viewBox="0 0 384 60" aria-hidden="true">
        <line class="mid" x1="0" y1="30" x2="384" y2="30" />
        <polyline class="curve" points={WIN} />
      </svg>
    </div>
  </StubSection>
{/if}

<style>
  .line {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-top: 6px;
    flex-wrap: nowrap;
  }

  .overlays {
    margin: 0 0 6px;
  }

  .g {
    color: var(--ink-3);
    font-size: 9.5px;
  }

  .w54 {
    width: 54px;
    flex: none;
  }

  .pause {
    margin-left: 8px;
  }

  .gap {
    flex: 1;
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

  .seg {
    display: inline-flex;
    border: 1px solid var(--seg-edge);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .seg span {
    padding: 0 8px;
    height: 20px;
    display: inline-flex;
    align-items: center;
    font-size: 9.5px;
    color: var(--seg-ink);
    border-left: 1px solid var(--seg-edge);
    white-space: nowrap;
  }

  .seg span:first-child {
    border-left: 0;
  }

  .seg span.on {
    background: var(--cyan);
    color: var(--cyan-ink);
    font-weight: 700;
  }

  .momentum {
    margin-top: 10px;
  }

  .momentum :global(h3) {
    margin-bottom: 4px;
  }

  svg {
    display: block;
  }

  svg .rule {
    stroke: var(--rule-2);
  }

  svg .mid {
    stroke: var(--rule);
  }

  svg text {
    fill: var(--ink-3);
    font: 400 9px var(--fb);
  }

  svg .home {
    fill: var(--cyan);
  }

  svg .away {
    fill: var(--warn);
  }

  svg .curve {
    fill: none;
    stroke: var(--cyan);
    stroke-width: 1.5;
  }

  .win svg {
    margin-top: 4px;
  }

  .bar {
    display: flex;
    height: 18px;
    gap: 2px;
    font: 700 9.5px/18px var(--fd);
  }

  .bar i {
    font-style: normal;
    padding-left: 5px;
    white-space: nowrap;
    overflow: hidden;
  }

  .bar .h {
    background: var(--cyan);
    color: var(--cyan-ink);
  }

  .bar .d {
    background: var(--notice-neutral);
    color: var(--on-neutral);
  }

  .bar .a {
    background: var(--warn);
    color: var(--on-warn);
  }
</style>
