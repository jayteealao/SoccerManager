// The player's settings as the page reads them from the launcher, and the motion word each
// setting puts on the page.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { DEFAULTS, MOTIONS, SPEEDS, motionWord, readSettings } from '../src/lib/settings.js';

test('the defaults are 1x, follow Windows and commentary on', () => {
  assert.deepEqual({ ...DEFAULTS }, { speed: 1, motion: 'follow', commentary: true });
  assert.deepEqual([...SPEEDS], [1, 2, 4, 8]);
  assert.deepEqual([...MOTIONS], ['follow', 'reduce', 'full']);
});

test('a settings block is read value by value, and a wrong value takes its default', () => {
  assert.deepEqual(readSettings({ speed: 4, motion: 'reduce', commentary: false }), {
    speed: 4,
    motion: 'reduce',
    commentary: false,
  });
  assert.deepEqual(readSettings({ speed: 3, motion: 'slow', commentary: 'no' }), { ...DEFAULTS });
  assert.deepEqual(readSettings(null), { ...DEFAULTS });
  assert.deepEqual(readSettings({ speed: 8 }), { ...DEFAULTS, speed: 8 });
});

test('follow takes the system preference; reduce and full override it', () => {
  assert.equal(motionWord('follow', true), 'reduce');
  assert.equal(motionWord('follow', false), 'full');
  assert.equal(motionWord('reduce', false), 'reduce');
  assert.equal(motionWord('full', true), 'full');
});
