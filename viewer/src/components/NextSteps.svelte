<!-- The full-time report's Next list: New match (cyan, the screen's one cyan choice) and
     Return to start, as the start screen's option rows at the report's smaller size. Both wait
     until the whole match is stored: while it is stored they are faded and disabled, and a
     status line says what the page waits for. The change from storing to ready is not
     animated. -->
<script>
  import MenuList from './MenuList.svelte';
  import SectionLabel from './SectionLabel.svelte';

  let { ready = false, onchoose = () => {} } = $props();

  let items = $derived([
    {
      id: 'new',
      label: 'New match',
      sub: 'Match setup with these two teams picked',
      primary: ready,
      disabled: !ready,
    },
    {
      id: 'return',
      label: 'Return to start',
      sub: 'The start screen. Save replay first to watch it again.',
      disabled: !ready,
    },
  ]);
</script>

<div class="next">
  <SectionLabel label="Next" note={ready ? 'the match is stored' : 'waits until the match is stored'} level={4} />
  <MenuList {items} {onchoose} label="Next" size="small" />
  {#if !ready}
    <p class="storing" role="status"><span class="dot" aria-hidden="true">●</span>Storing the match for the replay…</p>
  {/if}
</div>

<style>
  .storing {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 0 2px;
    font-size: 10px;
    color: var(--ink-2);
  }

  .dot {
    color: var(--cyan);
  }
</style>
