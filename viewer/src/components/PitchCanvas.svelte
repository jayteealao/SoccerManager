<!-- The pitch box (742 × 312 on the match screen; the replay's is 752 × 290): the canvas the
     pitch renderer draws the match into, the goal banner over it, and the sketch's pitch
     overlays, which are stubs. A panel passed as the body (the loading steps, an error, first
     run) covers the box; `dim` holds the last frame at half strength while the engine
     reconnects. The canvas names its box, which the renderer reads. -->
<script>
  import StubSection from './StubSection.svelte';

  let {
    attach = () => {},
    detach = () => {},
    width = 742,
    height = 312,
    drawn = false,
    dim = false,
    banner = null,
    overlays = true,
    children,
  } = $props();

  let canvas = $state();

  $effect(() => {
    if (canvas) {
      const drawnOn = canvas;
      attach(drawnOn);
      return () => detach(drawnOn);
    }
  });
</script>

<div class="pitchbox" class:dim class:blank={!drawn} style:width="{width}px" style:height="{height}px">
  <canvas
    bind:this={canvas}
    {width}
    {height}
    data-width={width}
    data-height={height}
    style:width="{width}px"
    style:height="{height}px"
    hidden={!drawn}
    role="img"
    aria-label="The pitch: both teams and the ball at the rendered tick"
  ></canvas>
  {#if drawn && overlays}
    <!-- STUB: the view's labels, the overlay legend and the ground's conditions belong to the
         pitch overlays, which this version does not build. Drawn for layout and feel only. -->
    <StubSection note="pitch overlay labels">
      <span class="chips tl"><span class="tagc">TOP-DOWN · SLIGHT TILT</span><span class="btn">Attacking →</span></span>
      <span class="legend">Lanes: pass completion chance · red rings: pressing · dotted: our lines</span>
      <span class="cond">RAIN · PITCH WORN AT BOTH GOALMOUTHS · 21,480</span>
    </StubSection>
  {/if}
  {#if banner}
    {#key banner.id}
      <div class="banner" role="status">{banner.text}</div>
    {/key}
  {/if}
  {@render children?.()}
</div>

<style>
  .pitchbox {
    position: relative;
    border-radius: var(--radius-md);
    overflow: hidden;
    box-shadow: 0 0 0 2px var(--pitch-deep);
    flex: none;
  }

  .pitchbox.blank {
    background: var(--pitch-deep);
  }

  .pitchbox.dim canvas {
    opacity: 0.5;
  }

  canvas {
    position: absolute;
    inset: 0;
    display: block;
  }

  canvas[hidden] {
    display: none;
  }

  .chips {
    position: absolute;
    display: flex;
    gap: 4px;
  }

  .tl {
    left: 8px;
    top: 8px;
  }

  .tagc {
    display: inline-block;
    background: var(--cyan);
    color: var(--cyan-ink);
    font: 700 9.5px var(--fd);
    letter-spacing: 0.06em;
    padding: 0 6px;
    border-radius: var(--radius-sm);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    height: 17px;
    padding: 0 12px;
    border-radius: var(--radius-sm);
    font: 600 9px var(--fb);
    background: var(--button-dark);
    box-shadow: inset 0 0 0 1px var(--rule);
    color: var(--ink);
  }

  .legend {
    position: absolute;
    right: 8px;
    top: 8px;
    font-size: 9px;
    background: var(--pitch-chip);
    color: var(--ink);
    padding: 2px 8px;
    border-radius: var(--radius-sm);
  }

  .cond {
    position: absolute;
    left: 8px;
    bottom: 8px;
    font: 700 9px var(--fd);
    letter-spacing: 0.05em;
    background: var(--pitch-chip);
    color: var(--ink);
    padding: 2px 7px;
    border-radius: var(--radius-sm);
  }

  /* The goal banner comes in, holds and leaves within 1.5 s, on transform and opacity. */
  .banner {
    position: absolute;
    left: 50%;
    top: 40px;
    transform: translateX(-50%);
    background: var(--cyan);
    color: var(--cyan-ink);
    font: 800 13px var(--fd);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 4px 14px;
    border-radius: var(--radius-sm);
    white-space: nowrap;
    animation: banner calc(var(--goal-in) + var(--goal-hold) + var(--goal-out)) var(--ease) both;
  }

  @keyframes banner {
    0% {
      opacity: 0;
      transform: translate(-50%, -6px);
    }
    17% {
      opacity: 1;
      transform: translate(-50%, 0);
    }
    87% {
      opacity: 1;
      transform: translate(-50%, 0);
    }
    100% {
      opacity: 0;
      transform: translate(-50%, 0);
    }
  }

  /* The reduced-motion setting, or the system's preference when it follows Windows. */
  :global(:root[data-motion='reduce']) .banner {
      animation: none;
  }
</style>
