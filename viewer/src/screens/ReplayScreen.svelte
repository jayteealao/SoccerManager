<!-- The replay, ported from the sketch's Match Analysis replay board: the shell with MATCH
     ANALYSIS in the header and CONTINUE in the cyan block, which goes back to the report or
     the match; then two columns of 770 px and 1fr. The left column holds the pitch at
     752 × 290, drawn from the stored match at the rendered tick, the timeline over every
     stored tick, and the playback row: Back 10 seconds, play or pause, Forward 10 seconds and
     today's 1× to 8× speeds. The view toggles, 0.25× and 0.5×, Clip this moment, Add to video
     session, "Your screen" and the whole right column (the catalogue, the xG box, analyst
     depth and the traced figure) are stubs. After Skip to result the timeline is hatched
     from the skip point to the end, and inside that part the pitch tag adds NOT WATCHED
     LIVE; with no skip both are as before. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import Crest from '../components/Crest.svelte';
  import Glyph from '../components/Glyph.svelte';
  import MatchTimeline from '../components/MatchTimeline.svelte';
  import PitchCanvas from '../components/PitchCanvas.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import StubSection from '../components/StubSection.svelte';
  import { FAST_FORWARD, PAUSE, PLAY, REWIND } from '../components/icons.js';
  import { SPEEDS } from '../lib/playback.js';
  import { fixtureTitle } from '../lib/scoreboard.js';

  let { session } = $props();

  const TABS = [
    { id: 'replay', label: 'Replay', active: true },
    { id: 'screens', label: 'Your screens', stub: true },
    { id: 'catalogue', label: 'Catalogue', menu: true, stub: true },
    { id: 'trends', label: 'Trends', menu: true, stub: true },
    { id: 'opposition', label: 'Opposition', menu: true, stub: true },
    { id: 'glossary', label: 'Glossary', menu: true, stub: true },
  ];

  const TOGGLES = [
    ['Marking jobs', true],
    ['Runs', true],
    ['Pitch control', false],
    ['Options the taker saw', false],
  ];

  const CATALOGUE = [
    ['Shooting and xG', 22],
    ['Passing and progression', 31],
    ['Pressing and regains', 14],
    ['Set pieces', 16],
    ['Goalkeeping', 12],
    ['Running and fatigue', 15],
    ['Shape and space', 18],
    ['Model figures', 16],
    ['Visualisations', 36],
  ];

  const DEPTH = [
    ['Shot maps, xG timeline, pass maps, heat maps', 'Level 1', true],
    ['Pass networks, pressing and regain maps', 'Level 2', true],
    ['Pitch control and marking jobs in replays', 'Level 3 · Senior analyst', false],
    ['Decision quality: the option a player missed', 'Level 4 · Head of analysis', false],
    ['Opponent decisions from their recent matches', 'Level 4 · Head of analysis', false],
  ];

  let names = $derived(session.teams ? session.teams.map((t) => t['team.name']) : ['Home', 'Away']);
  let empty = $derived(!session.history || session.history.count === 0);
  let source = $derived(session.stored ? 'from the replay file' : 'from the match record');
  /// The skip point of the match on show, once the engine has played the rest; null with no skip.
  let skippedFrom = $derived(session.skip?.state === 'ready' ? session.skip.from : null);
  let unseen = $derived(skippedFrom !== null && session.tick > skippedFrom);
</script>

<AppShell
  section="analysis"
  title="Match Analysis"
  subtitle="{fixtureTitle(names, session.score)} · Replay"
  date="REPLAY"
  dateSub={session.clockText}
  action="Continue"
  onaction={() => session.closeReplay()}
  tabs={TABS}
  navLabel="Analysis views"
>
  {#snippet crest()}
    <Crest team={session.teams?.[0] ?? null} />
  {/snippet}

  <div class="screen" data-screen="replay">
    <!-- Goals and cards are read out as the replay reaches them, as on the match screen: the
         canvas alone carries no text. -->
    <p class="vh" aria-live="polite" data-replay-spoken>{session.spoken}</p>
    <div class="cols">
      <div class="left">
        <div class="head">
          <SectionLabel label="Replay" note="{session.clockText} · {source}" />
          <!-- STUB: the replay's view toggles need marking jobs, runs, pitch control and the
               taker's options, which the engine does not record. -->
          <StubSection note="replay view toggles" inline>
            {#each TOGGLES as [label, on] (label)}<span class="tg" class:on>{label}</span>{/each}
          </StubSection>
        </div>

        <PitchCanvas
          width={752}
          height={290}
          attach={(canvas) => session.attachCanvas(canvas, 'replay')}
          resize={(canvas, box) => session.resizeCanvas(canvas, box)}
          detach={(canvas) => session.detachCanvas(canvas, 'replay')}
          drawn={!empty}
          overlays={false}
        >
          <span class="tagpos"
            ><span class="tagc">REPLAY · {session.clockText}{unseen ? ' · NOT WATCHED LIVE' : ''}</span></span
          >
        </PitchCanvas>

        <MatchTimeline
          tick={session.tick}
          max={session.scrubMax}
          disabled={empty}
          {skippedFrom}
          onscrub={(t) => session.scrubTo(t)}
          onrelease={() => session.scrubEnd()}
        />

        <div class="row" role="group" aria-label="Playback">
          <button class="ib" type="button" aria-label="Back 10 seconds" disabled={empty} onclick={() => session.step(-10)}>
            <Glyph glyph={{ d: REWIND }} size={11} />
          </button>
          <button
            class="ib"
            class:on={session.playing}
            type="button"
            aria-label={session.playing ? 'Pause' : 'Play'}
            disabled={empty}
            onclick={() => session.setPlaying(!session.playing)}
          >
            <Glyph glyph={{ d: session.playing ? PAUSE : PLAY }} size={11} />
          </button>
          <button class="ib" type="button" aria-label="Forward 10 seconds" disabled={empty} onclick={() => session.step(10)}>
            <Glyph glyph={{ d: FAST_FORWARD }} size={11} />
          </button>
          <span class="seg" role="group" aria-label="Speed">
            <!-- STUB: slow motion needs playback below real time, which the scheduler does not
                 have. -->
            <StubSection note="slow motion speeds" inline fade={false}>
              <span class="part stubbed">0.25×</span><span class="part stubbed">0.5×</span>
            </StubSection>
            {#each SPEEDS as s (s)}
              <button
                type="button"
                class="part"
                class:on={s === session.speed}
                aria-pressed={s === session.speed}
                aria-label="{s}x"
                disabled={empty}
                onclick={() => session.selectSpeed(s)}>{s}×</button
              >
            {/each}
          </span>
          <!-- STUB: clips and video sessions need a clip store the viewer does not have. -->
          <StubSection note="clip this moment and video session" inline>
            <span class="btn dk">Clip this moment</span><span class="btn dk">Add to video session</span>
          </StubSection>
        </div>

        <SectionLabel label="Your screen: Set pieces" note="4 of 6 panels · drag to arrange" rule later />
        <!-- STUB: analysis screens need the statistics catalogue, which the engine does not
             compute. Drawn for layout and feel only. -->
        <StubSection note="your screen">
          <div class="panels">
            <div class="panel"><h5>Shot map · xG</h5><div class="shotmap"></div></div>
            <div class="panel">
              <h5>Corners</h5>
              <div class="kv"><span>Won</span><b class="num">8 – 3</b></div>
              <div class="kv"><span>First contact</span><b class="num">3 – 3</b></div>
              <div class="kv"><span>xG from corners</span><b class="num">0.18 – 0.46</b></div>
            </div>
            <div class="panel">
              <h5>Aerial duels</h5>
              <div class="kv"><span>Vell</span><b class="num">2 of 7</b></div>
              <div class="kv"><span>Okonjo</span><b class="num">5 of 6</b></div>
              <div class="kv"><span>Delmas</span><b class="num">4 of 6</b></div>
            </div>
            <div class="lock"><b>Marking-job breakdown</b><span>Needs a Senior analyst (level 3).</span></div>
          </div>
        </StubSection>
      </div>

      <div class="right">
        <SectionLabel label="Catalogue" note="180 statistics and visualisations" later />
        <!-- STUB: the catalogue, the definitions, analyst depth and traced figures need the
             statistics catalogue and staff, which the engine does not have. Drawn for layout
             and feel only. -->
        <StubSection note="catalogue">
          <div class="inp">Search: "near post"</div>
          <div class="cat">
            {#each CATALOGUE as [label, count] (label)}
              <div class="kv rule"><span>{label}</span><b class="num">{count}</b></div>
            {/each}
            <div class="kv rule"><span class="cy">All</span><b class="num cy">180</b></div>
          </div>
          <div class="box">
            <div class="kv"><b class="w">Expected goals (xG)</b><span class="btn">+ Add</span></div>
            <p class="def">
              How likely a shot like this is to be scored, from where it was taken, the angle, body
              part, pressure and type of assist.
            </p>
            <div class="kv"><span>Tonight</span><b class="num">1.96 – 1.04</b></div>
          </div>
        </StubSection>
        <SectionLabel label="Analyst depth" note="analyst level 2 of 4" later />
        <StubSection note="analyst depth">
          {#each DEPTH as [label, level, open] (label)}
            <div class="depth"><b class:ok={open}>{open ? '✓' : '·'}</b><span>{label}</span><span class="lv">{level}</span></div>
          {/each}
        </StubSection>
        <SectionLabel label="Traced figure" note="every number opens its moments" later />
        <StubSection note="traced figure">
          <div class="traced"><b>Near-post header wins, Port Varrow: 3</b></div>
          <div class="traced"><b class="cy num">55'</b><span>Hask v Vell · header over</span></div>
          <div class="traced"><b class="cy num">58'</b><span>Hask v Vell · goal</span></div>
        </StubSection>
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

  /* The board's 770 px and the rest at 1280, as shares, so both columns grow together. */
  .cols {
    display: grid;
    grid-template-columns: minmax(0, 770fr) minmax(0, 425fr);
    grid-template-rows: minmax(0, 1fr);
    margin-top: 8px;
    flex: 1;
    min-height: 0;
  }

  .left {
    padding-right: 18px;
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
  }

  .right {
    border-left: 1px solid var(--rule);
    padding: 0 18px;
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
  }

  .head {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  .head :global(.sl) {
    flex: 1;
    justify-content: flex-start;
  }

  .tg {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 19px;
    padding: 0 7px;
    margin-left: 4px;
    border-radius: var(--radius-sm);
    font-size: 9.5px;
    background: var(--toggle-ground);
    box-shadow: inset 0 0 0 1px var(--toggle-edge);
    color: var(--toggle-ink);
    white-space: nowrap;
  }

  .tg.on {
    background: var(--toggle-on-ground);
    box-shadow: inset 0 0 0 1px var(--cyan);
    color: var(--ink);
  }

  .tg.on::before {
    content: '✓';
    color: var(--cyan);
    font-weight: 800;
  }

  .tagpos {
    position: absolute;
    left: 8px;
    top: 8px;
  }

  .tagc {
    display: inline-block;
    background: var(--cyan);
    color: var(--cyan-ink);
    font: 700 9.5px var(--fd);
    letter-spacing: 0.06em;
    padding: 0 6px;
    border-radius: var(--radius-sm);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: 2px;
  }

  .ib {
    position: relative;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: inline-grid;
    place-items: center;
    background: transparent;
    border: 1px solid var(--control-edge);
    color: var(--control-ink);
    cursor: pointer;
    padding: 0;
  }

  /* The drawn button is 24 px; the hit area is 40 px tall and 30 px wide. */
  .ib::after {
    content: '';
    position: absolute;
    inset: -8px -3px;
  }

  .ib.on {
    background: var(--cyan);
    color: var(--cyan-ink);
    border-color: var(--cyan);
  }

  .seg {
    display: inline-flex;
    border: 1px solid var(--seg-edge);
    border-radius: var(--radius-sm);
  }

  /* A part is drawn 20 px high; its hit area reaches the step's hit height. */
  button.part::after {
    content: '';
    position: absolute;
    inset: min(-2px, calc((20px - var(--hit)) / 2)) 0;
  }

  .part:last-child {
    border-radius: 0 1px 1px 0;
  }

  .part {
    position: relative;
    padding: 0 8px;
    height: 20px;
    display: inline-flex;
    align-items: center;
    font: 400 9.5px var(--fb);
    color: var(--seg-ink);
    background: transparent;
    border: 0;
    border-left: 1px solid var(--seg-edge);
    white-space: nowrap;
    cursor: pointer;
  }

  .part.stubbed:first-child {
    border-left: 0;
    border-radius: 1px 0 0 1px;
  }

  .part.stubbed {
    cursor: default;
    opacity: var(--stub-opacity);
  }

  .part.on {
    background: var(--cyan);
    color: var(--cyan-ink);
    font-weight: 700;
  }

  .part:focus-visible {
    outline-offset: -3px;
    outline-color: var(--cyan-ink);
  }

  .part:not(.on):focus-visible {
    outline-color: var(--cyan);
  }

  .ib:hover:not(:disabled),
  .part:hover:not(:disabled) {
    filter: brightness(1.12);
  }

  .ib:active:not(:disabled),
  .part:active:not(:disabled) {
    filter: brightness(0.92);
  }

  button:disabled {
    cursor: default;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 12px;
    margin-left: 6px;
    border-radius: var(--radius-sm);
    font: 600 10px var(--fb);
    color: var(--band-ink);
    background: var(--navy-600);
    white-space: nowrap;
  }

  .btn.dk {
    background: var(--button-dark);
    box-shadow: inset 0 0 0 1px var(--rule);
    color: var(--ink);
  }

  .panels {
    display: grid;
    grid-template-columns: minmax(0, 256fr) repeat(3, minmax(0, 158fr));
    gap: 8px;
  }

  .panel {
    background: var(--proposal-ground);
    border: 1px solid var(--rule-2);
    border-radius: 3px;
    padding: 7px 9px;
    min-width: 0;
    overflow: hidden;
  }

  .panel h5 {
    margin: 0 0 5px;
    font: 700 9.5px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink);
  }

  .shotmap {
    height: 110px;
    background: var(--pitch-stripe);
    border-radius: var(--radius-sm);
  }

  .lock {
    border: 1px dashed var(--toggle-edge);
    border-radius: 3px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    color: var(--ink-3);
    gap: 4px;
    padding: 8px;
    font-size: 9.5px;
  }

  .lock b {
    color: var(--ink);
    font-size: 10px;
  }

  .kv {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    padding: 1px 0;
    font-size: 10px;
    color: var(--ink-2);
  }

  .kv b {
    color: var(--ink);
    font-weight: 600;
  }

  .kv.rule {
    padding: 3px 0;
    border-bottom: 1px solid var(--rule-2);
  }

  .inp {
    display: flex;
    align-items: center;
    height: 24px;
    padding: 0 8px;
    margin-bottom: 6px;
    background: var(--toggle-ground);
    border: 1px solid var(--toggle-edge);
    border-radius: var(--radius-sm);
    color: var(--ink-3);
    font-size: 10.5px;
  }

  .cat {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0 14px;
  }

  .box {
    border: 1px solid var(--rule);
    background: var(--proposal-ground);
    border-radius: 3px;
    padding: 9px 10px;
    margin: 8px 0 10px;
  }

  .w {
    font-size: 12px;
  }

  .def {
    margin: 3px 0;
    font-size: 10px;
    color: var(--read-ink);
  }

  .depth {
    display: grid;
    grid-template-columns: 14px 1fr auto;
    gap: 6px;
    align-items: center;
    padding: 3px 0;
    border-bottom: 1px solid var(--rule-2);
    font-size: 10px;
    color: var(--ink);
  }

  .depth .lv {
    font-size: 9px;
    color: var(--ink-3);
  }

  .traced {
    display: flex;
    gap: 8px;
    font-size: 10px;
    padding: 2px 0;
    color: var(--ink);
  }

  .ok {
    color: var(--good);
  }

  .cy {
    color: var(--cyan);
  }

  .num {
    font-variant-numeric: tabular-nums;
  }

  /* Read by a screen reader, never drawn. */
  .vh {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: 0;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  /* Compact: one column, the pitch at full width with its controls at 44 px, then the
     catalogue under it. The body scrolls down to it. */
  @media (max-width: 1023px), (max-height: 599px) {
    .screen {
      height: auto;
    }

    .cols {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto;
      flex: none;
    }

    .left,
    .right {
      overflow: visible;
    }

    .left {
      padding-right: 0;
    }

    .right {
      border-left: 0;
      border-top: 1px solid var(--rule);
      margin-top: 14px;
      padding: 14px 0 0;
    }

    .ib {
      width: 44px;
      height: 44px;
    }

    .ib::after {
      inset: -1px;
    }

    .part {
      height: 44px;
      min-width: 44px;
      justify-content: center;
    }

    button.part::after {
      inset: 0;
    }

    .panels {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
