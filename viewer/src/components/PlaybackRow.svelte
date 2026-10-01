<!-- The playback row under the pitch: round 24 px icon buttons (the one in effect filled
     cyan) with 40 px hit areas, Next stop, the speed segment 1× to 8×, and the ghost Skip to
     result, which opens the Skip decision. Skip to result is live only on a kicked-off live
     engine match before full time; otherwise it is disabled and faded. The row is one group
     named "Playback"; every control is disabled while there is nothing to play. -->
<script>
  import Glyph from './Glyph.svelte';
  import { FAST_FORWARD, PAUSE, PLAY, REWIND } from './icons.js';
  import { SPEEDS } from '../lib/playback.js';

  let {
    playing = false,
    speed = 1,
    disabled = false,
    onprevious = () => {},
    ontoggle = () => {},
    onlive = () => {},
    onnext = () => {},
    onspeed = () => {},
    canSkip = false,
    onskip = () => {},
  } = $props();
</script>

<div class="row" class:off={disabled} role="group" aria-label="Playback">
  <button class="ib" type="button" aria-label="Previous stop" {disabled} onclick={onprevious}>
    <Glyph glyph={{ d: REWIND }} size={11} />
  </button>
  <button
    class="ib"
    class:on={playing}
    type="button"
    aria-label={playing ? 'Pause' : 'Play'}
    {disabled}
    onclick={ontoggle}
  >
    <Glyph glyph={{ d: playing ? PAUSE : PLAY }} size={11} />
  </button>
  <button class="ib" type="button" aria-label="Back to live" {disabled} onclick={onlive}>
    <Glyph glyph={{ d: FAST_FORWARD }} size={11} />
  </button>
  <button class="btn dk" type="button" {disabled} onclick={onnext}>
    <Glyph glyph={{ d: FAST_FORWARD }} size={10} /> Next stop
  </button>
  <span class="seg" role="group" aria-label="Speed">
    {#each SPEEDS as s (s)}
      <button
        type="button"
        class:on={s === speed}
        aria-pressed={s === speed}
        aria-label="{s}x"
        {disabled}
        onclick={() => onspeed(s)}>{s}×</button
      >
    {/each}
  </span>
  <button class="btn gh skip" type="button" aria-label="Skip to result" disabled={disabled || !canSkip} onclick={onskip}>
    <Glyph glyph={{ d: FAST_FORWARD }} size={10} /> Skip to result
  </button>
  <span class="gap"></span>
  <span class="note">Rewind works while paused · back to live</span>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
  }

  .row.off {
    opacity: 0.45;
  }

  .ib {
    position: relative;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: inline-grid;
    place-items: center;
    background: transparent;
    border: 1px solid var(--control-edge);
    color: var(--control-ink);
    cursor: pointer;
    padding: 0;
    transition:
      background-color var(--tab-change) var(--ease),
      filter var(--tab-change) var(--ease);
  }

  /* The drawn button is 24 px; the hit area is 40 px tall, and 30 px wide so that the
   * areas of neighbouring buttons, 6 px apart, never overlap. */
  .ib::after {
    content: '';
    position: absolute;
    inset: -8px -3px;
  }

  .ib.on {
    background: var(--cyan);
    color: var(--cyan-ink);
    border-color: var(--cyan);
  }

  .btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-sm);
    font: 600 10px var(--fb);
    cursor: pointer;
    white-space: nowrap;
  }

  .btn::after {
    content: '';
    position: absolute;
    inset: -1px 0;
  }

  .dk {
    background: var(--button-dark);
    box-shadow: inset 0 0 0 1px var(--rule);
    color: var(--ink);
  }

  .gh {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--control-edge);
    color: var(--ghost-ink);
  }

  .seg {
    display: inline-flex;
    border: 1px solid var(--seg-edge);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .seg button {
    position: relative;
    padding: 0 8px;
    height: 20px;
    display: inline-flex;
    align-items: center;
    font: 400 9.5px var(--fb);
    color: var(--seg-ink);
    background: transparent;
    border: 0;
    border-left: 1px solid var(--seg-edge);
    white-space: nowrap;
    cursor: pointer;
  }

  .seg button:first-child {
    border-left: 0;
  }

  .seg button.on {
    background: var(--cyan);
    color: var(--cyan-ink);
    font-weight: 700;
  }

  /* Focus inside the segment is drawn inside the part, which the segment's edge would clip. */
  .seg button:focus-visible {
    outline-offset: -3px;
    outline-color: var(--cyan-ink);
  }

  .seg button:not(.on):focus-visible {
    outline-color: var(--cyan);
  }

  .ib:hover:not(:disabled),
  .btn:hover:not(:disabled),
  .seg button:hover:not(:disabled) {
    filter: brightness(1.12);
  }

  .ib:active:not(:disabled),
  .btn:active:not(:disabled),
  .seg button:active:not(:disabled) {
    filter: brightness(0.92);
  }

  button:disabled {
    cursor: default;
  }

  /* Skip to result has nothing to do before kick-off, after full time or on a replay: it
   * keeps the faded look the row drew before the skip flow was built. */
  .skip:disabled {
    opacity: var(--stub-opacity);
  }

  /* The drawn button is 22 px; the hit area is 40 px tall. */
  .skip::after {
    inset: -9px 0;
  }

  .gap {
    flex: 1;
  }

  .note {
    font-size: 9.5px;
    color: var(--ink-3);
    white-space: nowrap;
  }
</style>
