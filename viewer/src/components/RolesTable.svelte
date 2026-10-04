<!-- The grasp table of the sketch's in-match Tactics board: each player on the pitch with his
     role and duty as live selects (an edit is queued as a change), and his grasp of the
     tactic, a LATER stub because the engine has no familiarity model yet. -->
<script>
  import SectionLabel from './SectionLabel.svelte';
  import StubSection from './StubSection.svelte';
  import { suitableRoles, words } from '../lib/tactics-panel.js';

  let { schema, tactics, rows = [], disabled = false, onedit = () => {} } = $props();

  let formation = $derived(schema?.formations?.[tactics?.formation ?? 0] ?? null);

  function positionOf(slot) {
    return formation?.slots?.[slot]?.position ?? '';
  }

  function change(row, slot, field, value) {
    const now = tactics.roles[slot];
    const next = { squad: row.squad, role: now.role, duty: now.duty, [field]: value };
    onedit({ role: next }, slot);
  }
</script>

<SectionLabel label="Roles and duties" note="queued for the next stoppage" />
<table class="tbl">
  <thead>
    <tr>
      <th class="r">#</th>
      <th>Player</th>
      <th>Role · duty</th>
      <th class="grasp" colspan="2">
        <StubSection note="grasp column head" inline later>Grasp</StubSection>
      </th>
    </tr>
  </thead>
  <tbody>
    {#each rows as row, slot (row.wire)}
      {@const current = tactics?.roles?.[slot]}
      <tr class:off={row.sentOff}>
        <td class="r num">{row.shirt}</td>
        <td class="name" data-may-truncate>{row.name}</td>
        <td>
          {#if current}<div class="pick">
            <select
              class="sel"
              aria-label="{row.name}: role"
              value={String(current.role)}
              disabled={disabled || row.sentOff || row.squad === null}
              onchange={(e) => change(row, slot, 'role', Number(e.currentTarget.value))}
            >
              {#each suitableRoles(schema, positionOf(slot)) as r (r)}
                <option value={String(r)}>{words(schema.roles[r].name)}</option>
              {/each}
            </select>
            <select
              class="sel duty"
              aria-label="{row.name}: duty"
              value={String(current.duty)}
              disabled={disabled || row.sentOff || row.squad === null}
              onchange={(e) => change(row, slot, 'duty', Number(e.currentTarget.value))}
            >
              {#each schema.duties as d, i (i)}
                <option value={String(i)}>{words(d.name)}</option>
              {/each}
            </select></div>
          {/if}
        </td>
        <td class="grasp" colspan="2">
          <!-- STUB: the player's grasp of the tactic needs a familiarity model. -->
          <StubSection note="grasp of the tactic" inline>
            <i class="track"></i><b class="num">—</b>
          </StubSection>
        </td>
      </tr>
    {/each}
  </tbody>
</table>
<!-- STUB: the note on team familiarity reads a model the engine does not have yet. -->
<StubSection note="familiarity note" later>
  <p class="note">Team familiarity is the average grasp of the eleven on the pitch.</p>
</StubSection>

<style>
  .tbl {
    width: 100%;
    border-collapse: collapse;
    font-size: 10px;
  }

  th {
    font: 600 9.5px var(--fd);
    color: var(--ink-2);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    text-align: left;
    padding: 6px 3px;
    white-space: nowrap;
  }

  td {
    padding: 0 3px;
    height: var(--hit);
    white-space: nowrap;
    color: var(--ink);
  }

  tbody tr:nth-child(odd) td {
    background: var(--ground-2);
  }

  tbody tr:nth-child(even) td {
    background: var(--ground-3);
  }

  .r {
    text-align: right;
  }

  .name {
    max-width: 96px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .pick {
    display: flex;
    gap: 4px;
    align-items: center;
    height: var(--hit);
  }

  .sel {
    height: var(--hit);
    max-width: 118px;
    padding: 0 2px 0 5px;
    background: var(--select-ground);
    border: 1px solid var(--select-edge);
    border-radius: var(--radius-sm);
    color: var(--select-ink);
    font: 500 9.5px var(--fb);
  }

  .sel.duty {
    max-width: 70px;
  }

  .grasp {
    width: 90px;
  }

  .track {
    display: inline-block;
    width: 60px;
    height: 5px;
    background: var(--bar-track);
    border-radius: 1px;
    vertical-align: middle;
    margin-right: 6px;
  }

  .off td {
    color: var(--ink-3);
  }

  .note {
    margin: 5px 0 0;
    font-size: 9.5px;
    color: var(--ink-3);
    line-height: 1.4;
  }

  /* In a narrow column the grasp stub gives way, so the role and duty keep their room. */
  @container (max-width: 380px) {
    .grasp {
      display: none;
    }

    .sel {
      max-width: 92px;
    }
  }
</style>
