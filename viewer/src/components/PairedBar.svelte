<!-- The sketch's paired statistic: the home value, the label, the away value, and a bar
     split between the two sides (cyan for home, grey for away). A row with no values yet
     splits the bar evenly. The numbers carry the meaning; the bar only shows the share. -->
<script>
  let { label, home, away } = $props();

  let total = $derived(Number(home) + Number(away));
  let share = $derived(total > 0 ? Math.round((Number(home) / total) * 100) : 50);
</script>

<div class="pair">
  <div class="row">
    <b class="num">{home}</b>
    <span>{label}</span>
    <b class="num">{away}</b>
  </div>
  <div class="bar" role="presentation">
    <i class="home" style:flex={share}></i>
    <i class="away" style:flex={100 - share}></i>
  </div>
</div>

<style>
  .pair {
    margin-bottom: 5px;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    color: var(--ink-2);
  }

  b {
    color: var(--ink);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  span {
    font-size: 9.5px;
  }

  .bar {
    display: flex;
    gap: 2px;
    height: 4px;
  }

  .home {
    background: var(--cyan);
  }

  .away {
    background: var(--bar-away);
  }
</style>
