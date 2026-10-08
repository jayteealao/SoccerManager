<!-- Match setup, ported from the approved MatchSetup and MatchSetupSame boards: the shell with
     NEW MATCH in the header and KICK OFF (or PICK AN AWAY TEAM) in the cyan block; the
     1 Home team, 2 Away team and 3 Kick off tabs; the 54 px fact strip; then three columns of
     1fr, 1fr and 360 px. The two club tables pick the home and the away team; the same club
     twice shows the Same team alert and disables Kick off. The right column shows the match,
     the home team's ground and its size, and the rest of the round the launcher will play
     beside it (the round preview, from the same seed). Kick off posts the pair; Back to start
     returns. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import ClubPicker from '../components/ClubPicker.svelte';
  import Crest from '../components/Crest.svelte';
  import Glyph from '../components/Glyph.svelte';
  import InfoStrip from '../components/InfoStrip.svelte';
  import KvRow from '../components/KvRow.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import TouchlineMark from '../components/TouchlineMark.svelte';
  import { PLAY } from '../components/icons.js';
  import { convertedNotice, groundName, groundSize, setupFacts, teamOf, versions } from '../lib/front-door-model.js';

  let { door } = $props();

  let home = $derived(door.teams.find((t) => t.id === door.picks.home) ?? null);
  let away = $derived(door.teams.find((t) => t.id === door.picks.away) ?? null);
  let same = $derived(Boolean(home && away && home.id === away.id));
  let problem = $derived(door.pickProblem);
  let v = $derived(versions(door.status));
  let tabs = $derived([
    { id: 'home', label: '1 Home team', active: false },
    { id: 'away', label: '2 Away team', active: same },
    { id: 'kickoff', label: '3 Kick off', active: !same },
  ]);
  let others = $derived(door.round?.fixtures ?? []);
  let converted = $derived(convertedNotice(home));

  function tab(id) {
    globalThis.document?.querySelector(`[data-column="${id}"] button`)?.focus();
  }
</script>

<AppShell
  section="home"
  title="New match"
  subtitle="Pick the home team and the away team · the ground follows the home team"
  date="TOUCHLINE {v.touchline}"
  dateSub="Engine ready"
  action={problem ? 'Pick an away team' : 'Kick off'}
  onaction={() => door.kickOff()}
  busy={door.busy || Boolean(problem)}
  {tabs}
  ontab={tab}
  navLabel="Match setup steps"
  current="step"
>
  {#snippet crest()}
    <TouchlineMark size={30} />
  {/snippet}
  <div class="screen" data-screen="setup">
    <InfoStrip facts={setupFacts(home, away, door.round)}>
      {#snippet lead()}
        <Crest team={teamOf(home)} width={22} height={24} />
      {/snippet}
    </InfoStrip>

    <div class="cols">
      <div data-column="home">
        <SectionLabel label="1 · Home team" note="{door.teams.length} sample teams" />
        <ClubPicker
          label="Home team"
          teams={door.teams}
          picked={door.picks.home}
          onpick={(id) => door.pick('home', id)}
        />
        {#if converted}
          <p class="converted" role="status" data-converted={converted.club}>
            <b>{converted.club}:</b> {converted.text}
          </p>
        {/if}
      </div>
      <div data-column="away">
        <SectionLabel label="2 · Away team" note="{door.teams.length} sample teams" />
        <ClubPicker
          label="Away team"
          teams={door.teams}
          picked={door.picks.away}
          other={same ? null : door.picks.home}
          otherWord="Home team"
          onpick={(id) => door.pick('away', id)}
        />
        {#if same}
          <div class="alert" role="alert">
            <b>Same team</b><span>A team cannot play itself. Pick a different away team.</span>
          </div>
        {/if}
      </div>
      <div data-column="kickoff">
        <SectionLabel label="3 · Kick off" note={same ? 'Waiting for a different away team' : 'Your match'} />
        <div class="versus">
          <div class="side">
            <Crest team={teamOf(home)} width={42} height={44} />
            <b>{home?.name ?? '—'}</b>
          </div>
          <b class="v">v</b>
          <div class="side" class:dim={same}>
            <Crest team={teamOf(away)} width={42} height={44} />
            <b>{away?.name ?? '—'}</b>
          </div>
        </div>
        <KvRow key="Ground" value={groundName(home)} />
        <KvRow key="Pitch" value={groundSize(home)} />
        <KvRow key="Engine" value={v.engine} />
        <div class="hr" role="presentation"></div>
        <SectionLabel label="The rest of the round" note="on your clock" />
        <div class="round" data-round={others.length}>
          {#if same || !door.round}
            <p class="note">{same ? 'The round follows once the two teams differ.' : 'Reading the round…'}</p>
          {:else}
            {#each others as fixture, i (i)}
              <div class="og">
                <span class="h">{fixture.home?.name ?? '—'} <Crest team={teamOf(fixture.home)} width={11} height={12} /></span>
                <b class="num">0 – 0</b>
                <span class="a"><Crest team={teamOf(fixture.away)} width={11} height={12} /> {fixture.away?.name ?? '—'}</span>
              </div>
            {/each}
          {/if}
        </div>
        <p class="note">The other matches play in the background and show their events at their minute on your clock.</p>
        {#if door.message}<p class="message" role="alert">{door.message}</p>{/if}
        <div class="actions">
          <button class="btn cy" type="button" disabled={Boolean(problem) || door.busy} data-kickoff onclick={() => door.kickOff()}>
            <Glyph glyph={{ d: PLAY }} size={10} /> Kick off
          </button>
          <button class="btn gh" type="button" onclick={() => door.open('start')}>Back to start</button>
        </div>
      </div>
    </div>
  </div>
</AppShell>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  /* The board's two halves and 360 px at 1280, as shares; at the compact step the two
     clubs side by side and the match settings under them. */
  .cols {
    display: grid;
    grid-template-columns: minmax(0, 417fr) minmax(0, 417fr) minmax(0, 360fr);
    margin-top: 12px;
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .cols {
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    }

    .cols > div:nth-child(3) {
      grid-column: 1 / -1;
      border-left: 0;
      border-top: 1px solid var(--rule);
      margin-top: 14px;
      padding: 14px 0 0;
    }
  }

  .cols > div {
    min-width: 0;
    padding-right: 18px;
  }

  .cols > div + div {
    border-left: 1px solid var(--rule);
    padding-left: 18px;
  }

  .cols > div:last-child {
    padding-right: 0;
  }

  .alert {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-top: 8px;
    padding: 7px 10px;
    background: var(--ground-2);
    border-radius: var(--radius-sm);
    box-shadow: inset 0 0 0 1px var(--bad);
    font-size: 10.5px;
  }

  .alert b {
    color: var(--bad);
  }

  /* The converted-file notice (board 5): a full cyan hairline, never a side stripe. */
  .converted {
    margin: 10px 0 0;
    padding: 7px 10px;
    background: var(--new-goal-ground);
    box-shadow: inset 0 0 0 1px var(--cyan);
    border-radius: var(--radius-sm);
    font-size: 10px;
    line-height: 1.45;
    color: var(--ink-2);
  }

  .converted b {
    color: var(--ink);
    font-weight: 600;
  }

  .versus {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 18px;
    padding: 10px 0 12px;
  }

  .side {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  .side.dim {
    opacity: 0.55;
  }

  .side b {
    display: block;
    font: 700 12px var(--fd);
    letter-spacing: 0.03em;
    text-transform: uppercase;
    margin-top: 4px;
  }

  .v {
    font: 800 18px var(--fd);
    color: var(--ink-3);
  }

  .hr {
    height: 1px;
    background: var(--rule);
    margin: 10px 0;
  }

  .og {
    display: grid;
    grid-template-columns: 1fr 40px 1fr;
    gap: 6px;
    align-items: center;
    height: 22px;
    font-size: 10px;
    border-bottom: 1px solid var(--rule-2);
  }

  .og .h {
    text-align: right;
    white-space: nowrap;
    display: inline-flex;
    justify-content: flex-end;
    align-items: center;
    gap: 4px;
  }

  .og .a {
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .og b {
    text-align: center;
    font: 700 10px var(--fd);
  }

  .note {
    margin: 5px 0 0;
    font-size: 9.5px;
    line-height: 1.45;
    color: var(--ink-3);
  }

  .message {
    margin: 8px 0 0;
    font-size: 10.5px;
    color: var(--bad);
  }

  .actions {
    display: flex;
    gap: 6px;
    margin-top: 12px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: max(28px, var(--hit));
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-sm);
    font: 600 10px var(--fb);
    cursor: pointer;
  }

  .btn.cy {
    background: var(--cyan);
    color: var(--cyan-ink);
  }

  .btn.gh {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--control-edge);
    color: var(--ghost-ink);
  }

  .btn:hover:not(:disabled) {
    filter: brightness(1.12);
  }

  .btn:active:not(:disabled) {
    filter: brightness(0.92);
  }

  .btn:disabled {
    cursor: default;
    opacity: 0.45;
  }
</style>
