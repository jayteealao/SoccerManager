<!-- The viewer's views over one session: Tactics, the Pre-match line-ups, the match, the
     Touchline, the half-time and full-time report and the replay. The match screen stays
     mounted, hidden behind the others, so its pitch keeps its canvas and the match plays on
     behind whichever view is open; the replay draws the same match on its own canvas. -->
<script>
  import MatchScreen from './screens/MatchScreen.svelte';
  import PrematchScreen from './screens/PrematchScreen.svelte';
  import ReplayScreen from './screens/ReplayScreen.svelte';
  import ReportScreen from './screens/ReportScreen.svelte';
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
{:else if session.view === 'report' && session.report}
  <ReportScreen {session} />
{:else if session.view === 'replay'}
  <ReplayScreen {session} />
{/if}

<div class="view" hidden={session.view !== 'match'}>
  <MatchScreen {session} />
</div>

<style>
  .view[hidden] {
    display: none;
  }
</style>
