<!-- The tactic picker: the formation and the style on show, and a list of the styles the
     engine loaded. Picking a style sets every team instruction and the game plans to its own;
     the player instructions stay. A tactic that matches no style reads Custom. The open list
     is the one part of the board with a shadow. -->
<script>
  let {
    formation = '',
    options = [],
    current = null,
    changes = 0,
    disabled = false,
    onpick = () => {},
  } = $props();

  let open = $state(false);
  let root = $state(null);
  const menuId = `style-menu-${Math.random().toString(36).slice(2, 8)}`;

  let shown = $derived(options.find((o) => o.id === current)?.name ?? 'Custom');

  function pick(id) {
    open = false;
    onpick(id);
  }

  // On open, focus moves to the style on show (or the first), so the list is reached at once.
  function toggle() {
    open = !open;
    if (open) {
      queueMicrotask(() => {
        const list = root?.querySelector('.menu');
        (list?.querySelector('[aria-pressed="true"]') ?? list?.querySelector('button'))?.focus();
      });
    }
  }

  function keydown(event) {
    if (event.key === 'Escape' && open) {
      event.preventDefault();
      open = false;
      root?.querySelector('button.pick')?.focus();
    }
  }

  function focusout(event) {
    if (open && !root?.contains(event.relatedTarget)) {
      open = false;
    }
  }
</script>

<!-- Escape reaches the wrapper from the button or the menu. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<span class="wrap" bind:this={root} onkeydown={keydown} onfocusout={focusout}>
  <button
    type="button"
    class="pick"
    aria-expanded={open}
    aria-controls={menuId}
    aria-label="Tactic: {formation} · {shown}"
    {disabled}
    onclick={toggle}
  >
    <span>{formation} · <b>{shown}</b></span>
    <svg width="8" height="8" viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 6 8 10.5 12.5 6" /></svg>
  </button>
  {#if open}
    <div class="menu" id={menuId}>
      <div class="head" id="{menuId}-head">Choose a style · sets every team instruction and its game plans</div>
      <div role="group" aria-label="Styles" aria-describedby="{menuId}-head {menuId}-foot">
      {#each options as option (option.id)}
        <button
          type="button"
          class="opt"
          class:on={option.id === current}
          aria-pressed={option.id === current}
          onclick={() => pick(option.id)}
        >
          <b>{option.name}</b><span>{option.note}</span>
          {#if option.id === current && changes > 0}<em>{changes} change{changes > 1 ? 's' : ''}</em>{/if}
        </button>
      {/each}
      </div>
      <div class="foot" id="{menuId}-foot">A new style replaces your team instructions and game plans. Player instructions stay.</div>
    </div>
  {/if}
</span>

<style>
  .wrap {
    position: relative;
  }

  .pick {
    min-width: 200px;
    height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 0 8px;
    border: 1px solid var(--select-edge);
    border-radius: var(--radius-sm);
    background: var(--select-ground);
    color: var(--select-ink);
    font-size: 10.5px;
    cursor: pointer;
  }

  .pick:disabled {
    cursor: default;
    opacity: 0.6;
  }

  .pick svg path {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }

  .pick:focus-visible,
  .opt:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: 1px;
  }

  .menu {
    position: absolute;
    left: 0;
    top: 28px;
    z-index: 5;
    width: 320px;
    padding: 4px;
    background: var(--select-ground);
    border: 1px solid var(--select-edge);
    border-radius: 3px;
    box-shadow: var(--popover-shadow);
  }

  .head,
  .foot {
    font-size: 9px;
    color: var(--ink-3);
    padding: 4px 6px;
  }

  .head {
    font: 700 8.5px var(--fd);
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .foot {
    border-top: 1px solid var(--rule);
    margin-top: 3px;
  }

  .opt {
    display: grid;
    grid-template-columns: 110px 1fr auto;
    gap: 6px;
    align-items: baseline;
    width: 100%;
    margin: 0;
    padding: 4px 6px;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--select-ink);
    text-align: left;
    font-size: 9.5px;
    cursor: pointer;
  }

  .opt b {
    font: 600 10px var(--fd);
  }

  .opt span {
    color: var(--ink-3);
  }

  .opt em {
    font: 600 9px var(--fd);
    font-style: normal;
    color: var(--cyan);
  }

  .opt:hover {
    background: var(--ground-2);
  }

  .opt.on {
    background: var(--cyan);
    color: var(--cyan-ink);
  }

  .opt.on span,
  .opt.on em {
    color: var(--cyan-ink);
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .pick {
      height: 44px;
      min-width: 260px;
    }

    .opt {
      min-height: 44px;
    }
  }
</style>
