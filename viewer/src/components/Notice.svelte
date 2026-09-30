<!-- A one-line notice: a tag word for its kind, then the message. The kind is always named
     in the tag, so colour is never the only signal. A notice that needs attention now
     (`bad`) is announced as an alert; the others as a status. -->
<script>
  const WORDS = {
    good: 'CONNECTED',
    mid: 'LAG',
    warn: 'RECONNECTING',
    bad: 'STREAM ENDED',
    neutral: 'FULL TIME',
  };

  let { kind = 'neutral', word, message } = $props();

  let tag = $derived(word ?? WORDS[kind] ?? WORDS.neutral);
</script>

<div class="notice {kind}" role={kind === 'bad' ? 'alert' : 'status'}>
  <span class="tag">{tag}</span>
  <span>{message}</span>
</div>

<style>
  .notice {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 30px;
    padding: 0 10px;
    background: var(--ground-2);
    box-shadow: inset 0 0 0 1px var(--edge);
    border-radius: var(--radius-sm);
    font-size: 10.5px;
    margin-bottom: 6px;
  }

  .tag {
    display: inline-block;
    background: var(--edge);
    color: var(--on-edge);
    font: 700 9.5px var(--fd);
    letter-spacing: 0.06em;
    padding: 0 6px;
    border-radius: var(--radius-sm);
  }

  .good {
    --edge: var(--good);
    --on-edge: var(--on-good);
  }

  .mid {
    --edge: var(--mid);
    --on-edge: var(--on-mid);
  }

  .warn {
    --edge: var(--warn);
    --on-edge: var(--on-warn);
  }

  .bad {
    --edge: var(--bad);
    --on-edge: var(--on-bad);
  }

  .neutral {
    --edge: var(--notice-neutral);
    --on-edge: var(--on-neutral);
  }
</style>
