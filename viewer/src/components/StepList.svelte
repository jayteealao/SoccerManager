<!-- What a wait is doing, step by step: ✓ done, ● in progress, ○ waiting, each with a
     word and a thin bar. Used in place of a spinner. -->
<script>
  const MARKS = { done: '✓', current: '●', pending: '○' };
  const WORDS = { done: 'Done', current: 'In progress', pending: 'Waiting' };

  let { steps = [] } = $props();

  // The step in progress, or the end, in words a screen reader announces as it changes.
  const summary = $derived.by(() => {
    if (steps.length === 0) {
      return '';
    }
    const at = steps.findIndex((step) => step.state !== 'done');
    if (at < 0) {
      return `All ${steps.length} steps done`;
    }
    const step = steps[at];
    return `Step ${at + 1} of ${steps.length}: ${step.label}, ${step.word ?? WORDS[step.state]}`;
  });
</script>

<p class="vh" role="status">{summary}</p>

<ol class="steps">
  {#each steps as step, i (i)}
    <li class={step.state}>
      <b><span class="mark" aria-hidden="true">{MARKS[step.state]}</span> {i + 1}. {step.label}</b>
      <span class="word">{step.word ?? WORDS[step.state]}</span>
      <div class="bar" role="presentation">
        <i
          style:width="{step.state === 'done' ? 100 : step.state === 'current' ? (step.progress ?? 50) : 0}%"
        ></i>
      </div>
    </li>
  {/each}
</ol>

<style>
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    margin-bottom: 8px;
  }

  b {
    font: 700 10px var(--fd);
    letter-spacing: 0.03em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .word {
    font-size: 9px;
    color: var(--ink-3);
    line-height: 1.3;
  }

  .done .mark {
    color: var(--good);
  }

  .current .mark {
    color: var(--cyan);
  }

  .pending .mark {
    color: var(--ink-3);
  }

  .bar {
    height: 5px;
    background: var(--bar-track);
    border-radius: 1px;
    overflow: hidden;
  }

  .bar i {
    display: block;
    height: 100%;
    background: var(--cyan);
    transition: width var(--tab-change) var(--ease);
  }

  .done .bar i {
    background: var(--good);
  }

  /* Read by a screen reader, never drawn. */
  .vh {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: 0;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
</style>
