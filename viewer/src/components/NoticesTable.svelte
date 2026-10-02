<!-- The open-source notices: one row per shipped package (the fonts, the viewer's npm
     packages, the engine's crates) with its version, its licence and what it is used for (the
     package's own description). The selected row is drawn as the selection; choosing a row
     shows its licence text beside the table. The list comes from the notices file the build
     wrote, never from text in the source. -->
<script>
  let { packages = [], selected = 0, onselect = () => {} } = $props();
</script>

<div class="wrap">
  <table class="tbl" aria-label="Open-source notices">
    <thead>
      <tr><th>Package</th><th>Version</th><th>Licence</th><th>Used for</th></tr>
    </thead>
    <tbody>
      {#each packages as pkg, i (`${pkg.kind}:${pkg.name}:${pkg.version}`)}
        <tr class:me={i === selected} aria-selected={i === selected ? 'true' : undefined} data-package={pkg.name}>
          <td>
            <button type="button" class="pick" aria-pressed={i === selected} onclick={() => onselect(i)}>{pkg.name}</button>
          </td>
          <td class="num">{pkg.version ?? '—'}</td>
          <td>{pkg.licence}</td>
          <td class="used">{pkg.description ?? ''}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .wrap {
    max-height: 560px;
    overflow-y: auto;
  }

  .tbl {
    width: 100%;
    border-collapse: collapse;
    font-size: 10.5px;
    table-layout: fixed;
  }

  th {
    position: sticky;
    top: 0;
    background: var(--ground);
    text-align: left;
    font: 600 9.5px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-3);
    padding: 0 6px 5px;
    border-bottom: 1px solid var(--rule);
  }

  th:nth-child(1) {
    width: 30%;
  }

  th:nth-child(2) {
    width: 14%;
  }

  th:nth-child(3) {
    width: 22%;
  }

  td {
    height: 24px;
    padding: 0 6px;
    border-bottom: 1px solid var(--rule-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  tr.me td {
    background: var(--picked-ground);
  }

  tr.me td:first-child {
    box-shadow: inset 2px 0 0 var(--cyan);
  }

  .pick {
    border: 0;
    padding: 0;
    background: none;
    color: var(--ink);
    font: inherit;
    cursor: pointer;
    border-radius: var(--radius-sm);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .pick:hover {
    color: var(--cyan);
  }

  .used {
    color: var(--ink-2);
  }
</style>
