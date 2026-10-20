<!-- The Plans & opposition tab of the tactics board: the side's game plans (its game-state
     rules), checked top to bottom, the higher one winning a clash. Each plan reads "If <every
     condition> → <every change>", moves up or down, opens in the editor or is deleted; New plan
     is off at the tactics file's limit. A timeline shows when each plan with a time condition
     can switch on. Each edit sends the side's whole game state: before kick-off in the draft,
     during play at once. Opposition instructions, shouts and the other manager are faded stubs. -->
<script>
  import { tick } from 'svelte';
  import GamePlanEditor from './GamePlanEditor.svelte';
  import PlanTimeline from './PlanTimeline.svelte';
  import StubSection from './StubSection.svelte';
  import { fullWords, maxPlans, moved, newCondition, planCount, planRows } from '../lib/game-plans.js';
  import { defaultLevel, levelNames, primitivesOf } from '../lib/primitives.js';

  let { dugout } = $props();

  let schema = $derived(dugout.schema);
  let state = $derived(dugout.gameState);
  let rules = $derived(state.rules ?? []);
  let rows = $derived(planRows(state, schema));
  let full = $derived(rules.length >= maxPlans(schema));
  let disabled = $derived(!dugout.boardEditable);

  /// The plan open in the editor: its index, or -1 for a new plan, or null.
  let open = $state(null);
  /// The plan deleted last, { rule, at }, until Undo puts it back or another edit lands.
  let deleted = $state(null);
  let root = $state(null);

  /// Puts focus, once the list has redrawn, on the button `which` of plan `at` (0-based, or a
  /// function that gives it from the redrawn list), or
  /// on New plan when there is no such plan: focus never falls to the page when the control
  /// that held it goes away.
  async function focusPlan(at, which = 'edit') {
    await tick();
    const i = typeof at === 'function' ? at() : at;
    const button =
      root?.querySelector(`[data-plan-index="${i}"] [data-act="${which}"]:not(:disabled)`) ??
      root?.querySelector(`[data-plan-index="${i}"] [data-act="edit"]`) ??
      root?.querySelector('[data-act="new"]');
    button?.focus();
  }

  function close() {
    const was = open;
    open = null;
    focusPlan(was === -1 ? () => rules.length - 1 : was);
  }

  function send(next, label) {
    dugout.setGameState({ rules: next, instructions: state.instructions ?? [] }, label);
  }

  function fresh() {
    const p = primitivesOf(schema).find((x) => x.name === 'mentality') ?? primitivesOf(schema)[0];
    const names = levelNames(schema, p);
    return {
      when: [newCondition('score', schema), newCondition('time_left', schema)],
      set: [{ primitive: p.name, level: names[Math.min(names.length - 1, defaultLevel(schema, p) + 1)] }],
    };
  }

  function save(rule) {
    if (open === -1) {
      send([...rules, rule], `New plan ${rules.length + 1}`);
    } else {
      send(rules.map((r, i) => (i === open ? rule : r)), `Plan ${open + 1} changed`);
    }
    deleted = null;
    close();
  }

  function remove(i) {
    deleted = { rule: rules[i], at: i };
    send(rules.filter((_, k) => k !== i), `Plan ${i + 1} deleted`);
    if (open === i) {
      open = null;
    }
    focusPlan(() => Math.min(i, rules.length - 1));
  }

  /// Puts the plan deleted last back where it was.
  function undo() {
    if (!deleted || full) {
      return;
    }
    const { rule, at } = deleted;
    deleted = null;
    send([...rules.slice(0, at), rule, ...rules.slice(at)], `Plan ${at + 1} put back`);
    focusPlan(at);
  }

  function move(i, delta) {
    const next = moved(rules, i, delta);
    if (next !== rules) {
      deleted = null;
      send(next, `Plan ${i + 1} moved ${delta < 0 ? 'up' : 'down'}`);
      // Focus follows the plan, not the place it left.
      focusPlan(i + delta, delta < 0 ? 'up' : 'down');
    }
  }
</script>

<div class="board" data-board="plans" bind:this={root}>
  <div class="cols">
    <div class="main">
      <h3 class="sl">
        Game plans
        <em>checked from the top · the higher plan wins a clash · <b data-plan-count>{planCount(state, schema)}</b></em>
      </h3>
      <ol class="list">
        {#each rows as row, i (i)}
          <li class="plan" data-plan={row.n} data-plan-index={i}>
            <span class="n num">{row.n}</span>
            <span class="if">If</span>
            <span class="conds">
              {#each row.when as w, k (k)}<span class="cond {w.tone}">{w.text}</span>{/each}
            </span>
            <span class="ar" aria-hidden="true">→</span>
            <span class="sets">
              {#each row.set as s, k (k)}
                <span class="set">{s.label} <b>{s.word}</b>{#if s.waits}<i> · at the next stoppage</i>{/if}</span>
              {/each}
            </span>
            <span class="acts">
              <button type="button" class="x" aria-label="Move plan {row.n} up" data-act="up" disabled={disabled || i === 0} onclick={() => move(i, -1)}>↑</button>
              <button type="button" class="x" aria-label="Move plan {row.n} down" data-act="down" disabled={disabled || i === rows.length - 1} onclick={() => move(i, 1)}>↓</button>
              <button type="button" class="btn" aria-label="Edit plan {row.n}" data-act="edit" {disabled} onclick={() => (open = i)}>Edit</button>
              <button type="button" class="x" aria-label="Delete plan {row.n}" data-act="delete" {disabled} onclick={() => remove(i)}>×</button>
            </span>
          </li>
          {#if open === i}
            <li class="edit">
              {#key open}
                <GamePlanEditor
                  {schema}
                  rule={rules[i]}
                  n={row.n}
                  ofCount={rules.length}
                  onsave={save}
                  oncancel={close}
                  ondelete={() => remove(i)}
                />
              {/key}
            </li>
          {/if}
        {:else}
          <li class="none">No game plans: the side keeps its instructions whatever the score.</li>
        {/each}
        {#if open === -1}
          <li class="edit">
            <GamePlanEditor {schema} rule={fresh()} n={rules.length + 1} ofCount={rules.length + 1} onsave={save} oncancel={close} />
          </li>
        {/if}
      </ol>
      <div class="foot">
        <button type="button" class="btn cy" data-act="new" disabled={disabled || full || open !== null} onclick={() => (open = -1)}>New plan</button>
        {#if full}<span class="g" data-plans-full>{fullWords(schema)}</span>{/if}
        <span class="g" role="status">
          {#if deleted}Plan {deleted.at + 1} deleted.
            <button type="button" class="btn" disabled={disabled || full} onclick={undo}>Undo</button>{/if}
        </span>
      </div>

      <h3 class="sl">When plans can switch on<em>minutes of the 90</em></h3>
      <PlanTimeline {state} {schema} />
    </div>

    <div class="side">
      <StubSection note="opposition instructions">
        <h3 class="sl">Opposition instructions<em>per player of theirs</em></h3>
        <div class="g">Show onto his weaker foot, tight marking, tackle hard.</div>
      </StubSection>
      <StubSection note="shouts">
        <h3 class="sl">Shouts<em>for a few minutes</em></h3>
        <div class="g">Calm down, demand more, focus.</div>
      </StubSection>
      <StubSection note="the other manager">
        <h3 class="sl">The other manager<em>what he tends to do</em></h3>
        <div class="g">How he reacts to the score and the clock.</div>
      </StubSection>
    </div>
  </div>
</div>

<style>
  .board {
    min-width: 0;
  }

  .cols {
    display: grid;
    grid-template-columns: minmax(0, 860fr) minmax(0, 360fr);
    gap: 16px;
  }

  .sl {
    font: 700 10.5px var(--fd);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--cyan);
    margin: 4px 0 6px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .sl em {
    font: 500 9.5px var(--fb);
    font-style: normal;
    letter-spacing: 0;
    text-transform: none;
    color: var(--ink-3);
  }

  .sl em b {
    color: var(--ink-2);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .plan {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 26px;
    padding: 2px 4px;
    border-bottom: 1px solid var(--rule-2);
    font-size: 10px;
    color: var(--ink-2);
  }

  .n {
    font: 700 10px var(--fd);
    font-variant-numeric: tabular-nums;
    min-width: 14px;
    color: var(--ink);
  }

  .if {
    font: 700 8.5px var(--fd);
    text-transform: uppercase;
    color: var(--ink-3);
  }

  .conds,
  .sets {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .conds {
    flex: 1;
  }

  .sets {
    flex: 1.2;
  }

  .cond {
    padding: 0 5px;
    border: 1px solid var(--rule);
    border-radius: var(--radius-sm);
    font-size: 9.5px;
    line-height: 16px;
    color: var(--ink);
  }

  .cond.red {
    border-color: var(--bad);
  }

  .cond.en {
    border-color: var(--warn);
  }

  .set b {
    color: var(--ink);
    font-weight: 600;
  }

  .set i {
    font-style: normal;
    color: var(--warn);
  }

  .ar {
    color: var(--ink-3);
  }

  .acts {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .none,
  .g {
    font-size: 9.5px;
    color: var(--ink-3);
  }

  .none {
    padding: 6px 4px;
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 0 12px;
  }

  .btn {
    min-height: var(--hit);
    padding: 0 9px;
    margin: 0;
    border: 1px solid var(--control-edge);
    border-radius: var(--radius-sm);
    background: var(--button-dark);
    color: var(--ghost-ink);
    font: 600 9.5px var(--fd);
    cursor: pointer;
  }

  .btn.cy {
    background: var(--cyan);
    border-color: var(--cyan);
    color: var(--cyan-ink);
  }

  .btn:hover:not(:disabled) {
    border-color: var(--ink-3);
  }

  .btn:disabled,
  .x:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .x {
    display: inline-grid;
    place-items: center;
    min-width: var(--hit);
    min-height: var(--hit);
    margin: 0;
    padding: 0 3px;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--ink-2);
    font: 700 11px var(--fd);
    cursor: pointer;
  }

  .x:hover:not(:disabled) {
    color: var(--ink);
    background: var(--rule);
  }

  .btn:focus-visible,
  .x:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: 1px;
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .cols {
      grid-template-columns: minmax(0, 1fr);
    }

    .plan {
      flex-wrap: wrap;
    }

  }
</style>
