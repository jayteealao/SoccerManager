<!-- The banner for a change the engine did not take: it names the change, the level the
     board keeps, and the engine's reason as a code for a report, with Try again and Dismiss.
     It fades in over 200 ms, and not at all under reduced motion. -->
<script>
  import { reasonSentence } from '../lib/board-words.js';

  let { refused, onretry = () => {}, ondismiss = () => {} } = $props();
</script>

<div class="err" role="alert" data-refused={refused.queue_id}>
  <b class="cap">Not applied</b>
  <span class="text">
    <b>{refused.label}</b> did not go through{#if refused.kept}, so it stays at <b>{refused.kept}</b>{/if}.
    {reasonSentence(refused.code)} Try again; if it keeps failing, report it with this code:
    <span class="code">{refused.code}</span>
  </span>
  <button type="button" class="btn" onclick={() => onretry(refused.queue_id)}>Try again</button>
  <button type="button" class="x" aria-label="Dismiss" onclick={() => ondismiss(refused.queue_id)}>
    <svg width="9" height="9" viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9" /></svg>
  </button>
</div>

<style>
  .err {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 6px 0 2px;
    padding: 6px 10px;
    background: var(--ground-2);
    border: 1px solid var(--bad);
    border-radius: 3px;
    font-size: 10px;
    color: var(--ink);
  }

  .cap {
    font: 700 10px var(--fd);
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--bad);
    white-space: nowrap;
  }

  .text {
    flex: 1;
    min-width: 0;
  }

  .code {
    font: 500 9px var(--fm);
    color: var(--ink-2);
  }

  .btn {
    height: 20px;
    padding: 0 10px;
    margin: 0;
    border: 1px solid var(--control-edge);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--ghost-ink);
    font: 600 9.5px var(--fd);
    cursor: pointer;
    white-space: nowrap;
  }

  .x {
    display: inline-grid;
    place-items: center;
    width: 18px;
    height: 18px;
    margin: 0;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--ink-2);
    cursor: pointer;
  }

  .x path {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }

  .btn:focus-visible,
  .x:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: 1px;
  }

  @media (prefers-reduced-motion: no-preference) {
    .err {
      animation: err-in 200ms var(--ease);
    }
  }

  @keyframes err-in {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }

    to {
      opacity: 1;
      transform: none;
    }
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .btn,
    .x {
      min-height: 44px;
      min-width: 44px;
    }

    .err {
      flex-wrap: wrap;
    }
  }
</style>
