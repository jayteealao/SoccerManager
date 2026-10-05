<!-- The shell test page's content: the shell with the Match tab active, a fact strip, and a
     stub body. The fixture is the sample the approved boards use. -->
<script>
  import AppShell from './components/AppShell.svelte';
  import InfoStrip from './components/InfoStrip.svelte';
  import SectionLabel from './components/SectionLabel.svelte';
  import StubSection from './components/StubSection.svelte';

  const tabs = [
    { id: 'match', label: 'Match', active: true },
    { id: 'touchline', label: 'Touchline', stub: true },
    { id: 'tactics', label: 'Tactics', menu: true },
    { id: 'squad', label: 'Squad', menu: true, stub: true },
    { id: 'stats', label: 'Stats', menu: true, stub: true },
    { id: 'analysis', label: 'Analysis', menu: true, stub: true },
    { id: 'other-grounds', label: 'Other grounds', menu: true, stub: true },
    { id: 'highlights', label: 'Highlights', menu: true, stub: true },
  ];

  const facts = [
    { value: 'Ashford Rovers v Port Varrow', label: 'Heritage Cup · Round 4' },
    { value: '1 – 1', label: 'Score', num: true },
    { value: "67'", label: 'Minute', num: true },
    { value: '1×', label: 'Speed', num: true },
  ];
</script>

<AppShell
  title="Ashford Rovers 1–1 Port Varrow"
  subtitle="Heritage Cup · Round 4 · Live 67'"
  date="TUE 3 NOVEMBER"
  dateSub="Live 20:52"
  action="Resume"
  {tabs}
>
  {#snippet crest()}
    <svg class="crest" width="30" height="32" viewBox="0 0 24 26" aria-hidden="true">
      <path class="shield" d="M12 1 22 4v8c0 6.5-4.5 10.5-10 13C6.5 22.5 2 18.5 2 12V4z" />
      <path class="stripe" d="M2.6 8.4h18.8v2.6H2.6z" />
      <text x="12" y="19.4" text-anchor="middle">AR</text>
    </svg>
  {/snippet}

  <InfoStrip {facts} />
  <div class="stub-body">
    <SectionLabel label="Match" note="the screen body arrives with the match screen" />
    <!-- STUB: the match screen body; the shell test shows the shell only. -->
    <StubSection note="screen body" later>
      <p>The pitch, the playback row, the commentary and the other grounds.</p>
    </StubSection>
  </div>
</AppShell>

<style>
  .crest {
    flex: none;
  }

  .shield {
    fill: var(--club-sample);
    stroke: var(--band-ink);
    stroke-width: 1.3;
  }

  .stripe,
  text {
    fill: var(--band-ink);
  }

  text {
    font: 800 6.4px var(--fd);
  }

  .stub-body {
    margin-top: 12px;
  }

  p {
    margin: 0;
    color: var(--ink-2);
  }
</style>
