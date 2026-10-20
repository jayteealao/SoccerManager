<!-- A phase card's mini pitch, upright with our goal at the bottom: the turf, the markings,
     the band of the pitch the phase is about, and the eleven slots of the formation. A
     transition phase marks the whole pitch and an arrow the way play turns. Drawn only. -->
<script>
  import { uprightPlace } from '../lib/lineup-editor.js';

  let { slots = [], zone = 'middle', side = 'in', label = '' } = $props();

  const W = 84;
  const H = 114;
  const BANDS = { own: [H * (2 / 3), H / 3], middle: [H / 3, H / 3], their: [0, H / 3] };

  let band = $derived(BANDS[zone] ?? null);
  let dots = $derived(
    slots.map((s) => {
      const place = uprightPlace(s.x, s.y);
      return { x: (place.left / 100) * W, y: (place.top / 100) * H, keeper: s.position === 'GK' };
    })
  );
</script>

<svg class="pitch" viewBox="0 0 {W} {H}" role="img" aria-label={label}>
  <rect class="turf" x="0" y="0" width={W} height={H} />
  {#if band}
    <rect class="band" x="0" y={band[0]} width={W} height={band[1]} />
  {:else}
    <rect class="band whole" x="0" y="0" width={W} height={H} />
  {/if}
  <g class="lines">
    <rect x="0.5" y="0.5" width={W - 1} height={H - 1} />
    <line x1="0" y1={H / 2} x2={W} y2={H / 2} />
    <circle cx={W / 2} cy={H / 2} r="10" />
    <rect x="18" y="0.5" width="48" height="17" />
    <rect x="18" y={H - 17.5} width="48" height="17" />
  </g>
  {#if zone === 'transition'}
    <path
      class="arrow"
      d={side === 'in' ? `M${W - 10} ${H - 22} L${W - 10} 24 M${W - 14} 30 L${W - 10} 24 L${W - 6} 30` : `M10 24 L10 ${H - 22} M6 ${H - 28} L10 ${H - 22} L14 ${H - 28}`}
    />
  {/if}
  {#each dots as d, i (i)}
    <circle class="dot" class:keeper={d.keeper} cx={d.x} cy={d.y} r="3.2" />
  {/each}
</svg>

<style>
  .pitch {
    display: block;
    width: 84px;
    height: 114px;
    border-radius: var(--radius-sm);
  }

  .turf {
    fill: var(--pitch);
  }

  .band {
    fill: var(--pitch-stripe);
    stroke: var(--cyan);
    stroke-width: 1;
  }

  .band.whole {
    fill: none;
    stroke: none;
  }

  .lines {
    fill: none;
    stroke: var(--pitch-line);
    stroke-opacity: 0.55;
    stroke-width: 1;
  }

  .arrow {
    fill: none;
    stroke: var(--cyan);
    stroke-width: 1.6;
  }

  .dot {
    fill: var(--tactic-marker);
    stroke: var(--tactic-marker-ink);
    stroke-width: 0.8;
  }

  .dot.keeper {
    fill: var(--keeper-home);
  }

  @media (min-width: 1600px) {
    .pitch {
      width: 108px;
      height: 146px;
    }
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .pitch {
      width: 108px;
      height: 146px;
    }
  }
</style>
