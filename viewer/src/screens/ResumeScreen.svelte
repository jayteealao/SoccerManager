<!-- The Resume a saved match screen, ported from the approved OldEngine board: the shell with
     RESUME A SAVED MATCH in the header, the save's day and NEW MATCH in the cyan block, the
     Saved matches tab (Replays is a stub), the 54 px fact strip (the saved match, who saved
     it, this version, the engines it carries, and Cannot resume with its reason), then two
     columns of 700 px and 1fr. The left column holds the alert that names the save's version,
     why it cannot continue and what to do, with Start a new match and Open a replay. The
     board's right column draws the other case, a save of the previous release, for contrast:
     that case has no screen of its own (it opens on the loading steps with the engine
     version), so the column stays empty here. Nothing on this screen deletes the save. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import Crest from '../components/Crest.svelte';
  import Glyph from '../components/Glyph.svelte';
  import InfoStrip from '../components/InfoStrip.svelte';
  import SurfacePanel from '../components/SurfacePanel.svelte';
  import { RAIL } from '../components/icons.js';
  import { resumeModel } from '../lib/resume.js';

  let { session } = $props();

  const TABS = [
    { id: 'saved', label: 'Saved matches', active: true },
    { id: 'replays', label: 'Replays', stub: true },
  ];
  const DOCUMENT = RAIL.find((g) => g?.id === 'prep');

  let model = $derived(resumeModel(session.resumeInfo));
  let fileInput = $state();

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
    await session.openReplay(bytes, file.name);
  }
</script>

{#if model}
  <AppShell
    section="home"
    title={model.title}
    subtitle={model.subtitle}
    date={model.date}
    dateSub={model.dateSub}
    action="New match"
    onaction={() => session.newMatchEngine()}
    busy={session.busy}
    tabs={TABS}
    navLabel="Saved match views"
  >
    {#snippet crest()}
      <Crest />
    {/snippet}
    <div class="screen" data-screen="resume">
      <InfoStrip facts={model.facts}>
        {#snippet lead()}
          <span class="lead"><Glyph glyph={DOCUMENT} size={20} /></span>
        {/snippet}
      </InfoStrip>

      <div class="cols">
        <div>
          <SurfacePanel kind="refusal" word="Cannot resume" title={model.heading}>
            <p>{model.body}</p>
            {#if model.hint}<p>{model.hint}</p>{/if}
            {#snippet actions()}
              <button class="btn cy" type="button" disabled={session.busy} onclick={() => session.newMatchEngine()}
                >Start a new match</button
              >
              <button class="btn gh" type="button" disabled={session.busy} onclick={pickReplay}>Open a replay</button>
            {/snippet}
          </SurfacePanel>
          <p class="note">{model.note}</p>
        </div>
        <div></div>
      </div>
    </div>

    <input
      class="file"
      type="file"
      accept=".smfx"
      tabindex="-1"
      aria-hidden="true"
      bind:this={fileInput}
      onchange={openFile}
    />
  </AppShell>
{/if}

<style>
  .screen {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .lead {
    display: inline-grid;
    place-items: center;
    color: var(--band-ink);
  }

  .cols {
    display: grid;
    grid-template-columns: 700px 1fr;
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

  .note {
    margin: 6px 0 0;
    font-size: 9.5px;
    color: var(--ink-3);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    height: 28px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-sm);
    font: 600 10px var(--fb);
    cursor: pointer;
  }

  .btn.cy {
    background: var(--cyan);
    color: var(--cyan-ink);
  }

  .btn.gh {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--control-edge);
    color: var(--ghost-ink);
  }

  .btn:hover:not(:disabled) {
    filter: brightness(1.12);
  }

  .btn:active:not(:disabled) {
    filter: brightness(0.92);
  }

  .btn:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .file {
    display: none;
  }
</style>
