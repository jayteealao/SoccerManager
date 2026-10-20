<!-- The plan editor: "When all of these are true" (the conditions the tactics file offers,
     each with its value) and "Then change" (any team instruction, the mentality, or the
     formation, which waits for the next stoppage), with Save plan, Cancel and Delete plan.
     The draft is the written form the game-state change carries. -->
<script>
  import { onMount, untrack } from 'svelte';
  import ChoiceSegments from './ChoiceSegments.svelte';
  import LevelPips from './LevelPips.svelte';
  import { controlFor } from '../lib/board-cards.js';
  import { levelWord, primitiveWords } from '../lib/board-words.js';
  import { CONDITION_LABELS, conditionKinds, conditionRange, newCondition, ruleProblem } from '../lib/game-plans.js';
  import { defaultLevel, levelNames, primitivesOf } from '../lib/primitives.js';

  let { schema, rule, n, ofCount, onsave = () => {}, oncancel = () => {}, ondelete = null } = $props();

  // The draft starts from the plan once; the list re-creates the editor for another plan.
  let draft = $state(untrack(() => structuredClone($state.snapshot(rule))));

  let kinds = $derived(conditionKinds(schema));
  let unused = $derived(kinds.filter((k) => !draft.when.some((c) => c.kind === k)));
  let changeable = $derived(primitivesOf(schema).filter((p) => !draft.set.some((a) => a.primitive === p.name)));
  let problem = $derived(ruleProblem(draft));
  let root = $state(null);
  const footId = `plan-foot-${Math.random().toString(36).slice(2, 8)}`;

  // Opening the editor puts focus on its first field, so a keyboard player lands in the plan.
  onMount(() => {
    root?.querySelector('select, button')?.focus();
  });

  function trySave() {
    if (problem === null) {
      onsave($state.snapshot(draft));
    }
  }

  const between = (lo, hi, list) => list.filter((v) => v >= lo && v <= hi);
  const count = (c) => between(conditionRange(schema, c.kind).min, conditionRange(schema, c.kind).max, [1, 2, 3, 4, 5]);
  const minutes = (c) => between(conditionRange(schema, c.kind).min, conditionRange(schema, c.kind).max, [5, 10, 15, 20, 25, 30, 35, 40, 45]);
  const plays = (c) => between(conditionRange(schema, c.kind).min, conditionRange(schema, c.kind).max, [-1, -0.5, -0.2, 0, 0.01, 0.2, 0.5, 1]);
  const energy = (c) => between(conditionRange(schema, c.kind).min, conditionRange(schema, c.kind).max, [0.3, 0.4, 0.5, 0.6, 0.7, 0.8]);

  function info(name) {
    const p = primitivesOf(schema).find((x) => x.name === name);
    const names = levelNames(schema, p);
    return {
      p,
      names,
      words: names.map((_, i) => levelWord(name, i, names)),
      full: primitiveWords(name, names).full,
      control: controlFor(p, names.length),
    };
  }

  function setCondition(i, c) {
    draft.when[i] = c;
  }

  function addChange(value) {
    if (value === 'formation') {
      draft.set.push({ formation: schema.formations[0].name });
      return;
    }
    const at = info(value);
    draft.set.push({ primitive: value, level: at.names[defaultLevel(schema, at.p)] });
  }
</script>

<div class="ed" role="group" aria-label="Edit plan {n}" bind:this={root}>
  <div class="head"><b>Plan {n}</b><span>of {ofCount} · checked {n === 1 ? 'first' : 'in order'}</span></div>
  <div class="sub">When all of these are true</div>
  {#each draft.when as c, i (c.kind)}
    <div class="erow" data-condition={c.kind}>
      <span class="what">{CONDITION_LABELS[c.kind] ?? c.kind}</span>
      {#if c.kind === 'score'}
        <select
          aria-label="Score"
          value={c.state}
          onchange={(e) => {
            const state = e.currentTarget.value;
            setCondition(i, state === 'level' ? { kind: 'score', state } : { kind: 'score', state, by: c.by ?? 1 });
          }}
        >
          <option value="trailing">Losing</option>
          <option value="level">Drawing</option>
          <option value="leading">Winning</option>
        </select>
        {#if c.state !== 'level'}
          <span class="g">by</span>
          <select aria-label="Goals" value={String(c.by ?? 1)} onchange={(e) => setCondition(i, { ...c, by: Number(e.currentTarget.value) })}>
            {#each count(c) as v (v)}<option value={String(v)}>{v}</option>{/each}
          </select>
        {/if}
      {:else if c.kind === 'time_left'}
        <select aria-label="Minutes left" value={String(c.minutes)} onchange={(e) => setCondition(i, { ...c, minutes: Number(e.currentTarget.value) })}>
          {#each minutes(c) as v (v)}<option value={String(v)}>{v}</option>{/each}
        </select>
        <span class="g">minutes or less left</span>
      {:else if c.kind === 'red_cards_ours' || c.kind === 'red_cards_theirs'}
        <span class="g">at least</span>
        <select aria-label="Players sent off" value={String(c.at_least ?? 1)} onchange={(e) => setCondition(i, { ...c, at_least: Number(e.currentTarget.value) })}>
          {#each count(c) as v (v)}<option value={String(v)}>{v}</option>{/each}
        </select>
        <span class="g">{c.at_least === 1 || c.at_least === undefined ? 'player' : 'players'}</span>
      {:else if c.kind === 'run_of_play'}
        <select
          aria-label="Better or worse"
          value={c.above !== undefined ? 'above' : 'below'}
          onchange={(e) => {
            const v = c.above ?? c.below ?? 0;
            setCondition(i, e.currentTarget.value === 'above' ? { kind: c.kind, above: v } : { kind: c.kind, below: v });
          }}
        >
          <option value="above">better than</option>
          <option value="below">worse than</option>
        </select>
        <select
          aria-label="Run of play"
          value={String(c.above ?? c.below ?? 0)}
          onchange={(e) => {
            const v = Number(e.currentTarget.value);
            setCondition(i, c.above !== undefined ? { kind: c.kind, above: v } : { kind: c.kind, below: v });
          }}
        >
          {#each plays(c) as v (v)}<option value={String(v)}>{v}</option>{/each}
        </select>
        <span class="g">in chances made, ours less theirs, over the last {schema?.game_state?.run_of_play_window_min ?? 15} minutes</span>
      {:else}
        <span class="g">below</span>
        <select aria-label="Energy" value={String(c.below)} onchange={(e) => setCondition(i, { ...c, below: Number(e.currentTarget.value) })}>
          {#each energy(c) as v (v)}<option value={String(v)}>{Math.round(v * 100)}%</option>{/each}
        </select>
      {/if}
      <span class="sp"></span>
      <button type="button" class="x" aria-label="Remove the condition {CONDITION_LABELS[c.kind] ?? c.kind}" onclick={() => draft.when.splice(i, 1)}>×</button>
    </div>
  {/each}
  {#if unused.length}
    <div class="add">
      <select aria-label="Add condition" value="" onchange={(e) => { const k = e.currentTarget.value; if (k) { draft.when.push(newCondition(k, schema)); e.currentTarget.value = ''; } }}>
        <option value="">Add condition…</option>
        {#each unused as k (k)}<option value={k}>{CONDITION_LABELS[k] ?? k}</option>{/each}
      </select>
    </div>
  {/if}

  <div class="sub">Then change</div>
  {#each draft.set as a, i (a.primitive ?? 'formation')}
    <div class="erow" data-change={a.primitive ?? 'formation'}>
      {#if a.formation !== undefined}
        <span class="what">Formation</span>
        <select aria-label="Formation" value={a.formation} onchange={(e) => (draft.set[i] = { formation: e.currentTarget.value })}>
          {#each schema.formations as f (f.name)}<option value={f.name}>{f.name}</option>{/each}
        </select>
        <span class="wa">at the next stoppage</span>
      {:else}
        {@const at = info(a.primitive)}
        <span class="what">{at.full}</span>
        {#if at.control === 'pips'}
          <LevelPips
            full={at.full}
            words={at.words}
            value={at.names.indexOf(a.level)}
            def={defaultLevel(schema, at.p)}
            onchange={(l) => (draft.set[i] = { primitive: a.primitive, level: at.names[l] })}
          />
          <b class="w">{at.words[at.names.indexOf(a.level)]}</b>
        {:else if at.control === 'select'}
          <select aria-label={at.full} value={a.level} onchange={(e) => (draft.set[i] = { primitive: a.primitive, level: e.currentTarget.value })}>
            {#each at.names as name, k (name)}<option value={name}>{at.words[k]}</option>{/each}
          </select>
        {:else}
          <ChoiceSegments
            full={at.full}
            words={at.words}
            value={at.names.indexOf(a.level)}
            onchange={(l) => (draft.set[i] = { primitive: a.primitive, level: at.names[l] })}
          />
        {/if}
      {/if}
      <span class="sp"></span>
      <button type="button" class="x" aria-label="Remove the change {a.formation !== undefined ? 'Formation' : primitiveWords(a.primitive, []).full}" onclick={() => draft.set.splice(i, 1)}>×</button>
    </div>
  {/each}
  <div class="add">
    <select aria-label="Add change" value="" onchange={(e) => { const v = e.currentTarget.value; if (v) { addChange(v); e.currentTarget.value = ''; } }}>
      <option value="">Add change…</option>
      {#if !draft.set.some((a) => a.formation !== undefined)}<option value="formation">Formation</option>{/if}
      {#each changeable as p (p.name)}<option value={p.name}>{primitiveWords(p.name, levelNames(schema, p)).full}</option>{/each}
    </select>
    <span class="g">any team instruction, the mentality or the formation</span>
  </div>

  <div class="foot">
    <button
      type="button"
      class="btn cy"
      aria-disabled={problem !== null}
      aria-describedby={footId}
      onclick={trySave}>Save plan</button
    >
    <button type="button" class="btn" onclick={oncancel}>Cancel</button>
    <span class="g" id={footId} role="status">{problem ?? 'The plan switches off when these stop being true. If two plans change the same thing, the higher one wins.'}</span>
    {#if ondelete}<button type="button" class="btn del" onclick={ondelete}>Delete plan</button>{/if}
  </div>
</div>

<style>
  .ed {
    border: 1px solid var(--cyan);
    border-radius: 3px;
    background: var(--queue-ground);
    padding: 8px 10px;
    margin: 4px 0 6px;
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .head b {
    font: 700 11.5px var(--fd);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ink);
  }

  .head span,
  .g {
    font-size: 9px;
    color: var(--ink-3);
  }

  .sub {
    font: 700 8px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-3);
    padding: 6px 0 1px;
  }

  .erow {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 24px;
    border-bottom: 1px solid var(--queue-rule);
    font-size: 9.5px;
    color: var(--ink-2);
  }

  .what {
    min-width: 120px;
    color: var(--ink);
  }

  .w {
    font: 600 9.5px var(--fd);
    min-width: 56px;
    color: var(--ink);
  }

  .wa {
    font-size: 9.5px;
    color: var(--warn);
  }

  .sp {
    flex: 1;
  }

  select {
    height: 18px;
    padding: 0 4px;
    border: 1px solid var(--select-edge);
    border-radius: var(--radius-sm);
    background: var(--select-ground);
    color: var(--select-ink);
    font: 600 9px var(--fd);
  }

  .add {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 5px 0 2px;
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }

  .foot .g {
    flex: 1;
  }

  .btn {
    min-height: var(--hit);
    padding: 0 10px;
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

  .btn.del {
    color: var(--bad);
  }

  .btn:hover:not([aria-disabled='true']) {
    border-color: var(--ink-3);
  }

  .btn[aria-disabled='true'] {
    opacity: 0.45;
    cursor: default;
  }

  .x {
    min-width: var(--hit);
    min-height: var(--hit);
    margin: 0;
    padding: 0 3px;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--ink-2);
    font: 700 12px var(--fd);
    cursor: pointer;
  }

  .btn:focus-visible,
  .x:focus-visible,
  select:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: 1px;
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .erow,
    .foot {
      flex-wrap: wrap;
    }

    select {
      min-height: 44px;
    }
  }

  @media (pointer: coarse) {
    select {
      min-height: var(--hit);
    }
  }
</style>
