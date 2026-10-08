<!-- The Squad screen, ported whole from the sketch's players-squad screen (boards 1 and 2): the
     shell with SQUAD in the header; the sub-navigation (back to the view it opened from, then
     Squad and the sketch's other squad views as stubs); the navy strip (players and view, and
     average condition built; load, familiarity, morale and homegrown LATER); the toolbar (the
     view chip with Save view, the Columns chip and its menu, Position and Find a player built;
     Available only, Hide loanees and Compare LATER); the squad table driven by the active
     named view; and the three footer notes, LATER. A player's name opens his panel. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import ColumnMenu from '../components/ColumnMenu.svelte';
  import Crest from '../components/Crest.svelte';
  import InfoStrip from '../components/InfoStrip.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import SquadTable from '../components/SquadTable.svelte';
  import StubSection from '../components/StubSection.svelte';
  import { allColumns, nextSort, shownColumns, sortRows, squadRows } from '../lib/columns.js';
  import { defaultView, freeName, MAX_VIEWS, readPart, resetView, sameView } from '../lib/views.js';

  let { session } = $props();

  const COLUMNS = allColumns();

  let names = $derived(session.teams ? session.teams.map((t) => t['team.name']) : ['Home', 'Away']);
  let from = $derived(session.squadFrom === 'report' ? 'Report' : session.squadFrom === 'touchline' ? 'Touchline' : session.squadFrom === 'match' ? 'Match' : 'Tactics');
  let tabs = $derived([
    { id: 'back', label: from },
    { id: 'squad', label: 'Squad', active: true },
    { id: 'registration', label: 'Registration', stub: true },
    { id: 'understanding', label: 'Understanding', menu: true, stub: true },
    { id: 'dressing-room', label: 'Dressing room', menu: true, stub: true },
    { id: 'development', label: 'Development', menu: true, stub: true },
    { id: 'contracts', label: 'Contracts', menu: true, stub: true },
    { id: 'reports', label: 'Reports', menu: true, stub: true },
    { id: 'compare', label: 'Compare', menu: true, stub: true },
  ]);

  let part = $derived(session.squadViews ?? readPart(null));
  let stored = $derived(part.views.find((v) => v.name === part.active) ?? part.views[0] ?? defaultView());
  /// The view on show: the stored active view, with any change the open menu has made.
  let draft = $state(null);
  let working = $derived(draft ?? stored);
  let menuOpen = $state(false);
  let position = $state('all');
  let find = $state('');

  let squad = $derived(session.squad);
  let ctx = $derived({ ratings: session.squadRatings });
  let columns = $derived(shownColumns(working, COLUMNS));
  let positions = $derived([...new Set(squad.map((p) => p['player.position']).filter(Boolean))]);
  let rows = $derived.by(() => {
    const needle = find.trim().toLowerCase();
    const picked = squadRows(squad).filter(
      (r) =>
        (position === 'all' || r.player['player.position'] === position) &&
        (!needle || String(r.player['player.name'] ?? '').toLowerCase().includes(needle))
    );
    return sortRows(picked, working.sort, ctx, COLUMNS);
  });
  let unfiltered = $derived(position === 'all' && !find.trim() && !working.sort);

  let viewWord = $derived(
    draft && !sameView(draft, stored)
      ? 'changed'
      : session.viewsSaved === 'saving'
        ? 'saving'
        : session.viewsSaved === 'not-saved'
          ? 'not saved'
          : session.viewsAdapter?.kind === 'memory'
            ? 'kept for this session'
            : 'saved'
  );

  let averageCondition = $derived.by(() => {
    const values = squad.map((p) => p['player.condition']).filter((v) => typeof v === 'number');
    return values.length ? `${Math.round(values.reduce((a, b) => a + b, 0) / values.length)}%` : '—';
  });

  let facts = $derived([
    { value: `First team · ${squad.length} players`, label: `Showing ${rows.length} · view “${working.name}”` },
    { value: averageCondition, label: 'Average condition', num: true },
    { value: 'Load', label: 'A staff reading', stub: true },
    { value: 'Team familiarity', label: 'The eleven picked', stub: true },
    { value: 'Morale', label: 'The dressing room', stub: true },
    { value: 'Homegrown', label: 'Registration lists', stub: true },
  ]);

  let columnsWord = $derived(
    working.columns
      .slice(0, 3)
      .map((id) => COLUMNS.find((c) => c.id === id)?.label)
      .filter(Boolean)
      .join(' · ') + (working.columns.length > 3 ? ' …' : '')
  );

  /// Saves `views` with `active` on show, as the club's views.
  function store(views, active) {
    draft = null;
    session.saveViews({ active, views });
  }

  /// The club's views with the view on show replaced by `view`.
  function withView(view) {
    const list = part.views.some((v) => v.name === view.name)
      ? part.views.map((v) => (v.name === view.name ? view : v))
      : [...part.views, view];
    return list;
  }

  function sortBy(id) {
    const view = { ...working, sort: nextSort(working.sort, id, COLUMNS) };
    store(withView(view), view.name);
  }

  function done() {
    menuOpen = false;
    if (draft && !sameView(draft, stored)) {
      store(withView(draft), draft.name);
    } else {
      draft = null;
    }
  }

  function pickView(name) {
    draft = null;
    store(part.views, name);
  }

  function saveAsNew() {
    if (part.views.length >= MAX_VIEWS) {
      return;
    }
    const view = { ...working, name: freeName(part.views) };
    store([...part.views, view], view.name);
  }

  function tab(id) {
    if (id === 'back') {
      session.closeSquad();
    }
  }
</script>

<AppShell
  section="squad"
  title="Squad"
  subtitle="{names[0]} first team"
  date={session.dateWord}
  dateSub={session.dateClock}
  action={session.action}
  onaction={() => session.act()}
  busy={session.actionBusy}
  {tabs}
  ontab={tab}
  navLabel="Squad views"
  menu={session.onMenu}
  menuOpen={session.menuOpen}
>
  {#snippet crest()}
    <Crest team={session.teams?.[0] ?? null} />
  {/snippet}

  <div class="screen" data-screen="squad">
    <InfoStrip {facts}>
      {#snippet lead()}
        <Crest team={session.teams?.[0] ?? null} width={22} height={24} />
      {/snippet}
    </InfoStrip>

    <div class="bar">
      <label class="sel view">
        <span>View:</span>
        <select data-view={working.name} value={part.active} onchange={(e) => pickView(e.currentTarget.value)}>
          {#each part.views as view (view.name)}
            <option value={view.name}>{view.name}</option>
          {/each}
        </select>
        <span class="g" data-view-state={viewWord}>· {viewWord}</span>
      </label>
      <span class="chip on">First team + bench</span>
      <label class="chip">
        <span>Position:</span>
        <select bind:value={position} aria-label="Position">
          <option value="all">All</option>
          {#each positions as p (p)}<option value={p}>{p}</option>{/each}
        </select>
      </label>
      <StubSection note="available only filter" inline><span class="chip">Available only</span></StubSection>
      <StubSection note="hide loanees filter" inline><span class="chip">Hide loanees</span></StubSection>
      <div class="anchor">
        <button
          type="button"
          class="chip"
          class:on={menuOpen}
          aria-haspopup="dialog"
          aria-expanded={menuOpen}
          data-columns-chip
          onclick={() => (menuOpen ? done() : (menuOpen = true))}
        >
          Columns: {columnsWord}
        </button>
        {#if menuOpen}
          <ColumnMenu
            view={working}
            {viewWord}
            onchange={(next) => (draft = next)}
            onreset={() => (draft = resetView(working))}
            ondone={done}
          />
        {/if}
      </div>
      <label class="inp">
        <span class="sr">Find a player</span>
        <input type="search" placeholder="Find a player" bind:value={find} />
      </label>
      <button type="button" class="btn" data-save-view disabled={part.views.length >= MAX_VIEWS} onclick={saveAsNew}>Save view</button>
      <StubSection note="compare two players" inline><span class="btn gh">Compare 2</span></StubSection>
    </div>

    <div class="table">
      <SquadTable
        {rows}
        {columns}
        sort={working.sort}
        {ctx}
        selected={session.panelPlayer}
        separate={unfiltered}
        onsort={sortBy}
        onopen={(i) => session.openPlayer(i)}
      />
    </div>

    <div class="notes">
      <!-- STUB: familiarity, the staff's load reading and registration need models the engine
           does not have yet. -->
      <StubSection note="team familiarity note">
        <SectionLabel label="Team familiarity" later />
        <p>Worked out from the eleven you pick, never stored: each player keeps his own grasp of the tactic.</p>
      </StubSection>
      <StubSection note="load note">
        <SectionLabel label="Load is a staff reading" later />
        <p>Matches in recent days, read by the fitness coach, with how sure he is.</p>
      </StubSection>
      <StubSection note="registration note">
        <SectionLabel label="Registration" later />
        <p>League and cup lists, homegrown counts and the form over the last 3 and 10 matches.</p>
      </StubSection>
    </div>
  </div>
</AppShell>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .bar {
    display: flex;
    gap: 8px;
    align-items: center;
    margin: 10px 0 8px;
    flex-wrap: wrap;
  }

  .anchor {
    position: relative;
  }

  .sel,
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: max(22px, var(--hit));
    padding: 0 9px;
    border-radius: var(--radius-sm);
    background: var(--toggle-ground);
    border: 1px solid var(--toggle-edge);
    font: 400 9.5px var(--fb);
    color: var(--seg-ink);
  }

  .sel.view {
    min-width: 210px;
  }

  /* A label grows round its select, which keeps the smallest hit area itself. */
  label.sel,
  label.chip {
    height: auto;
    min-height: max(22px, var(--hit));
  }

  button.chip {
    cursor: pointer;
  }

  .chip.on {
    border-color: var(--cyan);
    color: var(--cyan);
  }

  /* The select fills its chip, so the whole chip is its hit area. */
  select {
    align-self: stretch;
    min-width: var(--hit);
    min-height: var(--hit);
    border: 0;
    background: transparent;
    color: var(--ink);
    font: 600 9.5px var(--fb);
  }

  select option {
    background: var(--well);
  }

  .inp {
    display: inline-flex;
    align-items: center;
    margin-left: auto;
    width: 170px;
    min-height: max(24px, var(--hit));
    padding: 0 8px;
    border-radius: var(--radius-sm);
    background: var(--toggle-ground);
    border: 1px solid var(--toggle-edge);
  }

  .inp input {
    align-self: stretch;
    min-height: var(--hit);
    width: 100%;
    border: 0;
    background: transparent;
    color: var(--ink);
    font: 400 10.5px var(--fb);
  }

  .inp input::placeholder {
    color: var(--ink-3);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    height: max(22px, var(--hit));
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-sm);
    font: 600 10px var(--fb);
    cursor: pointer;
    color: var(--ink);
    background: var(--navy-600);
  }

  .btn:hover:not([disabled]) {
    filter: brightness(1.12);
  }

  .btn[disabled] {
    opacity: 0.5;
    cursor: default;
  }

  .btn.gh {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--control-edge);
    color: var(--ghost-ink);
  }

  .chip:focus-visible,
  .btn:focus-visible,
  select:focus-visible,
  .inp:focus-within {
    outline: 2px solid var(--cyan);
    outline-offset: 1px;
  }

  .g {
    color: var(--ink-3);
  }

  .table {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }

  .notes {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0 28px;
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--rule);
    font-size: 9.5px;
    color: var(--ink-3);
    line-height: 1.4;
  }

  .notes p {
    margin: 0;
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .notes {
      grid-template-columns: minmax(0, 1fr);
      gap: 10px;
    }

    .inp {
      margin-left: 0;
    }
  }
</style>
