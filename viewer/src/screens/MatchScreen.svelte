<!-- The match screen, ported from the Touchline Full Game sketch's Match live screen: the shell
     with the fixture in the header, the clock state in the date block and the one next action
     in the cyan block; the 66 px score strip; then two body columns of 760 px and 1fr. The
     left column holds the pitch, the playback row and the timeline; the right the commentary,
     the statistics and the other grounds of the matchday on the player's clock. Each screen state (loading, kick-off, live, paused,
     error, first run, reconnecting) draws the board's body for it. Parts the viewer does not
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
  let facts = $derived(stripFacts(session.stats));
  let steps = $derived(
    session.steps.map((s) => ({ label: s.label, state: STEP_STATE[s.state], word: s.word }))
  );
  let playing = $derived(screen === 'live' || screen === 'paused');
  let drawn = $derived(session.teams !== null && screen !== 'loading' && screen !== 'first-run');
  let panel = $derived(session.panel);

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
        {#if playing}
          <MatchStub part="overlays" />
        {/if}

        <PitchCanvas
          attach={(canvas) => session.attachCanvas(canvas)}
          {drawn}
          dim={screen === 'reconnecting'}
          banner={session.banner}
          overlays={playing}
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
          {#if playing}
            <MatchStub part="highlights" />
            <MatchStub part="momentum" />
          {/if}
        {/if}
      </div>

      <div class="right">
        {#if playing || screen === 'reconnecting'}
          <MatchStub part="win-probability" />
          <div class="hr"></div>
        {/if}
        <!-- Commentary Off in Settings hides the column; goals and cards still show on the
             pitch and in the score strip. -->
        {#if session.commentary}
          <Commentary
            rows={session.feedRows}
            spoken={session.spoken}
            state={screen === 'loading' || screen === 'error'
              ? 'skeleton'
              : screen === 'first-run'
                ? 'idle'
                : 'rows'}
          />
        {/if}
        {#if screen !== 'first-run'}
          {#if session.commentary}<div class="hr"></div>{/if}
          <OtherGrounds
            grounds={session.grounds}
            round={session.matchday?.round ?? 1}
            skeleton={screen === 'loading' || screen === 'error'}
          />
        {/if}
        {#if playing || screen === 'reconnecting'}
          <div class="hr"></div>
          <MatchStats stats={session.stats} {names} />
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
    grid-template-columns: 760px 1fr;
    grid-template-rows: minmax(0, 1fr);
    margin-top: 10px;
    flex: 1;
    min-height: 0;
  }

  .left {
    padding-right: 18px;
    min-width: 0;
  }

  .right {
    border-left: 1px solid var(--rule);
    padding-left: 18px;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .hr {
    border-top: 1px solid var(--rule);
    margin: 8px 0;
    flex: none;
  }

  .loading {
    position: absolute;
    inset: 0;
  }

  .steps {
    position: absolute;
    left: 24px;
    top: 22px;
    width: 380px;
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

  /* On the dark pitch ground the step words take --ink-2: --ink-3 measures 2.69:1 there. */
  .steps :global(.word) {
    color: var(--ink-2);
  }

  .skels {
    position: absolute;
    right: 24px;
    top: 22px;
    bottom: 22px;
    width: 260px;
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

  .file {
    display: none;
  }
</style>
