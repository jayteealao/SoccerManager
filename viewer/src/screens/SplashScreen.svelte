<!-- The splash, ported from the approved Splash and SplashReady boards: the logo lockup at
     160 by 200, the three steps of the engine's start under it, and along the foot the studio
     mark (a placeholder, as the board draws it) and the powered-by line. It stays while the
     reveal plays and the engine has not answered, and never less than 1.5 s; then the ready
     state holds for 1.5 s, which a key or a click ends at once. A key or a click before the
     answer finishes the reveal only. With reduced motion the last frame shows at once. -->
<script>
  import LogoReveal from '../components/LogoReveal.svelte';
  import StepList from '../components/StepList.svelte';
  import StubSection from '../components/StubSection.svelte';
  import { BUILD_VERSION, versions } from '../lib/front-door-model.js';

  let { door } = $props();

  let ready = $derived(door.splash.phase === 'ready');
  let version = $derived(door.status ? versions(door.status).engine : BUILD_VERSION);
  let steps = $derived(
    ready
      ? [
          { label: 'Match engine found', word: `Touchline match engine ${version}`, state: 'done' },
          {
            label: 'Rules, teams and grounds loaded',
            word: `Laws of the Game rule pack · ${door.teams.length} sample teams`,
            state: 'done',
          },
          { label: 'Viewer connected', word: 'Local connection open', state: 'done' },
        ]
      : [
          {
            label: 'Match engine found',
            word: door.splash.answered ? `Touchline match engine ${version}` : 'Looking for the match engine',
            state: door.splash.answered ? 'done' : 'current',
            progress: 62,
          },
          {
            label: 'Rules, teams and grounds',
            word: 'Loading the rule pack and the sample teams',
            state: door.splash.answered ? 'current' : 'pending',
            progress: 62,
          },
          { label: 'Viewer connection', word: 'Waits for the engine', state: 'pending' },
        ]
  );
</script>

<!-- A key or a click anywhere on the splash is handled by the page (Root.svelte). -->
<div class="splash" data-screen="splash" data-phase={door.splash.phase}>
  <div class="top">
    <LogoReveal {version} reveal finished={door.splash.finished || ready} />
    <div class="after" class:shown={ready || door.splash.finished}>
      <h2 class="sl">
        {ready ? 'Ready' : 'Starting the match engine'}
        <em>{ready ? 'Everything is loaded' : 'This takes a few seconds'}</em>
      </h2>
      <StepList {steps} />
      {#if ready}
        <p class="ready"><span class="tag">ANY KEY</span>Press any key or click to continue</p>
      {:else}
        <p class="wait">The start screen opens when the engine answers.</p>
      {/if}
    </div>
  </div>
  <div class="foot">
    <!-- STUB: the studio mark; the publisher's own mark replaces this placeholder. -->
    <StubSection note="studio mark placeholder" fade={false}>
      <span class="placeholder">[STUDIO MARK]</span>
    </StubSection>
    <p class="powered">
      <b>Powered by the Touchline match engine {version}</b><br />Open-source notices: Licences and about, on the start
      screen
    </p>
  </div>
</div>

<style>
  .splash {
    width: 1280px;
    height: 800px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    background: var(--page);
    color: var(--ink);
    cursor: default;
  }

  .top {
    padding: 200px 0 0 160px;
  }

  .after {
    width: 520px;
    margin: 60px 0 0 4px;
    animation: fade 250ms var(--ease) 1800ms both;
  }

  .after.shown {
    animation: none;
  }

  .sl {
    margin: 0 0 8px;
    font: 700 11px var(--fd);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--cyan);
  }

  .sl em {
    font: 400 10px var(--fb);
    font-style: normal;
    letter-spacing: 0;
    text-transform: none;
    color: var(--ink-3);
    margin-left: 6px;
  }

  .ready {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 14px 0 0;
    font: 600 13px var(--fb);
  }

  .tag {
    font: 700 11px var(--fd);
    letter-spacing: 0.06em;
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    background: var(--cyan);
    color: var(--cyan-ink);
  }

  .wait {
    margin: 14px 0 0;
    font-size: 11px;
    color: var(--ink-3);
  }

  .foot {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    padding: 0 40px 32px;
  }

  .placeholder {
    width: 150px;
    height: 44px;
    display: grid;
    place-items: center;
    border: 1px dashed var(--rule);
    border-radius: var(--radius-sm);
    font: 700 9.5px var(--fd);
    letter-spacing: 0.08em;
    color: var(--ink-3);
  }

  .powered {
    margin: 0;
    text-align: right;
    font-size: 10.5px;
    color: var(--ink-3);
    line-height: 1.6;
  }

  .powered b {
    font: 700 10.5px var(--fd);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-2);
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
