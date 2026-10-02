<!-- The splash's lockup: the mark at 120 px, the TOUCHLINE name at 76 px with its line, and
     the cyan MATCH ENGINE block with the version. With `reveal` it plays the 1.8 s logo reveal
     of 02c-craft.md (the mark's parts, then the name wipes out from behind the mark, then the
     line and the block); `finished` jumps to the last frame. Reduced motion stops every
     animation through the root rule, which leaves the last frame. -->
<script>
  import TouchlineMark from './TouchlineMark.svelte';

  let { version = '', reveal = true, finished = false } = $props();

  let block = $derived(version ? `MATCH ENGINE ${version}` : 'MATCH ENGINE');
</script>

<div class="lockup" class:reveal class:finished data-reveal={finished ? 'finished' : reveal ? 'playing' : 'still'}>
  <TouchlineMark size={120} {reveal} {finished} />
  <div class="words">
    <h1 class="name">Touchline</h1>
    <p class="sub">Football match simulation</p>
    <span class="block">{block}</span>
  </div>
</div>

<style>
  .lockup {
    display: flex;
    align-items: center;
    gap: 28px;
  }

  .words {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }

  .name {
    margin: 0;
    font: 800 76px/0.95 var(--fd);
    letter-spacing: 0.02em;
    text-transform: uppercase;
    color: var(--ink);
  }

  .sub {
    margin: 8px 0 0;
    font: 500 14px var(--fb);
    color: var(--ink-2);
  }

  .block {
    display: inline-flex;
    margin-top: 14px;
    background: var(--cyan);
    color: var(--cyan-ink);
    clip-path: polygon(14px 0, 100% 0, calc(100% - 14px) 100%, 0 100%);
    padding: 6px 30px;
    font: 800 13px var(--fd);
    letter-spacing: 0.12em;
  }

  .reveal .name {
    animation: wipe 500ms var(--ease) 1100ms both;
  }

  .reveal .sub {
    animation: fade 300ms var(--ease) 1400ms both;
  }

  .reveal .block {
    animation: slide 300ms var(--ease) 1500ms both;
  }

  .reveal.finished .name,
  .reveal.finished .sub,
  .reveal.finished .block {
    animation: none;
  }

  @keyframes wipe {
    from {
      clip-path: inset(0 100% 0 0);
    }
    to {
      clip-path: inset(0 0 0 0);
    }
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes slide {
    from {
      opacity: 0;
      transform: translateX(24px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
</style>
