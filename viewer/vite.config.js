// The viewer's build and its tests. The development build has three pages: `index.html`, the
// viewer, `handshake.html`, the diagnostic page that checks the engine's socket, and
// `shell-test.html`, the test page the shell screenshots and the keyboard walk open. The
// release build (`vite build --mode release`, into `dist-release/`) is what the game ships:
// the viewer and the handshake page, without the shell test page. Both carry each skin's
// font licence texts in `fonts/`, and `notices.json`, the open-source notices of every crate,
// npm package and font the game ships (scripts/notices.mjs). Nothing is fetched from the
// network at run time: fonts and skins ship inside the build.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';
import { defineConfig } from 'vite';

import { assemble, crateNotices, fontNotices, npmNotices, previousCrates, repoAllow } from './scripts/notices.mjs';

const page = (name) => fileURLToPath(new URL(name, import.meta.url));
const SKINS = fileURLToPath(new URL('src/skins/', import.meta.url));

/// Every skin's `fonts/OFL-*.txt`, emitted as `fonts/<name>`: the fonts are OFL 1.1, which
/// asks that the licence travel with them.
function fontLicences() {
  return {
    name: 'font-licences',
    generateBundle() {
      for (const skin of fs.readdirSync(SKINS, { withFileTypes: true })) {
        const folder = path.join(SKINS, skin.name, 'fonts');
        if (!skin.isDirectory() || !fs.existsSync(folder)) {
          continue;
        }
        for (const name of fs.readdirSync(folder)) {
          if (/^OFL-.*\.txt$/.test(name)) {
            this.emitFile({
              type: 'asset',
              fileName: `fonts/${name}`,
              source: fs.readFileSync(path.join(folder, name)),
            });
          }
        }
      }
    },
  };
}

/// The version the workspace builds, which the splash names before the engine answers.
function workspaceVersion() {
  const toml = fs.readFileSync(fileURLToPath(new URL('../Cargo.toml', import.meta.url)), 'utf8');
  return toml.match(/\[workspace\.package\][^[]*?^version\s*=\s*"([^"]+)"/m)?.[1] ?? '';
}

/// `notices.json`: the fonts, the npm packages inside this bundle, the engine program's crates
/// and, when SM_PREVIOUS_NOTICES names its list, the previous engine's crates. The build fails,
/// naming each package, when one has no licence text or no allowed licence.
function notices(version) {
  return {
    name: 'notices',
    apply: 'build',
    generateBundle(_options, bundle) {
      const modules = Object.values(bundle)
        .filter((item) => item.type === 'chunk')
        .flatMap((chunk) => Object.keys(chunk.modules ?? {}));
      try {
        const file = assemble({
          version,
          fonts: fontNotices(),
          npm: npmNotices(modules),
          crates: crateNotices(),
          previous: previousCrates(),
          allow: repoAllow(),
        });
        this.emitFile({ type: 'asset', fileName: 'notices.json', source: `${JSON.stringify(file, null, 1)}
` });
      } catch (error) {
        this.error(error.message);
      }
    },
  };
}

export default defineConfig(({ mode }) => {
  const release = mode === 'release';
  const version = workspaceVersion();
  const input = {
    index: page('index.html'),
    handshake: page('handshake.html'),
  };
  if (!release) {
    input['shell-test'] = page('shell-test.html');
  }
  return {
    plugins: [svelte(), svelteTesting(), fontLicences(), notices(version)],
    define: { __TOUCHLINE_VERSION__: JSON.stringify(version) },
    build: {
      outDir: release ? 'dist-release' : 'dist',
      rollupOptions: { input },
    },
    test: {
      // Logic tests run in Node; a component test opts into jsdom with a
      // `// @vitest-environment jsdom` line at its top.
      environment: 'node',
      include: ['tests/**/*.test.js'],
    },
  };
});
