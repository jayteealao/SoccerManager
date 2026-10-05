<!-- The handshake page, ported from the sketch's Handshake board: the shell with HANDSHAKE in
     the header and RUN AGAIN in the cyan block; the 54 px fact strip (isolation, the memory
     gauge, the tick frames counted and the socket, each as a word); then "What this page
     does" and the log, newest last. The page opens the engine's socket, prints the hello and
     a sample of the tick frames as text, and counts frames until the close, so a fault it
     shows is a fault in the handshake and never in the renderer. -->
<script>
  import AppShell from '../components/AppShell.svelte';
  import DiagnosticLog from '../components/DiagnosticLog.svelte';
  import Glyph from '../components/Glyph.svelte';
  import InfoStrip from '../components/InfoStrip.svelte';
  import SectionLabel from '../components/SectionLabel.svelte';
  import { RAIL } from '../components/icons.js';

  let { run } = $props();

  const TABS = [{ id: 'handshake', label: 'Handshake', active: true }];
  const ANALYSIS = RAIL.find((g) => g?.id === 'analysis');
  const TONE = { ok: 'good', bad: 'bad', plain: null };

  let facts = $derived([
    { value: 'Handshake', label: 'Connection check for the match viewer' },
    ...run.facts.map((f) => ({ value: f.value, label: f.label, tone: TONE[f.kind], num: f.label === 'Tick frames' })),
  ]);
</script>

<AppShell
  section="analysis"
  title="Handshake"
  subtitle="Touchline · diagnostic page"
  date="DIAGNOSTIC"
  dateSub={run.running ? 'Running' : 'Done'}
  action="Run again"
  onaction={() => run.run()}
  tabs={TABS}
  navLabel="Diagnostic views"
>
  <div class="screen" data-screen="handshake">
    <InfoStrip {facts}>
      {#snippet lead()}
        <span class="lead"><Glyph glyph={ANALYSIS} size={18} /></span>
      {/snippet}
    </InfoStrip>

    <div class="cols">
      <div>
        <SectionLabel label="What this page does" />
        <p class="about">
          This page opens the engine's socket, prints the hello message, and counts tick frames
          until the socket closes. It also checks that the page is isolated, which the memory
          gauge needs.
        </p>
      </div>
      <div class="logcol">
        <SectionLabel label="Log" note="newest last" />
        <DiagnosticLog lines={run.lines} label="Handshake log, newest last" />
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

  .lead {
    display: inline-grid;
    place-items: center;
    color: var(--band-ink);
  }

  .cols {
    display: grid;
    grid-template-columns: minmax(0, 380fr) minmax(0, 815fr);
    grid-template-rows: minmax(0, 1fr);
    margin-top: 12px;
    flex: 1;
    min-height: 0;
  }

  .cols > div {
    min-width: 0;
    min-height: 0;
  }

  .logcol {
    border-left: 1px solid var(--rule);
    padding: 0 18px;
    overflow-y: auto;
  }

  .cols > div:first-child {
    padding-right: 18px;
  }

  .about {
    margin: 0;
    font-size: 11px;
    line-height: 1.55;
    color: var(--ink-2);
  }
</style>
