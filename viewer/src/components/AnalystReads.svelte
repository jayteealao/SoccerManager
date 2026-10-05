<!-- STUB: the analysts' in-match reads of the other team, ported from the sketch for the look.
     The engine has no analysts, so the block is drawn faded with the LATER mark, reads no
     engine data and takes no focus. -->
<script>
  import SectionLabel from './SectionLabel.svelte';
  import StubSection from './StubSection.svelte';

  let { name = '' } = $props();

  const READS = [
    'How they defend without the ball, and who tracks whom.',
    'Which of their players is tiring, and where it shows.',
    'Where they go from their keeper’s kicks.',
  ];
</script>

<section class="reads">
  <SectionLabel label="Analysts’ read of {name}" later />
  <StubSection note="analyst reads">
    <p class="conf"><span class="tag">Reading</span> Confidence</p>
    <ul>
      {#each READS as text (text)}
        <li>{text}</li>
      {/each}
    </ul>
  </StubSection>
</section>

<style>
  .conf {
    margin: 0 0 4px;
    font-size: 10px;
    color: var(--ink-2);
  }

  .tag {
    display: inline-block;
    padding: 0 4px;
    margin-right: 4px;
    border: 1px solid var(--reading-edge);
    border-radius: var(--radius-sm);
    font: 700 8px var(--fd);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--reading-ink);
  }

  ul {
    margin: 0;
    padding-left: 14px;
    font-size: 10.5px;
    line-height: 1.5;
    color: var(--ink);
  }
</style>
