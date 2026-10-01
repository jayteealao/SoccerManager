// The port keeps every former test: the twenty-three ported files hold at least as many tests
// as the former page's own tests held for the same modules. The former page is gone, so its
// counts are kept here as they stood when it was deleted (148 tests in all). Each ported
// module lives in the viewer project, and no page file lives outside it.

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import { REPO_ROOT } from './helpers.js';

/// The former page's test files and the tests each held, read from `web/tests` before it was
/// deleted.
export const FORMER = Object.freeze({
  colour: 7,
  decode: 6,
  feed: 8,
  'goal-moment': 6,
  history: 9,
  interpolate: 4,
  lead: 4,
  lineup: 7,
  lineups: 6,
  mark: 6,
  'match-state': 10,
  pending: 6,
  pitch: 2,
  playback: 3,
  recovery: 9,
  'replay-file': 11,
  'replay-migration': 8,
  'replay-record': 6,
  report: 3,
  schedule: 8,
  stats: 4,
  stoppages: 9,
  'tactics-panel': 6,
});

const MODULES = [
  'socket',
  'decode',
  'interpolate',
  'playback',
  'replay-file',
  'schedule',
  'match-state',
  'history',
  'lead',
  'pending',
  'stoppages',
  'colour',
  'mark',
  'signal',
  'goal-moment',
  'recovery',
  'launcher',
  'scoreboard',
  'feed',
  'stats',
  'pitch',
  'lineup',
  'lineups',
  'tactics-panel',
  'substitution-picker',
  'lineup-editor',
  'report',
  'handshake',
];

/// Folders whose `.mjs`, `.html` and `.css` files are not page files: the browser suite,
/// installs and builds, and local working notes.
const NOT_PAGE = ['viewer/', 'e2e/', 'node_modules/', 'target/', '.ai/', '.scratch/'];

const count = (file) => (fs.readFileSync(file, 'utf8').match(/^test\(/gm) ?? []).length;

/// What breaks the rule that the viewer project holds every page file: a `web/` folder at the
/// root, or a tracked `.mjs`, `.html` or `.css` file outside the allowed folders.
export function strayPageFiles(files, webExists) {
  const faults = webExists ? ['web/ still exists'] : [];
  for (const file of files) {
    if (/\.(mjs|html|css)$/.test(file) && !NOT_PAGE.some((dir) => file.startsWith(dir))) {
      faults.push(file);
    }
  }
  return faults;
}

const tracked = () =>
  execFileSync('git', ['ls-files'], { cwd: REPO_ROOT, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 })
    .split('\n')
    .filter(Boolean);

test('the ported test files hold at least the former count', () => {
  let ported = 0;
  let former = 0;
  for (const [name, before] of Object.entries(FORMER)) {
    const now = count(path.join(REPO_ROOT, 'viewer/tests', `${name}.test.js`));
    assert.ok(now >= before, `${name}: ${now} ported of ${before}`);
    ported += now;
    former += before;
  }
  assert.equal(Object.keys(FORMER).length, 23, 'twenty-three former files');
  assert.equal(former, 148, 'the former files held 148 tests');
  assert.ok(ported >= 148, `${ported} ported tests`);
});

test('every ported module lives in the viewer project', () => {
  for (const name of MODULES) {
    assert.ok(
      fs.existsSync(path.join(REPO_ROOT, 'viewer/src/lib', `${name}.js`)),
      `viewer/src/lib/${name}.js`
    );
  }
});

test('no page file lives outside the viewer project, and the former page folder is gone', () => {
  assert.deepEqual(strayPageFiles(tracked(), fs.existsSync(path.join(REPO_ROOT, 'web'))), []);
});

test('the page-file check fails on a former page folder or a stray page file', () => {
  assert.deepEqual(strayPageFiles(['viewer/src/main.js', 'e2e/support/page.mjs'], false), []);
  assert.deepEqual(strayPageFiles([], true), ['web/ still exists']);
  assert.deepEqual(strayPageFiles(['web/main.mjs', 'docs/page.html', 'crates/x/style.css'], false), [
    'web/main.mjs',
    'docs/page.html',
    'crates/x/style.css',
  ]);
});
