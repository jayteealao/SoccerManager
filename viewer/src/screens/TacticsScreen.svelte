<!-- The Tactics screen, ported from the sketch's two Tactics boards (pre-match and in match):
     the shell with TACTICS in the header, the green pitch panel on the left (the tactic bar,
     the in-possession and out-of-possession pitches, the four cards and the Limits and Weights
     cards) and a right column that holds the squad list before kick-off, and the roles table,
     the substitution picker and the queued changes during play. Parts the engine has no model
     for yet are LATER stubs: drawn, faded and inert. A stored match is read-only. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import Crest from '../components/Crest.svelte';
  import InstructionsPanel from '../components/InstructionsPanel.svelte';
  import Notice from '../components/Notice.svelte';
  import QueuedChanges from '../components/QueuedChanges.svelte';
  import RolesTable from '../components/RolesTable.svelte';
  import SquadList from '../components/SquadList.svelte';
  import StubSection from '../components/StubSection.svelte';
  import SubPicker from '../components/SubPicker.svelte';
  import TacticsBar from '../components/TacticsBar.svelte';
  import TacticsPitch from '../components/TacticsPitch.svelte';
  import { uprightPlace } from '../lib/lineup-editor.js';
  import { markerTag } from '../lib/tactics-board.js';

  let { session } = $props();

  /// The sub-navigation: Match goes back to the pitch; Shapes is the one Tactics view built;
  /// Squad opens the Squad screen; the others are stubs until their screens are ported.
  const TABS = [
    { id: 'match', label: 'Match' },
    { id: 'shapes', label: 'Shapes', active: true },
    { id: 'squad', label: 'Squad' },
    { id: 'roles', label: 'Roles', stub: true },
    { id: 'instructions', label: 'Instructions', menu: true, stub: true },
    { id: 'plans', label: 'Plans & opposition', menu: true, stub: true },
    { id: 'set-pieces', label: 'Set pieces', menu: true, stub: true },
    { id: 'familiarity', label: 'Familiarity', menu: true, stub: true },
    { id: 'analysis', label: 'Analysis', menu: true, stub: true },
  ];

  const CARDS = [
    { id: 'ball-lost', title: 'Ball lost', text: 'What the team does the moment it loses the ball.' },
    { id: 'ball-won', title: 'Ball won', text: 'What the team does the moment it wins the ball.' },
    { id: 'roles', title: 'Roles', text: 'The catalogue of roles, each with a duty.' },
    { id: 'marking', title: 'Marking', text: 'Zonal lines and man-marking.' },
  ];

  let dugout = $derived(session.dugout);
  let version = $derived(dugout.version);
  let schema = $derived(dugout.schema);
  let tactics = $derived(dugout.tactics);
  let preMatch = $derived(dugout.phase === 'pre-match' || dugout.phase === 'kicking-off');
  let editable = $derived(dugout.phase === 'pre-match');
  let live = $derived(dugout.live);
  let editor = $derived(dugout.editor);
  let names = $derived(session.teams ? session.teams.map((t) => t['team.name']) : ['Home', 'Away']);

  let subtitle = $derived(
    preMatch
      ? `${names[0]} v ${names[1]} · pick the eleven, then kick off`
      : dugout.phase === 'stored'
        ? `${session.title} · a stored match: the tactics are shown only`
        : `${session.title} · changes queue for the next stoppage`
  );

  const surname = (name) => String(name ?? '').split(' ').at(-1);

  /// The out-of-possession markers: before kick-off the editor's slots, during play the
  /// eleven on the pitch at the rendered tick.
  let markers = $derived.by(() => {
    version;
    if (!schema || !tactics) {
      return [];
    }
    if (preMatch && editor) {
      return editor.slotRows((n) => dugout.roleOf(n)).map((row) => ({
        key: row.n,
        n: row.n,
        left: row.place.left,
        top: row.place.top,
        position: row.position,
        keeper: row.position === 'GK',
        empty: row.player === null,
        shirt: row.player?.shirt ?? '',
        name: surname(row.player?.name),
        picked: row.picked,
        label: row.label,
        ...(row.player ? markerTag(schema, tactics, row.n) : { tag: '', token: '--pitch-ink' }),
      }));
    }
    const formation = schema.formations[tactics.formation];
    return dugout.homeRows.map((row, n) => {
      const spot = formation.slots[n];
      const place = uprightPlace(spot.x, spot.y);
      return {
        key: row.wire,
        n,
        left: place.left,
        top: place.top,
        position: spot.position,
        keeper: spot.position === 'GK',
        shirt: row.shirt,
        name: surname(row.name),
        ...markerTag(schema, tactics, n),
      };
    });
  });

  /// The in-possession pitch is a stub: the same places, numbers only, no data of its own.
  let shadow = $derived(markers.map((m) => ({ key: m.key, left: m.left, top: m.top, shirt: m.shirt, keeper: m.keeper })));

  let list = $derived.by(() => {
    version;
    return editor ? editor.squadList() : null;
  });
  let pickedText = $derived.by(() => {
    version;
    return editor ? editor.pickedText() : null;
  });
  let canEmpty = $derived.by(() => {
    version;
    return editor ? editor.canEmpty : false;
  });
  let ready = $derived.by(() => {
    version;
    return editor ? editor.ready : false;
  });
  let reason = $derived.by(() => {
    version;
    return editor ? editor.reason : null;
  });

  function act(fn) {
    dugout.lineupAction(fn);
  }

  function edit(change, slot = null) {
    dugout.editTactics(change, slot);
  }

  let side = $state(null);

  /// The NOT READY link: brings the squad row the reason is about into view and focuses it,
  /// or the list's first row when no one row is at fault.
  function showCulprit() {
    const at = editor?.culprit();
    const hook =
      at?.kind === 'slot' ? `[data-place="slot-${at.n}"]` : at ? `[data-squad="${at.index}"]` : null;
    const row =
      (hook && side?.querySelector(`button${hook}`)) ||
      side?.querySelector('button[data-squad], button[data-place]');
    if (row) {
      row.scrollIntoView?.({ block: 'nearest' });
      row.focus({ preventScroll: true });
    }
  }
</script>

<AppShell
  section="tactics"
  title="Tactics"
  {subtitle}
  date={session.dateWord}
  dateSub={session.dateClock}
  action={session.action}
  onaction={() => session.act()}
  busy={session.actionBusy}
  tabs={TABS}
  ontab={(id) => (id === 'squad' ? session.openSquad() : session.show(id === 'match' ? 'match' : 'tactics'))}
  menu={session.onMenu}
  menuOpen={session.menuOpen}
>
  {#snippet crest()}
    <Crest team={session.teams?.[0] ?? null} />
  {/snippet}

  <div class="screen" data-screen="tactics" data-phase={dugout.phase}>
    <div class="tpanel">
      <TacticsBar
        {schema}
        {tactics}
        preMatch={editable}
        {live}
        {ready}
        {reason}
        onformation={(f) => dugout.setFormation(f)}
        onmentality={(m) => edit({ mentality: m })}
        onreason={showCulprit}
      />

      <div class="shapes">
        <!-- STUB: the in-possession shape needs a shape model the engine does not have yet. -->
        <StubSection note="in-possession shape">
          <div class="shape">
            <span class="lab in">In possession</span>
            <TacticsPitch markers={shadow} label="In possession shape" />
            <span class="later-tag">LATER</span>
          </div>
        </StubSection>

        <div class="shape">
          <span class="lab out">Out of possession · {schema?.formations?.[tactics?.formation ?? 0]?.name ?? ''}</span>
          <TacticsPitch
            {markers}
            interactive={editable}
            label="Out of possession shape: the eleven slots"
            onpick={(n) => act((e) => e.pickPlace({ kind: 'slot', n }))}
            onempty={(n) => act((e) => e.emptyPlace({ kind: 'slot', n }))}
            ondrop={(from, n) => act((e) => e.drop(from, { kind: 'slot', n }))}
          />
        </div>

        <!-- STUB: Ball lost, Ball won, the roles catalogue and marking need models the engine
             does not have yet. -->
        <StubSection note="tactic cards">
          <div class="cards">
            {#each CARDS as card (card.id)}
              <div class="tcard">
                <h4>{card.title}<span class="later">LATER</span></h4>
                <p>{card.text}</p>
                <span class="more">Change</span>
              </div>
            {/each}
          </div>
        </StubSection>
      </div>

      <InstructionsPanel {schema} {tactics} disabled={!editable && !live} onedit={(c) => edit(c)} />
    </div>

    <div class="side" bind:this={side}>
      {#if preMatch && list}
        <SquadList
          {list}
          total={dugout.squad.length}
          shape={schema?.formations?.[tactics?.formation ?? 0]?.name ?? ''}
          benchSize={editor.benchSize}
          {pickedText}
          {canEmpty}
          onpickrow={(i) => act((e) => e.pickRow(i))}
          onpickplace={(p) => act((e) => e.pickPlace(p))}
          ondrop={(from, to) => act((e) => e.drop(from, to))}
          onempty={() => act((e) => e.emptyPicked())}
        />
      {:else if dugout.phase === 'stored' || !schema}
        <Notice kind="neutral" word="STORED" message="This match is stored or takes no lineup, so its tactics cannot change here." />
      {:else}
        <RolesTable {schema} {tactics} rows={dugout.homeRows} disabled={!live} onedit={edit} />
        <div class="hr"></div>
        <SubPicker
          picker={dugout.picker}
          editing={dugout.editing?.kind === 'substitution'}
          onqueue={(off, on) => dugout.substitute(off, on)}
        />
        <div class="gap"></div>
        <QueuedChanges
          chips={dugout.chips}
          editing={dugout.editing}
          cancelRefused={dugout.cancelRefused}
          {live}
          onedit={(id) => dugout.startEdit(id)}
          onstopedit={() => dugout.stopEdit()}
          oncancel={(id) => dugout.cancel(id)}
          ondismiss={(id) => dugout.dismiss(id)}
        />
      {/if}
    </div>
  </div>
</AppShell>

<style>
  /* The board's 770 px panel and the squad beside it at 1280, as shares, so both grow
     together. */
  .screen {
    display: grid;
    grid-template-columns: minmax(0, 770fr) minmax(0, 409fr);
    grid-template-rows: minmax(0, 1fr);
    column-gap: 16px;
    height: 100%;
    padding-top: 4px;
  }

  .tpanel {
    align-self: start;
    min-width: 0;
    container-type: inline-size;
    padding: 9px 12px 12px;
    background: var(--pitch);
    border-radius: 6px;
  }

  /* The two shapes and the tactic cards, as the board draws them at 1280: 300, 300 and
     126 px. */
  .shapes {
    display: grid;
    grid-template-columns: minmax(0, 300fr) minmax(0, 300fr) minmax(0, 126fr);
    gap: 10px;
    margin-top: 7px;
  }

  .shape {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    min-width: 0;
  }

  .lab {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    font: 700 10px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--pitch-ink);
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    white-space: nowrap;
  }

  .lab.in {
    background: var(--navy-700);
  }

  .lab.out {
    background: var(--pitch-tag);
  }

  .later-tag {
    font: 600 9px var(--fd);
    letter-spacing: 0.06em;
    color: var(--pitch-ink);
  }

  .cards {
    min-width: 0;
  }

  .tcard {
    background: var(--pitch-card);
    border: 1px solid var(--pitch-card-edge);
    border-radius: var(--radius-md);
    padding: 9px 10px 8px;
    margin-bottom: 8px;
  }

  .tcard h4 {
    margin: 0 0 5px;
    font: 700 9.5px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--pitch-ink);
  }

  .tcard p {
    margin: 0 0 6px;
    color: var(--pitch-card-ink);
    font-size: 9.5px;
    line-height: 1.45;
  }

  .later {
    margin-left: 6px;
    font: 600 9px var(--fd);
    letter-spacing: 0.06em;
    color: var(--pitch-card-ink);
  }

  .more {
    font: 700 9.5px var(--fd);
    letter-spacing: 0.06em;
    color: var(--pitch-ink);
    text-transform: uppercase;
  }

  .side {
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    container-type: inline-size;
  }

  .hr {
    border-top: 1px solid var(--rule);
    margin: 10px 0;
  }

  .gap {
    height: 10px;
  }

  /* A panel under 740 px, as the Tactics-768 board draws it: the two shapes side by side,
     the tactic cards in a row of four under them. */
  @container (max-width: 739px) {
    .shapes {
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    }

    /* The cards' stub box is the grid's last item. */
    .shapes > :global(:last-child) {
      grid-column: 1 / -1;
    }

    .cards {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 10px;
    }

    .tcard {
      margin-bottom: 0;
    }
  }

  /* Wide and up, as the Tactics-1920 board draws it: the panel keeps 865 px and the squad
     list takes the rest, from 520 px. */
  @media (min-width: 1600px) {
    .screen {
      grid-template-columns: minmax(0, 865px) minmax(520px, 1fr);
    }
  }

  /* Compact: the squad under the panel. The body scrolls. */
  @media (max-width: 1023px), (max-height: 599px) {
    .screen {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto;
      row-gap: 16px;
      height: auto;
    }

    .side {
      overflow: visible;
    }
  }
</style>
