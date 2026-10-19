<!-- One instruction on the tactics board: its label, its control and its word. A graded
     instruction has five pips; a choice has segments, or a menu when it has many; a switch
     has Off and On. An instruction that differs from the style shows a cyan dot before the
     label and its word in cyan (the title gives the style's level); one the engine did not
     take shows a red outline and "⚠ <kept level>". Every word comes from the board's word
     table, never the tactics file's ids. `inline` lays it out in a strip instead of a row. -->
<script>
  import ChoiceSegments from './ChoiceSegments.svelte';
  import LevelPips from './LevelPips.svelte';

  let {
    label,
    full,
    words = [],
    control = 'pips',
    value = 0,
    def = null,
    changed = false,
    error = false,
    styleWord = null,
    disabled = false,
    inline = false,
    primitive = '',
    onchange = () => {},
  } = $props();

  let word = $derived(error ? `⚠ ${words[value] ?? ''}` : (words[value] ?? ''));
  let title = $derived(changed && styleWord ? `Style level: ${styleWord}` : undefined);
</script>

<div
  class="row {control}"
  class:inline
  class:ch={changed}
  class:err={error}
  data-primitive={primitive}
  {title}
>
  <span class="l">{label}</span>
  {#if control === 'pips'}
    <LevelPips {full} {words} {value} {def} {disabled} {onchange} />
    <b class="w">{word}</b>
  {:else if control === 'select'}
    <select
      class="sel"
      aria-label={full}
      {disabled}
      value={String(value)}
      onchange={(e) => onchange(Number(e.currentTarget.value))}
    >
      {#each words as w, i (i)}
        <option value={String(i)}>{w}</option>
      {/each}
    </select>
  {:else}
    <ChoiceSegments {full} {words} {value} {disabled} {onchange} />
  {/if}
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto 52px;
    gap: 5px;
    align-items: center;
    min-height: 20px;
    border-bottom: 1px solid var(--rule-2);
    font-size: 9.5px;
  }

  .row:last-child {
    border-bottom: 0;
  }

  .row.select,
  .row.segments,
  .row.switch {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .row.stack {
    grid-template-columns: minmax(0, 1fr);
    gap: 2px;
    padding: 2px 0 3px;
  }

  .row.stack :global(.seg) {
    justify-self: start;
  }

  .row.inline {
    display: inline-flex;
    gap: 6px;
    min-height: 0;
    border-bottom: 0;
    white-space: nowrap;
  }

  .row.inline .w {
    width: auto;
    min-width: 44px;
    text-align: left;
  }

  .l {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ink-2);
  }

  .w {
    font: 600 9.5px var(--fd);
    text-align: right;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ink);
  }

  .row.ch .l {
    color: var(--ink);
  }

  .row.ch .l::before {
    content: '';
    display: inline-block;
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--cyan);
    margin-right: 4px;
    vertical-align: 1px;
  }

  .row.ch .w {
    color: var(--cyan);
  }

  .row.err {
    box-shadow: inset 0 0 0 1px var(--bad);
    padding: 0 3px;
    border-radius: var(--radius-sm);
  }

  .row.err .w {
    color: var(--bad);
  }

  .sel {
    justify-self: end;
    height: 17px;
    padding: 0 4px;
    border: 1px solid var(--select-edge);
    border-radius: var(--radius-sm);
    background: var(--select-ground);
    color: var(--select-ink);
    font: 600 9px var(--fd);
  }

  .sel:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: 1px;
  }

  @media (min-width: 1600px) {
    .row {
      grid-template-columns: minmax(0, 1fr) auto 64px;
      min-height: 22px;
    }

    .l {
      font-size: 10px;
    }
  }

  @media (max-width: 1023px), (max-height: 599px) {
    .row {
      min-height: 44px;
      grid-template-columns: minmax(0, 1fr) auto 78px;
      font-size: 10.5px;
    }

    .l,
    .w {
      font-size: 10.5px;
    }

    .sel {
      height: 44px;
      font-size: 10.5px;
      padding: 0 10px;
    }
  }
</style>
