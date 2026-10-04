<!-- The closed page, ported from the approved Closed board: the mark and TOUCHLINE HAS CLOSED
     in a 520 px column, where the match was saved (when one was playing), You can close this
     tab, the three steps the quit took, and how to play again. -->
<script>
  import StepList from '../components/StepList.svelte';
  import TouchlineMark from '../components/TouchlineMark.svelte';
  import { closedModel } from '../lib/front-door-model.js';

  let { door } = $props();

  let model = $derived(closedModel(door.closed, door.status));
</script>

<div class="closed" data-screen="closed">
  <div class="column">
    <div class="title">
      <TouchlineMark size={48} />
      <h1>Touchline has closed</h1>
    </div>
    <p class="line">{model.line}</p>
    <p class="close">You can close this tab.</p>
    <StepList steps={model.steps} />
    <p class="again">To play again, start Touchline from the Start menu or the desktop shortcut.</p>
  </div>
</div>

<style>
  .closed {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    background: var(--page);
    color: var(--ink);
  }

  .column {
    width: 520px;
  }

  .title {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  h1 {
    margin: 0;
    font: 800 26px/1.1 var(--fd);
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .line {
    margin: 18px 0 6px;
    font-size: 13px;
    line-height: 1.55;
    color: var(--ink-2);
  }

  .close {
    margin: 0 0 18px;
    font: 600 13px var(--fb);
  }

  .again {
    margin: 14px 0 0;
    font-size: 11px;
    color: var(--ink-3);
  }
</style>
