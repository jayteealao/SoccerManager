// Skip to the next stoppage: one index merged from the two marks the protocol already
// sends, with a goal and the restart that follows it counted once.

import assert from 'node:assert/strict';
import test from 'node:test';

import { COLLAPSE_TICKS, Stoppages } from '../stoppages.mjs';

test('a goal and its restart collapse to one entry at the earlier tick', () => {
  const stoppages = new Stoppages();
  stoppages.add(1000);
  stoppages.add(1012);
  assert.equal(stoppages.length, 1);
  assert.equal(stoppages.next(0), 1000);
});

test('a restart with no event of its own stands alone', () => {
  const stoppages = new Stoppages();
  stoppages.add(1000);
  stoppages.add(1000 + COLLAPSE_TICKS + 1);
  assert.equal(stoppages.length, 2);
  assert.equal(stoppages.next(1000), 1026);
});

test('the next stoppage past the last one is null', () => {
  const stoppages = new Stoppages();
  stoppages.add(1);
  stoppages.add(500);
  assert.equal(stoppages.next(500), null);
  assert.equal(stoppages.next(9999), null);
});

test('the previous stoppage walks backwards and runs out at the first', () => {
  const stoppages = new Stoppages();
  for (const tick of [1, 500, 2000, 9000]) {
    stoppages.add(tick);
  }
  assert.equal(stoppages.prev(9000), 2000);
  assert.equal(stoppages.prev(2001), 2000);
  assert.equal(stoppages.prev(2000), 500);
  assert.equal(stoppages.prev(1), null);
});
