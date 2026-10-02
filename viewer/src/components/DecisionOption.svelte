<!-- One option of a decision, as the Head knock board draws it: a row on the raised ground
     with a 12 px ring, a title and a sub-line; the chosen row turns cyan with a filled ring.
     It is a radio (role radio, aria-checked) inside the decision's radiogroup: the chosen
     row takes the Tab stop and the arrow keys move the choice, as a radio group does. The
     row is at least 24 px tall; focus draws the 2 px cyan ring 2 px outside it. -->
<script>
  let {
    title,
    sub = '',
    checked = false,
    disabled = false,
    onselect = () => {},
    onmove = () => {},
  } = $props();

  function key(event) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowRight') {
      event.preventDefault();
      onmove(1);
    } else if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') {
      event.preventDefault();
      onmove(-1);
    }
  }
</script>

<button
  class="opt"
  class:on={checked}
  type="button"
  role="radio"
  aria-checked={checked}
  tabindex={checked ? 0 : -1}
  {disabled}
  onclick={onselect}
  onkeydown={key}
>
  <span class="rad" aria-hidden="true"></span>
  <span class="txt">
    <b>{title}</b>
    {#if sub}<span class="sub">{sub}</span>{/if}
  </span>
</button>

<style>
  .opt {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    width: 100%;
    min-height: 24px;
    padding: 7px 10px;
    margin: 0 0 4px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--ground-2);
    color: var(--ink);
    font: 400 10.5px var(--fb);
    text-align: left;
    cursor: pointer;
    transition:
      background-color var(--tab-change) var(--ease),
      filter var(--tab-change) var(--ease);
  }

  .opt.on {
    background: var(--cyan);
    color: var(--cyan-ink);
    font-weight: 600;
  }

  .opt:hover:not(:disabled) {
    filter: brightness(1.12);
  }

  .opt:active:not(:disabled) {
    filter: brightness(0.92);
  }

  .opt:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: 2px;
  }

  .opt:disabled {
    cursor: default;
    opacity: var(--stub-opacity);
  }

  .rad {
    flex: none;
    width: 12px;
    height: 12px;
    margin-top: 2px;
    border-radius: 50%;
    border: 1.5px solid var(--ink-3);
    box-sizing: border-box;
  }

  .opt.on .rad {
    border-color: var(--cyan-ink);
    background: radial-gradient(circle, var(--cyan-ink) 0 3px, transparent 3.5px);
  }

  .txt {
    flex: 1;
    min-width: 0;
  }

  b {
    display: block;
    font-weight: 700;
  }

  .sub {
    display: block;
    font-size: 9.5px;
    font-weight: 400;
    color: var(--ink-2);
  }

  .opt.on .sub {
    color: var(--cyan-ink);
    font-weight: 500;
  }

  /* The reduced-motion setting, or the system's preference when it follows the system. */
  :global(:root[data-motion='reduce']) .opt {
      transition: none;
  }
</style>
