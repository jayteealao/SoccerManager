<!-- The rule-pack checks on the Pre-match line-ups: each rule the engine plays by, marked with
     a tick and said in words, then the rules it has no model for yet as LATER rows, and what
     a level score after ninety minutes means. -->
<script>
  import SectionLabel from './SectionLabel.svelte';
  import StubSection from './StubSection.svelte';

  let { rules } = $props();
</script>

<section aria-label="Rule pack">
  <SectionLabel label="Rule pack" note="the rules this match plays by" />
  <ul>
    {#each rules.live as row (row.text)}
      <li><span class="mk {row.tone}" aria-hidden="true">{row.mark}</span>{row.text}</li>
    {/each}
  </ul>
  <!-- STUB: rules the engine has no model for yet. -->
  <StubSection note="later rules" later>
    <ul class="later">
      {#each rules.later as text (text)}
        <li><span class="mk">–</span>{text}</li>
      {/each}
    </ul>
  </StubSection>
  <div class="level">
    <span>Level after 90</span>
    <b>{rules.level}</b>
  </div>
</section>

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 3px;
    font-size: 10.5px;
    color: var(--ink-2);
  }

  li {
    display: flex;
    gap: 8px;
  }

  ul.later {
    margin-top: 3px;
  }

  .mk {
    width: 10px;
    flex: none;
    color: var(--ink-3);
    font-weight: 700;
  }

  .mk.good {
    color: var(--good);
  }

  .level {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    margin-top: 8px;
    font-size: 10.5px;
    color: var(--ink-2);
  }

  .level b {
    color: var(--ink);
    font-weight: 600;
  }
</style>
