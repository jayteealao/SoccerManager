<!-- The start screen, ported from the approved Start and StartNoSave boards: the shell with
     TOUCHLINE in the header, the version and Engine ready in the date block and NEW MATCH in
     the cyan block; the Start, Replays, Settings and Licences and about tabs; the 54 px fact
     strip; then three columns of 330 px, 1fr and 400 px. The left column holds the six
     choices (Resume disabled with its reason when nothing is saved; Quit after a rule, with
     no confirmation outside a match). The middle column shows the saved match on its pitch
     with its facts, or the empty state that teaches what Resume will do. The right column's
     Career block and Recent replays are LATER stubs: the game keeps no career and no replay
     library yet. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import InfoStrip from '../components/InfoStrip.svelte';
  import KvRow from '../components/KvRow.svelte';
  import MenuList from '../components/MenuList.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import StubSection from '../components/StubSection.svelte';
  import TouchlineMark from '../components/TouchlineMark.svelte';
  import Glyph from '../components/Glyph.svelte';
  import { LOCK, PLAY, PLUS } from '../components/icons.js';
  import { safeKit, SKIN_TOKENS, TOKENS } from '../lib/colour.js';
  import { savedClock, savedEngine, scoreLine, startFacts, startItems, versions } from '../lib/front-door-model.js';

  let { door, pickReplay = () => {} } = $props();

  const TABS = [
    { id: 'start', label: 'Start', active: true },
    { id: 'replays', label: 'Replays' },
    { id: 'settings', label: 'Settings' },
    { id: 'licences', label: 'Licences and about' },
  ];
  const CAREER = [
    ['New career', 'Pick a club and create a world'],
    ['Load career', 'Save slots'],
    ['Newcomer path', 'Areas open over a first season'],
  ];

  let saved = $derived(door.savedMatch);
  let v = $derived(versions(door.status));
  let items = $derived(startItems(saved));

  /// The players' markers on the saved pitch, in each team's kit colour.
  let markers = $derived.by(() => {
    const positions = saved?.positions;
    if (!positions) {
      return [];
    }
    const theme = globalThis.document?.documentElement.dataset.theme;
    const tokens = SKIN_TOKENS[theme] ?? TOKENS;
    const kits = (saved.teams ?? []).map((name) => {
      const team = door.teams.find((t) => t.name === name);
      return team ? safeKit({ primary: team.kit[0], secondary: team.kit[1] }, tokens) : null;
    });
    return positions.players.map(([team, x, y]) => ({ x, y, fill: kits[team]?.fill, ring: kits[team]?.ring }));
  });
  let ground = $derived(saved?.positions?.pitch ?? [105, 68]);

  function choose(id) {
    if (id === 'new') {
      door.open('setup');
    } else if (id === 'resume') {
      door.resume();
    } else if (id === 'replays') {
      pickReplay();
    } else if (id === 'settings' || id === 'licences') {
      door.open(id);
    } else if (id === 'quit') {
      door.quit();
    }
  }

  function tab(id) {
    if (id === 'replays') {
      pickReplay();
    } else if (id !== 'start') {
      door.open(id);
    }
  }
</script>

<AppShell
  section="home"
  title="Touchline"
  subtitle="Match engine {v.engine} · {saved ? '1 saved match' : 'no saved match'}"
  date="TOUCHLINE {v.touchline}"
  dateSub="Engine ready"
  action="New match"
  onaction={() => door.open('setup')}
  busy={door.busy}
  tabs={TABS}
  ontab={tab}
  navLabel="Start screen views"
>
  {#snippet crest()}
    <TouchlineMark size={30} />
  {/snippet}
  <div class="screen" data-screen="start">
    <InfoStrip facts={startFacts(door.status)}>
      {#snippet lead()}
        <TouchlineMark size={24} />
      {/snippet}
    </InfoStrip>

    <div class="cols">
      <div>
        <SectionLabel label="Start" note="↑ ↓ to move · Enter to choose" />
        <MenuList {items} onchoose={choose} />
        {#if door.message}<p class="message" role="alert">{door.message}</p>{/if}
      </div>

      <div>
        {#if saved}
          <SectionLabel label="Saved match" note="Resume where you stopped" />
          <svg
            class="pitch"
            width="100%"
            height="236"
            viewBox="-4 -4 {ground[0] + 8} {ground[1] + 8}"
            preserveAspectRatio="xMidYMid meet"
            role="img"
            aria-label="The saved match's pitch{markers.length ? ' with the players where they stood' : ''}"
            data-markers={markers.length}
          >
            <rect class="turf" x="-4" y="-4" width={ground[0] + 8} height={ground[1] + 8}></rect>
            {#each Array.from({ length: 10 }, (_, i) => i) as i (i)}
              {#if i % 2 === 0}<rect class="stripe" x={(ground[0] / 10) * i} y="0" width={ground[0] / 10} height={ground[1]}></rect>{/if}
            {/each}
            <g class="lines">
              <rect x="0" y="0" width={ground[0]} height={ground[1]}></rect>
              <line x1={ground[0] / 2} y1="0" x2={ground[0] / 2} y2={ground[1]}></line>
              <circle cx={ground[0] / 2} cy={ground[1] / 2} r="9.15"></circle>
              <rect x="0" y={ground[1] / 2 - 20.16} width="16.5" height="40.32"></rect>
              <rect x={ground[0] - 16.5} y={ground[1] / 2 - 20.16} width="16.5" height="40.32"></rect>
              <rect x="0" y={ground[1] / 2 - 9.16} width="5.5" height="18.32"></rect>
              <rect x={ground[0] - 5.5} y={ground[1] / 2 - 9.16} width="5.5" height="18.32"></rect>
            </g>
            {#each markers as m, i (i)}
              <circle class="marker" cx={m.x} cy={m.y} r="1.5" style:fill={m.fill} style:stroke={m.ring}></circle>
            {/each}
          </svg>
          <div class="facts">
            <div>
              <KvRow key="Fixture" value="{saved.teams?.[0] ?? 'Home'} v {saved.teams?.[1] ?? 'Away'}" />
              <KvRow key="Score" value={saved.score ? `${saved.score[0]} – ${saved.score[1]}` : '—'} />
            </div>
            <div>
              <KvRow key="Stopped at" value={savedClock(saved)} />
              <KvRow key="Engine" value={savedEngine(saved)} />
            </div>
          </div>
          {#if saved}
            <div class="actions">
              <button class="btn cy" type="button" disabled={door.busy} onclick={() => door.resume()}>
                <Glyph glyph={{ d: PLAY }} size={10} /> Resume at {savedClock(saved)}
              </button>
            </div>
          {/if}
        {:else}
          <SectionLabel label="Saved match" />
          <div class="empty" data-empty="saved">
            <div>
              <b>No saved match yet</b>
              <p>
                When you leave a match or quit, Touchline saves it here, so you can finish it later on the same engine.
              </p>
              <button class="btn cy" type="button" onclick={() => door.open('setup')}>
                <Glyph glyph={{ d: PLUS }} size={10} /> New match
              </button>
            </div>
          </div>
        {/if}
        <p class="sr" aria-live="polite">{saved ? scoreLine(saved) : ''}</p>
      </div>

      <div>
        <!-- STUB: career: new career, load career, save slots. -->
        <SectionLabel label="Career" later />
        <StubSection note="career: new career, load career, save slots">
          {#each CAREER as [name, sub] (name)}
            <div class="career">
              <span class="lock"><Glyph glyph={{ d: LOCK }} size={11} /></span>
              <span class="ctext"><b>{name}</b><span>{sub}</span></span>
              <span class="later">LATER</span>
            </div>
          {/each}
        </StubSection>
        <div class="hr" role="presentation"></div>
        <!-- STUB: recent replays: the game keeps no replay library; a saved replay is a file. -->
        <SectionLabel label="Recent replays" note="newest first" later />
        <StubSection note="recent replays list: no replay library yet">
          {#each [0, 1, 2, 3] as i (i)}
            <div class="replay-row"><span></span><b>– – –</b><span></span></div>
          {/each}
        </StubSection>
        <p class="note">Replays play back on the engine that recorded them.</p>
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

  .cols {
    display: grid;
    grid-template-columns: 330px 1fr 400px;
    margin-top: 12px;
    min-height: 0;
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

  .pitch {
    display: block;
    background: var(--pitch);
    border-radius: var(--radius-md);
  }

  .turf {
    fill: var(--pitch);
  }

  .stripe {
    fill: var(--pitch-stripe);
  }

  .lines rect,
  .lines line,
  .lines circle {
    fill: none;
    stroke: var(--pitch-line);
    stroke-width: 0.35;
    stroke-opacity: 0.8;
  }

  .marker {
    stroke-width: 0.45;
  }

  .facts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0 18px;
    margin-top: 10px;
  }

  .actions {
    display: flex;
    gap: 6px;
    margin-top: 10px;
  }

  .empty {
    height: 236px;
    border-radius: var(--radius-md);
    background: var(--ground-2);
    display: grid;
    place-items: center;
    text-align: center;
  }

  .empty > div {
    max-width: 380px;
  }

  .empty b {
    display: block;
    font: 800 18px var(--fd);
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .empty p {
    margin: 6px 0 12px;
    font-size: 11px;
    line-height: 1.5;
    color: var(--ink-2);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
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

  .btn:hover:not(:disabled) {
    filter: brightness(1.12);
  }

  .btn:active:not(:disabled) {
    filter: brightness(0.92);
  }

  .btn:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .career {
    display: flex;
    align-items: center;
    gap: 8px;
    border-bottom: 1px solid var(--rule-2);
    padding: 5px 0;
  }

  .lock {
    color: var(--ink-3);
    display: inline-flex;
  }

  .ctext {
    flex: 1;
  }

  .ctext b {
    display: block;
    font-size: 10.5px;
    color: var(--ink-2);
  }

  .ctext span {
    font-size: 9.5px;
    color: var(--ink-3);
  }

  .later {
    font: 700 8.5px var(--fd);
    letter-spacing: 0.08em;
    color: var(--ink-3);
  }

  .hr {
    height: 1px;
    background: var(--rule);
    margin: 12px 0;
  }

  .replay-row {
    display: grid;
    grid-template-columns: 1fr 40px 1fr;
    gap: 6px;
    align-items: center;
    height: 24px;
    border-bottom: 1px solid var(--rule-2);
    font-size: 10px;
    text-align: center;
    color: var(--ink-3);
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

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    margin: 0;
  }
</style>
