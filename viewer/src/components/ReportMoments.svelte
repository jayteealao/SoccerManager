<!-- The report's Goals and cards: one row per moment, in match order, with its minute, its
     kind as a word (cyan for a home goal, the away state colour for an away goal, the card
     colour for a card) and the player and club. The kind is always a word; the colour only
     repeats it. With no moment yet the list says so. -->
<script>
  let { moments = [], teams = null } = $props();

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
  <ol class="moments">
    {#each moments as moment (`${moment.tick}-${moment.kind}-${moment.side}`)}
      <li>
        <b class="min num">{moment.minute}</b>
        <b class={tone(moment)}>{moment.kind}</b>
        <span class="who">{who(moment)}</span>
      </li>
    {/each}
  </ol>
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
