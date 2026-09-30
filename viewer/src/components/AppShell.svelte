<!-- The Touchline Full Game shell: the 40 px icon rail, the 52 px slanted navy header band
     with the date block and the cyan action block, the 30 px sub-navigation tabs, and the
     body. Only the current rail section, the live tabs and the action block take focus; the
     other rail sections, the stub tabs and the header icons are inert stubs. -->
<script>
  import Glyph from './Glyph.svelte';
  import StubSection from './StubSection.svelte';
  import { BACK, CARET, FORWARD, GLOBE, RAIL, SEARCH, UP_DOWN } from './icons.js';

  let {
    section = 'match',
    title,
    subtitle = '',
    date,
    dateSub = '',
    action,
    onaction = () => {},
    busy = false,
    tabs = [],
    ontab = () => {},
    crest,
    children,
  } = $props();
</script>

<div class="app">
  <nav class="rail" aria-label="Sections">
    {#each RAIL as glyph, i (glyph?.id ?? `gap-${i}`)}
      {#if glyph === null}
        <span class="gap"></span>
      {:else if glyph.id === section}
        <a class="on" href="#{glyph.id}" aria-current="page" aria-label={glyph.label}>
          <Glyph {glyph} />
        </a>
      {:else}
        <!-- STUB: this rail section is not built in the viewer; drawn for layout and feel only. -->
        <StubSection note="rail section: {glyph.id}" inline fade={false}>
          <span class="glyph"><Glyph {glyph} /></span>
        </StubSection>
      {/if}
    {/each}
    <!-- STUB: the inbox count and the new-item mark. -->
    <div class="rail-foot">
      <StubSection note="rail badge" fade={false}>
        <span class="badge num">12</span>
        <span class="dotb"></span>
      </StubSection>
    </div>
  </nav>

  <div class="main">
    <header class="hd">
      <div class="band">
        <!-- STUB: back and forward through screens. -->
        <StubSection note="screen history arrows" inline fade={false}>
          <span class="arr">
            <Glyph glyph={{ d: BACK }} size={13} />
            <Glyph glyph={{ d: FORWARD }} size={13} />
          </span>
        </StubSection>
        {#if crest}{@render crest()}{/if}
        <!-- STUB: the club switcher and search. -->
        <StubSection note="club switcher and search" inline fade={false}>
          <span class="ud"><Glyph glyph={{ stroke: UP_DOWN, width: 1.7 }} size={12} /></span>
          <Glyph glyph={{ stroke: SEARCH, width: 1.9 }} size={13} />
        </StubSection>
        <div class="t">
          <b>{title}</b>
          <span>{subtitle}</span>
        </div>
      </div>
      <!-- STUB: the world view and help. -->
      <div class="hicons">
        <StubSection note="world view and help" inline fade={false}>
          <Glyph glyph={{ stroke: GLOBE, width: 1.3 }} size={17} />
          <span class="help">?</span>
        </StubSection>
      </div>
      <div class="date">
        <b>{date}</b>
        <span>{dateSub}</span>
      </div>
      <button class="cont" type="button" disabled={busy} aria-busy={busy} onclick={() => busy || onaction()}>
        {action}
      </button>
    </header>

    <nav class="subnav" aria-label="Match views">
      {#each tabs as tab (tab.id)}
        {#if tab.stub}
          <!-- STUB: a view this workflow does not build; drawn for layout and feel only. -->
          <StubSection note="tab: {tab.id}" inline fade={false}>
            <span class="tab">
              {tab.label}{#if tab.menu}<Glyph glyph={{ stroke: CARET, width: 1.8 }} size={8} />{/if}
            </span>
          </StubSection>
        {:else}
          <button
            class="tab"
            class:on={tab.active}
            type="button"
            aria-current={tab.active ? 'page' : undefined}
            onclick={() => ontab(tab.id)}
          >
            {tab.label}{#if tab.menu}<Glyph glyph={{ stroke: CARET, width: 1.8 }} size={8} />{/if}
          </button>
        {/if}
      {/each}
    </nav>

    <main class="body">
      {@render children?.()}
    </main>
  </div>
</div>

<style>
  .app {
    width: 1280px;
    height: 800px;
    display: flex;
    background: var(--ground);
    border-radius: var(--radius-lg);
    overflow: hidden;
    position: relative;
    font: 400 11px/1.35 var(--fb);
    color: var(--ink);
    box-shadow: 0 0 0 1px var(--frame-ring);
    text-align: left;
  }

  .rail {
    width: 40px;
    flex: none;
    background: var(--rail);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 14px 0 12px;
    gap: 12px;
    color: var(--rail-glyph);
  }

  .rail .gap {
    height: 5px;
  }

  .rail a,
  .rail .glyph {
    display: inline-grid;
    place-items: center;
    color: inherit;
    line-height: 0;
  }

  .rail a.on {
    position: relative;
    color: var(--cyan);
    border-radius: var(--radius-sm);
  }

  /* The drawn glyph is 15 px; the hit area is the rail's full 40 px width. */
  .rail a.on::after {
    content: '';
    position: absolute;
    inset: -6px -12px;
  }

  .rail-foot {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }

  .badge {
    width: 21px;
    height: 21px;
    border: 1.5px solid var(--rail-badge);
    border-radius: 50%;
    font: 600 9px/18px var(--fd);
    text-align: center;
  }

  .dotb {
    width: 6px;
    height: 4px;
    background: var(--cyan);
    border-radius: 1px;
  }

  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    position: relative;
  }

  .hd {
    height: 52px;
    display: flex;
    align-items: flex-start;
    flex: none;
  }

  .band {
    position: relative;
    height: 52px;
    width: 790px;
    flex: none;
    background: linear-gradient(
      90deg,
      var(--navy-700) 0%,
      var(--navy-500) 62%,
      var(--navy-600) 100%
    );
    clip-path: polygon(0 0, 100% 0, calc(100% - 20px) 100%, 0 100%);
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 40px 0 20px;
    overflow: hidden;
    color: var(--band-ink);
  }

  .band::after {
    content: '';
    position: absolute;
    right: 26px;
    top: -24px;
    width: 330px;
    height: 110px;
    background: repeating-linear-gradient(
      115deg,
      transparent 0 24px,
      var(--band-watermark) 24px 50px
    );
    pointer-events: none;
  }

  .arr {
    display: flex;
    gap: 8px;
    margin-right: 18px;
  }

  .ud {
    color: var(--band-arrow);
    display: inline-flex;
  }

  .t {
    display: flex;
    flex-direction: column;
    line-height: 1.15;
    margin-left: 4px;
    min-width: 0;
  }

  .t b {
    font: 700 14.5px var(--fd);
    letter-spacing: 0.03em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .t span {
    font-size: 10px;
    color: var(--navy-sub);
    white-space: nowrap;
  }

  .hicons {
    flex: 1;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 11px;
    height: 44px;
    padding-right: 24px;
    color: var(--header-icon);
  }

  .hicons :global(.stub) {
    display: inline-flex;
    align-items: center;
    gap: 11px;
  }

  .help {
    width: 17px;
    height: 17px;
    border-radius: 50%;
    background: var(--help-ground);
    color: var(--help-ink);
    display: grid;
    place-items: center;
    font: 800 11px/1 var(--fd);
  }

  .date {
    width: 152px;
    height: 44px;
    flex: none;
    background: var(--navy-700);
    clip-path: polygon(18px 0, 100% 0, calc(100% - 18px) 100%, 0 100%);
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: flex-end;
    padding-right: 28px;
    margin-right: -14px;
    line-height: 1.2;
    color: var(--band-ink);
  }

  .date b {
    font: 700 11px var(--fd);
    letter-spacing: 0.05em;
  }

  .date span {
    font-size: 9.5px;
    color: var(--navy-sub);
  }

  .cont {
    width: 180px;
    height: 44px;
    flex: none;
    border: 0;
    background: var(--cyan);
    color: var(--cyan-ink);
    clip-path: polygon(18px 0, 100% 0, 100% 100%, 0 100%);
    font: 800 12.5px var(--fd);
    letter-spacing: 0.1em;
    text-transform: uppercase;
    cursor: pointer;
    padding-left: 14px;
    transition: filter var(--tab-change) var(--ease);
  }

  .cont:hover:not(:disabled) {
    filter: brightness(1.12);
  }

  .cont:active:not(:disabled) {
    filter: brightness(0.92);
  }

  /* The slant's clip-path would cut an outside ring off, so the ring is drawn inside. */
  .cont:focus-visible {
    outline: 2px solid var(--cyan-ink);
    outline-offset: -6px;
  }

  .cont:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .subnav {
    height: 30px;
    display: flex;
    align-items: center;
    gap: 15px;
    padding: 0 20px;
    flex: none;
    margin-top: 5px;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font: inherit;
    font-size: 10.5px;
    line-height: 1.35;
    color: var(--tab-ink);
    white-space: nowrap;
    background: none;
    border: 0;
    padding: 0;
    cursor: pointer;
    border-radius: var(--radius-sm);
    transition:
      background-color var(--tab-change) var(--ease),
      color var(--tab-change) var(--ease);
  }

  button.tab:hover:not(.on) {
    color: var(--cyan);
  }

  button.tab:active:not(.on) {
    opacity: 0.8;
  }

  /* A tab is drawn at its text height; the hit area reaches 24 px. */
  button.tab {
    position: relative;
  }

  button.tab::after {
    content: '';
    position: absolute;
    inset: -4px -7px;
  }

  .tab.on {
    background: var(--cyan);
    color: var(--cyan-ink);
    padding: 1px 8px;
    font-weight: 600;
  }

  .body {
    flex: 1;
    min-height: 0;
    position: relative;
    padding: 0 20px 14px;
    display: block;
  }
</style>
