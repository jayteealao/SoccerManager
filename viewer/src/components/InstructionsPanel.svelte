<!-- The two cards under the pitches, as the sketch's Tactics board draws them: Limits (where
     players may go) and Weights (how often an option is chosen). The team instructions are
     live selects in their card; the per-player limits and the weight bars are LATER stubs,
     because the engine has no per-player limit or weight model yet. -->
<script>
  import StubSection from './StubSection.svelte';
  import { instructionRows } from '../lib/tactics-board.js';

  let { schema, tactics, disabled = false, onedit = () => {} } = $props();

  let limits = $derived(schema ? instructionRows(schema, tactics?.instructions, 'limits') : []);
  let weights = $derived(schema ? instructionRows(schema, tactics?.instructions, 'weights') : []);
</script>

{#snippet rows(list)}
  {#each list as row (row.name)}
    <label class="kv">
      <span>{row.label}</span>
      <select
        class="sel"
        value={String(row.value)}
        {disabled}
        data-instruction={row.name}
        onchange={(e) => onedit({ instruction: row.index, level: Number(e.currentTarget.value) })}
      >
        {#each row.levels as level (level.value)}
          <option value={String(level.value)}>{level.label}</option>
        {/each}
      </select>
    </label>
  {/each}
{/snippet}

<div class="cards">
  <section class="tcard" aria-labelledby="limits-title">
    <h4 id="limits-title">Limits · always held</h4>
    <p>Where players may go. A habit can break a limit only under pressure or low mood.</p>
    {@render rows(limits)}
    <!-- STUB: per-player limits need a limit model the engine does not have yet. -->
    <StubSection note="per-player limits" later>
      <div class="kv"><span>Per-player limits</span><b>—</b></div>
    </StubSection>
  </section>
  <section class="tcard" aria-labelledby="weights-title">
    <h4 id="weights-title">Weights · players lean</h4>
    <p>How often an option is chosen. Decisions and teamwork set how faithfully each player follows.</p>
    {@render rows(weights)}
    <!-- STUB: the weight bars need a weight model the engine does not have yet. -->
    <StubSection note="weight bars" later>
      <div class="wbar"><span>Weight bars</span><i class="track"><i class="fill"></i></i></div>
    </StubSection>
  </section>
</div>

<style>
  .cards {
    position: absolute;
    left: 12px;
    right: 12px;
    top: 508px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .tcard {
    background: var(--pitch-card);
    border: 1px solid var(--pitch-card-edge);
    border-radius: var(--radius-md);
    padding: 9px 10px 8px;
  }

  h4 {
    margin: 0 0 5px;
    font: 700 9.5px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--pitch-ink);
  }

  p {
    margin: 0 0 2px;
    color: var(--pitch-card-ink);
    font-size: 9.5px;
    line-height: 1.45;
  }

  .kv {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    padding: 1px 0;
    min-height: 22px;
    color: var(--pitch-card-ink);
  }

  .kv b {
    color: var(--pitch-ink);
    font-weight: 600;
  }

  .sel {
    height: 20px;
    min-width: 96px;
    padding: 0 4px 0 6px;
    background: var(--select-ground);
    border: 1px solid var(--select-edge);
    border-radius: var(--radius-sm);
    color: var(--select-ink);
    font: 500 9.5px var(--fb);
  }

  .wbar {
    display: grid;
    grid-template-columns: 96px 1fr;
    gap: 6px;
    align-items: center;
    height: 17px;
    color: var(--pitch-card-ink);
  }

  .track {
    display: block;
    height: 5px;
    background: var(--pitch-deep);
    border-radius: 1px;
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    width: 50%;
    background: var(--weight-bar);
  }
</style>
