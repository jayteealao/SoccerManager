<!-- A planted component the token scan must pass: tokens only, and a comment that names
     #123456 and oklch(0.5 0.1 250) without setting them. -->
<p class="a">text</p>

<style>
  /* color: #123456 in a comment is not a value. */
  .a {
    color: var(--ink);
    font: 600 12px var(--fb);
    fill: currentColor;
  }
</style>
