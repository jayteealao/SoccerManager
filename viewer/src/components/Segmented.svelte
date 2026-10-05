<!-- A segmented control: one group of toggle buttons, one pressed, joined by 1 px rules.
     `options` are `{ value, label }`; `onchange(value)` is called when another is pressed. -->
<script>
  let { options = [], value, label, onchange = () => {} } = $props();
</script>

<span class="seg" role="group" aria-label={label}>
  {#each options as option (option.value)}
    <button
      type="button"
      aria-pressed={option.value === value}
      data-value={option.value}
      onclick={() => option.value !== value && onchange(option.value)}
    >
      {option.label}
    </button>
  {/each}
</span>

<style>
  .seg {
    display: inline-flex;
    flex: none;
    border: 1px solid var(--seg-edge);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  button {
    border: 0;
    height: var(--hit);
    min-width: var(--hit);
    padding: 0 12px;
    font: 600 10.5px var(--fd);
    letter-spacing: 0.03em;
    cursor: pointer;
    background: var(--toggle-ground);
    color: var(--toggle-ink);
    transition:
      background-color var(--tab-change) var(--ease),
      color var(--tab-change) var(--ease);
  }

  button + button {
    border-left: 1px solid var(--seg-edge);
  }

  button[aria-pressed='true'] {
    background: var(--cyan);
    color: var(--cyan-ink);
    cursor: default;
  }

  button:hover:not([aria-pressed='true']) {
    color: var(--seg-ink);
  }

  /* The group clips its children, so the ring is drawn inside the button. */
  button:focus-visible {
    outline-offset: -3px;
  }
</style>
