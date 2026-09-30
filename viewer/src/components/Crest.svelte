<!-- A club crest: the sketch's shield in the club's kit colour, made safe, with its capitals
     in whichever of paper and ink reads on it. Kit colours appear only here, on the pitch and
     in the score. Without a team the shield takes the sample club colour and no capitals. -->
<script>
  import { SKIN_TOKENS, TOKENS, safeKit } from '../lib/colour.js';
  import { initials } from '../lib/scoreboard.js';

  let { team = null, width = 30, height = 32 } = $props();

  let kit = $derived.by(() => {
    if (!team) {
      return null;
    }
    const theme = globalThis.document?.documentElement.dataset.theme;
    return safeKit(
      { primary: team['team.kit.primary'], secondary: team['team.kit.secondary'] },
      SKIN_TOKENS[theme] ?? TOKENS
    );
  });
</script>

<svg class="crest" {width} {height} viewBox="0 0 24 26" aria-hidden="true" focusable="false">
  <path class="shield" style:fill={kit?.fill} d="M12 1 22 4v8c0 6.5-4.5 10.5-10 13C6.5 22.5 2 18.5 2 12V4z" />
  <path class="stripe" style:fill={kit?.number} d="M2.6 8.4h18.8v2.6H2.6z" />
  {#if team}
    <text x="12" y="19.4" text-anchor="middle" style:fill={kit?.number}>{initials(team['team.name'])}</text>
  {/if}
</svg>

<style>
  .crest {
    flex: none;
  }

  .shield {
    fill: var(--club-sample);
    stroke: var(--band-ink);
    stroke-width: 1.3;
  }

  .stripe,
  text {
    fill: var(--band-ink);
  }

  text {
    font: 800 6.4px var(--fd);
  }
</style>
