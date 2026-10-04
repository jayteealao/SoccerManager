// The two checks can fail: the token scan refuses a planted literal colour, a planted font
// name and a planted --ink-4, and the three planted layout rule breaks (a fluid font size,
// layout read in script and an @media width off the window steps); the contrast check refuses a planted low pair and a pair a
// hair under 4.5:1 that a rounded comparison would pass.

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { test } from 'vitest';

import { REPO_ROOT } from './helpers.js';

const VIEWER = path.join(REPO_ROOT, 'viewer');
const run = (script, ...args) =>
  spawnSync(process.execPath, [path.join(VIEWER, 'scripts', script), ...args], {
    cwd: VIEWER,
    encoding: 'utf8',
  });
const fixture = (name) => path.join(VIEWER, 'scripts', 'fixtures', name);

test('the token scan names every planted literal and exits 1', () => {
  const out = run('token-scan.mjs', fixture('literal.svelte'));
  assert.equal(out.status, 1, out.stdout + out.stderr);
  assert.match(out.stderr, /literal\.svelte:6: a hex colour/);
  assert.match(out.stderr, /literal\.svelte:7: a font family/);
  assert.match(out.stderr, /literal\.svelte:11: --ink-4 outside StubSection/);
  assert.match(out.stdout, /^3 literal/m);
});

test('the token scan passes tokens and ignores a value named in a comment', () => {
  const out = run('token-scan.mjs', fixture('clean.svelte'));
  assert.equal(out.status, 0, out.stdout + out.stderr);
});

test('the scan names each planted layout rule break and exits 1', () => {
  const out = run('token-scan.mjs', fixture('layout-planted.svelte'));
  assert.equal(out.status, 1, out.stdout + out.stderr);
  assert.match(out.stderr, /layout-planted\.svelte:5: layout read in script \(getBoundingClientRect\)/);
  assert.match(out.stderr, /layout-planted\.svelte:13: a fluid font size/);
  assert.match(out.stderr, /layout-planted\.svelte:16: an @media size that is not a window step \(min-width: 1500px\)/);
  assert.doesNotMatch(out.stderr, /layout-planted\.svelte:22:/, 'the window steps 1600px and 599px pass');
  assert.match(out.stdout, /^3 literal/m);
});

test('the scan refuses a compact width without the compact height, across wrapped lines', () => {
  const out = run('token-scan.mjs', fixture('compact-planted.svelte'));
  assert.equal(out.status, 1, out.stdout + out.stderr);
  assert.match(out.stderr, /compact-planted\.svelte:5: a compact width without the compact height/);
  assert.doesNotMatch(out.stderr, /compact-planted\.svelte:11:/, 'the wrapped compact pair passes');
  assert.match(out.stderr, /compact-planted\.svelte:18: an @media size that is not a window step \(max-width: 1100px\)/);
  assert.match(out.stdout, /^2 literal/m);
});

test('the token scan passes the viewer source', () => {
  const out = run('token-scan.mjs');
  assert.equal(out.status, 0, out.stdout + out.stderr);
});

test('the contrast check fails a low pair and a 4.4999 pair, and reports an inert pair', () => {
  const out = run(
    'contrast.mjs',
    path.join(VIEWER, 'src/skins/broadcast-blue/tokens.css'),
    fixture('bad-pairs.json')
  );
  assert.equal(out.status, 1, out.stdout + out.stderr);
  assert.match(out.stderr, /#777777 on #888888 .* is under 4\.5:1/);
  assert.match(out.stderr, /#009a9a on #232427 .*: 4\.50:1 is under 4\.5:1/);
  assert.match(out.stdout, /^inert 2\.46:1 +#5d6067 on #232427/m);
  assert.match(out.stdout, /3 pairs, 2 failing/);
});

test('the contrast check passes the broadcast-blue skin', () => {
  const out = run('contrast.mjs');
  assert.equal(out.status, 0, out.stdout + out.stderr);
  assert.match(out.stdout, /pass {2}4\.50:1 {2}--navy-sub on --navy-500/);
});

test('the contrast check covers every shipped skin, the light skin read from OKLCH', () => {
  const out = run('contrast.mjs');
  assert.equal(out.status, 0, out.stdout + out.stderr);
  assert.match(out.stdout, /^broadcast-blue: \d+ pairs, 0 failing$/m);
  assert.match(out.stdout, /^interim-light: \d+ pairs, 0 failing$/m);

  // Its tokens are OKLCH; each is read as the hex it renders as, never skipped.
  const light = run('contrast.mjs', path.join(VIEWER, 'src/skins/interim-light/tokens.css'));
  assert.equal(light.status, 0, light.stdout + light.stderr);
  assert.doesNotMatch(light.stderr, /token not defined/);
  assert.match(light.stdout, /pass {2}\d+\.\d\d:1 {2}--navy-sub on --navy-600/);
});
