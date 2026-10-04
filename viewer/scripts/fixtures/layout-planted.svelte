<!-- A planted component the scan's layout rules must refuse: one break per rule. -->
<script>
  let box = $state(0);
  function measure(el) {
    box = el.getBoundingClientRect().width;
  }
</script>

<p class="a" use:measure>text</p>

<style>
  .a {
    font-size: clamp(10px, 1.2vw, 14px);
  }

  @media (min-width: 1500px) {
    .a {
      padding: 4px;
    }
  }

  @media (min-width: 1600px), (max-height: 599px) {
    .a {
      margin: 4px;
    }
  }
</style>
