<!-- The half-time and full-time report, ported from the sketch's Post-match report board: the
     shell with the result in the header and CONTINUE in the cyan block; the 66 px score strip;
     "How you saw it" with the match timeline and its goals and cards; then four columns of
     270, 330, 1fr and 290 px. The match figures are paired bars counted from the page's one
     event list, so the report never disagrees with the feed; the goals and cards list them
     one by one. The other grounds list the matchday's other results, final once every
     ground has ended; a ground still running shows its minute, and its later events appear
     as they arrive. Highlights, "What each change did" and "What it means" are stubs. At full
     time the last column holds the Next list (New match and Return to start, on a page that
     opened on the start screen), Replay the whole match, Save replay, Open a replay and Back
     to the match at full time, with "What it means" under them; the cyan block reads NEW
     MATCH, or CONTINUE on a page with no start screen. While a played-through match is still
     being stored the report is loading: the steps and skeleton blocks of the Report loading
     board, and Save replay waits.

     After Skip to result the report first shows the engine playing the rest (the Report
     loading board for a skip): the scores hidden, the skip point and the newest minute in
     the strip, and four steps; while the whole match is stored, the ready layout with every
     action that could drop the match disabled (the Report storing board); then, ready, the
     base report with the skip marks of the
     Report after a skip board: "skipped from" in the header, the skip cell, the timeline
     hatched after the skip point, and NOT LIVE on every later goal and card. A report with
     no skip renders as before. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import Crest from '../components/Crest.svelte';
  import Glyph from '../components/Glyph.svelte';
  import OtherGrounds from '../components/OtherGrounds.svelte';
  import PairedBar from '../components/PairedBar.svelte';
  import ReportMoments from '../components/ReportMoments.svelte';
  import NextSteps from '../components/NextSteps.svelte';
  import ScoreStrip from '../components/ScoreStrip.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import StepList from '../components/StepList.svelte';
  import StubSection from '../components/StubSection.svelte';
  import { PLAY } from '../components/icons.js';
  import { KIND } from '../lib/match-state.js';
  import { allEnded, finalSummary, groundsAt, headline } from '../lib/matchday.js';
  import { clockAt } from '../lib/recovery.js';
  import { TICKS_PER_SECOND } from '../lib/schedule.js';
  import { fixtureTitle, initials } from '../lib/scoreboard.js';
  import { minuteOf, notLive, skippedWord, skipSteps, totalMinutes } from '../lib/skip.js';
  import { stripFacts } from '../lib/stats.js';

  let { session } = $props();

  const MINUTE = TICKS_PER_SECOND * 60;
  const MARKS = [0, 15, 30, 45, 60, 75, 90];
  // The timeline's drawn width and its inset, as the board draws them.
  const TRACK = { x: 10, width: 1140 };
  // The width the skip words take after the skip point, "SKIPPED AT 00:00 · NOT WATCHED LIVE →".
  const SKIP_WORDS_WIDTH = 230;

  let report = $derived(session.report);
  let full = $derived(report?.kind === KIND.fullTime);
  let loading = $derived(report?.state === 'loading');
  let model = $derived(report?.model ?? { score: [0, 0], rows: [], moments: [], teams: [] });
  let names = $derived(session.teams ? session.teams.map((t) => t['team.name']) : ['Home', 'Away']);
  let breakWord = $derived(full ? 'Full time' : 'Half-time');

  /// The skip point after Skip to result, or null; `playingRest` while the engine plays the
  /// rest. Once full time is reached the whole match is stored (`storing`), and the report
  /// shows its final figures with its actions waiting.
  let skippedFrom = $derived(report?.skippedFrom ?? null);
  let skipped = $derived(skippedFrom !== null);
  let playingRest = $derived(skipped && report?.state === 'playing-rest');
  let storing = $derived(report?.state === 'storing');
  /// The full-time next steps: offered on a page that opened on the start screen, and ready
  /// once the match is stored.
  let nextOffered = $derived(full && session.nextOffered);
  let nextReady = $derived(session.nextReady);
  let skipClock = $derived(skipped ? clockAt(skippedFrom) : '');
  let total = $derived(totalMinutes(session.hello?.ticks_expected, session.hello?.knockout === true));
  let newestMinute = $derived(Math.min(total, minuteOf(session.skip?.newest ?? skippedFrom ?? 0)));
  let restSteps = $derived(
    playingRest ? skipSteps(skippedFrom, session.skip?.newest ?? skippedFrom, total, 'playing') : []
  );

  /// The sub-navigation: Report is this view; Replay opens the replay at full time. The rest
  /// are stubs until their screens are built.
  let tabs = $derived([
    { id: 'report', label: 'Report', active: true },
    { id: 'ratings', label: 'Ratings', stub: true },
    { id: 'causes', label: 'Causes', menu: true, stub: true },
    { id: 'highlights', label: 'Highlights', menu: true, stub: true },
    { id: 'stats', label: 'Stats', menu: true, stub: true },
    { id: 'press', label: 'Press conference', menu: true, stub: true },
    { id: 'replay', label: 'Replay', menu: true, stub: !full || playingRest || storing },
  ]);

  let facts = $derived.by(() => {
    const base = stripFacts(session.stats);
    if (playingRest) {
      return [
        { value: skipClock, label: 'Skipped at' },
        { value: `${newestMinute}' of ${total}`, label: 'Engine at full speed' },
      ];
    }
    if (skipped) {
      return [
        base[0],
        { value: `Skipped at ${skipClock}`, label: 'The engine played the rest', flex: 0.95 },
        {
          value: headline(session.matchday),
          label: session.matchday?.fixtures.length ? `Other grounds: ${finalSummary(session.matchday)}` : 'No other matches',
          flex: 0.85,
        },
      ];
    }
    return base;
  });

  /// The other grounds on the report: at the break for half time; at full time every event
  /// that has arrived, since the player's match is over and the others finish after it.
  let grounds = $derived(
    groundsAt(session.matchday, full ? Number.MAX_SAFE_INTEGER : (report?.tick ?? 0), {
      final: true,
      total: totalMinutes(session.hello?.ticks_expected),
      stored: session.stored,
    })
  );

  /// The timeline spans the whole match: 90 minutes, or longer when the match ran on.
  let span = $derived(Math.max(90 * MINUTE, report?.tick ?? 0));
  const at = (tick) => TRACK.x + (Math.min(tick, span) / span) * TRACK.width;
  /// The minute marks; after a skip, the marks under the skip words are left out, so the
  /// words never sit on a number.
  let marks = $derived(
    MARKS.map((m) => ({ m, x: at(m * MINUTE) })).filter(
      (mark) => skippedFrom === null || mark.x < at(skippedFrom) - 14 || mark.x > at(skippedFrom) + SKIP_WORDS_WIDTH
    )
  );
  let markers = $derived(
    model.moments.map((m) => ({
      x: at(m.tick),
      hollow: notLive(m, skippedFrom),
      letter: m.kind === 'Goal' ? 'G' : m.kind === 'Yellow card' ? 'Y' : 'R',
      tone: m.kind === 'Goal' ? (m.side === 1 ? 'away' : 'home') : m.kind === 'Yellow card' ? 'yellow' : 'red',
      key: `${m.tick}-${m.kind}-${m.side}`,
    }))
  );
  let timelineLabel = $derived.by(() => {
    const seen = skipped
      ? `Timeline: you watched 0 to ${skipClock} live; the engine played ${skipClock} to full time after you skipped`
      : `Timeline: you watched 0 to ${clockAt(report?.tick ?? 0)} live`;
    if (model.moments.length === 0) {
      return `${seen}, with no goals or cards.`;
    }
    return `${seen}. ${model.moments.map((m) => `${m.kind} at ${m.minute}`).join(', ')}.`;
  });

  const STEPS = [
    { label: 'Write the report', state: 'done', word: 'Figures, goals and cards' },
    { label: 'Store the whole match', state: 'current', word: "The replay from 0' to full time", progress: 90 },
  ];

  let fileInput = $state();

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

  function tab(id) {
    if (id === 'replay') {
      session.showReplay();
    }
  }
</script>

<AppShell
  title={playingRest ? 'Skip to result' : fixtureTitle(names, model.score)}
  subtitle={playingRest
    ? `${fixtureTitle(names)} · the engine plays the rest`
    : skipped
      ? `${breakWord} · ${skippedWord(skippedFrom)}`
      : `${breakWord} · report`}
  date={playingRest ? 'PLAYING THE REST' : breakWord.toUpperCase()}
  dateSub={playingRest ? `from ${skipClock}` : clockAt(report?.tick ?? 0)}
  action={playingRest ? 'Please wait' : nextOffered ? 'New match' : 'Continue'}
  busy={playingRest || (nextOffered && !nextReady)}
  onaction={() => (playingRest ? null : nextOffered ? session.nextStep('new') : session.closeReport())}
  {tabs}
  ontab={tab}
  navLabel="Report views"
>
  {#snippet crest()}
    <Crest team={session.teams?.[0] ?? null} />
  {/snippet}

  <div
    class="screen"
    data-screen="report"
    data-kind={report?.kind}
    data-state={report?.state}
    data-skipped={skipped ? skippedFrom : undefined}
  >
    <ScoreStrip
      teams={session.teams}
      score={model.score}
      scorers={playingRest ? ['Scores hidden until full time', 'Scores hidden until full time'] : session.scorers}
      hidden={playingRest}
      tag={{
        text:
          playingRest
            ? `PLAYING THE REST · ${newestMinute}'`
            : loading
              ? 'FULL TIME · STORING'
              : breakWord.toUpperCase(),
        tone: 'cyan',
        live: false,
      }}
      {facts}
    />

    {#if playingRest}
      <div class="cols loading">
        <div>
          <SectionLabel
            label="Playing the rest of the match"
            note={session.engineVersion ? `engine ${session.engineVersion}` : ''}
          />
          <StepList steps={restSteps} />
          <p class="g">No cancel: the match ends the same way either way, and the replay keeps all of it.</p>
        </div>
        <div class="skels" aria-hidden="true">
          <div class="skel" style:height="50px" style:margin-bottom="10px"></div>
          <div class="skelcols">
            {#each [0, 1, 2, 3] as c (c)}
              <div>
                <div class="skel" style:height="12px" style:width="60%" style:margin-bottom="10px"></div>
                {#each [0, 1, 2, 3, 4, 5, 6] as r (r)}
                  <div class="skel" style:height="14px" style:margin-bottom="8px"></div>
                {/each}
              </div>
            {/each}
          </div>
        </div>
      </div>
    {:else if loading}
      <div class="cols loading">
        <div>
          <SectionLabel label="Storing the whole match" note={session.engineWord} />
          <StepList steps={STEPS} />
          <p class="g">No cancel: the engine is closing the match, and the replay keeps all of it.</p>
          <div class="actions">
            <button class="btn" type="button" disabled>Save replay</button>
            <p class="g reason">{session.saveBlocked}</p>
          </div>
        </div>
        <div class="skels" aria-hidden="true">
          <div class="skel" style:height="50px" style:margin-bottom="10px"></div>
          <div class="skelcols">
            {#each [0, 1, 2, 3] as c (c)}
              <div>
                <div class="skel" style:height="12px" style:width="60%" style:margin-bottom="10px"></div>
                {#each [0, 1, 2, 3, 4, 5, 6] as r (r)}
                  <div class="skel" style:height="14px" style:margin-bottom="8px"></div>
                {/each}
              </div>
            {/each}
          </div>
        </div>
      </div>
    {:else}
      <div class="saw">
        {#if skipped}
          <SectionLabel
            label="How you saw it"
            note="0' to {skipClock} live · the rest played by the engine from the exact moment you skipped"
          />
        {:else}
          <SectionLabel label="How you saw it" note="0' to {clockAt(report?.tick ?? 0)} live" />
        {/if}
      </div>
      {#if skipped}
        <svg class="timeline" width="1160" height="44" role="img" aria-label={timelineLabel}>
          <defs>
            <pattern id="skip-hatch" width="8" height="8" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
              <rect class="h1" width="4" height="8"></rect>
              <rect class="h2" x="4" width="4" height="8"></rect>
            </pattern>
          </defs>
          <rect class="hatched" x={TRACK.x} y="18" width={TRACK.width} height="7" rx="3"></rect>
          <rect class="seen" x={TRACK.x} y="18" width={at(skippedFrom) - TRACK.x} height="7" rx="3"></rect>
          <line class="head" x1={at(skippedFrom)} y1="10" x2={at(skippedFrom)} y2="32"></line>
          <text class="unseen" x={at(skippedFrom) + 6} y="40">SKIPPED AT {skipClock} · NOT WATCHED LIVE →</text>
          {#each markers as m (m.key)}
            <circle class="mk {m.tone}" class:hollow={m.hollow} cx={m.x} cy="9" r="6.5"></circle>
            <text class="mt {m.tone}" class:hollow={m.hollow} x={m.x} y="12" text-anchor="middle">{m.letter}</text>
          {/each}
          {#each marks as mark (mark.m)}
            <text class="min" x={mark.x} y="40" text-anchor="middle">{mark.m}'</text>
          {/each}
        </svg>
      {:else}
        <svg class="timeline" width="1160" height="44" role="img" aria-label={timelineLabel}>
          <rect class="track" x={TRACK.x} y="18" width={TRACK.width} height="7" rx="3"></rect>
          <rect class="seen" x={TRACK.x} y="18" width={at(report?.tick ?? 0) - TRACK.x} height="7" rx="3"></rect>
          <line class="head" x1={at(report?.tick ?? 0)} y1="10" x2={at(report?.tick ?? 0)} y2="32"></line>
          {#each markers as m (m.key)}
            <circle class="mk {m.tone}" cx={m.x} cy="9" r="6.5"></circle>
            <text class="mt {m.tone}" x={m.x} y="12" text-anchor="middle">{m.letter}</text>
          {/each}
          {#each marks as mark (mark.m)}
            <text class="min" x={mark.x} y="40" text-anchor="middle">{mark.m}'</text>
          {/each}
        </svg>
      {/if}

      <div class="cols ready">
        <div>
          <SectionLabel label="Match figures" note="{initials(names[0])} · {initials(names[1])}" />
          {#each model.rows as row (row.id)}
            <PairedBar label={row.label} home={row.counts[0]} away={row.counts[1]} />
          {/each}
        </div>

        <div>
          <SectionLabel label="Goals and cards" />
          <ReportMoments moments={model.moments} teams={session.teams} {skippedFrom} />
          <div class="hr"></div>
          <OtherGrounds
            {grounds}
            round={session.matchday?.round ?? 1}
            report
            final={full && allEnded(session.matchday)}
          />
        </div>

        <div>
          <SectionLabel label="Highlights" note="ranked by win-probability swing" later />
          <!-- STUB: highlights ranked by win-probability swing need a win-probability model the
               engine does not have. Drawn for layout and feel only. -->
          <StubSection note="highlights">
            {#each [["84'", 'GOAL Palova', '+38%', 'ok'], ["58'", 'GOAL Hask, near-post header', '−22%', 'bd'], ["23'", "GOAL Oduya from Morrow's cut-back", '+16%', 'ok'], ["88'", "Reyna saves Ferrie's low shot", '+9%', 'ok']] as [min, text, swing, tone] (min)}
              <div class="hl"><b class="num min">{min}</b><span>{text}</span><b class="num {tone}">{swing}</b></div>
            {/each}
          </StubSection>
          <div class="hr"></div>
          <SectionLabel label="What each change did" later />
          <!-- STUB: the effect of each change needs a model the engine does not have. -->
          <StubSection note="what each change did">
            <div class="ch"><b class="num min">69'</b><span><b>Aydin on for Hart</b><br /><span class="g">Regains in their half 2 → 7</span></span></div>
            <div class="ch"><b class="num min">78'</b><span><b>Palova for Sandvik</b><br /><span class="g">Scored; our left went quiet</span></span></div>
          </StubSection>
        </div>

        <div>
          {#if full}
            {#if nextOffered}
              <NextSteps ready={nextReady} onchoose={(id) => session.nextStep(id)} />
              <div class="hr"></div>
            {/if}
            <div class="actions">
              <!-- One cyan element per screen: with the Next list on show, New match is it. -->
              <button
                class="btn tall"
                class:cy={!nextOffered}
                type="button"
                disabled={!nextReady}
                onclick={() => session.replayWhole()}
              >
                <Glyph glyph={{ d: PLAY }} size={10} /> Replay the whole match
              </button>
              <div class="pair">
                <!-- With the Next list, Replay is the one navy fill: Save replay is outlined. -->
                <button
                  class="btn"
                  class:gh={nextOffered}
                  type="button"
                  disabled={!nextReady || session.saveBlocked !== null || session.saving}
                  onclick={() => session.saveReplay()}>Save replay</button
                >
                <button class="btn gh" type="button" onclick={pickReplay}>Open a replay</button>
              </div>
              <button class="btn gh" type="button" onclick={() => session.closeReport()}>Back to the match at full time</button>
              {#if session.saved}
                <p class="g saved" role="status">Saved {session.saved.name}</p>
              {/if}
            </div>
            <div class="hr"></div>
          {/if}
          <SectionLabel label="What it means" later />
          <!-- STUB: what the result means (the table, the board, the next match) needs a season
               the engine does not have. -->
          <StubSection note="what it means">
            <div class="kv"><span>League table</span><b>Up to 2nd</b></div>
            <div class="kv"><span>Board confidence</span><b>Good ► Good</b></div>
            <div class="kv"><span>Next</span><b>Harlow Vale (A) · Sat 21 Nov</b></div>
          </StubSection>
        </div>
      </div>
    {/if}
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

  .saw {
    margin-top: 10px;
  }

  .saw :global(.sl) {
    justify-content: flex-start;
    margin-bottom: 0;
  }

  .timeline {
    display: block;
    flex: none;
  }

  .track {
    fill: var(--bar-track);
  }

  .seen {
    fill: var(--cyan);
  }

  .head {
    stroke: var(--ink);
    stroke-width: 2;
  }

  .mk.home {
    fill: var(--cyan);
  }

  .mk.away,
  .mk.red {
    fill: var(--bad);
  }

  .mk.yellow {
    fill: var(--mid);
  }

  .mt {
    font: 700 8px var(--fb);
  }

  .mt.home {
    fill: var(--cyan-ink);
  }

  .mt.away,
  .mt.red {
    fill: var(--on-bad);
  }

  .mt.yellow {
    fill: var(--on-mid);
  }

  .min {
    fill: var(--ink-3);
    font: 400 9px var(--fb);
  }

  /* After a skip: the board's hatch under the track, the skip words, and hollow markers for
   * the goals and cards nobody watched live (the state colour as the ring and the letter). */
  .h1 {
    fill: var(--hatch-1);
  }

  .h2 {
    fill: var(--hatch-2);
  }

  .hatched {
    fill: url(#skip-hatch);
  }

  .unseen {
    fill: var(--ink-2);
    font: 700 9.5px var(--fd);
    letter-spacing: 0.06em;
  }

  .mk.hollow {
    fill: var(--ground);
    stroke-width: 2;
  }

  .mk.hollow.home {
    stroke: var(--cyan);
  }

  .mk.hollow.away,
  .mk.hollow.red {
    stroke: var(--bad);
  }

  .mk.hollow.yellow {
    stroke: var(--mid);
  }

  .mt.hollow.home {
    fill: var(--cyan);
  }

  .mt.hollow.away,
  .mt.hollow.red {
    fill: var(--bad);
  }

  .mt.hollow.yellow {
    fill: var(--mid);
  }

  .cols {
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    flex: 1;
    min-height: 0;
  }

  .cols.ready {
    grid-template-columns: 270px 330px 1fr 290px;
    margin-top: 6px;
  }

  /* As the board draws its columns: each keeps 18 px on its right, and each after the first
   * is split from the one before by a rule. */
  .cols.ready > div {
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    padding-right: 18px;
  }

  .cols.ready > div + div {
    border-left: 1px solid var(--rule);
    padding-left: 18px;
  }

  .cols.loading {
    grid-template-columns: 400px 1fr;
    gap: 36px;
    margin-top: 12px;
  }

  .skel {
    background: var(--skeleton);
    border-radius: var(--radius-sm);
  }

  .skelcols {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 18px;
  }

  .hr {
    border-top: 1px solid var(--rule);
    margin: 10px 0;
  }

  .g {
    margin: 0;
    font-size: 9.5px;
    color: var(--ink-3);
  }

  .hl {
    display: grid;
    grid-template-columns: 26px 1fr 40px;
    gap: 5px;
    align-items: center;
    min-height: 21px;
    border-bottom: 1px solid var(--rule-2);
    font-size: 10px;
    color: var(--ink);
  }

  .hl b:last-child {
    text-align: right;
  }

  .ch {
    display: grid;
    grid-template-columns: 28px 1fr;
    gap: 6px;
    padding: 2px 0;
    border-bottom: 1px solid var(--rule-2);
    font-size: 10px;
    color: var(--ink);
  }

  .num.min {
    font: 700 10px var(--fd);
  }

  .ok {
    color: var(--good);
  }

  .bd {
    color: var(--bad);
  }

  .kv {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    padding: 3px 0;
    color: var(--ink-2);
    font-size: 10.5px;
  }

  .kv b {
    color: var(--ink);
    font-weight: 600;
  }

  .actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .loading .actions {
    margin-top: 12px;
    align-items: flex-start;
  }

  .pair {
    display: flex;
    gap: 6px;
  }

  .pair .btn {
    flex: 1;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 24px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-sm);
    font: 600 10px var(--fb);
    cursor: pointer;
    color: var(--band-ink);
    background: var(--navy-600);
  }

  .btn.tall {
    height: 28px;
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

  .reason,
  .saved {
    font-size: 9.5px;
  }

  .file {
    display: none;
  }
</style>
