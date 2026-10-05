<!-- An upright tactics pitch, drawn 300 by 430 and filling its column at that aspect, own goal
     at the bottom, as the sketch's Tactics board draws both shapes: the striped turf, the
     markings, and a ringed marker per slot with the
     player's name and role tag under it. With `interactive`, each marker is a button named
     by its slot, player, role fit and fitness: a click picks it, Delete empties it, and a
     squad row dropped on it takes the slot. Otherwise the markers are drawn only. -->
<script>
  let {
    markers = [],
    interactive = false,
    label = '',
    onpick = () => {},
    onempty = () => {},
    ondrop = () => {},
  } = $props();

  const W = 300;
  const H = 430;
  const STRIPES = Array.from({ length: 12 }, (_, i) => i);
  const STRIPE = H / 12;

  function keydown(event, n) {
    if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault();
      onempty(n);
    }
  }

  function dragover(event) {
    event.preventDefault();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'move';
    }
  }

  function drop(event, n) {
    event.preventDefault();
    const from = Number(event.dataTransfer?.getData('text/plain'));
    if (Number.isInteger(from)) {
      ondrop(from, n);
    }
  }
</script>

<div class="pitch" role={interactive ? 'group' : 'img'} aria-label={label}>
  <svg viewBox="0 0 {W} {H}" aria-hidden="true" focusable="false">
    {#each STRIPES as i (i)}
      <rect class={i % 2 === 0 ? 'stripe' : 'stripe-2'} x="0" y={i * STRIPE} width={W} height={STRIPE + 0.6} />
    {/each}
    <g class="lines">
      <rect x="1" y="1" width={W - 2} height={H - 2} />
      <line x1="0" y1={H / 2} x2={W} y2={H / 2} />
      <circle cx={W / 2} cy={H / 2} r="39" />
      <rect x="63" y="1" width="174" height="64.5" />
      <rect x="63" y={H - 65.5} width="174" height="64.5" />
      <rect x="108" y="1" width="84" height="23.65" />
      <rect x="108" y={H - 24.65} width="84" height="23.65" />
    </g>
    <circle class="spot" cx={W / 2} cy={H / 2} r="2" />
  </svg>

  {#each markers as m (m.key)}
    {#if interactive}
      <button
        type="button"
        class="tk"
        class:picked={m.picked}
        class:empty={m.empty}
        style:left="{m.left}%"
        style:top="{m.top}%"
        aria-label={m.label}
        aria-pressed={m.picked}
        data-slot={m.n}
        onclick={() => onpick(m.n)}
        onkeydown={(e) => keydown(e, m.n)}
        ondragover={dragover}
        ondrop={(e) => drop(e, m.n)}
      >
        <span class="d" class:keeper={m.keeper}>{m.empty ? '' : m.shirt}</span>
        <span class="c">
          {#if m.empty}{m.position}{:else}{m.name}{#if m.tag} <em style:color="var({m.token})">{m.tag}</em>{/if}{/if}
        </span>
      </button>
    {:else}
      <span class="tk" style:left="{m.left}%" style:top="{m.top}%">
        <span class="d" class:keeper={m.keeper}>{m.shirt ?? ''}</span>
        {#if m.name || m.tag}
          <span class="c">{m.name}{#if m.tag} <em style:color="var({m.token})">{m.tag}</em>{/if}</span>
        {/if}
      </span>
    {/if}
  {/each}
</div>

<style>
  .pitch {
    position: relative;
    width: 100%;
    aspect-ratio: 300 / 430;
    border-radius: var(--radius-md);
    overflow: hidden;
    box-shadow: 0 0 0 2px var(--pitch-deep);
  }

  svg {
    display: block;
    width: 100%;
    height: 100%;
  }

  .stripe {
    fill: var(--pitch-stripe);
  }

  .stripe-2 {
    fill: var(--pitch-stripe-2);
  }

  .lines {
    fill: none;
    stroke: var(--pitch-line);
    stroke-opacity: 0.55;
    stroke-width: 1.4;
  }

  .spot {
    fill: var(--pitch-line);
    fill-opacity: 0.55;
  }

  .tk {
    position: absolute;
    transform: translate(-50%, -50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    color: var(--pitch-ink);
    border-radius: var(--radius-sm);
  }

  button.tk {
    cursor: pointer;
  }

  /* The marker is drawn 19 px wide with its caption under it; its hit area is the step's
     hit size around the ring. */
  button.tk::after {
    content: '';
    position: absolute;
    left: 50%;
    top: 50%;
    width: max(100%, var(--hit));
    height: max(100%, var(--hit));
    transform: translate(-50%, -50%);
  }

  /* The pitch green takes a light ring: cyan measures under 3:1 on the turf. */
  button.tk:focus-visible {
    outline: 2px solid var(--pitch-line);
    outline-offset: 2px;
  }

  .d {
    width: 19px;
    height: 19px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font: 800 9px/1 var(--fd);
    border: 1.5px solid var(--pitch-line);
    box-shadow: 0 1px 3px var(--marker-shadow);
    background: var(--tactic-marker);
    color: var(--tactic-marker-ink);
  }

  .d.keeper {
    background: var(--keeper-home);
    color: var(--keeper-home-ink);
  }

  .picked .d {
    box-shadow:
      0 0 0 2px var(--pitch-line),
      0 0 0 4px var(--cyan);
  }

  .empty .d {
    background: transparent;
    border-style: dashed;
  }

  .c {
    margin-top: 1px;
    background: var(--chip);
    border-radius: var(--radius-sm);
    padding: 0 4px;
    font: 600 8.5px/13px var(--fd);
    white-space: nowrap;
    color: var(--pitch-ink);
  }

  .c em {
    font-style: normal;
    margin-left: 3px;
  }
</style>
