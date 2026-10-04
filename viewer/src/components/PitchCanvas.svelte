<!-- The pitch box: the canvas the pitch renderer draws the match into, the goal banner over it,
     and the sketch's pitch overlays, which are stubs. A panel passed as the body (the loading
     steps, an error, first run) covers the box; `dim` holds the last frame at half strength
     while the engine reconnects.

     The box fills its column at the aspect `width` by `height` (742 by 312 on the match
     screen; the replay's is 752 by 290), up to `maxWidth` when a screen sets one. A
     ResizeObserver on the box is the one layout the viewer measures in script: each change
     reports the box in CSS pixels and in device pixels through `resize`, so the renderer
     draws sharp at its size. The device pixels come from devicePixelContentBoxSize where the
     browser has it (Chrome, Edge, Firefox); otherwise from the CSS size times the device
     pixel ratio times the page's zoom (Safari). The canvas also names its box in data-width
     and data-height. The renderer owns the canvas's backing store: its width and height
     attributes are set here once, because setting them again would clear a held frame. -->
<script>
  import StubSection from './StubSection.svelte';

  let {
    attach = () => {},
    detach = () => {},
    width = 742,
    height = 312,
    maxWidth = null,
    resize = () => {},
    drawn = false,
    dim = false,
    banner = null,
    overlays = true,
    children,
  } = $props();

  let canvas = $state();
  let box = $state();
  let measured = $state(null);

  $effect(() => {
    if (canvas) {
      const drawnOn = canvas;
      attach(drawnOn);
      return () => detach(drawnOn);
    }
  });

  /// The box an observer entry reports: CSS pixels and device pixels, whole numbers.
  function sizes(entry, el) {
    const css = entry.contentBoxSize?.[0];
    const cssWidth = css ? css.inlineSize : entry.contentRect.width;
    const cssHeight = css ? css.blockSize : entry.contentRect.height;
    const device = entry.devicePixelContentBoxSize?.[0];
    const ratio = (globalThis.devicePixelRatio || 1) * (el.currentCSSZoom ?? 1);
    // The browser's device-pixel box is snapped to the screen's pixels, within a pixel of the
    // CSS box times the ratio; an emulated scale (a test's device scale factor) reports it in
    // CSS pixels instead, so a size far from the product is not taken.
    const snapped = (reported, cssSize) =>
      reported !== undefined && Math.abs(reported - cssSize * ratio) <= 1 ? reported : Math.round(cssSize * ratio);
    return {
      width: cssWidth,
      height: cssHeight,
      deviceWidth: snapped(device?.inlineSize, cssWidth),
      deviceHeight: snapped(device?.blockSize, cssHeight),
    };
  }

  $effect(() => {
    if (!box || !globalThis.ResizeObserver) {
      return;
    }
    const el = box;
    // The observer reports at most once a frame, after layout and before paint, so the
    // canvas redraws at its new size in the same frame.
    const observer = new ResizeObserver(([entry]) => {
      const next = sizes(entry, el);
      if (next.width > 0 && next.height > 0) {
        measured = next;
        resize(canvas, next);
      }
    });
    try {
      observer.observe(el, { box: 'device-pixel-content-box' });
    } catch {
      observer.observe(el);
    }
    return () => observer.disconnect();
  });
</script>

<div
  class="pitchbox"
  class:dim
  class:blank={!drawn}
  bind:this={box}
  style:aspect-ratio="{width} / {height}"
  style:max-width={maxWidth}
>
  <canvas
    bind:this={canvas}
    {width}
    {height}
    data-width={measured?.width ?? width}
    data-height={measured?.height ?? height}
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
    width: 100%;
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
    width: 100%;
    height: 100%;
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

  /* The reduced-motion setting, or the system's preference when it follows the system. */
  :global(:root[data-motion='reduce']) .banner {
      animation: none;
  }
</style>
