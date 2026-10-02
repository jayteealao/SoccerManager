<!-- Licences and about, ported from the approved Licences board: the shell with LICENCES AND
     ABOUT in the header and BACK TO START in the cyan block; the 54 px fact strip; then three
     columns of 290 px, 1fr and 380 px. About names the versions and the protocol; Made by
     shows the studio mark (a placeholder) and the publisher's name. The middle column lists
     every package of the notices file the build wrote; the right column shows the selected
     package's licence text. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import InfoStrip from '../components/InfoStrip.svelte';
  import KvRow from '../components/KvRow.svelte';
  import NoticesTable from '../components/NoticesTable.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import StubSection from '../components/StubSection.svelte';
  import TouchlineMark from '../components/TouchlineMark.svelte';
  import { licenceFacts, versions } from '../lib/front-door-model.js';

  let { door } = $props();

  const TABS = [
    { id: 'start', label: 'Start' },
    { id: 'replays', label: 'Replays', stub: true },
    { id: 'settings', label: 'Settings' },
    { id: 'licences', label: 'Licences and about', active: true },
  ];

  let selected = $state(0);
  let v = $derived(versions(door.status));
  let file = $derived(door.notices.file);
  let packages = $derived(file?.packages ?? []);
  let chosen = $derived(packages[selected] ?? null);
  let back = $derived(door.returnTo === 'match' ? 'Back to match' : 'Back to start');

  function tab(id) {
    if (id === 'start') {
      door.back();
    } else if (id === 'settings') {
      door.open('settings');
    }
  }
</script>

<AppShell
  section="home"
  title="Licences and about"
  subtitle="Touchline {v.touchline} · open-source notices generated at build"
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
  <div class="screen" data-screen="licences">
    <InfoStrip facts={licenceFacts(door.status, file)}>
      {#snippet lead()}
        <TouchlineMark size={24} />
      {/snippet}
    </InfoStrip>

    <div class="cols">
      <div>
        <SectionLabel label="About" />
        <KvRow key="Touchline" value={v.touchline} />
        <KvRow key="Match engine" value={v.engine} />
        <KvRow key="Previous engine" value={v.previous ?? 'None'} />
        <KvRow key="Protocol" value={door.status?.['protocol.version'] ?? '—'} />
        <p class="note">
          Touchline is open source under the MIT licence or the Apache licence 2.0, at your choice. The previous engine
          ships beside this one so a match saved on it finishes on it.
        </p>
        <div class="hr" role="presentation"></div>
        <SectionLabel label="Made by" />
        <!-- STUB: the studio mark; the publisher's own mark replaces this placeholder. -->
        <StubSection note="studio mark placeholder" fade={false}>
          <span class="placeholder">[STUDIO MARK]</span>
        </StubSection>
        <p class="maker">Soccer Manager contributors</p>
      </div>

      <div>
        <SectionLabel label="Open-source notices" note="the same list ships in the release folder" />
        {#if door.notices.state === 'ready'}
          <NoticesTable {packages} {selected} onselect={(i) => (selected = i)} />
        {:else if door.notices.state === 'missing'}
          <p class="note" role="status">This build carries no notices file. The release build writes notices.json beside the page.</p>
        {:else}
          <p class="note" role="status">Reading the notices…</p>
        {/if}
      </div>

      <div>
        <SectionLabel
          label="Licence text"
          note={chosen ? `${chosen.name}${chosen.version ? ` ${chosen.version}` : ''} · ${chosen.licence}` : ''}
        />
        <!-- The text scrolls, so it takes focus for the keyboard to scroll it. -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <pre class="text" tabindex="0" aria-label="Licence text">{chosen?.text ?? ''}</pre>
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
    grid-template-columns: 290px 1fr 380px;
    margin-top: 12px;
    min-height: 0;
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

  .maker {
    margin: 6px 0 0;
    font-size: 10.5px;
  }

  .text {
    height: 520px;
    margin: 0;
    overflow: auto;
    background: var(--rail);
    box-shadow: inset 0 0 0 1px var(--rule);
    border-radius: 3px;
    padding: 10px 12px;
    font: 500 10px/1.6 var(--fm);
    color: var(--ink-2);
    white-space: pre-wrap;
  }
</style>
