<!-- The navy fact strip under the tabs: 54 px, or 62 px (`tall`) on the Pre-match line-ups.
     The facts come before the detail. The first cell is wider and reads left to right, with
     an optional lead (a crest) before it; the rest are centred. A fact with `stub` is a part
     the engine has no model for yet: drawn faded with the LATER mark, inert and hidden. The
     66 px score strip of the match screens is a separate component. -->
<script>
  import StubSection from './StubSection.svelte';

  let { facts = [], tall = false, lead } = $props();
</script>

<div class="strip" class:tall role="group" aria-label="Facts">
  {#each facts as fact, i (i)}
    <div class="cell">
      {#if i === 0 && lead}{@render lead()}{/if}
      {#if fact.stub}
        <!-- STUB: a fact the engine has no model for yet. -->
        <StubSection note="strip fact: {fact.label}" later>
          <b class:num={fact.num}>{fact.value}</b>
          <span>{fact.label}</span>
        </StubSection>
      {:else}
        <div class="txt">
          <b class:num={fact.num}>{fact.value}</b>
          <span>{fact.label}</span>
        </div>
      {/if}
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

  .strip.tall {
    height: 62px;
  }

  .cell {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    text-align: center;
    line-height: 1.25;
    padding: 0 10px;
    min-width: 0;
  }

  .cell:first-child {
    flex: 1.35;
    justify-content: flex-start;
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

  .cell :global(.stub.fade) {
    text-align: center;
  }

  .cell :global(.stub .later) {
    display: block;
    margin: 1px 0 0;
  }
</style>
