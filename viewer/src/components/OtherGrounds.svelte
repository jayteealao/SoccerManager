<!-- The other grounds of the matchday, ported from the OtherGroundsStates board's right-column
     list: one 20 px row per fixture on the grid 1fr, 44 px, 1fr, 30 px, with each club's
     12 px crest, the score in the middle and the ground's own minute (KO, HT, FT) on the
     right. A new goal sits in the cyan-edged block with a GOAL tag for 8 seconds of the
     player's clock; a goal that arrived late reads GOAL · LATE and says when it was shown; a
     ground a fault stopped reads "!" with its words. A state is always a word or a mark, never
     colour alone. The list is read-only: it takes no focus. `grounds` is what `groundsAt`
     returns. `report` draws the report's list, and `final` notes that every ground has ended. -->
<script>
  import Crest from './Crest.svelte';
  import SectionLabel from './SectionLabel.svelte';

  let { grounds, round = 1, skeleton = false, report = false, final = false } = $props();

  let note = $derived(
    skeleton || grounds?.state !== 'rows'
      ? `Matchday ${round}`
      : final
        ? `Matchday ${round} · final`
        : `Matchday ${round} · on your clock`
  );
</script>

<section class="grounds" aria-label="Other grounds">
  <SectionLabel label="Other grounds" {note} />
  {#if skeleton}
    {#each [0, 1, 2] as i (i)}<div class="skel"></div>{/each}
  {:else if grounds?.state !== 'rows'}
    <p class="none">{grounds?.words ?? 'No other matches this matchday.'}</p>
    {#if grounds?.hint}<p class="g hint">{grounds.hint}</p>{/if}
  {:else}
    <ul>
      {#each grounds.rows as row (row.fixture)}
        <li class:block={row.flag !== null} class:new={row.flag === 'new'} class:late={row.flag === 'late'}>
          <div class="fixture" class:ruled={row.flag === null && !row.unavailable}>
            <span class="home">{row.home['team.name']} <Crest team={row.home} width={12} height={13} /></span>
            <b class="num score" class:g={!row.started || row.unavailable}
              >{#if row.score}{row.score[0]} – {row.score[1]}{:else}–{/if}</b
            >
            <span class="away"><Crest team={row.away} width={12} height={13} /> {row.away['team.name']}</span>
            <span
              class="num minute"
              class:w={row.flag !== null || (row.ended && !report && !row.unavailable)}
              class:g={row.flag === null && !(row.ended && !report) && !row.unavailable}
              class:wa={row.unavailable}>{row.minute}</span
            >
          </div>
          {#if row.flag !== null}
            <div class="what">
              <span class="tag">{row.tag}</span>
              <span class:w={row.flag === 'new'} class:g={row.flag === 'late'}>{row.words}</span>
            </div>
          {:else if row.unavailable}
            <div class="failed">
              <b class="wa">Result unavailable.</b>
              <span class="g">A fault stopped this match at {row.failedAt}. A bug report is saved.</span>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
    {#if !report}<p class="g note">Each event shows when your clock reaches its minute.</p>{/if}
  {/if}
</section>

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .fixture {
    display: grid;
    grid-template-columns: 1fr 44px 1fr 30px;
    gap: 6px;
    align-items: center;
    height: 20px;
    font-size: 10px;
  }

  .fixture.ruled {
    border-bottom: 1px solid var(--rule-2);
  }

  .fixture span {
    white-space: nowrap;
    overflow: hidden;
  }

  .fixture :global(.crest) {
    vertical-align: middle;
  }

  .home,
  .minute {
    text-align: right;
  }

  .score {
    text-align: center;
    font: 700 10px var(--fd);
    color: var(--ink);
  }

  .num {
    font-variant-numeric: tabular-nums;
  }

  .g {
    color: var(--ink-3);
  }

  .w {
    color: var(--ink);
  }

  .wa {
    color: var(--warn);
  }

  .score.g {
    color: var(--ink-3);
  }

  /* The new goal's block and the late goal's: an outline appears in 200 ms and holds while
     the session keeps the flag, 8 seconds of the player's clock. */
  .block {
    border-radius: var(--radius-sm);
    padding: 1px 6px 4px;
    margin: 1px 0;
    animation: arrive 200ms var(--ease) both;
  }

  .new {
    background: var(--new-goal-ground);
    box-shadow: inset 0 0 0 1.5px var(--cyan);
  }

  .late {
    background: var(--ground-2);
  }

  .what {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
    margin-top: 1px;
  }

  .tag {
    display: inline-block;
    background: var(--cyan);
    color: var(--cyan-ink);
    font: 700 9.5px var(--fd);
    letter-spacing: 0.06em;
    padding: 0 6px;
    border-radius: var(--radius-sm);
    white-space: nowrap;
  }

  .late .tag {
    background: var(--not-live-ground);
    color: var(--ink-2);
  }

  .failed {
    font-size: 9.5px;
    padding-bottom: 3px;
    border-bottom: 1px solid var(--rule-2);
  }

  .none {
    margin: 0;
    font-size: 10.5px;
  }

  .hint,
  .note {
    margin: 0;
    font-size: 9.5px;
  }

  .note {
    margin-top: 4px;
  }

  .skel {
    height: 14px;
    margin-bottom: 8px;
    background: var(--skeleton);
    border-radius: var(--radius-sm);
  }

  @keyframes arrive {
    from {
      opacity: 0;
    }
  }

  /* The reduced-motion setting, or the system's preference when it follows Windows. */
  :global(:root[data-motion='reduce']) .block {
      animation: none;
  }
</style>
