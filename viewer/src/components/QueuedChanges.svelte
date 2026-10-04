<!-- The queued-changes block of the sketch's in-match Tactics board: each change the engine
     acknowledged, with its state word as a tag (Queued, Applies now, Applied, Rejected), its
     words, and the engine's reason under a refused one. A queued change has Edit and Cancel;
     a refused one has Dismiss. The tag's word always carries the state, never colour alone. -->
<script>
  import SectionLabel from './SectionLabel.svelte';

  let {
    chips = [],
    editing = null,
    cancelRefused = null,
    live = false,
    onedit = () => {},
    onstopedit = () => {},
    oncancel = () => {},
    ondismiss = () => {},
  } = $props();

  let waiting = $derived(chips.filter((c) => c.state === 'queued' || c.state === 'applies-now').length);
  const TAG = { queued: 'queued', 'applies-now': 'now', applied: 'applied', rejected: 'rejected' };
</script>

<section class="q" aria-label="Queued changes">
  <SectionLabel label="Queued · waits for a stoppage" note="{waiting} {waiting === 1 ? 'change' : 'changes'} waiting" />
  <p class="lead">Applies at the next stoppage that allows it. Nothing changes on the pitch until then.</p>
  {#if cancelRefused}
    <p class="refused" role="status">The engine kept the change: {cancelRefused}</p>
  {/if}
  {#if chips.length === 0}
    <p class="none">Nothing queued.</p>
  {/if}
  <ul>
    {#each chips as chip (chip.queue_id)}
      {@const edited = editing?.queue_id === chip.queue_id}
      <li data-queue-id={chip.queue_id} data-state={chip.state}>
        <span class="tagc {TAG[chip.state]}">{chip.word}</span>
        <div class="what">
          <b>{chip.label}</b>
          {#if chip.reason}
            <div class="g">The engine refused it: {chip.reason}</div>
          {:else if edited}
            <div class="g">Editing: the next {chip.kind === 'substitution' ? 'substitution' : 'tactics change'} replaces it.</div>
          {/if}
        </div>
        <span class="acts">
          {#if chip.editable && live}
            {#if edited}
              <button class="btn gh" type="button" onclick={onstopedit}>Stop editing</button>
            {:else}
              <button class="btn gh" type="button" aria-label="Edit: {chip.label}" onclick={() => onedit(chip.queue_id)}>Edit</button>
            {/if}
            <button class="btn gh" type="button" aria-label="Cancel: {chip.label}" onclick={() => oncancel(chip.queue_id)}>Cancel</button>
          {:else if chip.state === 'rejected'}
            <button class="btn gh" type="button" aria-label="Dismiss: {chip.label}" onclick={() => ondismiss(chip.queue_id)}>Dismiss</button>
          {/if}
        </span>
      </li>
    {/each}
  </ul>
</section>

<style>
  .q {
    border: 1.5px solid var(--cyan);
    background: var(--queue-ground);
    border-radius: var(--radius-sm);
    padding: 9px 10px;
    color: var(--ink);
  }

  .q :global(.sl) {
    margin-bottom: 6px;
  }

  .lead {
    margin: 0 0 4px;
    font-size: 10px;
    color: var(--ink-2);
  }

  .refused {
    margin: 0 0 4px;
    font-size: 10px;
    color: var(--ink);
  }

  .none {
    margin: 0;
    padding: 5px 0;
    border-top: 1px solid var(--queue-rule);
    font-size: 10px;
    color: var(--ink-3);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: grid;
    grid-template-columns: 84px 1fr auto;
    gap: 8px;
    align-items: center;
    padding: 5px 0;
    border-top: 1px solid var(--queue-rule);
  }

  .tagc {
    justify-self: start;
    display: inline-block;
    font: 700 9.5px var(--fd);
    letter-spacing: 0.06em;
    padding: 0 6px;
    border-radius: var(--radius-sm);
    white-space: nowrap;
  }

  .tagc.now {
    background: var(--cyan);
    color: var(--cyan-ink);
  }

  .tagc.queued {
    background: var(--ground-2);
    color: var(--ink);
  }

  .tagc.applied {
    background: var(--good);
    color: var(--on-good);
  }

  .tagc.rejected {
    background: var(--bad);
    color: var(--on-bad);
  }

  .what {
    min-width: 0;
  }

  .what b {
    font-weight: 600;
  }

  .g {
    font-size: 9.5px;
    color: var(--ink-3);
  }

  .acts {
    display: inline-flex;
    gap: 4px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    height: var(--hit);
    min-width: var(--hit);
    justify-content: center;
    padding: 0 7px;
    border: 0;
    border-radius: var(--radius-sm);
    font: 600 10px var(--fb);
    cursor: pointer;
    white-space: nowrap;
  }

  .btn.gh {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--control-edge);
    color: var(--ghost-ink);
  }

  .btn:hover {
    filter: brightness(1.12);
  }
</style>
