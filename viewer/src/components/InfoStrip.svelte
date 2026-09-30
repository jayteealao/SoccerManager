<!-- The navy fact strip under the tabs (54 px): the facts before the detail. The first
     cell is wider and reads left to right; the rest are centred. The 66 px score strip of
     the match screens is a separate component. -->
<script>
  let { facts = [] } = $props();
</script>

<div class="strip" role="group" aria-label="Facts">
  {#each facts as fact, i (i)}
    <div class="cell">
      <b class:num={fact.num}>{fact.value}</b>
      <span>{fact.label}</span>
    </div>
  {/each}
</div>

<style>
  .strip {
    height: 54px;
    background: var(--navy-600);
    color: var(--band-ink);
    display: flex;
    align-items: center;
    margin-top: 6px;
    flex: none;
  }

  .cell {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    line-height: 1.25;
    padding: 0 10px;
    min-width: 0;
  }

  .cell:first-child {
    flex: 1.35;
    align-items: flex-start;
    text-align: left;
    padding-left: 20px;
  }

  b {
    display: block;
    font: 600 12px var(--fb);
    white-space: nowrap;
  }

  b.num {
    font-variant-numeric: tabular-nums;
  }

  span {
    display: block;
    font-size: 9.5px;
    color: var(--navy-sub);
    white-space: nowrap;
  }
</style>
