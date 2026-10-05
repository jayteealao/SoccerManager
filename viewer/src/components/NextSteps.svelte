<!-- The full-time report's Next list: New match and Return to start, as the start screen's
     option rows at the report's smaller size, both unfilled: the header's NEW MATCH block is
     the screen's one cyan choice. Both wait until the replay is ready: until then they are
     faded and disabled, and a status line says what the page waits for. "Save" is kept for
     the replay file. The change to ready is not animated. A hidden live region, in the page from the start, reads out both moments. -->
<script>
  import MenuList from './MenuList.svelte';
  import SectionLabel from './SectionLabel.svelte';

  let { ready = false, onchoose = () => {} } = $props();

  const STORING = 'Getting the replay ready…';
  const READY = 'The replay is ready. New match and Return to start are available.';

  /// The live region starts empty and takes its words after it is in the page, so screen
  /// readers speak the storing line and then the moment the choices are ready.
  let spoken = $state('');
  $effect(() => {
    spoken = ready ? READY : STORING;
  });

  let items = $derived([
    {
      id: 'new',
      label: 'New match',
      sub: 'Match setup with these two teams picked',
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
  <SectionLabel label="Next" note={ready ? 'the replay is ready' : 'waits until the replay is ready'} level={4} />
  <MenuList {items} {onchoose} label="Next" size="small" />
  {#if !ready}
    <p class="storing"><span class="dot" aria-hidden="true">●</span>{STORING}</p>
  {/if}
  <p class="sr" role="status" aria-live="polite">{spoken}</p>
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

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
