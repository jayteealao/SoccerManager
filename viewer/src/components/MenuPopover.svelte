<!-- The in-match menu: a 250 px popover under the header, beside the Menu button, over the
     paused match. Its header names the pause and the clock; then Resume match (Esc), Return to
     start screen, Settings, Licences and about, a rule, and Quit Touchline. The first item
     takes focus when it opens; ↑ and ↓ move, Tab stays inside. -->
<script>
  import { restoreFocus } from '../lib/focus.js';
  import Glyph from './Glyph.svelte';
  import { DOCUMENT, HOME, LEAVE, PLAY, SLIDERS } from './icons.js';

  let { clock = '', onchoose = () => {} } = $props();

  const ITEMS = [
    { id: 'resume', glyph: { d: PLAY }, label: 'Resume match', key: 'Esc' },
    { id: 'return', glyph: { d: HOME }, label: 'Return to start screen' },
    { id: 'settings', glyph: { d: SLIDERS }, label: 'Settings' },
    { id: 'licences', glyph: { d: DOCUMENT }, label: 'Licences and about' },
    { id: 'quit', glyph: { d: LEAVE }, label: 'Quit Touchline', rule: true },
  ];

  let menu = $state();

  $effect(() => {
    // Focus goes back where it came from when the menu closes (WCAG 2.4.3).
    const before = globalThis.document?.activeElement;
    menu?.querySelector('button')?.focus();
    return () => restoreFocus(before);
  });

  function keys(event) {
    const buttons = [...menu.querySelectorAll('button')];
    const at = buttons.indexOf(globalThis.document.activeElement);
    let next = null;
    if (event.key === 'ArrowDown' || (event.key === 'Tab' && !event.shiftKey)) {
      next = buttons[(at + 1) % buttons.length];
    } else if (event.key === 'ArrowUp' || (event.key === 'Tab' && event.shiftKey)) {
      next = buttons[(at - 1 + buttons.length) % buttons.length];
    } else if (event.key === 'Home') {
      next = buttons[0];
    } else if (event.key === 'End') {
      next = buttons[buttons.length - 1];
    }
    if (next) {
      next.focus();
      event.preventDefault();
    }
  }
</script>

<div class="menu" role="menu" aria-label="Match menu" tabindex="-1" bind:this={menu} onkeydown={keys}>
  <div class="head">Match paused · <span class="num">{clock}</span></div>
  {#each ITEMS as item (item.id)}
    {#if item.rule}<div class="rule" role="separator"></div>{/if}
    <button type="button" role="menuitem" data-item={item.id} onclick={() => onchoose(item.id)}>
      <Glyph glyph={item.glyph} size={12} />
      <span class="label">{item.label}</span>
      {#if item.key}<span class="key">{item.key}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .menu {
    position: absolute;
    top: 52px;
    right: 380px;
    width: 250px;
    background: var(--well);
    border: 1px solid var(--seg-edge);
    border-radius: 3px;
    box-shadow: var(--popover-shadow);
    padding: 6px;
    z-index: 5;
    animation: open 150ms var(--ease) both;
  }

  .menu:focus {
    outline: none;
  }

  .head {
    padding: 4px 8px 6px;
    font: 700 10px var(--fd);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--cyan);
  }

  button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 9px;
    height: max(30px, var(--hit));
    padding: 0 8px;
    border: 0;
    border-radius: var(--radius-sm);
    text-align: left;
    font: 600 11px var(--fb);
    cursor: pointer;
    background: transparent;
    color: var(--ink);
  }

  button:focus,
  button:hover {
    background: var(--cyan);
    color: var(--cyan-ink);
  }

  .label {
    flex: 1;
  }

  .key {
    font-size: 9.5px;
    opacity: 0.8;
  }

  .rule {
    border-top: 1px solid var(--rule);
    margin: 6px 0;
  }

  @keyframes open {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
