<!-- The match timeline: a range over every tick received, named "Rewind to a tick", with
     the cyan playhead at the rendered tick and the minute marks under it. Dragging rewinds and
     holds the frame; letting go resumes whatever was playing. The skip flow later hatches the
     part a skip plays without watching. -->
<script>
  import { TICKS_PER_SECOND } from '../lib/schedule.js';

  let { tick = 0, max = 1, disabled = false, onscrub = () => {}, onrelease = () => {} } = $props();

  const MINUTE = TICKS_PER_SECOND * 60;
  const MARKS = [0, 15, 30, 45, 60, 75, 90];

  let share = $derived(max > 0 ? Math.min(1, Math.max(0, tick / max)) : 0);
  let marks = $derived(
    MARKS.filter((m) => m * MINUTE <= max).map((m) => ({ m, at: (m * MINUTE) / max }))
  );
</script>

<div class="timeline">
  <div class="track" role="presentation">
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
    aria-valuetext="Minute {Math.floor(tick / MINUTE)}"
    oninput={(e) => onscrub(Number(e.currentTarget.value))}
    onchange={() => onrelease()}
  />
  <div class="marks" aria-hidden="true">
    {#each marks as mark (mark.m)}
      <span style:left="{mark.at * 100}%">{mark.m}'</span>
    {/each}
  </div>
</div>

<style>
  .timeline {
    position: relative;
    width: 742px;
    height: 26px;
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

  .head {
    position: absolute;
    top: -4px;
    width: 2px;
    height: 12px;
    margin-left: -1px;
    background: var(--cyan);
  }

  input {
    position: absolute;
    inset: 0 0 12px;
    width: 100%;
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
</style>
