// AC-b, in part: a rendered marker lies on the segment between two received ticks.
//
// The other part of AC-b is read from the page's own test hook during the browser drive.
// This file proves the property of the function, which no drive can prove exhaustively.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { COMPONENT_COUNT } from '../src/lib/decode.js';
import { between } from '../src/lib/interpolate.js';

const a = new Int16Array(COMPONENT_COUNT);
const b = new Int16Array(COMPONENT_COUNT);
for (let i = 0; i < COMPONENT_COUNT; i += 1) {
  a[i] = -2000 + i * 31;
  b[i] = 2000 - i * 17;
}

test('a fraction of zero returns the earlier tick exactly', () => {
  const out = new Int16Array(COMPONENT_COUNT);
  between(a, b, 0, out);
  assert.deepEqual([...out], [...a]);
});

test('a fraction of one returns the later tick exactly', () => {
  const out = new Int16Array(COMPONENT_COUNT);
  between(a, b, 1, out);
  assert.deepEqual([...out], [...b]);
});

test('every fraction inside the range lies on the segment', () => {
  const out = new Int16Array(COMPONENT_COUNT);
  for (let step = 0; step <= 100; step += 1) {
    const t = step / 100;
    between(a, b, t, out);
    for (let i = 0; i < COMPONENT_COUNT; i += 1) {
      const exact = a[i] + (b[i] - a[i]) * t;
      // One centimetre, which is the whole resolution of the wire format.
      assert.ok(Math.abs(out[i] - exact) <= 1, `component ${i} at t=${t}: ${out[i]} vs ${exact}`);
      const low = Math.min(a[i], b[i]);
      const high = Math.max(a[i], b[i]);
      assert.ok(out[i] >= low && out[i] <= high, `component ${i} left the segment`);
    }
  }
});

test('a fraction beyond the range is clamped and never extrapolates', () => {
  const out = new Int16Array(COMPONENT_COUNT);
  between(a, b, 1.5, out);
  assert.deepEqual([...out], [...b], 'a fraction past the next tick stops at the next tick');
  between(a, b, 40, out);
  assert.deepEqual([...out], [...b]);
  between(a, b, -0.5, out);
  assert.deepEqual([...out], [...a], 'a negative fraction stops at the earlier tick');
  between(a, b, Number.NaN, out);
  assert.deepEqual([...out], [...a], 'a fraction that is not a number stops at the earlier tick');
});
