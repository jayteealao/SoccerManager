<!-- The tactic bar across the top of the pitch panel, as the sketch's Tactics board draws it:
     the TACTIC word, the shape and mentality selects, the base-shape line, and at the right
     the lineup's verdict before kick-off (READY, or NOT READY with the reason, in a status
     region) or, during play, team familiarity (a LATER stub: no engine model yet). -->
<script>
  import StubSection from './StubSection.svelte';
  import { shapeLine } from '../lib/tactics-board.js';
  import { words } from '../lib/tactics-panel.js';

  let {
    schema,
    tactics,
    preMatch = false,
    live = false,
    ready = false,
    reason = null,
    onformation = () => {},
    onmentality = () => {},
  } = $props();

  let formations = $derived(schema?.formations ?? []);
  let mentalities = $derived(schema?.mentalities ?? []);
</script>

<div class="bar">
  <b class="word">TACTIC</b>
  <label class="field">
    <span>Shape</span>
    <select
      class="sel"
      value={String(tactics?.formation ?? 0)}
      disabled={!preMatch}
      onchange={(e) => onformation(Number(e.currentTarget.value))}
    >
      {#each formations as f, i (i)}
        <option value={String(i)}>{f.name}</option>
      {/each}
    </select>
  </label>
  <label class="field">
    <span>Mentality</span>
    <select
      class="sel"
      value={String(tactics?.mentality ?? 0)}
      disabled={!preMatch && !live}
      onchange={(e) => onmentality(Number(e.currentTarget.value))}
    >
      {#each mentalities as m, i (i)}
        <option value={String(i)}>{words(m.name)}</option>
      {/each}
    </select>
  </label>
  <span class="line">{shapeLine(schema, tactics)}</span>
  <span class="gap"></span>
  {#if preMatch}
    <div class="verdict" role="status">
      {#if ready}
        <span class="tagc good">READY</span><span class="why">The lineup is legal.</span>
      {:else}
        <span class="tagc bad">NOT READY</span><span class="why">{reason}</span>
      {/if}
    </div>
  {:else}
    <!-- STUB: team familiarity needs a familiarity model the engine does not have yet. -->
    <StubSection note="team familiarity" inline later>
      <span class="line">Team familiarity</span><b class="num fam">—</b>
    </StubSection>
  {/if}
</div>

<style>
  .bar {
    position: absolute;
    left: 12px;
    right: 12px;
    top: 9px;
    height: 28px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 8px;
    border-radius: var(--radius-md);
    background: var(--pitch-card);
    color: var(--pitch-ink);
  }

  .word {
    font: 700 12px var(--fd);
    letter-spacing: 0.06em;
  }

  .field {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font: 600 9.5px var(--fd);
    color: var(--pitch-card-ink);
  }

  .sel {
    height: 20px;
    padding: 0 4px 0 6px;
    background: var(--pitch-select);
    border: 1px solid var(--pitch-deep);
    border-radius: var(--radius-sm);
    color: var(--pitch-ink);
    font: 500 10px var(--fb);
  }

  .sel:disabled {
    opacity: 0.8;
  }

  .line {
    font: 600 9.5px var(--fd);
    color: var(--pitch-card-ink);
    white-space: nowrap;
  }

  .gap {
    margin-left: auto;
  }

  .verdict {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .tagc {
    display: inline-block;
    font: 700 9.5px var(--fd);
    letter-spacing: 0.06em;
    padding: 0 6px;
    border-radius: var(--radius-sm);
    white-space: nowrap;
  }

  .tagc.bad {
    background: var(--bad);
    color: var(--on-bad);
  }

  .tagc.good {
    background: var(--good);
    color: var(--on-good);
  }

  .why {
    font-size: 10.5px;
    color: var(--pitch-ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .fam {
    font: 800 15px var(--fd);
  }
</style>
