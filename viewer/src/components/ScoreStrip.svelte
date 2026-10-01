<!-- The navy score strip of the match screens (66 px): each club's crest, name and scorers,
     the score at 28 px with the clock tag under it, and the xG, possession and shots facts.
     The tag's word always names the state; its colour only repeats it. The LIVE dot pulses
     over 1.4 s, and not at all under reduced motion. While the engine plays the rest of a
     skipped match the score is `hidden` ("– –") until full time. A fact with `stub` is a
     part the viewer does not build yet: faded, inert and hidden; `flex` widens a cell as the
     board draws it. -->
<script>
  import Crest from './Crest.svelte';
  import StubSection from './StubSection.svelte';
  import { scoreText } from '../lib/scoreboard.js';

  let { teams = null, score = [0, 0], scorers = ['', ''], tag, facts = [], hidden = false } = $props();

  let names = $derived(teams ? teams.map((t) => t['team.name']) : ['Home', 'Away']);
</script>

<div class="strip" role="group" aria-label="Score">
  <div class="side home">
    <Crest team={teams?.[0] ?? null} width={38} height={41} />
    <div class="club">
      <b>{names[0]}</b>
      <span>{scorers[0]}</span>
    </div>
  </div>
  <div class="centre">
    {#if hidden}
      <b class="score num" aria-label="Score hidden until full time">– –</b>
    {:else}
      <b class="score num" aria-label="Score {scoreText(score[0], score[1])}">{scoreText(score[0], score[1])}</b>
    {/if}
    <span class="tag {tag.tone}">
      {#if tag.live}<span class="dot" aria-hidden="true"></span>{/if}{tag.text}
    </span>
  </div>
  <div class="side away">
    <Crest team={teams?.[1] ?? null} width={38} height={41} />
    <div class="club">
      <b>{names[1]}</b>
      <span>{scorers[1]}</span>
    </div>
  </div>
  {#each facts as fact (fact.label)}
    <div class="fact" style:flex={fact.flex}>
      {#if fact.stub}
        <!-- STUB: a fact the viewer does not build yet. -->
        <StubSection note="strip fact: {fact.label}">
          <b class="num">{fact.value}</b>
          <span>{fact.label}</span>
        </StubSection>
      {:else}
        <b class="num">{fact.value}</b>
        <span>{fact.label}</span>
      {/if}
    </div>
  {/each}
</div>

<style>
  .strip {
    height: 66px;
    margin-top: 6px;
    background: var(--navy-600);
    color: var(--band-ink);
    display: flex;
    align-items: center;
    flex: none;
  }

  .side,
  .centre,
  .fact {
    display: flex;
    align-items: center;
    text-align: center;
    line-height: 1.25;
    min-width: 0;
  }

  .side {
    flex: 1.1;
    gap: 12px;
  }

  .home {
    justify-content: flex-end;
    padding: 0 0 0 20px;
  }

  .home .club {
    text-align: right;
  }

  .away {
    justify-content: flex-start;
    padding: 0 10px 0 0;
  }

  .away .club {
    text-align: left;
  }

  .club b {
    display: block;
    font: 700 15px var(--fd);
    letter-spacing: 0.03em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .centre {
    flex: 0.8;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
    padding: 0 10px;
  }

  .score {
    display: block;
    font: 800 28px/1 var(--fd);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .fact {
    flex: 0.75;
    flex-direction: column;
    justify-content: center;
    padding: 0 10px;
  }

  .fact b {
    display: block;
    font: 600 12px var(--fb);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  span {
    display: block;
    font-size: 9.5px;
    color: var(--navy-sub);
    white-space: nowrap;
    min-height: 1em;
  }

  .tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-height: 0;
    font: 700 9.5px var(--fd);
    letter-spacing: 0.06em;
    padding: 0 6px;
    border-radius: var(--radius-sm);
  }

  .tag.cyan {
    background: var(--cyan);
    color: var(--cyan-ink);
  }

  .tag.bad {
    background: var(--bad);
    color: var(--on-bad);
  }

  .tag.mid {
    background: var(--mid);
    color: var(--on-mid);
  }

  .tag.warn {
    background: var(--warn);
    color: var(--on-warn);
  }

  .dot {
    display: inline-block;
    width: 7px;
    height: 7px;
    min-height: 0;
    border-radius: 50%;
    background: var(--cyan-ink);
    animation: pulse var(--live-pulse) ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.25;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .dot {
      animation: none;
    }
  }
</style>
