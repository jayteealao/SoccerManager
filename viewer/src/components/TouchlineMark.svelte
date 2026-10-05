<!-- The Touchline mark (DESIGN.md "The mark is the touchline"), in its pitchside variant for
     dark surfaces: the turf tile, the white line across it, the dark band below, one marker
     and the ball on the field, and a faint ring. The geometry is `markGeometry` in
     lib/mark.js, the function the favicon and the crests draw from. `reveal` plays the
     splash's logo reveal on the mark's parts (the tile, the line, the arc, the marker and the
     ball); `finished` shows the reveal's last frame at once. Reduced motion stops every
     animation through the root rule, which leaves the last frame. -->
<script>
  import { markGeometry } from '../lib/mark.js';

  let { size = 48, reveal = false, finished = false, label = 'Touchline' } = $props();

  const uid = $props.id();
  let g = $derived(markGeometry(size));
</script>

<svg
  class="mark"
  class:reveal
  class:finished
  width={size}
  height={size}
  viewBox="0 0 {size} {size}"
  role="img"
  aria-label={label}
>
  <clipPath id="mk-{uid}"><rect width={size} height={size} rx={g.radius}></rect></clipPath>
  <g class="tile" clip-path="url(#mk-{uid})">
    <rect class="field" width={size} height={g.lineY}></rect>
    <rect class="band" y={g.lineY} width={size} height={size - g.lineY}></rect>
    <circle class="arc" cx={g.arc.x} cy={g.arc.y} r={g.arc.r} stroke-width={Math.max(1, size * 0.03)}></circle>
    <rect class="line" y={g.lineY - g.lineHeight} width={size} height={g.lineHeight}></rect>
    <circle class="marker" cx={g.marker.x} cy={g.marker.y} r={g.marker.r}></circle>
    <circle class="ball" cx={g.ball.x} cy={g.ball.y} r={g.ball.r}></circle>
  </g>
  <rect class="ring" x="0.5" y="0.5" width={size - 1} height={size - 1} rx={g.radius}></rect>
</svg>

<style>
  .mark {
    flex: none;
    display: block;
  }

  .field {
    fill: var(--pitch);
  }

  .band {
    fill: var(--navy-900);
  }

  .arc {
    fill: none;
    stroke: var(--pitch-line);
    stroke-opacity: 0.35;
  }

  .line,
  .marker,
  .ball {
    fill: var(--pitch-line);
  }

  .ring {
    fill: none;
    stroke: var(--ink);
    stroke-opacity: 0.25;
  }

  /* The logo reveal (02c-craft.md, Motion): the tile, then the line draws from the left, the
     arc fades in, the marker appears and the ball rolls in. Only transform and opacity move;
     no part starts from scale 0. */
  .reveal .tile {
    transform-box: fill-box;
    transform-origin: center;
    animation: tile 400ms var(--ease) both;
  }

  .reveal .line {
    transform-box: fill-box;
    transform-origin: left center;
    animation: line 500ms var(--ease) 300ms both;
  }

  .reveal .arc {
    animation: fade 400ms var(--ease) 500ms both;
  }

  .reveal .marker {
    transform-box: fill-box;
    transform-origin: center;
    animation: pop 300ms var(--ease) 750ms both;
  }

  .reveal .ball {
    animation: ball 450ms var(--ease) 800ms both;
  }

  .reveal.finished .tile,
  .reveal.finished .line,
  .reveal.finished .arc,
  .reveal.finished .marker,
  .reveal.finished .ball {
    animation: none;
  }

  @keyframes tile {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  @keyframes line {
    from {
      transform: scaleX(0.02);
    }
    to {
      transform: scaleX(1);
    }
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.6);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  @keyframes ball {
    from {
      opacity: 0;
      transform: translateX(-40px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
</style>
