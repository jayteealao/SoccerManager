<!-- The handshake page's log, newest last: one line per check, message and sampled tick
     frame, in the monospace face on the rail ground. A line that passed is green and one that
     failed red, and each also says so in its words. Every line is a text node: the socket's
     words reach the page from the wire and are never parsed as markup. The log scrolls, so it
     takes focus and can be scrolled from the keyboard. -->
<script>
  let { lines = [], label = 'Log' } = $props();

  let box = $state();

  // Newest last: keep the newest line in view as lines arrive.
  $effect(() => {
    lines.length;
    if (box) {
      box.scrollTop = box.scrollHeight;
    }
  });
</script>

<!-- A scrolling region must be reachable from the keyboard (WCAG 2.1.1), so the log takes
     focus although it is not a control. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="log" role="log" aria-label={label} tabindex="0" bind:this={box}>
  {#each lines as line, i (i)}<span class={line.kind}>{line.text}{'\n'}</span>{/each}
</div>

<style>
  .log {
    background: var(--rail);
    box-shadow: inset 0 0 0 1px var(--rule);
    border-radius: 3px;
    padding: 10px 12px;
    font: 500 11px/1.7 var(--fm);
    color: var(--ink-2);
    white-space: pre-wrap;
    word-break: break-word;
    font-variant-numeric: tabular-nums;
    max-height: 560px;
    overflow-y: auto;
  }

  .log:focus-visible {
    outline: 2px solid var(--cyan);
    outline-offset: 2px;
  }

  .ok {
    color: var(--good);
  }

  .bad {
    color: var(--bad);
  }
</style>
