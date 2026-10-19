<!-- A short choice list side by side (Full-back runs, Cross type) or Off and On for a switch:
     one group of buttons, the chosen one cyan. Segments change at once, with no animation. -->
<script>
  let { full, words = [], value = 0, disabled = false, onchange = () => {} } = $props();
</script>

<span class="seg" role="group" aria-label={full}>
  {#each words as word, i (i)}
    <button
      type="button"
      class:on={i === value}
      aria-pressed={i === value}
      data-level={i}
      {disabled}
      onclick={() => !disabled && i !== value && onchange(i)}
    >
      {word}
    </button>
  {/each}
</span>

<style>
  .seg {
    display: inline-flex;
    border: 1px solid var(--seg-edge);
    border-radius: var(--radius-sm);
    overflow: hidden;
    justify-self: end;
  }

  button {
    margin: 0;
    border: 0;
    background: none;
    cursor: pointer;
    font: 600 9px/15px var(--fd);
    padding: 0 5px;
    color: var(--ink-2);
    white-space: nowrap;
  }

  button + button {
    border-left: 1px solid var(--seg-edge);
  }

  button.on {
    background: var(--cyan);
    color: var(--cyan-ink);
  }

  button:hover:not(.on):not(:disabled) {
    color: var(--seg-ink);
  }

  button:disabled {
    cursor: default;
  }

  button:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: -2px;
  }

  @media (max-width: 1023px), (max-height: 599px) {
    button {
      min-height: 44px;
      min-width: 44px;
      padding: 0 10px;
      font-size: 10.5px;
      line-height: 44px;
    }
  }
</style>
