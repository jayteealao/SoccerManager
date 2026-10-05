<!-- The Touchline Full Game shell: the 40 px icon rail, the 52 px slanted navy header band
     with the date block and the cyan action block, the 30 px sub-navigation tabs, and the
     body. Only the current rail section, the live tabs and the action block take focus; the
     other rail sections, the stub tabs and the header icons are inert stubs. An action note
     names a later step beside the action block (TEAM TALK on the Pre-match line-ups); it is
     a stub with the LATER mark. `navLabel` names the sub-navigation, and `current` is the
     `aria-current` value of its active tab: `page` for views, `step` for a stepper. `menu`, when given, draws
     the Menu button before the header icons (the in-match menu of the front door); without it
     the shell draws no button, as before. `menuOpen` says whether that menu is open.

     The shell fills its box at every window step (skins/base.css) and is the size container
     `shell`: the header band takes the spare width and its title ends in an ellipsis; below
     1280 px the stub header icons hide and the Menu button stays; at the compact step the
     tabs scroll sideways and every control is at least 44 px. -->
<script>
  import Glyph from './Glyph.svelte';
  import StubSection from './StubSection.svelte';
  import { BACK, CARET, FORWARD, GLOBE, MENU, RAIL, SEARCH, UP_DOWN } from './icons.js';

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
    actionNote = '',
    navLabel = 'Match views',
    current = 'page',
    crest,
    menu = null,
    menuOpen = false,
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
        <div class="t" data-may-truncate>
          <b>{title}</b>
          <span>{subtitle}</span>
        </div>
      </div>
      <!-- STUB: the world view and help. -->
      <div class="hicons">
        {#if menu}
          <button
            class="menu"
            type="button"
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            data-menu-button
            onclick={() => menu()}
          >
            <Glyph glyph={{ d: MENU }} size={12} />
            Menu
            <span class="esc">Esc</span>
          </button>
        {/if}
        {#if actionNote}
          <!-- STUB: a step this workflow does not build, named beside the action block. -->
          <StubSection note="action note: {actionNote}" inline later>
            <span class="anote">{actionNote}</span>
          </StubSection>
        {/if}
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

    <nav class="subnav" aria-label={navLabel}>
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
            aria-current={tab.active ? current : undefined}
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
  /* The shell fills the window, so it draws no frame radius and no frame ring. */
  .app {
    width: 100%;
    height: 100%;
    display: flex;
    background: var(--ground);
    container: shell / size;
    /* Clipped, not hidden: a hidden box still scrolls when focus or a click brings a part
       past its edge into view, which shifts the whole stage sideways. */
    overflow: clip;
    position: relative;
    font: 400 11px/1.35 var(--fb);
    color: var(--ink);
    text-align: left;
  }

  /* Above the body, so the current section's hit area may reach past the rail's edge. */
  .rail {
    position: relative;
    z-index: 1;
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

  /* The drawn glyph is 15 px; the hit area is the rail's full 40 px width and at least the
     step's hit height. */
  .rail a.on::after {
    content: '';
    position: absolute;
    inset: min(-6px, calc((15px - var(--hit)) / 2)) min(-12px, calc((15px - var(--hit)) / 2));
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

  /* The band takes the spare width; the date and action blocks keep their size. */
  .band {
    position: relative;
    height: 52px;
    flex: 1 1 0;
    min-width: 0;
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
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .t span {
    font-size: 10px;
    color: var(--navy-sub);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .hicons {
    flex: none;
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

  /* Below 1280 px the stub header icons give their room to the band; the Menu button is a
     live control and stays. */
  @container shell (max-width: 1279px) {
    .hicons {
      padding: 0;
    }

    .menu {
      margin: 0 12px;
    }

    .hicons :global(.stub) {
      display: none;
    }
  }

  .menu {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: var(--hit);
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--control-edge);
    color: var(--ghost-ink);
    font: 600 10px var(--fb);
    cursor: pointer;
    transition: filter var(--tab-change) var(--ease);
  }

  .menu:hover {
    filter: brightness(1.12);
  }

  .menu:active {
    filter: brightness(0.92);
  }

  .menu .esc {
    font-weight: 400;
    color: var(--ink-3);
  }

  .anote {
    font: 700 10.5px var(--fd);
    letter-spacing: 0.1em;
    text-transform: uppercase;
    white-space: nowrap;
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
    scroll-margin-inline: var(--gutter);
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

  /* The body scrolls up and down when its panels are taller than the window, never
     sideways. */
  .body {
    flex: 1;
    min-height: 0;
    position: relative;
    padding: 0 var(--gutter) 14px;
    display: block;
    overflow: hidden auto;
    scrollbar-width: thin;
  }

  /* The compact step: a narrower action block, 44 px tabs that scroll sideways, and the
     16 px gutter (from --gutter). */
  @media (max-width: 1023px), (max-height: 599px) {
    .cont {
      width: 150px;
    }

    .subnav {
      height: 44px;
      margin-top: 2px;
      gap: 6px;
      padding: 0 var(--gutter);
      overflow-x: auto;
      scrollbar-width: none;
    }

    .tab {
      min-height: 36px;
      padding: 0 10px;
    }

    .tab.on {
      padding: 0 10px;
    }

    button.tab::after {
      inset: -4px 0;
    }
  }
</style>
