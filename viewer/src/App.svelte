<!-- The viewer's views over one session: Tactics, the Squad screen and the player panel, the
     Pre-match line-ups, the match, the
     Touchline, the Skip decision, the half-time and full-time report, the replay, and Resume
     a saved match for a save no engine of this version can finish. The match screen stays
     mounted, hidden behind the others, so its pitch keeps its canvas and the match plays on
     behind whichever view is open; the replay draws the same match on its own canvas. -->
<script>
  import MatchScreen from './screens/MatchScreen.svelte';
  import PlayerPanelScreen from './screens/PlayerPanelScreen.svelte';
  import PrematchScreen from './screens/PrematchScreen.svelte';
  import ReplayScreen from './screens/ReplayScreen.svelte';
  import ReportScreen from './screens/ReportScreen.svelte';
  import ResumeScreen from './screens/ResumeScreen.svelte';
  import SkipScreen from './screens/SkipScreen.svelte';
  import SquadScreen from './screens/SquadScreen.svelte';
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
{:else if session.view === 'skip'}
  <SkipScreen {session} />
{:else if session.view === 'report' && session.report}
  <ReportScreen {session} />
{:else if session.view === 'replay'}
  <ReplayScreen {session} />
{:else if session.view === 'resume'}
  <ResumeScreen {session} />
{:else if session.view === 'squad'}
  <SquadScreen {session} />
{:else if session.view === 'player'}
  <PlayerPanelScreen {session} />
{/if}

<div class="view" hidden={session.view !== 'match'}>
  <MatchScreen {session} />
</div>

<style>
  .view {
    height: 100%;
  }

  .view[hidden] {
    display: none;
  }
</style>
