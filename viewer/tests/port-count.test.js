// The port keeps every former test: the sixteen ported files hold at least as many tests as
// the page's own `web/tests` held for the same modules (112 when the port began), and each
// ported module lives in the viewer project.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import { REPO_ROOT } from './helpers.js';

const PORTED = [
  'colour',
  'decode',
  'goal-moment',
  'history',
  'interpolate',
  'lead',
  'mark',
  'match-state',
  'pending',
  'playback',
  'recovery',
  'replay-file',
  'replay-migration',
  'replay-record',
  'schedule',
  'stoppages',
];

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
];

const count = (file) => (fs.readFileSync(file, 'utf8').match(/^test\(/gm) ?? []).length;

test('the ported test files hold at least the former count', () => {
  let ported = 0;
  let former = 0;
  for (const name of PORTED) {
    const now = count(path.join(REPO_ROOT, 'viewer/tests', `${name}.test.js`));
    const before = count(path.join(REPO_ROOT, 'web/tests', `${name}.test.mjs`));
    assert.ok(now >= before, `${name}: ${now} ported of ${before}`);
    ported += now;
    former += before;
  }
  assert.equal(former, 112, 'the former files held 112 tests');
  assert.ok(ported >= 112, `${ported} ported tests`);
});

test('every ported module lives in the viewer project', () => {
  for (const name of MODULES) {
    assert.ok(
      fs.existsSync(path.join(REPO_ROOT, 'viewer/src/lib', `${name}.js`)),
      `viewer/src/lib/${name}.js`
    );
  }
});
