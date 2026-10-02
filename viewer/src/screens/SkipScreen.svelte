<!-- The Skip decision, ported from the sketch's Skip to result board (the Head knock decision
     layout): the shell with "Skip to result" and the fixture, score and minute in the header,
     Paused over the clock in the date block and RESUME in the cyan block; the match tabs;
     the 54 px fact strip; then three columns of 330 px, 1fr and 400 px. The left column shows
     where the match stands: the paused frame on its own pitch canvas and the score, clock,
     queued change and mentality. The middle says what the engine does and what the player
     gives up. The right holds the decision: two options, Confirm and Keep watching, and what
     happens next. Nothing is sent until Confirm; Keep watching and Escape go back to the
     paused match at the same tick. The other grounds' line is a stub. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import Crest from '../components/Crest.svelte';
  import DecisionOption from '../components/DecisionOption.svelte';
  import Glyph from '../components/Glyph.svelte';
  import InfoStrip from '../components/InfoStrip.svelte';
  import KvRow from '../components/KvRow.svelte';
  import PitchCanvas from '../components/PitchCanvas.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import StubSection from '../components/StubSection.svelte';
  import { PAUSE } from '../components/icons.js';
  import { mentalityWord } from '../lib/prematch.js';
  import { clockAt } from '../lib/recovery.js';
  import { fixtureTitle } from '../lib/scoreboard.js';
  import { decisionFacts, minuteOf } from '../lib/skip.js';

  let { session } = $props();

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

  const OPTIONS = ['skip', 'watch'];

  let choice = $state('skip');
  let confirmButton = $state();

  let from = $derived(session.skip?.from ?? session.renderedTick);
  let clock = $derived(clockAt(from));
  let names = $derived(session.teams ? session.teams.map((t) => t['team.name']) : ['Home', 'Away']);
  let score = $derived(session.score);
  /// The period as the board writes it: "2nd half".
  let period = $derived(session.period.charAt(0) + session.period.slice(1).toLowerCase());
  let queued = $derived(session.dugout.chips.filter((c) => c.state === 'queued').map((c) => c.label));
  let facts = $derived(
    decisionFacts({
      from,
      score,
      scorers: session.scorers,
      subsUsed: session.play.subsUsed[0],
      limit: session.hello?.substitutions?.limit ?? 0,
      queued,
      engineVersion: session.engineVersion,
      period,
      mentality: mentalityWord(session.dugout.schema, session.dugout.tactics?.mentality),
    })
  );

  const engine = [
    'It plays on from the exact moment you paused, with the same state and the same random draws.',
    'The result is the result you would watch. Nothing is estimated.',
    'Your queued change still applies at the next stoppage.',
    'It plays at full speed, so the report opens in a few seconds.',
  ];

  let giveUp = $derived([
    'Watching the rest of the match live.',
    `New shouts, substitutions and tactic changes after ${clock}.`,
  ]);

  let next = $derived([
    ['Now', `The engine plays ${clock} to full time. Scores stay hidden until the report.`],
    ['Full time', 'The report opens: figures, goals and cards, and what you did not watch live.'],
    ['Replay', "Replay the whole match from 0', the skipped part marked."],
  ]);

  $effect(() => {
    confirmButton?.focus();
  });

  function confirm() {
    if (choice === 'skip') {
      session.confirmSkip();
    } else {
      session.keepWatching();
    }
  }

  function move(step) {
    const i = (OPTIONS.indexOf(choice) + step + OPTIONS.length) % OPTIONS.length;
    choice = OPTIONS[i];
    document.querySelector(`[data-option="${choice}"] [role="radio"]`)?.focus();
  }

  function key(event) {
    if (event.key === 'Escape') {
      event.preventDefault();
      session.keepWatching();
    }
  }

  function tab(id) {
    session.keepWatching();
    if (id !== 'match') {
      session.show(id);
    }
  }
</script>

<svelte:window onkeydown={key} />

<AppShell
  title="Skip to result"
  subtitle="{fixtureTitle(names, score)} · {minuteOf(from)}'"
  date="PAUSED"
  dateSub={clock}
  action="Resume"
  onaction={() => session.keepWatching(true)}
  tabs={TABS}
  ontab={tab}
>
  {#snippet crest()}
    <Crest team={session.teams?.[0] ?? null} />
  {/snippet}

  <div class="screen" data-screen="skip">
    <InfoStrip facts={facts.strip}>
      {#snippet lead()}
        <span class="lead" aria-hidden="true"><Glyph glyph={{ d: PAUSE }} size={14} /></span>
      {/snippet}
    </InfoStrip>

    <div class="cols">
      <div>
        <SectionLabel label="Where the match stands" note="frozen at {clock}" />
        <PitchCanvas
          width={310}
          height={170}
          attach={(canvas) => session.attachCanvas(canvas, 'skip')}
          detach={(canvas) => session.detachCanvas(canvas, 'skip')}
          drawn={session.teams !== null}
          overlays={false}
        >
          <span class="tagpos"><span class="tagc">PAUSED · {clock}</span></span>
        </PitchCanvas>
        <div class="hr"></div>
        <KvRow key="Score" value="{names[0]} {score[0]} – {score[1]} {names[1]}" />
        {#each facts.rows as row (row.key)}
          <KvRow key={row.key} value={row.value} />
        {/each}
      </div>

      <div>
        <SectionLabel label="What the engine does" note="same state, same random draws" />
        {#each engine as line (line)}
          <p class="line"><span class="cy" aria-hidden="true">›</span><span>{line}</span></p>
        {/each}
        <div class="box">
          <SectionLabel label="What you give up" />
          {#each giveUp as line (line)}
            <p class="line"><span class="cy" aria-hidden="true">›</span><span>{line}</span></p>
          {/each}
        </div>
        <p class="g">The replay holds the whole match afterwards, the skipped part included.</p>
      </div>

      <div>
        <SectionLabel label="Your decision" />
        <div class="options" role="radiogroup" aria-label="Your decision">
          <div data-option="skip">
            <DecisionOption
              title="Skip to the final whistle"
              sub="The report opens at full time. The replay holds the whole match."
              checked={choice === 'skip'}
              onselect={() => (choice = 'skip')}
              onmove={move}
            />
          </div>
          <div data-option="watch">
            <DecisionOption
              title="Keep watching"
              sub="Back to the live match at {clock}, paused."
              checked={choice === 'watch'}
              onselect={() => (choice = 'watch')}
              onmove={move}
            />
          </div>
        </div>
        <div class="buttons">
          <button class="btn cy confirm" type="button" bind:this={confirmButton} onclick={confirm}
            >{choice === 'skip' ? 'Confirm: skip to result' : 'Confirm: keep watching'}</button
          >
          <button class="btn gh" type="button" onclick={() => session.keepWatching()}>Keep watching</button>
        </div>
        <div class="hr"></div>
        <SectionLabel label="What happens next" />
        <div class="next">
          {#each next as [when, what] (when)}
            <div class="step"><b class="when">{when}</b><span class="what">{what}</span></div>
          {/each}
          <!-- STUB: the other grounds finishing with the match belong to the background matchday. -->
          <StubSection note="other grounds finish">
            <div class="step"><b class="when">Other grounds</b><span class="what">They finish too, and their results show in the report.</span></div>
          </StubSection>
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

  .lead {
    display: inline-grid;
    place-items: center;
    color: var(--band-ink);
  }

  .cols {
    display: grid;
    grid-template-columns: 330px 1fr 400px;
    grid-template-rows: minmax(0, 1fr);
    flex: 1;
    min-height: 0;
    margin-top: 10px;
  }

  .cols > div {
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    padding-right: 18px;
  }

  .cols > div + div {
    border-left: 1px solid var(--rule);
    padding-left: 18px;
  }

  .tagpos {
    position: absolute;
    left: 6px;
    bottom: 6px;
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

  .hr {
    border-top: 1px solid var(--rule);
    margin: 10px 0;
  }

  .line {
    display: flex;
    gap: 7px;
    margin: 0;
    padding: 2px 0;
    color: var(--read-ink);
    font-size: 10.5px;
  }

  .cy {
    color: var(--cyan);
  }

  .box {
    margin-top: 10px;
    padding: 9px 10px;
    border: 1px solid var(--rule);
    border-radius: 3px;
    background: var(--proposal-ground);
  }

  .box :global(.sl) {
    margin-bottom: 4px;
  }

  .g {
    margin: 8px 0 0;
    font-size: 9.5px;
    color: var(--ink-3);
  }

  .buttons {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 26px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-sm);
    font: 600 10px var(--fb);
    cursor: pointer;
    white-space: nowrap;
  }

  .confirm {
    flex: 1;
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

  .btn:hover {
    filter: brightness(1.12);
  }

  .btn:active {
    filter: brightness(0.92);
  }

  .next {
    margin: 0;
  }

  .step {
    display: grid;
    grid-template-columns: 70px 1fr;
    gap: 8px;
    padding: 3px 0;
    border-bottom: 1px solid var(--rule-2);
    font-size: 10px;
  }

  .when {
    font: 700 10px var(--fd);
    color: var(--cyan);
  }

  .what {
    margin: 0;
    color: var(--read-ink);
  }
</style>
