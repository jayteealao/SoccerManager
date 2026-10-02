<!-- Settings, ported from the approved Settings board: the shell with SETTINGS in the header
     and BACK TO START in the cyan block (back to the paused match when opened from its menu);
     the 54 px fact strip; then three columns of 330 px, 1fr and 360 px. Playback holds the
     default speed and Commentary its column; Motion holds the animation setting and the
     versions. Each change is saved at once and kept for the next launch. Look and access is a
     LATER stub. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import InfoStrip from '../components/InfoStrip.svelte';
  import KvRow from '../components/KvRow.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import Segmented from '../components/Segmented.svelte';
  import StubSection from '../components/StubSection.svelte';
  import TouchlineMark from '../components/TouchlineMark.svelte';
  import {
    COMMENTARY_OPTIONS,
    MOTION_OPTIONS,
    SPEED_OPTIONS,
    settingsFacts,
    versions,
  } from '../lib/front-door-model.js';

  let { door, systemReduces = false } = $props();

  const TABS = [
    { id: 'start', label: 'Start' },
    { id: 'replays', label: 'Replays', stub: true },
    { id: 'settings', label: 'Settings', active: true },
    { id: 'licences', label: 'Licences and about' },
  ];
  const LOOK = [
    ['Skin', 'Broadcast Blue'],
    ['Text size', '100%'],
    ['Colour modes', 'Standard'],
  ];

  let v = $derived(versions(door.status));
  let back = $derived(door.returnTo === 'match' ? 'Back to match' : 'Back to start');

  function tab(id) {
    if (id === 'start') {
      door.back();
    } else if (id === 'licences') {
      door.open('licences');
    }
  }
</script>

<AppShell
  section="home"
  title="Settings"
  subtitle="Saved as you change them · kept for the next launch"
  date="TOUCHLINE {v.touchline}"
  dateSub="Engine ready"
  action={back}
  onaction={() => door.back()}
  tabs={TABS}
  ontab={tab}
  navLabel="Start screen views"
>
  {#snippet crest()}
    <TouchlineMark size={30} />
  {/snippet}
  <div class="screen" data-screen="settings">
    <InfoStrip facts={settingsFacts(door.settings, systemReduces, door.saved)} />

    <div class="cols">
      <div>
        <SectionLabel label="Playback" />
        <div class="row">
          <span>Default speed</span>
          <Segmented
            label="Default speed"
            options={SPEED_OPTIONS}
            value={door.settings.speed}
            onchange={(speed) => door.saveSettings({ speed })}
          />
        </div>
        <p class="note">A new match starts at this speed. The playback row still changes it during a match.</p>
        <div class="hr" role="presentation"></div>
        <SectionLabel label="Commentary" />
        <div class="row">
          <span>Commentary column</span>
          <Segmented
            label="Commentary"
            options={COMMENTARY_OPTIONS}
            value={door.settings.commentary}
            onchange={(commentary) => door.saveSettings({ commentary })}
          />
        </div>
        <p class="note">
          Off hides the commentary column. Goals, cards and changes still show on the pitch and in the score strip.
        </p>
      </div>

      <div>
        <SectionLabel label="Motion" />
        <div class="row">
          <span>Animation</span>
          <Segmented
            label="Animation"
            options={MOTION_OPTIONS}
            value={door.settings.motion}
            onchange={(motion) => door.saveSettings({ motion })}
          />
        </div>
        <p class="note">
          Reduce removes the LIVE pulse, the goal banner, the new-goal outline fade and the splash animation. Follow
          system uses your operating system's setting, which is {systemReduces ? 'reduced motion' : 'full motion'} now.
        </p>
        <div class="hr" role="presentation"></div>
        <SectionLabel label="Versions" note="read only" />
        <KvRow key="Touchline" value={v.touchline} />
        <KvRow key="Match engine" value={v.engine} />
        <KvRow key="Previous engine" value={v.previous ? `${v.previous} · finishes matches saved on it` : 'None'} />
        <button class="more" type="button" onclick={() => door.open('licences')}>Licences and about</button>
      </div>

      <div>
        <!-- STUB: look and access settings: skin picker, text size, colour modes. -->
        <SectionLabel label="Look and access" later />
        <StubSection note="look and access settings: skin picker, text size, colour modes">
          {#each LOOK as [name, value] (name)}
            <div class="look"><span>{name}</span><b>{value}</b><span class="later">LATER</span></div>
          {/each}
        </StubSection>
      </div>
    </div>
  </div>
</AppShell>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .cols {
    display: grid;
    grid-template-columns: 330px 1fr 360px;
    margin-top: 12px;
  }

  .cols > div {
    min-width: 0;
    padding-right: 18px;
  }

  .cols > div + div {
    border-left: 1px solid var(--rule);
    padding-left: 18px;
  }

  .cols > div:last-child {
    padding-right: 0;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    padding: 6px 0;
    color: var(--ink-2);
  }

  .note {
    margin: 5px 0 0;
    font-size: 9.5px;
    line-height: 1.45;
    color: var(--ink-3);
  }

  .hr {
    height: 1px;
    background: var(--rule);
    margin: 12px 0;
  }

  .more {
    margin-top: 6px;
    border: 0;
    padding: 0;
    background: none;
    color: var(--cyan);
    font: 600 10.5px var(--fb);
    cursor: pointer;
    border-radius: var(--radius-sm);
  }

  .more:hover {
    text-decoration: underline;
  }

  .look {
    display: flex;
    align-items: center;
    gap: 8px;
    border-bottom: 1px solid var(--rule-2);
    padding: 5px 0;
    color: var(--ink-3);
  }

  .look span:first-child {
    flex: 1;
  }

  .later {
    font: 700 8.5px var(--fd);
    letter-spacing: 0.08em;
  }
</style>
