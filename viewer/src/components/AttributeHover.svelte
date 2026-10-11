<!-- The attribute hover of the player panel: anchored beside an attribute row, it lists the
     actions the attribute drives, as main or as one of the three shown supports, never a minor
     weight. It is a tooltip: the row opens it on pointer hover and on keyboard focus and names
     it with `aria-describedby`; it holds no control and takes no focus, and Escape, a pointer
     leaving or a blur closes it. It opens from the row's side in 150 ms; the page's reduced-
     motion rule shows it at once. -->
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
    left: calc(100% + 8px);
    width: 220px;
    padding: 10px 12px;
    background: var(--well);
    border: 1px solid var(--seg-edge);
    border-radius: var(--radius-md);
    box-shadow: var(--popover-shadow);
    transform-origin: left top;
    pointer-events: none;
    animation: open 150ms var(--ease) both;
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
    font-size: 10px;
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
