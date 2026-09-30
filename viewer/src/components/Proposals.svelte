<!-- The assistant's proposals on the Touchline, ported from the sketch: each pick the assistant
     has open at the rendered tick, numbered, with its change and its reason in words. Accept
     queues the pick like any other change; an accepted pick reads Queued with a tick. The
     gain and cost of each pick need a model the engine does not have: a LATER stub. With no
     pick open the block says so, and when the check runs. -->
<script>
  import SectionLabel from './SectionLabel.svelte';
  import StubSection from './StubSection.svelte';
  import { NO_PICK } from '../lib/touchline.js';

  let { rows = [], live = false, onaccept = () => {} } = $props();
</script>

<section aria-label="Assistant's proposals">
  <SectionLabel label="Assistant's proposals" note="advice only: nothing changes until you accept" />
  {#if rows.length === 0}
    <p class="none">{NO_PICK}</p>
  {:else}
    <ol>
      {#each rows as row (row.key)}
        <li class="pick" data-pick={row.pick.code}>
          <div class="top">
            <h4>{row.n}. {row.what}</h4>
            {#if row.accepted}
              <span class="queued">Queued ✓</span>
            {:else}
              <button class="btn cy" type="button" disabled={!live} aria-label="Accept: {row.text}" onclick={() => onaccept(row.pick)}
                >Accept</button
              >
            {/if}
          </div>
          <p class="why">Why: {row.reason}</p>
          <!-- STUB: what the pick gains and costs needs a model the engine does not have. -->
          <StubSection note="gain and cost: {row.n}" later>
            <p class="gc"><b class="g">Gain</b> — <b class="c">Cost</b> —</p>
          </StubSection>
        </li>
      {/each}
    </ol>
  {/if}
  <p class="foot">Advice can be wrong. Overruling it is fine.</p>
</section>

<style>
  ol {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }

  .pick {
    background: var(--proposal-ground);
    border: 1px solid var(--rule);
    border-radius: var(--radius-md);
    padding: 8px 10px;
  }

  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  h4 {
    margin: 0;
    font: 700 11.5px var(--fb);
    color: var(--ink);
  }

  .why {
    margin: 3px 0 2px;
    font-size: 10px;
    color: var(--ink-2);
  }

  .gc {
    margin: 0;
    font-size: 10px;
    color: var(--ink-2);
  }

  .g {
    color: var(--good);
  }

  .c {
    color: var(--warn);
    margin-left: 6px;
  }

  .queued {
    font: 700 10px var(--fb);
    color: var(--ink);
    white-space: nowrap;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    height: 22px;
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

  .none,
  .foot {
    margin: 0;
    font-size: 10px;
    line-height: 1.45;
    color: var(--ink-3);
  }

  .foot {
    margin-top: 8px;
  }
</style>
