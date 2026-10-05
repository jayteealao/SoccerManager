<!-- A confirmation over the paused match: the scrim, then a 460 px dialog with its cyan word,
     its title, its lines, the primary action and Keep playing. The primary action takes focus
     when it opens; Tab stays inside; Esc is Keep playing (the page's Esc handler). -->
<script>
  import { restoreFocus } from '../lib/focus.js';

  let { word, title, lines = [], primary, busy = false, onconfirm = () => {}, oncancel = () => {} } = $props();

  const uid = $props.id();
  let box = $state();

  $effect(() => {
    // Focus goes back where it came from when the dialog closes (WCAG 2.4.3).
    const before = globalThis.document?.activeElement;
    box?.querySelector('button')?.focus();
    return () => restoreFocus(before);
  });

  function trap(event) {
    if (event.key !== 'Tab') {
      return;
    }
    const buttons = [...box.querySelectorAll('button:not(:disabled)')];
    const at = buttons.indexOf(globalThis.document.activeElement);
    const next = buttons[(at + (event.shiftKey ? -1 : 1) + buttons.length) % buttons.length];
    next?.focus();
    event.preventDefault();
  }
</script>

<div class="scrim" role="presentation"></div>
<div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dlg-{uid}" bind:this={box} onkeydown={trap} tabindex="-1">
  <span class="word">{word}</span>
  <h2 id="dlg-{uid}">{title}</h2>
  {#each lines as line, i (i)}<p>{line}</p>{/each}
  <div class="actions">
    <button class="btn cy" type="button" disabled={busy} aria-busy={busy} data-confirm onclick={() => onconfirm()}>{primary}</button>
    <button class="btn gh" type="button" disabled={busy} data-cancel onclick={() => oncancel()}>Keep playing</button>
  </div>
</div>

<style>
  .scrim {
    position: absolute;
    inset: 0;
    background: var(--scrim);
    z-index: 4;
    animation: fade 150ms var(--ease) both;
  }

  /* Centred across, 250 px down at 1280 by 800, as the board places it. */
  .dialog {
    position: absolute;
    left: 50%;
    top: min(250px, 30%);
    width: min(460px, calc(100% - 32px));
    transform: translateX(-50%);
    background: var(--rail);
    box-shadow:
      inset 0 0 0 1px var(--rule),
      var(--popover-shadow);
    border-radius: var(--radius-md);
    z-index: 5;
    padding: 16px 18px;
    animation: fade 150ms var(--ease) both;
  }

  .dialog:focus {
    outline: none;
  }

  .word {
    display: block;
    font: 700 10px var(--fd);
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--cyan);
  }

  h2 {
    margin: 4px 0 8px;
    font: 800 18px var(--fd);
    letter-spacing: 0.02em;
    text-transform: uppercase;
    color: var(--ink);
  }

  p {
    margin: 0 0 6px;
    font-size: 11px;
    line-height: 1.5;
    color: var(--ink-2);
  }

  .actions {
    display: flex;
    gap: 6px;
    margin-top: 10px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    height: max(28px, var(--hit));
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

  .btn:disabled {
    cursor: default;
    opacity: 0.55;
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
