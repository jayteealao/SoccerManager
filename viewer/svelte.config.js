// Svelte compiler options. Runes mode everywhere: every component uses $props, $state and
// $derived, never the legacy `export let` syntax.
export default {
  compilerOptions: {
    runes: true,
  },
};
