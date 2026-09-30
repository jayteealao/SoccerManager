<!-- One 16-unit glyph from icons.js, drawn in the current text colour. Decorative: the
     control that holds it carries the name. -->
<script>
  let { glyph, size = 15 } = $props();
</script>

<svg class="ic" width={size} height={size} viewBox="0 0 16 16" aria-hidden="true" focusable="false">
  {#if glyph.d}
    <path d={glyph.d} fill="currentColor" fill-rule={glyph.evenodd ? 'evenodd' : 'nonzero'} />
  {/if}
  {#if glyph.stroke}
    <path d={glyph.stroke} fill="none" stroke="currentColor" stroke-width={glyph.width ?? 1.6} />
  {/if}
</svg>

<style>
  .ic {
    flex: none;
    vertical-align: middle;
  }
</style>
