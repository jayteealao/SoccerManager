<!-- The page: the front door's screens (the splash, the start screen, match setup, Settings,
     Licences and about, the closed page) and the match views of the session on show, with the
     in-match menu and its confirmations over the paused match. The match views stay mounted
     behind Settings and Licences when those open from the menu, so the paused match keeps its
     canvas; each new match mounts them afresh over its new session. -->
<script>
  import App from './App.svelte';
  import ConfirmDialog from './components/ConfirmDialog.svelte';
  import MenuPopover from './components/MenuPopover.svelte';
  import ClosedScreen from './screens/ClosedScreen.svelte';
  import LicencesScreen from './screens/LicencesScreen.svelte';
  import SettingsScreen from './screens/SettingsScreen.svelte';
  import SetupScreen from './screens/SetupScreen.svelte';
  import SplashScreen from './screens/SplashScreen.svelte';
  import StartScreen from './screens/StartScreen.svelte';

  import { screenName } from './lib/screen-name.js';

  let { door, systemReduces = () => false } = $props();

  let fileInput = $state();

  // The page fills the window and lays out at one of the window steps (skins/base.css). Below
  // 768 px wide or 600 px high the compact layout scales down to fit the window whole, as the
  // fixed stage did before: the zoom keeps the layout at 768 by 600 or larger, and the window
  // read here is the window's size, not an element's layout.
  const FLOOR_WIDTH = 768;
  const FLOOR_HEIGHT = 600;
  let viewWidth = $state(1280);
  let viewHeight = $state(800);
  const fit = $derived(Math.min(1, viewWidth / FLOOR_WIDTH, viewHeight / FLOOR_HEIGHT));

  // Every screen names itself in the tab title and, once the page has shown a first screen,
  // in a polite announcement, so a screen-reader user hears that the screen changed (WCAG
  // 2.4.2 and 4.1.3). Focus stays where the player put it.
  const name = $derived(screenName(door.view, door.session?.view));
  let announced = $state('');
  let first = true;
  $effect(() => {
    const title = name ? `${name} · Touchline` : 'Touchline';
    if (globalThis.document) {
      globalThis.document.title = title;
    }
    if (first) {
      first = false;
      return;
    }
    announced = name;
  });

  function pickReplay() {
    fileInput?.click();
  }

  async function openFile() {
    const file = fileInput.files?.[0];
    if (!file) {
      return;
    }
    const bytes = new Uint8Array(await file.arrayBuffer());
    fileInput.value = '';
    await door.openReplay(bytes, file.name);
  }

  function keydown(event) {
    if (door.view === 'splash') {
      door.press();
      return;
    }
    if (event.key === 'Escape' && door.view === 'match' && !event.defaultPrevented) {
      if (door.overlay || ['match', 'tactics', 'touchline'].includes(door.session?.view)) {
        if (door.session?.onMenu || door.overlay) {
          event.preventDefault();
          door.escape();
        }
      }
    }
  }

  function choose(id) {
    if (id === 'resume') {
      door.closeMenu();
    } else if (id === 'return' || id === 'quit') {
      door.ask(id);
    } else if (id === 'settings' || id === 'licences') {
      door.open(id);
    }
  }
</script>

<svelte:window
  bind:innerWidth={viewWidth}
  bind:innerHeight={viewHeight}
  onkeydown={keydown}
  onpointerdown={() => door.view === 'splash' && door.press()}
/>

<div class="fit" style:zoom={fit < 1 ? fit : null}>
<div class="stage" data-view={door.view} data-overlay={door.overlay ?? 'none'}>
  {#if door.view === 'splash'}
    <SplashScreen {door} />
  {:else if door.view === 'start'}
    <StartScreen {door} {pickReplay} />
  {:else if door.view === 'setup'}
    <SetupScreen {door} />
  {:else if door.view === 'settings'}
    <SettingsScreen {door} systemReduces={systemReduces()} />
  {:else if door.view === 'licences'}
    <LicencesScreen {door} />
  {:else if door.view === 'closed'}
    <ClosedScreen {door} />
  {/if}

  {#key door.session}
    {#if door.session && (door.view === 'match' || (door.returnTo === 'match' && (door.view === 'settings' || door.view === 'licences')))}
      <div class="match" hidden={door.view !== 'match'}>
        <App session={door.session} />
      </div>
    {/if}
  {/key}

  {#if door.view === 'match' && door.overlay === 'menu'}
    <MenuPopover clock={door.session?.clockText ?? ''} onchoose={choose} />
  {:else if door.view === 'match' && door.overlay === 'return'}
    <ConfirmDialog
      word="Leave the match"
      title="Return to the start screen?"
      lines={[
        `The match saves at ${door.saveClock} and waits on the start screen under Resume.`,
        'The other matches of the round save with it.',
      ]}
      primary="Save and leave"
      busy={door.busy}
      onconfirm={() => door.returnToStart()}
      oncancel={() => door.closeMenu()}
    />
  {:else if door.view === 'match' && door.overlay === 'quit'}
    <ConfirmDialog
      word="Quit"
      title="Quit Touchline?"
      lines={door.quitLines}
      primary="Save and quit"
      busy={door.busy}
      onconfirm={() => door.quit()}
      oncancel={() => door.closeMenu()}
    />
  {/if}

  <input class="file" type="file" accept=".smfx" tabindex="-1" aria-hidden="true" bind:this={fileInput} onchange={openFile} />
  <p class="vh" role="status" data-screen-announcer>{announced}</p>
</div>
</div>

<style>
  .fit,
  .stage,
  .match {
    width: 100%;
    height: 100%;
  }

  .fit {
    overflow: clip;
  }

  .stage {
    position: relative;
  }

  .match[hidden] {
    display: none;
  }

  .file {
    display: none;
  }

  /* Read by a screen reader, never drawn. */
  .vh {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: 0;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
</style>
