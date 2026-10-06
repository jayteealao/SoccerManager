<!-- One of match setup's two club tables: every sample team with its crest and its strength
     in stars (half stars, from the mean attribute of its first eleven), the picked row drawn
     as the selection with Picked, and the other side's pick named (Home team) so the same
     club twice is visible before it is refused. Each club's name is a button. -->
<script>
  import Crest from './Crest.svelte';
  import { teamOf } from '../lib/front-door-model.js';

  let { label, teams = [], picked = null, other = null, otherWord = '', onpick = () => {} } = $props();

  /// The strength (1 to 20) in half stars, 0 to 10.
  function halvesOf(strength) {
    return Math.round(Math.max(0, Math.min(20, strength)) / 2);
  }

  /// 1 to 20 as 0 to 5 stars in half stars, each `full`, `half` or `empty`.
  function stars(strength) {
    const halves = halvesOf(strength);
    return Array.from({ length: 5 }, (_, i) => (halves >= (i + 1) * 2 ? 'full' : halves === i * 2 + 1 ? 'half' : 'empty'));
  }

  /// The stars in words, with no decimal: "3 and a half of 5 stars".
  function starsLabel(strength) {
    const halves = halvesOf(strength);
    const whole = Math.floor(halves / 2);
    if (halves % 2 === 0) {
      return `${whole} of 5 stars`;
    }
    return whole === 0 ? 'Half a star of 5' : `${whole} and a half of 5 stars`;
  }

  const uid = $props.id();
</script>

<table class="tbl" aria-label={label}>
  <thead>
    <tr><th>Team</th><th>Strength</th><th class="r"><span class="vh">Pick</span></th></tr>
  </thead>
  <tbody>
    {#each teams as team (team.id)}
      <tr class:me={team.id === picked} aria-selected={team.id === picked ? 'true' : undefined} data-club={team.id}>
        <td>
          <button type="button" class="club" aria-pressed={team.id === picked} onclick={() => onpick(team.id)}>
            <Crest team={teamOf(team)} width={13} height={14} />
            {team.name}
          </button>
        </td>
        <td>
          <span class="stars" role="img" aria-label={starsLabel(team.strength)}>
            {#each stars(team.strength) as star, i (i)}
              <svg class="star {star}" width="9" height="9" viewBox="0 0 16 16" aria-hidden="true">
                <defs>
                  <clipPath id="half-{uid}-{team.id}-{i}"><rect width="8" height="16"></rect></clipPath>
                </defs>
                <path class="empty" d="M8 1l2.1 4.6 5 .5-3.8 3.3 1.1 4.9L8 11.8 3.6 14.3l1.1-4.9L.9 6.1l5-.5z" />
                {#if star !== 'empty'}
                  <path
                    class="fill"
                    clip-path={star === 'half' ? `url(#half-${uid}-${team.id}-${i})` : undefined}
                    d="M8 1l2.1 4.6 5 .5-3.8 3.3 1.1 4.9L8 11.8 3.6 14.3l1.1-4.9L.9 6.1l5-.5z"
                  />
                {/if}
              </svg>
            {/each}
          </span>
        </td>
        <td class="r">
          {#if team.id === picked}<b>Picked</b>{:else if team.id === other}<span class="g">{otherWord}</span>{/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  .tbl {
    width: 100%;
    border-collapse: collapse;
    font-size: 10.5px;
  }

  th {
    text-align: left;
    font: 600 9.5px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-3);
    padding: 0 6px 5px;
    border-bottom: 1px solid var(--rule);
  }

  td {
    height: var(--hit);
    padding: 0 6px;
    border-bottom: 1px solid var(--rule-2);
    white-space: nowrap;
  }

  .r {
    text-align: right;
  }

  tr.me td {
    background: var(--picked-ground);
  }

  tr.me td:first-child {
    box-shadow: inset 2px 0 0 var(--cyan);
  }

  /* The club fills its cell, so the whole cell takes the click. */
  .club {
    display: flex;
    width: 100%;
    min-height: var(--hit);
    align-items: center;
    gap: 6px;
    border: 0;
    padding: 0;
    background: none;
    color: var(--ink);
    font: inherit;
    cursor: pointer;
    border-radius: var(--radius-sm);
  }

  .club:hover {
    color: var(--cyan);
  }

  .stars {
    display: inline-flex;
    gap: 1px;
  }

  .star .empty {
    fill: var(--star-empty);
  }

  .star .fill {
    fill: var(--star);
  }

  .g {
    color: var(--ink-3);
  }

  .vh {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
