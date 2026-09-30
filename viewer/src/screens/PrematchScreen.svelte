<!-- The Pre-match line-ups, ported from the sketch's Pre-match board: the shell with the
     fixture in the header, KICK OFF in the cyan block and TEAM TALK beside it as a LATER note;
     the tall fact strip; then our line-up sheet, the broadcast kick-off pitch with both shapes
     and the other club's sheet; then the rule-pack checks and the panels the engine has no
     model for yet (conditions, crowd and referee, what is at stake, how much to watch) as
     LATER stubs. The page is read only: the lineup is set on Tactics, and "Change on Tactics"
     goes back there. Nothing is sent until KICK OFF. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import Crest from '../components/Crest.svelte';
  import InfoStrip from '../components/InfoStrip.svelte';
  import KickoffPitch from '../components/KickoffPitch.svelte';
  import LineupSheet from '../components/LineupSheet.svelte';
  import RulePackChecks from '../components/RulePackChecks.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import StubSection from '../components/StubSection.svelte';
  import { prematchLead } from '../lib/prematch.js';
  import { teamKit } from '../lib/team-kit.js';

  let { session } = $props();

  /// The sub-navigation of the matchday: Pre-match is the one view built here; the others
  /// are stubs until their screens are ported.
  const TABS = [
    { id: 'prematch', label: 'Pre-match', active: true },
    { id: 'team-talk', label: 'Team talk', stub: true },
    { id: 'match', label: 'Match', menu: true, stub: true },
    { id: 'touchline', label: 'Touchline', menu: true, stub: true },
    { id: 'half-time', label: 'Half-time', menu: true, stub: true },
    { id: 'report', label: 'Report', menu: true, stub: true },
    { id: 'analysis', label: 'Analysis', menu: true, stub: true },
  ];

  /// The strip cells the engine has no model for yet.
  const LATER_FACTS = [
    { value: '—', label: 'League places', stub: true },
    { value: '—', label: 'Weather', stub: true },
    { value: '—', label: 'Crowd', stub: true },
    { value: '—', label: 'Referee', stub: true },
    { value: '—', label: 'Match importance', stub: true },
  ];

  /// The info panels the engine has no model for yet, drawn for the look with their labels.
  const LATER_PANELS = [
    { id: 'conditions', label: 'Conditions', rows: ['Rain', 'Wind', 'Temperature', 'Pitch quality', 'Worn areas'] },
    { id: 'crowd', label: 'Crowd and referee', rows: ['Attendance', 'Away fans', 'Home edge', 'Referee', 'Cards per match'] },
    { id: 'stake', label: 'What is at stake', rows: ['Winner', 'Prize', 'Board objective', 'Rivalry'] },
  ];
  const WATCH = ['Key', 'Extended', 'Comprehensive', 'Full', 'Commentary'];

  let sheet = $derived(session.sheet());
  let teams = $derived(sheet?.teams ?? session.teams ?? []);
  let names = $derived(teams.map((t) => t['team.name']));
  let kits = $derived(teams.map((t) => teamKit(t)));
  let facts = $derived([prematchLead(session.hello), ...LATER_FACTS]);
  let title = $derived(names.length === 2 ? `${names[0]} v ${names[1]}` : 'Pre-match');
</script>

<AppShell
  {title}
  subtitle="Pre-match · the line-ups"
  date={session.dateWord}
  dateSub={session.dateClock}
  action={session.action}
  onaction={() => session.act()}
  busy={session.actionBusy}
  actionNote="Team talk"
  navLabel="Matchday views"
  tabs={TABS}
  ontab={() => {}}
>
  {#snippet crest()}
    <Crest team={teams[0] ?? null} />
  {/snippet}

  <div class="screen" data-screen="prematch">
    <InfoStrip {facts} tall>
      {#snippet lead()}
        <Crest team={teams[0] ?? null} width={30} height={32} />
      {/snippet}
    </InfoStrip>

    {#if sheet}
      <div class="cols">
        <LineupSheet team={teams[0]} sheet={sheet.home} shape={sheet.shapes[0]} side={0} />

        <div class="mid">
          <p class="kicker">Pre-match</p>
          <h2>Starting line-ups</h2>
          <KickoffPitch dots={sheet.dots} {kits} shapes={sheet.shapes} {names} />
          <p class="shapes">
            <b style:border-bottom-color={kits[0]?.fill}>{sheet.shapes[0]}</b>
            <span>v</span>
            <b style:border-bottom-color={kits[1]?.fill}>{sheet.shapes[1]}</b>
          </p>
          <p class="status">
            <span class="ok"><span aria-hidden="true">✓</span> Line-ups confirmed</span>
            <span class="how">Set on Tactics · read only here</span>
            <button class="btn gh" type="button" onclick={() => session.back()}>Change on Tactics</button>
          </p>
        </div>

        <LineupSheet team={teams[1]} sheet={sheet.away} shape={sheet.shapes[1]} side={1} />
      </div>

      <div class="info">
        <RulePackChecks rules={sheet.rules} />
        {#each LATER_PANELS as panel (panel.id)}
          <section class="panel">
            <SectionLabel label={panel.label} later />
            <!-- STUB: a panel the engine has no model for yet. -->
            <StubSection note="panel: {panel.label}">
              {#each panel.rows as row (row)}
                <div class="kv"><span>{row}</span><b>—</b></div>
              {/each}
            </StubSection>
          </section>
        {/each}
        <section class="panel">
          <SectionLabel label="How much to watch" later />
          <!-- STUB: highlight levels need the highlights the viewer does not build yet. -->
          <StubSection note="panel: how much to watch">
            <div class="watch">
              {#each WATCH as w (w)}<span class="opt">{w}</span>{/each}
            </div>
          </StubSection>
        </section>
      </div>
    {:else}
      <p class="none">The line-ups show once the engine has sent both teams.</p>
    {/if}
  </div>
</AppShell>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .cols {
    display: grid;
    grid-template-columns: 318px 1fr 250px;
    gap: 0 24px;
    margin-top: 10px;
    flex: none;
  }

  .mid {
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 0;
  }

  .kicker {
    margin: 0;
    font: 700 10px var(--fd);
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--cyan);
  }

  h2 {
    margin: 0 0 8px;
    font: 800 22px var(--fd);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ink);
  }

  .shapes {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 8px 0 0;
    font: 800 12px var(--fd);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ink);
  }

  .shapes b {
    border-bottom: 2px solid var(--club-sample);
    padding-bottom: 3px;
  }

  .shapes span {
    color: var(--ink-3);
    font-weight: 600;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 8px 0 0;
    font-size: 10px;
  }

  .ok {
    font: 700 10px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--good);
  }

  .how {
    color: var(--ink-2);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-sm);
    font: 600 10px var(--fb);
    cursor: pointer;
  }

  .btn.gh {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--control-edge);
    color: var(--ghost-ink);
  }

  .btn:hover {
    filter: brightness(1.12);
  }

  .info {
    display: grid;
    grid-template-columns: 1.15fr 1fr 1fr 1fr 0.95fr;
    margin-top: 16px;
    border-top: 1px solid var(--rule);
    padding-top: 12px;
    flex: 1;
    min-height: 0;
  }

  .info > :global(*) {
    padding: 0 14px;
    border-left: 1px solid var(--rule);
    min-width: 0;
  }

  .info > :global(:first-child) {
    padding-left: 0;
    border-left: 0;
  }

  .kv {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    padding: 2px 0;
    font-size: 10.5px;
    color: var(--ink-2);
  }

  .kv b {
    color: var(--ink);
  }

  .watch {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 5px;
  }

  .opt {
    display: block;
    height: 20px;
    line-height: 20px;
    text-align: center;
    font-size: 10px;
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--control-edge);
    border-radius: var(--radius-sm);
  }

  .none {
    margin: 20px 0;
    color: var(--ink-2);
  }
</style>
