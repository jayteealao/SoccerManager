// AC-g, part one: a full match's stored history stays far under the 300 MB budget, and
// the number reported is exact rather than estimated.
//
// Part two is the browser's own whole-page figure, read during the drive. This file owns
// the exact byte count, which is always available and never resolves late.

import assert from 'node:assert/strict';
import test from 'node:test';

import { COMPONENT_COUNT } from '../decode.mjs';
import { BUDGET_BYTES, History } from '../history.mjs';
import { captured, quiet } from './helpers.mjs';

const TICKS_PER_MATCH = 270_000;

const componentsFor = (tick) => {
  const out = new Int16Array(COMPONENT_COUNT);
  for (let i = 0; i < COMPONENT_COUNT; i += 1) {
    out[i] = ((tick * 7 + i * 11) % 9000) - 4500;
  }
  return out;
};

test('a whole match holds exactly 25,380,000 bytes', () => {
  const history = new History(TICKS_PER_MATCH);
  assert.equal(history.bytes(), 25_380_000);
  assert.equal(history.bytes(), TICKS_PER_MATCH * COMPONENT_COUNT * 2);
  assert.ok(history.bytes() < BUDGET_BYTES / 12, 'twelve times under the budget');
});

test('a read at a tick returns exactly what was written at that tick', () => {
  const history = new History(5_000);
  for (let tick = 1; tick <= 5_000; tick += 1) {
    history.append(tick, componentsFor(tick));
  }
  assert.equal(history.count, 5_000);
  assert.equal(history.firstTick, 1);
  assert.equal(history.newestTick, 5_000);

  const out = new Int16Array(COMPONENT_COUNT);
  for (const tick of [1, 2, 1234, 4999, 5000]) {
    assert.equal(history.tickAt(tick, out), true);
    assert.deepEqual([...out], [...componentsFor(tick)], `tick ${tick}`);
  }
  assert.equal(history.tickAt(0, out), false, 'a tick before the first is not stored');
  assert.equal(history.tickAt(5_001, out), false, 'a tick after the last is not stored');
});

test('growth past the expected ticks keeps every earlier tick', () => {
  // Stoppage time makes a match run longer than the hello predicts.
  const history = new History(100);
  for (let tick = 1; tick <= 450; tick += 1) {
    history.append(tick, componentsFor(tick));
  }
  assert.equal(history.count, 450);
  assert.ok(history.capacity >= 450);
  const out = new Int16Array(COMPONENT_COUNT);
  for (const tick of [1, 99, 100, 101, 256, 450]) {
    assert.equal(history.tickAt(tick, out), true, `tick ${tick} survived the growth`);
    assert.deepEqual([...out], [...componentsFor(tick)], `tick ${tick}`);
  }
});

test('the scrubber limit follows the newest tick past the expected ticks', () => {
  // A knockout shoot-out's sudden death can run past the announced maximum.
  const history = new History(100);
  assert.equal(history.scrubLimit, 100, 'an empty history ends at the announced ticks');
  for (let tick = 1; tick <= 100; tick += 1) {
    history.append(tick, componentsFor(tick));
  }
  assert.equal(history.scrubLimit, 100);
  for (let tick = 101; tick <= 130; tick += 1) {
    history.append(tick, componentsFor(tick));
  }
  assert.equal(history.scrubLimit, 130, 'the limit follows the newest tick');
});

test('the history reports both numbers, with the browser figure null until it resolves', () => {
  const history = new History(100);
  history.append(1, componentsFor(1));
  const [row] = captured(() => history.report());
  assert.equal(row.signal, 'viewer.history');
  assert.equal(row.ticks_stored, 1);
  assert.equal(row.history_bytes, history.bytes());
  assert.equal(row.page_bytes, null, 'a browser figure that never resolved is null');
  assert.ok(row.page_bytes_reason, 'a null figure always names why');
  assert.equal(row.budget_bytes, BUDGET_BYTES);

  history.pageBytes = 33_000_000;
  history.pageBytesReason = null;
  const [second] = captured(() => history.report());
  assert.equal(second.page_bytes, 33_000_000);
  assert.equal(second.page_bytes_reason, null);
});

test('a browser with no gauge is named, not silently null', () => {
  const history = new History(10);
  history.measurePage();
  assert.equal(history.pageBytes, null);
  assert.equal(history.pageBytesReason, 'the browser has no measureUserAgentSpecificMemory');
});

test('a browser that refuses the gauge is named with the refusal', () => {
  const history = new History(10);
  const before = globalThis.performance;
  globalThis.performance = {
    measureUserAgentSpecificMemory() {
      const error = new Error('performance.measureUserAgentSpecificMemory is not available.');
      error.name = 'SecurityError';
      throw error;
    },
  };
  try {
    history.measurePage();
  } finally {
    globalThis.performance = before;
  }
  assert.equal(history.pageBytes, null);
  assert.match(history.pageBytesReason, /^SecurityError: /);
  assert.equal(history.measuring, false, 'a refusal does not leave a request outstanding');
});

test('a resumed match truncates the history at its stoppage', () => {
  quiet(() => {
    const history = new History(10);
    const frame = new Int16Array(COMPONENT_COUNT);
    for (let tick = 1; tick <= 8; tick += 1) {
      frame[0] = tick;
      history.append(tick, frame);
    }
    assert.equal(history.truncate(5), 0);
    assert.equal(history.newestTick, 5);
    frame[0] = 60;
    history.append(6, frame);
    const out = new Int16Array(COMPONENT_COUNT);
    assert.ok(history.tickAt(6, out));
    assert.equal(out[0], 60);
    assert.equal(history.tickAt(7, out), false);
  });
});

test('a resume past the newest tick leaves a gap that is never drawn', () => {
  quiet(() => {
    const history = new History(10);
    const frame = new Int16Array(COMPONENT_COUNT);
    for (let tick = 1; tick <= 3; tick += 1) {
      history.append(tick, frame);
    }
    assert.equal(history.truncate(6), 3);
    assert.deepEqual(history.gaps, [[4, 6]]);
    history.append(7, frame);
    const out = new Int16Array(COMPONENT_COUNT);
    assert.equal(history.tickAt(5, out), false);
    assert.ok(history.tickAt(3, out));
    assert.ok(history.tickAt(7, out));
    assert.equal(history.newestTick, 7);
  });
});
