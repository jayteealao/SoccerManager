<!-- The other team's bench on the Touchline: who is left to come on and how many substitutes
     and windows the other manager has used, at the rendered tick. Who is warming up needs a
     model the engine does not have: a LATER stub. -->
<script>
  import SectionLabel from './SectionLabel.svelte';
  import StubSection from './StubSection.svelte';

  let { name = '', bench } = $props();
</script>

<section aria-label="{name} bench">
  <SectionLabel label="{name} bench" note="what their staff can see" />
  <!-- STUB: who is warming up needs a model the engine does not have. -->
  <StubSection note="warming up" later>
    <div class="kv"><span>Warming up</span><b>—</b></div>
  </StubSection>
  <div class="kv"><span>Their substitutes used</span><b class="num">{bench.text}</b></div>
  <ul>
    {#each bench.players as p (p.id)}
      <li data-may-truncate><span class="num">{p.shirt}</span> {p.name} <em>{p.position}</em></li>
    {/each}
  </ul>
</section>

<style>
  .kv {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    padding: 3px 0;
    font-size: 10.5px;
    color: var(--ink-2);
  }

  .kv b {
    color: var(--ink);
    font-weight: 600;
  }

  ul {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1px 10px;
    font-size: 10.5px;
    color: var(--ink);
  }

  li {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .num {
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
  }

  .kv b.num {
    color: var(--ink);
  }

  em {
    font-style: normal;
    font-size: 9px;
    color: var(--ink-3);
  }
</style>
