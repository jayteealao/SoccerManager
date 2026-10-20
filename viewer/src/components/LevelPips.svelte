<!-- Five pips for a graded instruction: the pips fill up to the level in cyan, and a short
     tick under one pip marks the default. The group is one stop for Tab: the arrow keys move
     the level, Home and End go to either end, and each pip is a button named
     "<instruction>: <word>, n of 5". Pips change at once, with no animation. -->
<script>
  let {
    full,
    words = [],
    value = 0,
    def = null,
    disabled = false,
    describedby = undefined,
    onchange = () => {},
  } = $props();

  let group = $state(null);

  function set(level) {
    if (!disabled && level !== value && level >= 0 && level < words.length) {
      onchange(level);
    }
  }

  function keydown(event) {
    const step = { ArrowRight: 1, ArrowUp: 1, ArrowLeft: -1, ArrowDown: -1 }[event.key];
    let next = null;
    if (step !== undefined) {
      next = Math.min(words.length - 1, Math.max(0, value + step));
    } else if (event.key === 'Home') {
      next = 0;
    } else if (event.key === 'End') {
      next = words.length - 1;
    }
    if (next === null) {
      return;
    }
    event.preventDefault();
    set(next);
    // The focus follows the level, so the next arrow press starts from it.
    queueMicrotask(() => group?.querySelector(`button[data-level="${next}"]`)?.focus());
  }
</script>

<!-- The arrow keys reach the group from the focused pip. -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<span class="pips" role="group" aria-label={full} aria-describedby={describedby} bind:this={group} onkeydown={keydown}>
  {#each words as word, i (i)}
    <button
      type="button"
      class="pip"
      class:on={i <= value}
      class:def={i === def}
      data-level={i}
      aria-label="{full}: {word}, {i + 1} of {words.length}"
      aria-pressed={i === value}
      aria-describedby={i === value ? describedby : undefined}
      tabindex={i === value ? 0 : -1}
      {disabled}
      onclick={() => set(i)}
    ></button>
  {/each}
</span>

<style>
  .pips {
    display: inline-flex;
  }

  .pip {
    position: relative;
    width: 11px;
    height: 20px;
    padding: 0;
    margin: 0;
    border: 0;
    background: none;
    cursor: pointer;
  }

  .pip::before {
    content: '';
    position: absolute;
    left: 1.5px;
    top: 6px;
    width: 8px;
    height: 8px;
    border-radius: 1px;
    background: var(--bar-track);
  }

  .pip.on::before {
    background: var(--cyan);
  }

  .pip.def::after {
    content: '';
    position: absolute;
    left: 3.5px;
    top: 16px;
    width: 4px;
    height: 2px;
    background: var(--ink-3);
  }

  .pip:hover:not(:disabled)::before {
    box-shadow: 0 0 0 1px var(--ink-2);
  }

  .pip:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: -1px;
    border-radius: var(--radius-sm);
  }

  /* A stored match: the pips show the level but are not a control, so they read as faded. */
  .pip:disabled {
    cursor: default;
    opacity: 0.5;
  }

  @media (min-width: 1600px) {
    .pip {
      height: 22px;
    }

    .pip::before {
      top: 7px;
    }

    .pip.def::after {
      top: 18px;
    }
  }

  /* Compact, and a touch screen at any step: every pip is a 44 px target. */
  @media (max-width: 1023px), (max-height: 599px), (pointer: coarse) {
    .pip {
      width: 44px;
      height: 44px;
    }

    .pip::before {
      left: 12px;
      top: 15px;
      width: 20px;
      height: 12px;
    }

    .pip.def::after {
      left: 18px;
      top: 32px;
      width: 8px;
    }
  }
</style>
