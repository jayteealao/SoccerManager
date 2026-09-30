<!-- The kick-off pitch of the broadcast line-up graphic (500 × 300): the sketch's striped turf
     with its markings as an inline SVG, and both elevens as 17 px ringed dots at the places the
     engine puts them for the kick-off, each with the shirt number and the surname under it.
     Our dots take our kit colour, theirs their kit colour; the keepers take the keeper colours.
     It is a static drawing: the canvas renderer is not needed for a pitch that never moves. -->
<script>
  let { dots = [[], []], kits = [null, null], shapes = ['', ''], names = ['', ''], width = 500, height = 300 } = $props();

  const STRIPES = 14;
  let stripe = $derived(width / STRIPES);
  let label = $derived(
    `Both elevens at kick-off: ${names[0]} in ${shapes[0] || 'their shape'}, ${names[1]} in ${shapes[1] || 'their shape'}`
  );
</script>

<div class="pitchbox" style:width="{width}px" style:height="{height}px" role="img" aria-label={label}>
  <svg {width} {height} aria-hidden="true" focusable="false">
    {#each { length: STRIPES } as _, i (i)}
      <rect class={i % 2 === 0 ? 'st' : 'st2'} x={i * stripe} y="0" width={stripe + 0.6} {height} />
    {/each}
    <g class="lines">
      <rect x="1" y="1" width={width - 2} height={height - 2} />
      <line x1={width / 2} y1="0" x2={width / 2} y2={height} />
      <circle cx={width / 2} cy={height / 2} r={height * 0.15} />
      <rect x="1" y={height * 0.21} width={width * 0.15} height={height * 0.58} />
      <rect x={width * 0.848} y={height * 0.21} width={width * 0.15} height={height * 0.58} />
      <rect x="1" y={height * 0.36} width={width * 0.055} height={height * 0.28} />
      <rect x={width * 0.943} y={height * 0.36} width={width * 0.055} height={height * 0.28} />
      <path
        d="M{width * 0.152} {height * 0.4}a18 30 0 0 1 0 {height * 0.2}M{width * 0.848} {height * 0.4}a18 30 0 0 0 0 {height * 0.2}"
      />
    </g>
    <circle class="spot" cx={width / 2} cy={height / 2} r="2" />
  </svg>
  {#each dots as side, s (s)}
    {#each side as dot (dot.slot)}
      <span
        class="dotp"
        class:away={s === 1}
        class:keeper={dot.keeper}
        style:left="{dot.left}%"
        style:top="{dot.top}%"
        style:background={dot.keeper ? null : kits[s]?.fill}
        style:color={dot.keeper ? null : kits[s]?.number}>{dot.shirt}</span
      >
      <span class="name" style:left="{dot.left}%" style:top="{dot.top}%">{dot.surname}</span>
    {/each}
  {/each}
</div>

<style>
  .pitchbox {
    position: relative;
    border-radius: var(--radius-md);
    overflow: hidden;
    box-shadow: 0 0 0 2px var(--pitch-deep);
    flex: none;
  }

  svg {
    position: absolute;
    inset: 0;
  }

  .st {
    fill: var(--pitch-stripe);
  }

  .st2 {
    fill: var(--pitch-stripe-2);
  }

  .lines {
    fill: none;
    stroke: var(--pitch-line);
    stroke-opacity: 0.55;
    stroke-width: 1.3;
  }

  .spot {
    fill: var(--pitch-line);
    fill-opacity: 0.55;
  }

  .dotp {
    position: absolute;
    transform: translate(-50%, -50%);
    width: 17px;
    height: 17px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font: 700 8.5px/1 var(--fd);
    font-variant-numeric: tabular-nums;
    border: 1.5px solid var(--pitch-line);
    box-shadow: 0 1px 3px var(--marker-shadow);
    background: var(--tactic-marker);
    color: var(--tactic-marker-ink);
  }

  .dotp.away {
    border-color: var(--ball-ring);
  }

  .dotp.keeper {
    background: var(--keeper-home);
    color: var(--keeper-home-ink);
  }

  .dotp.keeper.away {
    background: var(--keeper-away);
    color: var(--keeper-away-ink);
  }

  .name {
    position: absolute;
    transform: translate(-50%, 10px);
    font: 600 9px var(--fd);
    letter-spacing: 0.02em;
    color: var(--pitch-ink);
    text-shadow: 0 1px 2px var(--label-shadow);
    white-space: nowrap;
  }
</style>
