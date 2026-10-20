<!-- The attribute hover of the player panel: anchored beside an attribute row, it lists the
     actions the attribute drives, as main or as one of the three shown supports, never a minor
     weight. It is a tooltip: the row opens it on pointer hover and on keyboard focus and names
     it with `aria-describedby`; it holds no control and takes no focus. The pointer can move
     onto it and it stays open; Escape, a pointer leaving the row and the hover, or a blur
     closes it. It sits on the row's right, or on its left where the column sets
     `--hover-left: auto` and `--hover-right`. It opens from the row's side in 150 ms; the
     page's reduced-motion rule shows it at once. -->
<script>
  import { actionsOf } from '../lib/attribute-actions.js';

  let { id, name, label } = $props();

  let rows = $derived(actionsOf(name));
</script>

<div class="hover" {id} role="tooltip" data-hover={name}>
  <b class="title">{label} drives</b>
  <ul>
    {#each rows as row (row.action)}
      <li data-action={row.action} data-role={row.role}><span>{row.words}</span><em>{row.role}</em></li>
    {/each}
  </ul>
  <p class="foot">Each action reads one main attribute and up to three supports.</p>
</div>

<style>
  .hover {
    position: absolute;
    z-index: 9;
    top: -4px;
    left: var(--hover-left, calc(100% + 8px));
    right: var(--hover-right, auto);
    width: 220px;
    padding: 10px 12px;
    font: 400 10.5px var(--fb);
    font-variant-numeric: normal;
    text-align: left;
    background: var(--well);
    border: 1px solid var(--seg-edge);
    border-radius: var(--radius-md);
    box-shadow: var(--popover-shadow);
    transform-origin: left top;
    animation: open 150ms var(--ease) both;
  }

  /* A bridge over the gap on both sides, so the pointer can move from the row onto the hover
     without leaving the row's box. */
  .hover::before {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: -10px;
    right: -10px;
    z-index: -1;
  }

  .title {
    display: block;
    margin-bottom: 4px;
    font: 700 10px var(--fd);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--cyan);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    min-height: 24px;
    font-size: 10.5px;
    color: var(--ink-2);
    border-bottom: 1px solid var(--rule-2);
  }

  li em {
    font-style: normal;
    color: var(--ink-3);
  }

  li[data-role='main'] em {
    color: var(--ink);
    font-weight: 600;
  }

  .foot {
    margin: 6px 0 0;
    font-size: 9.5px;
    line-height: 1.4;
    color: var(--ink-3);
  }

  @keyframes open {
    from {
      opacity: 0;
      transform: scale(0.97);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
