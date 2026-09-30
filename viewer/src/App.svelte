<!-- The viewer's four views over one session: Tactics, the Pre-match line-ups, the match and
     the Touchline. The match screen stays mounted, hidden behind the others, so the pitch
     keeps its canvas and the match plays on behind whichever view is open. -->
<script>
  import MatchScreen from './screens/MatchScreen.svelte';
  import PrematchScreen from './screens/PrematchScreen.svelte';
  import TacticsScreen from './screens/TacticsScreen.svelte';
  import TouchlineScreen from './screens/TouchlineScreen.svelte';

  let { session } = $props();
</script>

{#if session.view === 'tactics'}
  <TacticsScreen {session} />
{:else if session.view === 'prematch'}
  <PrematchScreen {session} />
{:else if session.view === 'touchline'}
  <TouchlineScreen {session} />
{/if}

<div class="view" hidden={session.view !== 'match'}>
  <MatchScreen {session} />
</div>

<style>
  .view[hidden] {
    display: none;
  }
</style>
