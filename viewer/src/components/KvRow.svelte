<!-- One key and its value, on one line. The value uses tabular numerals so a changing
     number does not shift the row. -->
<script>
  let { key, value } = $props();
</script>

<div class="kv">
  <span>{key}</span>
  <b class="num">{value}</b>
</div>

<style>
  .kv {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    padding: 3px 0;
    color: var(--ink-2);
  }

  b {
    color: var(--ink);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
</style>
