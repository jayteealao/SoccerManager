<!-- The one-row substitution picker of the sketch's in-match Tactics board: Coming off and
     Coming on as native selects, Queue substitution, the count left in words, and the note
     that the engine applies it at the next stoppage that allows one. When nothing can be
     queued the button is disabled and the note gives the reason. While a queued substitution
     is edited, the button replaces it. The Touchline draws it `compact`, as the sketch's
     one-row picker with Queue sub, and adds New shape: a formation to queue with the
     substitution, "Keep" the current one first. -->
<script>
  import SectionLabel from './SectionLabel.svelte';
  import { remainingText } from '../lib/substitution-picker.js';

  let { picker, editing = false, compact = false, shapes = null, onqueue = () => {} } = $props();

  let off = $state('');
  let on = $state('');
  let shape = $state('keep');

  let offValue = $derived(
    picker.off.some((p) => String(p.squad) === off) ? off : String(picker.off[0]?.squad ?? '')
  );
  let onValue = $derived(
    picker.on.some((p) => String(p.squad) === on) ? on : String(picker.on[0]?.squad ?? '')
  );

  function queue() {
    if (picker.block || offValue === '' || onValue === '') {
      return;
    }
    const formation = shapes && shape !== 'keep' ? Number(shape) : null;
    onqueue(Number(offValue), Number(onValue), formation);
    shape = 'keep';
  }
</script>

{#if !compact}<SectionLabel label="Substitution" note={remainingText(picker.left, picker.limit)} />{/if}
<div class="pair" class:compact>
  <label>
    Coming off
    <select class="sel" value={offValue} disabled={!!picker.block} onchange={(e) => (off = e.currentTarget.value)}>
      {#each picker.off as p (p.squad)}
        <option value={String(p.squad)}>{p.shirt} {p.name} · {p.position}</option>
      {/each}
    </select>
  </label>
  <label>
    Coming on
    <select class="sel" value={onValue} disabled={!!picker.block} onchange={(e) => (on = e.currentTarget.value)}>
      {#each picker.on as p (p.squad)}
        <option value={String(p.squad)}>{p.shirt} {p.name} · {p.position}</option>
      {/each}
    </select>
  </label>
  {#if compact}
    <button class="btn cy" type="button" disabled={!!picker.block} onclick={queue}>
      {editing ? 'Replace sub' : 'Queue sub'}
    </button>
  {/if}
</div>
{#if shapes}
  <label class="shape">
    New shape
    <select class="sel" value={shape} disabled={!!picker.block} onchange={(e) => (shape = e.currentTarget.value)}>
      <option value="keep">Keep {shapes.current}</option>
      {#each shapes.list as f (f.value)}
        <option value={String(f.value)}>{f.name}</option>
      {/each}
    </select>
  </label>
{/if}
{#if compact}
  <p class="g">{picker.block ?? remainingText(picker.left, picker.limit)}</p>
{:else}
  <div class="go">
    <button class="btn cy" type="button" disabled={!!picker.block} onclick={queue}>
      {editing ? 'Replace substitution' : 'Queue substitution'}
    </button>
    <span class="g">
      {picker.block ?? 'The engine applies it at the next stoppage that allows one, or refuses it with a reason.'}
    </span>
  </div>
{/if}

<style>
  .pair {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-top: -4px;
  }

  .pair.compact {
    grid-template-columns: 1fr 1fr auto;
    align-items: end;
    margin-top: 0;
  }

  .shape {
    margin-top: 6px;
  }

  p.g {
    margin: 5px 0 0;
  }

  label {
    display: grid;
    gap: 3px;
    font: 600 9px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-3);
  }

  .sel {
    height: 24px;
    min-width: 0;
    padding: 0 4px 0 8px;
    background: var(--select-ground);
    border: 1px solid var(--select-edge);
    border-radius: var(--radius-sm);
    color: var(--ink);
    font: 500 11px var(--fb);
  }

  .go {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    height: 24px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-sm);
    font: 700 10px var(--fb);
    white-space: nowrap;
    flex: none;
    cursor: pointer;
  }

  .btn.cy {
    background: var(--cyan);
    color: var(--cyan-ink);
  }

  .btn:hover:not(:disabled) {
    filter: brightness(1.12);
  }

  .btn:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .g {
    font-size: 9.5px;
    color: var(--ink-3);
  }
</style>
