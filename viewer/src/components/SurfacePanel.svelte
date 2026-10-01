<!-- A whole-surface message: error, setup, refusal, or abandoned. A word names its kind,
     so colour is never the only signal; a title, the text, and the actions follow. An
     error or a refusal is announced as an alert. -->
<script>
  const WORDS = { error: 'Error', setup: 'Setup', refusal: 'Cannot resume', abandoned: 'Abandoned' };

  let { kind = 'error', word, title, children, actions } = $props();

  let urgent = $derived(kind === 'error' || kind === 'refusal');
</script>

<div class="surface {kind}" role={urgent ? 'alert' : 'status'}>
  <div class="surf">
    <span class="wd">{word ?? WORDS[kind]}</span>
    <h2>{title}</h2>
    {@render children?.()}
    {#if actions}
      <div class="actions">{@render actions()}</div>
    {/if}
  </div>
</div>

<style>
  .surface {
    background: var(--rail);
    box-shadow: inset 0 0 0 1px var(--rule);
    border-radius: var(--radius-md);
  }

  .surf {
    display: flex;
    flex-direction: column;
    gap: 9px;
    padding: 22px 26px;
  }

  .wd {
    font: 800 11px/1 var(--fd);
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .error .wd {
    color: var(--bad);
  }

  /* A save that cannot resume is a warning, not a fault: the board draws its word in the
     warning colour, with the word itself saying it. */
  .refusal .wd {
    color: var(--warn);
  }

  .setup .wd {
    color: var(--cyan);
  }

  .abandoned .wd {
    color: var(--ink-3);
  }

  h2 {
    margin: 0;
    font: 800 22px/1.15 var(--fd);
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .surf :global(p) {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--ink-2);
  }

  .actions {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }
</style>
