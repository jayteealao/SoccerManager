// The viewer's build and its tests. The build has two pages: `index.html`, the viewer, and
// `shell-test.html`, the test page the shell screenshots and the keyboard walk open. Nothing
// is fetched from the network at run time: fonts and skins ship inside the build.
import { fileURLToPath } from 'node:url';

import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';
import { defineConfig } from 'vite';

const page = (name) => fileURLToPath(new URL(name, import.meta.url));

export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  build: {
    rollupOptions: {
      input: {
        index: page('index.html'),
        'shell-test': page('shell-test.html'),
      },
    },
  },
  test: {
    // Logic tests run in Node; a component test opts into jsdom with a
    // `// @vitest-environment jsdom` line at its top.
    environment: 'node',
    include: ['tests/**/*.test.js'],
  },
});
