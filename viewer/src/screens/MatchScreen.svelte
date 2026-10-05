<!-- The match screen, ported from the Touchline Full Game sketch's Match live screen: the shell
     with the fixture in the header, the clock state in the date block and the one next action
     in the cyan block; the 66 px score strip; then two body columns. The left column holds
     the pitch, the playback row and the timeline; the side column holds four groups in this
     order: win probability (a stub), the commentary, the other grounds of the matchday on the
     player's clock, and the statistics.

     The window step places them. Standard: the side column is 300 to 440 px and the pitch is
     also limited by the window's height, so the playback row stays on screen. Wide: the side
     column takes 46% and its groups form two columns, the commentary beside the other
     grounds, win probability and the statistics. Large and huge: one side column of 380 to
     520 px and the pitch at its column's width. Compact: one column, the pitch at full width
     and the side groups in two columns under it. Each group is a size container. Each screen state (loading, kick-off, live, paused,
     error, first run, reconnecting, full time) draws the board's body for it. At full time the
     live layout stays, a FULL TIME tag sits under the score and on the stopped pitch, and the
     playback row holds only the replay controls. Parts the viewer does not
     build yet are stubs (MatchStub, the stub tabs): drawn, faded and inert. Skip to result in
     the playback row opens the Skip decision (`App.svelte`).
     The Touchline and Tactics tabs open those views over it (`App.svelte`); the match screen
     stays mounted and hidden, so the pitch keeps its canvas and the match plays on behind. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import Commentary from '../components/Commentary.svelte';
  import Crest from '../components/Crest.svelte';
  import MatchStats from '../components/MatchStats.svelte';
  import MatchStub from '../components/MatchStub.svelte';
  import MatchTimeline from '../components/MatchTimeline.svelte';
  import Notice from '../components/Notice.svelte';
  import OtherGrounds from '../components/OtherGrounds.svelte';
  import PitchCanvas from '../components/PitchCanvas.svelte';
  import PlaybackRow from '../components/PlaybackRow.svelte';
  import ScoreStrip from '../components/ScoreStrip.svelte';
  import StepList from '../components/StepList.svelte';
  import SurfacePanel from '../components/SurfacePanel.svelte';
  import { allEnded } from '../lib/matchday.js';
  import { clockAt } from '../lib/recovery.js';
  import { stripFacts } from '../lib/stats.js';

  let { session } = $props();

  /// The sub-navigation: Match, Touchline and Tactics are built; the others are stubs until their
  /// screens are ported.
  const TABS = [
    { id: 'match', label: 'Match', active: true },
    { id: 'touchline', label: 'Touchline' },
    { id: 'tactics', label: 'Tactics', menu: true },
    { id: 'squad', label: 'Squad', menu: true, stub: true },
    { id: 'stats', label: 'Stats', menu: true, stub: true },
    { id: 'analysis', label: 'Analysis', menu: true, stub: true },
    { id: 'other-grounds', label: 'Other grounds', menu: true, stub: true },
    { id: 'highlights', label: 'Highlights', menu: true, stub: true },
  ];

  /// The panel kinds, as SurfacePanel names them.
  const SURFACE_KIND = { 'first-run': 'setup', abandoned: 'abandoned' };
  const STEP_STATE = { done: 'done', active: 'current', waiting: 'pending' };

  let fileInput = $state();

  let screen = $derived(session.screen);
  let names = $derived(session.teams ? session.teams.map((t) => t['team.name']) : ['Home', 'Away']);
  let skippedAt = $derived(session.skip?.state === 'ready' ? clockAt(session.skip.from) : null);
  /// After a skip the strip's Possession cell says where the player skipped.
  let facts = $derived(
    stripFacts(session.stats).map((fact) =>
      fact.label === 'Possession' && skippedAt && fullTime ? { value: skippedAt, label: 'Skipped at' } : fact
    )
  );
  let steps = $derived(
    session.steps.map((s) => ({ label: s.label, state: STEP_STATE[s.state], word: s.word }))
  );
  let playing = $derived(screen === 'live' || screen === 'paused');
  let fullTime = $derived(session.isOver);
  /// The live layout: while the match plays, and at full time.
  let shown = $derived(session.underWay);
  let drawn = $derived(session.teams !== null && screen !== 'loading' && screen !== 'first-run');
  let panel = $derived(session.panel);
  /// At full time, with the pitch at or after the whistle and every ground ended, the other
  /// grounds read final, as the report's do.
  let groundsFinal = $derived(
    fullTime &&
      session.match.fullTimeTick !== null &&
      session.tick >= session.match.fullTimeTick &&
      allEnded(session.matchday)
  );

  function pickReplay() {
    fileInput?.click();
  }

  async function openFile() {
    const file = fileInput.files?.[0];
    if (!file) {
      return;
    }
    const bytes = new Uint8Array(await file.arrayBuffer());
    fileInput.value = '';
    await session.openReplay(bytes, file.name);
  }
</script>

<AppShell
  title={session.title}
  subtitle={session.subtitle}
  date={session.dateWord}
  dateSub={session.dateClock}
  action={session.action}
  onaction={() => session.act(pickReplay)}
  busy={session.actionBusy}
  tabs={TABS}
  ontab={(id) => session.show(id)}
  menu={session.onMenu}
  menuOpen={session.menuOpen}
>
  {#snippet crest()}
    <Crest team={session.teams?.[0] ?? null} />
  {/snippet}

  <div class="screen" data-screen={screen}>
    <ScoreStrip
      teams={session.teams}
      score={session.score}
      scorers={session.scorers}
      tag={session.tag}
      {facts}
    />

    <div class="cols">
      <div class="left">
        {#if shown}
          <MatchStub part="overlays" />
        {/if}

        <PitchCanvas
          attach={(canvas) => session.attachCanvas(canvas)}
          resize={(canvas, box) => session.resizeCanvas(canvas, box)}
          {drawn}
          dim={screen === 'reconnecting'}
          banner={session.banner}
          overlays={shown}
        >
          {#if screen === 'loading'}
            <div class="loading">
              <div class="steps">
                <h2>Getting the match ready</h2>
                <p>The pitch draws as soon as play starts.</p>
                <StepList {steps} />
              </div>
              <div class="skels" aria-hidden="true">
                <div class="skel big"></div>
                <div class="skel small"></div>
              </div>
            </div>
          {:else if panel && (screen === 'error' || screen === 'first-run')}
            <div class="cover">
              <SurfacePanel kind={SURFACE_KIND[panel.kind] ?? 'error'} word={panel.word} title={panel.title}>
                {#if panel.body}<p>{panel.body}</p>{/if}
                {#if panel.path !== undefined}<code>{panel.path}</code>{/if}
                {#if panel.instruction ?? panel.hint}<p>{panel.instruction ?? panel.hint}</p>{/if}
                {#snippet actions()}
                  {#each panel.actions as action (action)}
                    {#if action === 'restart'}
                      <button class="btn cy" type="button" disabled={session.busy} onclick={() => session.restartEngine()}
                        >{panel.restartLabel ?? 'Restart'}</button
                      >
                    {:else if action === 'abandon'}
                      <button class="btn gh" type="button" disabled={session.busy} onclick={() => session.abandonEngine()}
                        >Abandon</button
                      >
                    {:else if action === 'save-replay'}
                      <button
                        class="btn gh"
                        type="button"
                        disabled={session.saveBlocked !== null || session.saving}
                        onclick={() => session.saveReplay()}>Save replay</button
                      >
                    {:else if action === 'open-replay'}
                      <button class="btn gh" type="button" onclick={pickReplay}>Open a replay</button>
                    {/if}
                  {/each}
                {/snippet}
              </SurfacePanel>
            </div>
          {:else if fullTime}
            <span class="ftag">FULL TIME · {session.dateClock}</span>
          {/if}
        </PitchCanvas>

        {#if screen === 'reconnecting'}
          <div class="held">
            {#if session.notice}
              <Notice kind={session.notice.kind} word={session.notice.word} message={session.notice.message} />
            {/if}
            <p class="note">The pitch holds its last frame. Play resumes at the last stoppage.</p>
          </div>
        {:else if screen !== 'error' && screen !== 'first-run'}
          <PlaybackRow
            {fullTime}
            ontostart={() => session.rewind(session.history?.firstTick ?? 0)}
            onback10={() => session.step(-10)}
            onplayhere={() => session.playFromHere()}
            note={fullTime
              ? skippedAt
                ? `From ${skippedAt} the engine played on unwatched · the replay holds it`
                : 'The whole match is here · play it again from any minute'
              : undefined}
            playing={session.playing && screen === 'live'}
            speed={session.speed}
            disabled={screen === 'loading'}
            onprevious={() => session.previousStop()}
            ontoggle={() => session.setPlaying(!session.playing)}
            onlive={() => session.toNewest()}
            onnext={() => session.nextStop()}
            onspeed={(s) => session.selectSpeed(s)}
            canSkip={session.canSkip}
            onskip={() => session.openSkip()}
          />
          {#if session.notice && playing}
            <div class="notice">
              <Notice kind={session.notice.kind} word={session.notice.word} message={session.notice.message} />
            </div>
          {/if}
          {#if screen !== 'loading'}
            <MatchTimeline
              tick={session.tick}
              max={session.scrubMax}
              disabled={screen === 'kickoff'}
              skippedFrom={session.skip?.state === 'ready' ? session.skip.from : null}
              onscrub={(t) => session.scrubTo(t)}
              onrelease={() => session.scrubEnd()}
            />
          {/if}
          {#if shown}
            <MatchStub part="highlights" />
            <MatchStub part="momentum" />
          {/if}
        {/if}
      </div>

      <div class="side">
        {#if shown || screen === 'reconnecting'}
          <div class="grp grp-a">
            <MatchStub part="win-probability" />
          </div>
        {/if}
        <!-- Commentary Off in Settings hides the column; goals and cards still show on the
             pitch and in the score strip. -->
        {#if session.commentary}
          <div class="grp grp-b">
            <Commentary
              rows={session.feedRows}
              spoken={session.spoken}
              state={screen === 'loading' || screen === 'error'
                ? 'skeleton'
                : screen === 'first-run'
                  ? 'idle'
                  : 'rows'}
            />
          </div>
        {/if}
        {#if screen !== 'first-run'}
          <div class="grp grp-c">
            <OtherGrounds
              grounds={session.grounds}
              round={session.matchday?.round ?? 1}
              skeleton={screen === 'loading' || screen === 'error'}
              final={groundsFinal}
            />
          </div>
        {/if}
        {#if shown || screen === 'reconnecting'}
          <div class="grp grp-d">
            <MatchStats stats={session.stats} {names} />
          </div>
        {/if}
      </div>
    </div>
  </div>

  <input
    class="file"
    type="file"
    accept=".smfx"
    tabindex="-1"
    aria-hidden="true"
    bind:this={fileInput}
    onchange={openFile}
  />
</AppShell>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .cols {
    display: grid;
    grid-template-columns: minmax(0, 1fr) clamp(300px, 36%, 440px);
    grid-template-rows: minmax(0, 1fr);
    margin-top: 10px;
    flex: 1;
    min-height: 0;
  }

  .left {
    padding-right: 18px;
    min-width: 0;
  }

  /* At standard and wide the pitch is also limited by the window's height (the shell's
     height, less the header, the strip and the playback row), so the playback row stays on
     screen. The zoom is 1 at both steps. */
  .left :global(.pitchbox) {
    width: min(100%, calc((100cqb - 340px) * 742 / 312));
  }

  .side {
    border-left: 1px solid var(--rule);
    padding-left: 18px;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .grp {
    container-type: inline-size;
    min-width: 0;
    flex: none;
  }

  /* The commentary takes the column's spare height and scrolls within it. */
  .grp-b {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .grp + .grp {
    border-top: 1px solid var(--rule);
    margin-top: 8px;
    padding-top: 8px;
  }

  /* Wide: the side column takes 46%, and its groups form two columns (below). */
  @media (min-width: 1600px) {
    .cols {
      grid-template-columns: minmax(0, 1fr) minmax(0, 46%);
    }

    .side {
      grid-template-rows: auto auto minmax(0, 1fr);
    }
  }

  /* Wide and compact: the side groups form two columns: the commentary down the left, the
     other grounds, win probability and the statistics down the right. */
  @media (min-width: 1600px) and (max-width: 1919px), (max-width: 1023px), (max-height: 599px) {
    .side {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      grid-template-areas: 'b c' 'b a' 'b d';
      align-items: start;
    }

    .grp-a {
      grid-area: a;
    }

    .grp-b {
      grid-area: b;
      align-self: stretch;
      /* Its rows never size the grid: it takes the height the other groups give it. */
      contain: size;
      padding-right: 18px;
    }

    .grp-c {
      grid-area: c;
    }

    .grp-d {
      grid-area: d;
    }

    .grp-a,
    .grp-c,
    .grp-d {
      border-left: 1px solid var(--rule);
      padding-left: 18px;
    }

    .grp + .grp.grp-b,
    .grp + .grp.grp-c {
      border-top: 0;
      margin-top: 0;
      padding-top: 0;
    }

    .grp-a {
      border-top: 1px solid var(--rule);
      margin-top: 8px;
      padding-top: 8px;
    }
  }

  /* Large and huge: one side column of 380 to 520 px, and the pitch at its column's width. */
  @media (min-width: 1920px) {
    .cols {
      grid-template-columns: minmax(0, 1fr) clamp(380px, 28%, 520px);
    }

    .left :global(.pitchbox) {
      width: 100%;
    }
  }

  /* Compact: one column; the pitch at full width with the playback row under it, then the
     side groups in two columns (above). The body scrolls down to them. */
  @media (max-width: 1023px), (max-height: 599px) {
    .cols {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto;
      flex: none;
    }

    .left {
      padding-right: 0;
    }

    .left :global(.pitchbox) {
      width: 100%;
    }

    .side {
      grid-template-rows: auto auto auto;
      border-left: 0;
      padding-left: 0;
      border-top: 1px solid var(--rule);
      margin-top: 14px;
      padding-top: 14px;
    }
  }

  .loading {
    position: absolute;
    inset: 0;
  }

  .steps {
    position: absolute;
    left: 24px;
    top: 22px;
    right: calc(min(260px, 35%) + 40px);
    max-width: 380px;
  }

  .steps h2 {
    margin: 0;
    font: 800 20px var(--fd);
    text-transform: uppercase;
    letter-spacing: 0.02em;
    color: var(--ink);
  }

  .steps p {
    margin: 4px 0 14px;
    font-size: 10.5px;
    color: var(--ink-2);
  }

  /* On the dark pitch ground the step words and the marks of the steps to come take --ink-2:
     --ink-3 measures 2.69:1 there. The current step's mark is the step list's cyan dot, which
     measures 4.15:1 on the pitch, so the step list sits on the screen's ground. */
  .steps :global(.word),
  .steps :global(.pending .mark) {
    color: var(--ink-2);
  }

  .steps :global(ol),
  .steps :global(ul) {
    background: var(--ground);
    border-radius: var(--radius-sm);
    padding: 6px 8px;
  }

  .skels {
    position: absolute;
    right: 24px;
    top: 22px;
    bottom: 22px;
    width: min(260px, 35%);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .skel {
    border-radius: var(--radius-sm);
  }

  .skel.big {
    flex: 1;
    background: var(--pitch-skeleton);
  }

  .skel.small {
    height: 14px;
    width: 60%;
    background: var(--pitch-skeleton-2);
  }

  .cover {
    position: absolute;
    inset: 0;
  }

  .cover :global(.surface) {
    position: absolute;
    inset: 0;
  }

  .cover code {
    display: block;
    font: 500 11px var(--fm);
    background: var(--ground-2);
    color: var(--ink);
    padding: 6px 8px;
    border-radius: var(--radius-sm);
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
    opacity: 0.55;
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .btn {
      height: var(--hit);
      padding: 0 16px;
    }
  }

  .held {
    margin-top: 8px;
  }

  .notice {
    margin-top: 8px;
  }

  .note {
    margin: 0;
    font-size: 9.5px;
    color: var(--ink-3);
  }

  /* The full-time mark on the stopped pitch: the score strip's final tag, in the corner. */
  .ftag {
    position: absolute;
    right: 8px;
    bottom: 8px;
    z-index: 2;
    padding: 0 6px;
    border-radius: var(--radius-sm);
    background: var(--navy-900);
    color: var(--band-ink);
    font: 700 9.5px var(--fd);
    letter-spacing: 0.06em;
    line-height: 1.6;
    white-space: nowrap;
    pointer-events: none;
  }

  .file {
    display: none;
  }
</style>
