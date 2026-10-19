<!-- The Roles tab of the tactics board: one player's own instructions. The player is picked
     on the pitch or with the arrows; the table lists each instruction he has, beside the
     team's, with his own level (the tick marks the team's) and Remove; the menu adds one of
     the tactics file's player instructions, and disables one he already has or one his
     position does not take. Each edit sends the side's player instructions with that player's
     changed: before kick-off in the draft, during play at once. Roles and duties stay on the
     Shapes tab; role fit and the role's reading are faded stubs. -->
<script>
  import ChoiceSegments from './ChoiceSegments.svelte';
  import LevelPips from './LevelPips.svelte';
  import StubSection from './StubSection.svelte';
  import TacticsPitch from './TacticsPitch.svelte';
  import { uprightPlace } from '../lib/lineup-editor.js';
  import { levelWord, primitiveWords, reachWords } from '../lib/board-words.js';
  import { controlFor } from '../lib/board-cards.js';
  import { levelNames, primitiveIndex, primitivesOf } from '../lib/primitives.js';

  let { dugout, nameOf = (slot) => `Slot ${slot + 1}` } = $props();

  let slot = $state(9);

  let schema = $derived(dugout.schema);
  let levels = $derived(dugout.levels);
  let state = $derived(dugout.gameState);
  let disabled = $derived(!dugout.boardEditable);
  let formation = $derived(schema?.formations?.[dugout.tactics?.formation ?? 0] ?? null);
  let position = $derived(formation?.slots?.[slot]?.position ?? '');
  let overrides = $derived(schema?.game_state?.player_overrides ?? []);
  let name = $derived(nameOf(slot));

  const info = (primitive) => {
    const index = primitiveIndex(schema, primitive);
    const p = primitivesOf(schema)[index];
    const names = levelNames(schema, p);
    return {
      index,
      names,
      words: names.map((_, i) => levelWord(primitive, i, names)),
      full: primitiveWords(primitive, names).full,
      control: controlFor(p, names.length),
    };
  };

  let rows = $derived(
    (state.instructions ?? [])
      .filter((i) => i.slot === slot)
      .map((i) => {
        const at = info(i.primitive);
        return { ...at, primitive: i.primitive, own: at.names.indexOf(i.level), team: levels[at.index] };
      })
  );

  let menu = $derived(
    overrides.map((o) => {
      const at = info(o.primitive);
      const set = rows.some((r) => r.primitive === o.primitive);
      const reaches = !o.positions?.length || o.positions.includes(position);
      return {
        primitive: o.primitive,
        full: at.full,
        note: set ? 'already set' : reaches ? `team: ${at.words[levels[at.index]] ?? ''}` : reachWords(o.positions),
        enabled: !set && reaches,
      };
    })
  );

  let markers = $derived(
    (formation?.slots ?? []).map((s, n) => {
      const place = uprightPlace(s.x, s.y);
      const own = (state.instructions ?? []).some((i) => i.slot === n);
      return {
        key: n,
        n,
        left: place.left,
        top: place.top,
        position: s.position,
        keeper: s.position === 'GK',
        shirt: own ? '★' : String(n + 1),
        name: String(nameOf(n)).split(' ').at(-1),
        picked: n === slot,
        label: `${nameOf(n)}, ${s.position}${own ? ', has his own instructions' : ''}`,
        tag: s.position,
        token: '--pitch-ink',
      };
    })
  );

  /// The side's player instructions with this player's set to `mine`, the others kept.
  function send(mine, label) {
    const others = (state.instructions ?? []).filter((i) => i.slot !== slot);
    dugout.setGameState({ rules: state.rules ?? [], instructions: [...others, ...mine] }, label);
  }

  function mineWith(primitive, level) {
    return rows.map((r) => ({
      slot,
      primitive: r.primitive,
      level: r.names[r.primitive === primitive ? level : r.own],
    }));
  }

  function setOwn(row, level) {
    send(mineWith(row.primitive, level), `${name}: ${row.full} → ${row.words[level]}`);
  }

  function remove(row) {
    send(
      mineWith(null, 0).filter((i) => i.primitive !== row.primitive),
      `${name}: ${row.full} back to the team's`
    );
  }

  function add(primitive) {
    const at = info(primitive);
    const level = levels[at.index] ?? 0;
    send([...mineWith(null, 0), { slot, primitive, level: at.names[level] }], `${name}: own ${at.full}`);
    adding = false;
  }

  let adding = $state(false);

  const step = (d) => (slot = (slot + d + 11) % 11);
</script>

<div class="board" data-board="roles">
  <div class="who">
    <button type="button" class="x" aria-label="The player before" onclick={() => step(-1)}>‹</button>
    <h3 class="name"><b>{name}</b> <span>{position}</span></h3>
    <button type="button" class="x" aria-label="The player after" onclick={() => step(1)}>›</button>
    <em>player instructions replace the team instruction, for this player only</em>
  </div>

  <div class="cols">
    <div class="pitch">
      <TacticsPitch {markers} interactive label="Pick a player" onpick={(n) => (slot = n)} />
    </div>

    <div class="mid">
      <h3 class="sl">Player instructions · {name}<em>{rows.length} of {overrides.length} set · the tick marks the team instruction</em></h3>
      <table class="tbl">
        <thead><tr><th>Instruction</th><th>Team</th><th>His own</th><th></th><th></th></tr></thead>
        <tbody>
          {#each rows as row (row.primitive)}
            <tr data-own={row.primitive}>
              <td><b>{row.full}</b></td>
              <td class="g">{row.words[row.team]}</td>
              <td>
                {#if row.control === 'pips'}
                  <LevelPips
                    full="{name}’s own {row.full}"
                    words={row.words}
                    value={row.own}
                    def={row.team}
                    {disabled}
                    onchange={(l) => setOwn(row, l)}
                  />
                {:else}
                  <ChoiceSegments full="{name}’s own {row.full}" words={row.words} value={row.own} {disabled} onchange={(l) => setOwn(row, l)} />
                {/if}
              </td>
              <td><b class="own">{row.words[row.own]}</b> <span class="tag">OWN</span></td>
              <td>
                <button type="button" class="x" aria-label="Remove {name}’s own {row.full}" {disabled} onclick={() => remove(row)}>×</button>
              </td>
            </tr>
          {:else}
            <tr><td colspan="5" class="g">{name} follows every team instruction.</td></tr>
          {/each}
        </tbody>
      </table>
      <div class="acts">
        <button type="button" class="btn" aria-expanded={adding} {disabled} onclick={() => (adding = !adding)}>Add instruction</button>
        <button type="button" class="btn" disabled={disabled || rows.length === 0} onclick={() => send([], `${name}: every instruction back to the team's`)}>Clear all</button>
      </div>
      {#if adding}
        <div class="menu" role="listbox" aria-label="Add an instruction for {name}">
          <div class="head">Add an instruction for {name}</div>
          {#each menu as item (item.primitive)}
            <button
              type="button"
              role="option"
              class="opt"
              aria-selected="false"
              disabled={!item.enabled}
              aria-disabled={!item.enabled}
              data-add={item.primitive}
              onclick={() => add(item.primitive)}
            >
              <b>{item.full}</b><span>{item.note}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <StubSection note="role fit and the role's reading">
      <h3 class="sl">Role fit<em>what the role reads, habits and grasp</em></h3>
      <div class="g">How well {name} suits his role, from his attributes.</div>
    </StubSection>
  </div>
</div>

<style>
  .board {
    min-width: 0;
  }

  .who {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 4px 0 8px;
  }

  .name {
    margin: 0;
    font: 700 12px var(--fd);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ink);
  }

  .name span {
    color: var(--ink-3);
    margin-left: 4px;
  }

  .who em {
    margin-left: auto;
    font: 500 9.5px var(--fb);
    font-style: normal;
    color: var(--ink-3);
  }

  .cols {
    display: grid;
    grid-template-columns: minmax(0, 270fr) minmax(0, 650fr) minmax(0, 300fr);
    gap: 16px;
  }

  .pitch {
    max-width: 270px;
  }

  .sl {
    font: 700 10.5px var(--fd);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--cyan);
    margin: 0 0 6px;
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

  .tbl {
    width: 100%;
    border-collapse: collapse;
    font-size: 10px;
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
    border-bottom: 1px solid var(--rule-2);
    vertical-align: middle;
    color: var(--ink-2);
  }

  .tbl b {
    color: var(--ink);
  }

  .own {
    font: 600 9.5px var(--fd);
    color: var(--cyan);
  }

  .g {
    color: var(--ink-3);
    font-size: 9.5px;
  }

  .tag {
    font: 700 7.5px/11px var(--fd);
    letter-spacing: 0.05em;
    padding: 0 3px;
    border-radius: var(--radius-sm);
    background: var(--cyan);
    color: var(--cyan-ink);
  }

  .acts {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }

  .btn {
    height: 22px;
    padding: 0 10px;
    margin: 0;
    border: 1px solid var(--control-edge);
    border-radius: var(--radius-sm);
    background: var(--button-dark);
    color: var(--ghost-ink);
    font: 600 9.5px var(--fd);
    cursor: pointer;
  }

  .btn:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .x {
    display: inline-grid;
    place-items: center;
    min-width: 18px;
    height: 18px;
    margin: 0;
    padding: 0 3px;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--ink-2);
    font: 700 12px var(--fd);
    cursor: pointer;
  }

  .x:hover:not(:disabled) {
    color: var(--ink);
    background: var(--rule);
  }

  .x:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .menu {
    margin-top: 6px;
    padding: 4px;
    max-width: 360px;
    background: var(--select-ground);
    border: 1px solid var(--select-edge);
    border-radius: 3px;
  }

  .head {
    font: 700 8.5px var(--fd);
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--ink-3);
    padding: 4px 6px;
  }

  .opt {
    display: grid;
    grid-template-columns: 170px 1fr;
    gap: 6px;
    align-items: baseline;
    width: 100%;
    margin: 0;
    padding: 4px 6px;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--select-ink);
    text-align: left;
    font-size: 9.5px;
    cursor: pointer;
  }

  .opt b {
    font: 600 10px var(--fd);
  }

  .opt span {
    color: var(--ink-3);
  }

  .opt:hover:not(:disabled) {
    background: var(--ground-2);
  }

  .opt:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .btn:focus-visible,
  .x:focus-visible,
  .opt:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: 1px;
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .cols {
      grid-template-columns: minmax(0, 1fr);
    }

    .btn,
    .x,
    .opt {
      min-height: 44px;
      min-width: 44px;
    }
  }
</style>
