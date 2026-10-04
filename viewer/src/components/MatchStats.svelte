<!-- The statistics: the nine rows of the released stats message, home value, label, away
     value, in tabular numerals. Each row is named in full for a screen reader. Before the
     first message every row reads zero. -->
<script>
  import SectionLabel from './SectionLabel.svelte';
  import { toPanel } from '../lib/stats.js';

  let { stats = null, names = ['Home', 'Away'] } = $props();

  let rows = $derived(toPanel(stats, names));
</script>

<section class="stats" aria-label="Statistics">
  <SectionLabel label="Statistics" note="{names[0]} · {names[1]}" />
  <div class="rows">
    {#each rows as row (row.key)}
      <div class="row" role="group" aria-label={row.aria}>
        <b class="num" aria-hidden="true">{row.homeText}</b>
        <span aria-hidden="true">{row.label}</span>
        <b class="num away" aria-hidden="true">{row.awayText}</b>
      </div>
    {/each}
  </div>
</section>

<style>
  .stats {
    flex: none;
  }

  .row {
    display: grid;
    grid-template-columns: 56px 1fr 56px;
    align-items: center;
    min-height: 15px;
    font-size: 10px;
  }

  span {
    text-align: center;
    color: var(--ink-2);
  }

  b {
    color: var(--ink);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  b.away {
    text-align: right;
  }
</style>
