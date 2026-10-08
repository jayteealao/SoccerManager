<!-- The column menu of the squad table (board 2): a popover under the Columns chip. "Shown,
     in order" lists the view's columns, each with a ⋮⋮ grip to drag and ↑ ↓ ✕ buttons, so every
     move has a keyboard path; "Add a column" lists the columns not shown, by group; the greyed
     Later group names the columns the engine has no model for yet. Reset to default and Done
     close it. Focus moves into the menu when it opens and back to the Columns chip when it
     closes; Escape closes it; a moved row keeps focus, a removed row hands focus to the row
     after it (or before it), and an added column's new row takes focus. At the compact step,
     where the toolbar wraps, it hangs under the whole toolbar (SquadScreen makes the toolbar
     its box), no wider than it, with its two lists stacked. It opens in 150 ms with the product
     ease, and not at all under reduced motion (skins/base.css). -->
<script>
  import { tick } from 'svelte';

  import StubSection from './StubSection.svelte';
  import { choosable, columnIndex, LATER_COLUMNS, MENU_GROUPS } from '../lib/columns.js';
  import { restoreFocus } from '../lib/focus.js';
  import { addColumn, moveColumn, moveTo, removeColumn } from '../lib/views.js';

  let { view, viewWord = '', onchange = () => {}, onreset = () => {}, ondone = () => {} } = $props();

  let box = $state();
  let dragFrom = $state(null);

  const index = columnIndex();
  let shown = $derived(view.columns.map((id) => index.get(id)).filter(Boolean));
  let groups = $derived(
    MENU_GROUPS.map((g) => ({
      ...g,
      columns: choosable().filter((c) => c.group === g.id && !view.columns.includes(c.id)),
    })).filter((g) => g.columns.length > 0)
  );

  $effect(() => {
    const before = globalThis.document?.activeElement;
    box?.querySelector('button')?.focus();
    return () => restoreFocus(before);
  });

  /// Removes `column`, then hands focus to the row after it, or the one before it if it was
  /// the last: the button that had focus leaves with its row.
  function remove(column, i) {
    const next = shown[i + 1] ?? shown[i - 1];
    change(removeColumn(view, column.id), next?.id ?? null, 'remove');
  }

  /// Applies a change, then puts focus back on the same control of the row it moved.
  async function change(next, id = null, control = null) {
    onchange(next);
    if (id && control) {
      await tick();
      const row = [...(box?.querySelectorAll('[data-shown]') ?? [])].find((el) => el.dataset.shown === id);
      const target = row?.querySelector(`[data-control="${control}"]:not([disabled])`) ?? row?.querySelector('button:not([disabled])');
      target?.focus();
    }
  }

  function keydown(event) {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      ondone();
    }
  }

  function dragstart(event, i) {
    dragFrom = i;
    event.dataTransfer?.setData('text/plain', String(i));
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
    }
  }

  function drop(event, to) {
    event.preventDefault();
    const from = Number(event.dataTransfer?.getData('text/plain') ?? dragFrom);
    dragFrom = null;
    if (Number.isInteger(from)) {
      change(moveTo(view, from, to));
    }
  }
</script>

<div
  class="pop"
  role="dialog"
  aria-modal="false"
  aria-labelledby="column-menu-title"
  tabindex="-1"
  bind:this={box}
  onkeydown={keydown}
  data-column-menu
>
  <div class="top">
    <b id="column-menu-title">Columns</b>
    <span class="g">View: {view.name}{viewWord ? ` · ${viewWord}` : ''}</span>
  </div>
  <div class="cols">
    <div>
      <h4 class="sl">Shown, in order</h4>
      <ol class="shown">
        {#each shown as column, i (column.id)}
          <li
            data-shown={column.id}
            class:drop={dragFrom !== null && dragFrom !== i}
            draggable="true"
            ondragstart={(e) => dragstart(e, i)}
            ondragover={(e) => e.preventDefault()}
            ondrop={(e) => drop(e, i)}
            ondragend={() => (dragFrom = null)}
          >
            <span class="grip" aria-hidden="true">⋮⋮</span>
            <span class="name">{column.name}{#if view.sort?.column === column.id}<span class="cy"> sorted {view.sort.direction === 'down' ? '▼' : '▲'}</span>{/if}</span>
            <button type="button" data-control="up" aria-label="Move {column.name} up" disabled={i === 0} onclick={() => change(moveColumn(view, column.id, -1), column.id, 'up')}>↑</button>
            <button type="button" data-control="down" aria-label="Move {column.name} down" disabled={i === shown.length - 1} onclick={() => change(moveColumn(view, column.id, 1), column.id, 'down')}>↓</button>
            <button type="button" data-control="remove" aria-label="Remove {column.name}" disabled={shown.length <= 1} onclick={() => remove(column, i)}>✕</button>
          </li>
        {/each}
      </ol>
      <p class="g hint">Drag a row, or use ↑ ↓ with the keyboard. Select a header in the table to sort.</p>
    </div>
    <div>
      <h4 class="sl">Add a column</h4>
      {#each groups as group (group.id)}
        <div class="group" role="group" aria-label="Add a {group.label} column">
          <b class="w">+ {group.label}</b>
          <div class="adds">
            {#each group.columns as column (column.id)}
              <button type="button" class="add" data-add={column.id} onclick={() => change(addColumn(view, column.id), column.id, 'remove')}>{column.name}</button>
            {/each}
          </div>
        </div>
      {/each}
      <!-- STUB: columns the engine has no model for yet: shown greyed, never added. -->
      <StubSection note="later columns">
        <div class="later">
          <b>Later</b>
          <div>{LATER_COLUMNS.map((c) => c.name).join(' · ')}</div>
        </div>
      </StubSection>
    </div>
  </div>
  <div class="foot">
    <button type="button" class="btn gh" data-reset onclick={() => onreset()}>Reset to default</button>
    <button type="button" class="btn" data-done onclick={() => ondone()}>Done</button>
  </div>
</div>

<style>
  .pop {
    position: absolute;
    z-index: 9;
    top: 30px;
    right: 0;
    width: 470px;
    padding: 12px 14px;
    background: var(--well);
    border: 1px solid var(--seg-edge);
    border-radius: 3px;
    box-shadow: var(--popover-shadow);
    font-size: 10px;
    animation: open 150ms var(--ease) both;
  }

  .pop:focus {
    outline: none;
  }

  .top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    padding-bottom: 6px;
  }

  .top b {
    font: 700 11px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--cyan);
  }

  .cols {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }

  .sl {
    margin: 0 0 4px;
    font: 700 9.5px var(--fd);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--cyan);
  }

  .shown {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .shown li {
    display: flex;
    align-items: center;
    gap: 2px;
    min-height: var(--hit);
    border-bottom: 1px solid var(--rule-2);
  }

  .shown li:focus-within {
    background: var(--new-goal-ground);
    box-shadow: inset 0 0 0 1px var(--cyan);
  }

  .shown li.drop {
    box-shadow: inset 0 -2px 0 var(--cyan);
  }

  .grip {
    color: var(--ink-3);
    cursor: grab;
    padding: 0 4px;
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .shown button {
    display: inline-grid;
    place-items: center;
    width: max(24px, var(--hit));
    height: var(--hit);
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--ink-2);
    font: 600 11px var(--fb);
    cursor: pointer;
  }

  .shown button:hover:not([disabled]) {
    color: var(--ink);
    background: var(--ground-2);
  }

  .shown button:focus-visible,
  .add:focus-visible,
  .btn:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: 1px;
  }

  .shown button[disabled] {
    color: var(--ink-3);
    opacity: 0.5;
    cursor: default;
  }

  .hint {
    margin: 4px 0 0;
    font-size: 9px;
  }

  .group {
    margin-bottom: 5px;
  }

  .w {
    font-size: 9.5px;
    color: var(--ink);
  }

  .adds {
    display: flex;
    flex-wrap: wrap;
    gap: 0 2px;
  }

  .add {
    min-width: var(--hit);
    min-height: var(--hit);
    padding: 0 3px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--ink-2);
    font: 400 9px var(--fb);
    cursor: pointer;
  }

  .add:hover {
    color: var(--ink);
    background: var(--ground-2);
  }

  .later {
    margin-top: 6px;
    padding-top: 5px;
    border-top: 1px solid var(--rule-2);
    font-size: 9px;
  }

  .g {
    color: var(--ink-3);
  }

  .cy {
    color: var(--cyan);
    font-size: 9px;
  }

  .foot {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 10px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    height: var(--hit);
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-sm);
    font: 600 10px var(--fb);
    cursor: pointer;
    color: var(--ink);
    background: var(--navy-600);
  }

  .btn:hover {
    filter: brightness(1.12);
  }

  .btn.gh {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--control-edge);
    color: var(--ghost-ink);
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .pop {
      top: calc(100% + 4px);
      width: min(470px, 100%);
    }

    .cols {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  @keyframes open {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
</style>
