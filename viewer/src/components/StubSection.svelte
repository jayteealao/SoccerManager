<!-- STUB wrapper: a part the viewer does not build yet, kept in place for the look. Its
     content is inert (no focus, no clicks), hidden from assistive technology, and reads no
     engine data. By default it fades to the stub opacity and can carry a LATER label; the
     shell's own navigation stubs keep full strength, as the approved boards draw them. -->
<script>
  let { note, inline = false, fade = true, later = false, children } = $props();
</script>

{#if inline}
  <span class="stub" class:fade data-stub={note} inert aria-hidden="true">
    {@render children?.()}
    {#if later}<span class="later">LATER</span>{/if}
  </span>
{:else}
  <div class="stub" class:fade data-stub={note} inert aria-hidden="true">
    {@render children?.()}
    {#if later}<span class="later">LATER</span>{/if}
  </div>
{/if}

<style>
  .stub {
    display: contents;
  }

  .stub.fade {
    display: block;
    opacity: var(--stub-opacity);
  }

  span.stub.fade {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .later {
    font: 600 9px var(--fd);
    letter-spacing: 0.06em;
    color: var(--ink-4);
    margin-left: 6px;
  }
</style>
