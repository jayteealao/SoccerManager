<!-- The report's Goals and cards: one row per moment, in match order, with its minute, its
     kind as a word (cyan for a home goal, the away state colour for an away goal, the card
     colour for a card) and the player and club. The kind is always a word; the colour only
     repeats it. With no moment yet the list says so. After a skip (`skippedFrom`) a moment
     after the skip point carries the NOT LIVE tag, and a note under the list says what it
     means; with no skip the list is as before. -->
<script>
  import { notLive } from '../lib/skip.js';

  let { moments = [], teams = null, skippedFrom = null } = $props();

  let skipped = $derived(skippedFrom !== null);

  let names = $derived(teams ? teams.map((t) => t['team.name']) : ['Home', 'Away']);

  /// The player's surname from the hello rosters, or null.
  let players = $derived.by(() => {
    const map = new Map();
    for (const team of teams ?? []) {
      for (const player of team.roster ?? []) {
        map.set(player['player.id'], String(player['player.name'] ?? '').split(/\s+/).pop());
      }
    }
    return map;
  });

  function tone(moment) {
    if (moment.kind !== 'Goal') {
      return moment.kind === 'Yellow card' ? 'md' : 'bd';
    }
    return moment.side === 1 ? 'bd' : 'cy';
  }

  function who(moment) {
    const club = names[moment.side] ?? '';
    const player = moment.player == null ? null : players.get(moment.player);
    return player ? `${player} · ${club}` : club;
  }
</script>

{#if moments.length === 0}
  <p class="none">No goals or cards.</p>
{:else}
  <ol class="moments" class:skipped>
    {#each moments as moment (`${moment.tick}-${moment.kind}-${moment.side}`)}
      <li>
        <b class="min num">{moment.minute}</b>
        <b class={tone(moment)}>{moment.kind}</b>
        <span class="who">{who(moment)}</span>
        {#if skipped}
          {#if notLive(moment, skippedFrom)}<span class="nl">NOT LIVE</span>{:else}<span></span>{/if}
        {/if}
      </li>
    {/each}
  </ol>
{/if}
{#if skipped}
  <p class="note">Moments marked NOT LIVE happened after you skipped. The replay shows them.</p>
{/if}

<style>
  .moments {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: grid;
    grid-template-columns: 28px 66px 1fr;
    gap: 6px;
    align-items: center;
    min-height: 22px;
    border-bottom: 1px solid var(--rule-2);
    font-size: 10px;
    color: var(--ink);
  }

  .skipped li {
    grid-template-columns: 28px 66px 1fr auto;
  }

  .nl {
    font: 700 8.5px var(--fd);
    letter-spacing: 0.06em;
    padding: 0 6px;
    border-radius: var(--radius-sm);
    background: var(--not-live-ground);
    color: var(--ink-2);
    white-space: nowrap;
  }

  .note {
    margin: 6px 0 0;
    font-size: 9.5px;
    color: var(--ink-3);
  }

  .min {
    font: 700 10px var(--fd);
    font-variant-numeric: tabular-nums;
  }

  .cy {
    color: var(--cyan);
  }

  .md {
    color: var(--mid);
  }

  .bd {
    color: var(--bad);
  }

  .who {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .none {
    margin: 0;
    font-size: 10px;
    color: var(--ink-3);
  }
</style>
