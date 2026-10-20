<!-- The Phases tab of the tactics board, ported from the sketch's Phases screen: the top bar
     (the tactic picker, the changes against the style, the mentality and time wasting), the
     live strip and the Not applied banner during play, the four in-possession phase cards,
     the Block strip, the four out-of-possession phase cards, and the player transitions under
     them. Every primitive of the tactics file sits in exactly one place. Setting a control
     sends one change for that primitive: before kick-off it moves the draft; during play it
     applies at once and the players take it up over about the take-up time. -->
<script>
  import BoardRow from './BoardRow.svelte';
  import ChangeChips from './ChangeChips.svelte';
  import NotAppliedBanner from './NotAppliedBanner.svelte';
  import PhaseCard from './PhaseCard.svelte';
  import StubSection from './StubSection.svelte';
  import StylePicker from './StylePicker.svelte';
  import { BLOCK, CARDS, TOP_BAR } from '../lib/board-cards.js';
  import { rowModel } from '../lib/board-model.js';
  import { blockWord, levelWord, presetNote, styleWords } from '../lib/board-words.js';
  import { presetLevels } from '../lib/presets.js';
  import { levelNames, primitiveIndex, primitivesOf } from '../lib/primitives.js';

  let { dugout, nameOf = (slot) => `Slot ${slot + 1}`, ontab = () => {} } = $props();

  let version = $derived(dugout.version);
  let schema = $derived(dugout.schema);
  let levels = $derived(dugout.levels);
  let refused = $derived(dugout.refused);
  let disabled = $derived(!dugout.boardEditable);
  let formation = $derived(schema?.formations?.[dugout.tactics?.formation ?? 0] ?? null);
  let preset = $derived(dugout.presets.find((p) => p.name === dugout.style) ?? null);
  let styleLevels = $derived(preset ? presetLevels(schema, preset) : null);
  let changes = $derived.by(() => {
    version;
    levels;
    return dugout.styleChanges().length;
  });

  const model = (name) => rowModel(schema, name, { levels, styleLevels, refused });
  let topRows = $derived(TOP_BAR.map((r) => ({ placed: r, row: model(r.primitive) })));
  let blockRows = $derived(BLOCK.map((r) => ({ placed: r, row: model(r.primitive) })));
  let cards = $derived(
    CARDS.map((card) => ({ card, rows: card.rows.map((r) => model(r.primitive)), players: players(card) }))
  );
  let block = $derived(blockWord(levels[primitiveIndex(schema, 'line_of_engagement')] ?? 2));
  let options = $derived(dugout.presets.map((p) => ({ id: p.name, name: styleWords(p.name).name, note: presetNote(schema, p) })));

  /// The players with their own instruction on `card.players`, by name and word.
  function players(card) {
    if (!card.players) {
      return null;
    }
    const p = primitivesOf(schema)[primitiveIndex(schema, card.players)];
    const names = levelNames(schema, p);
    return (dugout.gameState.instructions ?? [])
      .filter((i) => i.primitive === card.players)
      .map((i) => ({ slot: i.slot, name: nameOf(i.slot), word: levelWord(i.primitive, names.indexOf(i.level), names) }));
  }

  /// The read-only player transitions: each player with his own instruction on winning or
  /// losing the ball, beside the team's.
  let transitions = $derived.by(() => {
    const slots = new Set(
      (dugout.gameState.instructions ?? [])
        .filter((i) => i.primitive === 'on_losing_the_ball' || i.primitive === 'on_winning_the_ball')
        .map((i) => i.slot)
    );
    const cell = (slot, name) => {
      const own = (dugout.gameState.instructions ?? []).find((i) => i.slot === slot && i.primitive === name);
      const index = primitiveIndex(schema, name);
      const p = primitivesOf(schema)[index];
      const names = levelNames(schema, p);
      return own
        ? { word: levelWord(name, names.indexOf(own.level), names), own: true }
        : { word: levelWord(name, levels[index], names), own: false };
    };
    return [...slots]
      .sort((a, b) => a - b)
      .map((slot) => ({ slot, name: nameOf(slot), lost: cell(slot, 'on_losing_the_ball'), won: cell(slot, 'on_winning_the_ball') }));
  });

  const set = (index, level) => dugout.setLevel(index, level);
</script>

<div class="board" data-board="phases">
  <div class="bar">
    <b class="cap">Tactic</b>
    <StylePicker
      formation={formation?.name ?? ''}
      {options}
      current={dugout.style}
      {changes}
      {disabled}
      onpick={(name) => dugout.pickStyle(name)}
    />
    {#if preset && changes > 0}
      <span class="st ch num">● {changes} change{changes > 1 ? 's' : ''}</span>
      <button type="button" class="btn" {disabled} onclick={() => dugout.pickStyle(preset.name)}
        >Reset to {styleWords(preset.name).name}</button
      >
    {:else if preset}
      <span class="st ok">✓ No changes</span>
    {/if}
    <span class="div"></span>
    {#each topRows as { placed, row } (placed.primitive)}
      {#if row}
        <BoardRow
          inline
          label={row.label}
          full={row.full}
          words={row.words}
          control={placed.control}
          value={row.value}
          def={row.def}
          changed={row.changed}
          error={row.error}
          styleWord={row.styleWord}
          primitive={row.primitive}
          {disabled}
          onchange={(level) => set(row.index, level)}
        />
      {/if}
    {/each}
    <span class="legend">
      <span><i class="dot"></i> changed from the style</span>
      <span><i class="notch"></i> default</span>
      <span><span class="tag">OWN</span> player instruction</span>
    </span>
  </div>

  <!-- Mounted for the whole of play, empty or not, so its first change is announced. -->
  {#if dugout.live}
    <ChangeChips
      chips={dugout.liveStrip}
      takeUp={schema?.game_state?.take_up_s ?? 30}
      oncancel={(id) => dugout.cancel(id)}
    />
  {/if}
  {#if refused}
    <NotAppliedBanner
      {refused}
      onretry={(id) => dugout.retry(id)}
      ondismiss={(id) => dugout.dismissRefusal(id)}
    />
  {/if}

  <h3 class="sl">In possession · {formation?.name ?? ''}<em>each instruction sits in the phase where it matters most</em></h3>
  <div class="grid">
    {#each cards.filter((c) => c.card.side === 'in') as c (c.card.id)}
      <PhaseCard
        card={c.card}
        rows={c.rows}
        slots={formation?.slots ?? []}
        players={c.players}
        {disabled}
        onset={set}
        onroles={() => ontab('roles')}
      />
    {/each}
  </div>

  <div class="block">
    <h3 class="sl-t">Out of possession · {formation?.name ?? ''}</h3>
    <span class="div"></span>
    <b class="cap">Block</b><span class="bw">{block}<span class="sr-only">, read from where the team presses from</span></span>
    {#each blockRows as { placed, row } (placed.primitive)}
      {#if row}
        <BoardRow
          inline
          label={row.label}
          full={row.full}
          words={row.words}
          control={placed.control}
          value={row.value}
          def={row.def}
          changed={row.changed}
          error={row.error}
          styleWord={row.styleWord}
          primitive={row.primitive}
          {disabled}
          onchange={(level) => set(row.index, level)}
        />
      {/if}
    {/each}
    <em>how high the team defends, in every phase below</em>
  </div>

  <div class="grid">
    {#each cards.filter((c) => c.card.side === 'out') as c (c.card.id)}
      <PhaseCard
        card={c.card}
        rows={c.rows}
        slots={formation?.slots ?? []}
        players={c.players}
        {disabled}
        onset={set}
        onroles={() => ontab('roles')}
      />
    {/each}
  </div>

  <div class="low">
    <StubSection note="time in each phase">
      <h3 class="sl">Time in each phase<em>from the last match</em></h3>
      <div class="stub-box">In possession · Out of possession · Transitions</div>
    </StubSection>
    <div>
      <h3 class="sl">Player transitions<em>the team instruction, unless the player has his own (Roles)</em></h3>
      <table class="tbl">
        <thead><tr><th>Player</th><th>When we lose it</th><th>When we win it</th></tr></thead>
        <tbody>
          {#each transitions as t (t.slot)}
            <tr>
              <td><b>{t.name}</b></td>
              <td>{t.lost.word} {#if t.lost.own}<span class="tag">OWN</span>{:else}<span class="team">team</span>{/if}</td>
              <td>{t.won.word} {#if t.won.own}<span class="tag">OWN</span>{:else}<span class="team">team</span>{/if}</td>
            </tr>
          {:else}
            <tr><td colspan="3" class="team">Every player follows the team instruction.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
    <StubSection note="analyst reading">
      <h3 class="sl">Weakest phase<em>analyst reading</em></h3>
      <div class="stub-box">The analyst reads the phases after a match.</div>
    </StubSection>
  </div>
</div>

<style>
  .board {
    min-width: 0;
    container-type: inline-size;
  }

  /* The bar and the Block strip wrap rather than clip when they are wider than the board. */
  .bar {
    display: flex;
    flex-wrap: wrap;
    row-gap: 6px;
    align-items: center;
    gap: 10px;
    min-height: 28px;
    margin-top: 2px;
  }

  .cap {
    font: 700 11px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink);
  }

  .st {
    font: 600 9.5px var(--fd);
    white-space: nowrap;
    padding: 0 6px;
    line-height: 18px;
    border-radius: var(--radius-sm);
    border: 1px solid currentColor;
  }

  .st.ok {
    color: var(--good);
  }

  .st.ch {
    color: var(--cyan);
  }

  .btn {
    min-height: var(--hit);
    padding: 0 10px;
    margin: 0;
    border: 1px solid var(--control-edge);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--ghost-ink);
    font: 600 9.5px var(--fd);
    cursor: pointer;
    white-space: nowrap;
  }

  .btn:hover:not(:disabled) {
    border-color: var(--ink-3);
  }

  .btn:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .btn:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: 1px;
  }

  .div {
    width: 1px;
    align-self: stretch;
    background: var(--rule);
    margin: 4px 0;
  }

  .legend {
    margin-left: auto;
    display: flex;
    gap: 10px;
    font-size: 9px;
    color: var(--ink-3);
    white-space: nowrap;
  }

  .legend > span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .dot {
    display: inline-block;
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--cyan);
  }

  .notch {
    display: inline-block;
    width: 5px;
    height: 2px;
    background: var(--ink-3);
  }

  .tag {
    font: 700 7.5px/11px var(--fd);
    letter-spacing: 0.05em;
    padding: 0 3px;
    border-radius: var(--radius-sm);
    background: var(--cyan);
    color: var(--cyan-ink);
  }

  .sl {
    font: 700 10.5px var(--fd);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--cyan);
    margin: 10px 0 6px;
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

  .grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
  }

  /* A board too narrow for four readable cards (the lower part of the standard step, about
     1024 to 1180 px wide) lays them out two by two, so no row label shrinks to nothing. */
  @container (max-width: 1099px) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  .block {
    display: flex;
    flex-wrap: wrap;
    row-gap: 6px;
    align-items: center;
    gap: 10px;
    margin: 10px 0 8px;
    min-height: 22px;
    white-space: nowrap;
  }

  .sl-t {
    margin: 0;
    font: 700 10.5px var(--fd);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--cyan);
  }

  .block .cap {
    font-size: 10px;
    color: var(--ink-2);
  }

  .bw {
    font: 700 10px var(--fd);
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--ink);
    border: 1px solid var(--seg-edge);
    border-radius: var(--radius-sm);
    padding: 0 6px;
    line-height: 18px;
  }

  .block em {
    margin-left: auto;
    font: 500 9.5px var(--fb);
    font-style: normal;
    color: var(--ink-3);
  }

  .low {
    display: grid;
    grid-template-columns: minmax(0, 400fr) minmax(0, 560fr) minmax(0, 300fr);
    gap: 16px;
    margin-top: 10px;
    border-top: 1px solid var(--rule);
    padding-top: 8px;
  }

  .stub-box {
    font-size: 9.5px;
    color: var(--ink-3);
  }

  .tbl {
    width: 100%;
    border-collapse: collapse;
    font-size: 9.5px;
  }

  .tbl th {
    text-align: left;
    font: 600 8.5px var(--fd);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ink-3);
    border-bottom: 1px solid var(--rule);
    padding: 2px 4px;
  }

  .tbl td {
    padding: 2px 4px;
    height: 18px;
    border-bottom: 1px solid var(--rule-2);
    color: var(--ink-2);
  }

  .tbl b {
    color: var(--ink);
  }

  .team {
    font-size: 8.5px;
    color: var(--ink-3);
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .bar,
    .block {
      flex-wrap: wrap;
      row-gap: 8px;
    }

    .block .sl-t,
    .block em,
    .legend {
      width: 100%;
      margin-left: 0;
    }

    .div {
      display: none;
    }

    .grid,
    .low {
      grid-template-columns: minmax(0, 1fr);
    }

    .btn {
      min-height: 44px;
    }
  }
</style>
