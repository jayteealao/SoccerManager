<!-- The match timeline: a range over every tick received, named "Rewind to a tick", with
     the cyan playhead at the rendered tick and the minute marks under it. Dragging rewinds and
     holds the frame; letting go resumes whatever was playing. After a skip (`skippedFrom`) the
     track from the skip point to the end is hatched, marked NOT WATCHED LIVE, and the
     slider's value text says when the tick is inside that part. With no skip the markup is
     as before. -->
<script>
  import { TICKS_PER_SECOND } from '../lib/schedule.js';

  let {
    tick = 0,
    max = 1,
    disabled = false,
    skippedFrom = null,
    onscrub = () => {},
    onrelease = () => {},
  } = $props();

  const MINUTE = TICKS_PER_SECOND * 60;
  const MARKS = [0, 15, 30, 45, 60, 75, 90];

  let share = $derived(max > 0 ? Math.min(1, Math.max(0, tick / max)) : 0);
  // The track fills its column. The NOT WATCHED LIVE words after the skip point take about
  // 115 px; the minute marks under them are left out, measured against the narrowest track
  // (about 580 px, at 1024 px wide), so the words never cover a mark at any width.
  const UNSEEN_SHARE = 115 / 580;

  let marks = $derived(
    MARKS.filter((m) => m * MINUTE <= max)
      .map((m) => ({ m, at: (m * MINUTE) / max }))
      .filter(
        (mark) =>
          skippedFrom === null ||
          !(max > 0) ||
          mark.at < skippedFrom / max ||
          mark.at > skippedFrom / max + UNSEEN_SHARE
      )
  );
  let skipAt = $derived(
    skippedFrom === null || !(max > 0) ? null : Math.min(1, Math.max(0, skippedFrom / max))
  );
  let valueText = $derived(
    `Minute ${Math.floor(tick / MINUTE)}${skippedFrom !== null && tick > skippedFrom ? ', not watched live' : ''}`
  );
</script>

<div class="timeline">
  <div class="track" role="presentation">
    {#if skipAt !== null}
      <i class="hatch" style:left="{skipAt * 100}%"></i>
    {/if}
    <i class="played" style:width="{share * 100}%"></i>
    <i class="head" style:left="{share * 100}%"></i>
  </div>
  <input
    type="range"
    min="0"
    {max}
    step="1"
    value={tick}
    {disabled}
    aria-label="Rewind to a tick"
    aria-valuetext={valueText}
    oninput={(e) => onscrub(Number(e.currentTarget.value))}
    onchange={() => onrelease()}
  />
  <div class="marks" aria-hidden="true">
    {#each marks as mark (mark.m)}
      <span style:left="{mark.at * 100}%">{mark.m}'</span>
    {/each}
    {#if skipAt !== null}
      <b class="unseen" style:left="{skipAt * 100}%">NOT WATCHED LIVE</b>
    {/if}
  </div>
</div>

<style>
  .timeline {
    position: relative;
    width: 100%;
    height: 28px;
    margin-top: 8px;
  }

  .track {
    position: absolute;
    left: 0;
    right: 0;
    top: 6px;
    height: 4px;
    background: var(--bar-track);
    border-radius: 1px;
  }

  .played {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    background: var(--navy-500);
  }

  /* The part played after a skip: the board's 45° stripes, 4 px of each hatch colour. */
  .hatch {
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    background: repeating-linear-gradient(
      45deg,
      var(--hatch-1) 0 2.83px,
      var(--hatch-2) 2.83px 5.66px
    );
  }

  .head {
    position: absolute;
    top: -4px;
    width: 2px;
    height: 12px;
    margin-left: -1px;
    background: var(--cyan);
  }

  /* The slider is invisible over the drawn track; its hit area is 24 px high, from the
     playback row above the track to 1 px above the minute marks. */
  input {
    position: absolute;
    inset: -8px 0 12px;
    width: 100%;
    height: auto;
    margin: 0;
    appearance: none;
    background: transparent;
    cursor: pointer;
  }

  input::-webkit-slider-runnable-track {
    height: 14px;
    background: transparent;
  }

  input::-webkit-slider-thumb {
    appearance: none;
    width: 12px;
    height: 14px;
    background: transparent;
  }

  input:disabled {
    cursor: default;
  }

  .marks {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 11px;
  }

  .marks span {
    position: absolute;
    transform: translateX(-50%);
    font: 400 9px/11px var(--fb);
    color: var(--ink-3);
    white-space: nowrap;
  }

  .marks span:first-child {
    transform: none;
  }

  .unseen {
    position: absolute;
    margin-left: 14px;
    font: 700 8.5px/11px var(--fd);
    letter-spacing: 0.06em;
    color: var(--ink-2);
    white-space: nowrap;
  }

  /* Compact: the slider's hit area is 44 px high, with the track at its middle and the minute
     marks under it. */
  @media (max-width: 1023px), (max-height: 599px) {
    .timeline {
      height: 56px;
    }

    .track {
      top: 20px;
    }

    input {
      inset: 0 0 12px;
    }
  }
</style>
