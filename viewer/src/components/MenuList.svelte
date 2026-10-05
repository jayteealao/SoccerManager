<!-- The start screen's option list: one row per choice, each a button with its glyph, its
     word in the display face and a grey line under it. The first live row is drawn in cyan as
     the default choice. A row with `rule` sits after a 1 px rule (Quit). A disabled row keeps
     its place and says why on its second line. ↑ and ↓ move between the live rows; Enter or
     Space chooses. A row with no glyph draws none; `size="small"` draws the report's smaller
     rows (the full-time Next list). -->
<script>
  import Glyph from './Glyph.svelte';

  let { items = [], onchoose = () => {}, label = 'Start', size = 'default' } = $props();

  let list = $state();

  function move(event) {
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') {
      return;
    }
    const buttons = [...list.querySelectorAll('button:not(:disabled)')];
    const at = buttons.indexOf(globalThis.document.activeElement);
    const step = event.key === 'ArrowDown' ? 1 : -1;
    const next = buttons[(at + step + buttons.length) % buttons.length];
    next?.focus();
    event.preventDefault();
  }
</script>

<!-- The group hands ↑ and ↓ to its buttons; each button takes its own Enter and Space. -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="list" class:small={size === 'small'} role="group" aria-label={label} bind:this={list} onkeydown={move}>
  {#each items as item (item.id)}
    {#if item.rule}<div class="hr" role="presentation"></div>{/if}
    <button
      type="button"
      class="row"
      class:on={item.primary}
      disabled={item.disabled}
      aria-disabled={item.disabled ? 'true' : undefined}
      data-choice={item.id}
      onclick={() => onchoose(item.id)}
    >
      {#if item.glyph}<span class="icon"><Glyph glyph={item.glyph} size={15} /></span>{/if}
      <span class="text">
        <b>{item.label}</b>
        <span class="sub">{item.sub}</span>
      </span>
    </button>
  {/each}
</div>

<style>
  .list {
    display: flex;
    flex-direction: column;
  }

  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 12px;
    margin-bottom: 4px;
    border: 0;
    border-radius: var(--radius-sm);
    text-align: left;
    cursor: pointer;
    background: var(--ground-2);
    color: var(--ink);
    transition: filter var(--tab-change) var(--ease);
  }

  .row.on {
    background: var(--cyan);
    color: var(--cyan-ink);
  }

  .row:hover:not(:disabled) {
    filter: brightness(1.12);
  }

  .row:active:not(:disabled) {
    filter: brightness(0.92);
  }

  .row:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .icon {
    width: 18px;
    display: grid;
    place-items: center;
    flex: none;
  }

  .text {
    min-width: 0;
  }

  .text b {
    display: block;
    font: 700 13px var(--fd);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .sub {
    font-size: 10px;
    color: var(--ink-3);
  }

  .row.on .sub {
    color: inherit;
  }

  .hr {
    height: 1px;
    background: var(--rule);
    margin: 10px 0;
  }

  .small .row {
    padding: 8px 12px;
  }

  .small .text b {
    font-size: 12px;
  }

  .small .sub {
    font-size: 9.5px;
    color: var(--ink-2);
  }

  .small .row.on .sub {
    color: inherit;
  }
</style>
