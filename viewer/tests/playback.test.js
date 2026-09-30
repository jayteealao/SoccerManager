// AC-H5: a speed the manager selects takes effect at once, even after the stream has
// ended and no further arrival will recompute it.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { Playback } from '../src/lib/playback.js';
import { TICKS_PER_SECOND } from '../src/lib/schedule.js';
import { quiet } from './helpers.js';

function stubbed() {
  const calls = { speed: [], onSpeed: [], onNotice: [] };
  const playback = new Playback({
    scheduler: { setSpeed: (speed) => calls.speed.push(speed) },
    onSpeed: (requested, effective) => calls.onSpeed.push([requested, effective]),
    onNotice: (text) => calls.onNotice.push(text),
  });
  return { playback, calls };
}

test('a selected speed applies with no arrivals', () => {
  const { playback, calls } = stubbed();
  playback.select(4);
  assert.equal(calls.speed.at(-1), 4);
  assert.deepEqual(calls.onSpeed.at(-1), [4, 4]);
});

test('selecting at or below the sustained rate clears the lag notice', () => {
  const { playback, calls } = stubbed();
  quiet(() => {
    playback.select(8);
    const perTick = 1000 / (TICKS_PER_SECOND * 3);
    for (let i = 0; i < TICKS_PER_SECOND * 3 * 3; i += 1) {
      playback.noteArrival(i * perTick, i);
    }
  });
  assert.equal(playback.noticeShown, true);
  assert.equal(typeof calls.onNotice.at(-1), 'string');

  quiet(() => playback.select(2));
  assert.equal(playback.effective, 2);
  assert.equal(calls.speed.at(-1), 2);
  assert.deepEqual(calls.onSpeed.at(-1), [2, 2]);
  assert.equal(playback.noticeShown, false);
  assert.equal(calls.onNotice.at(-1), null);
});

test('a slow arrival rate with a healthy lead is an engine held near the pitch, not lag', () => {
  const { playback } = stubbed();
  quiet(() => {
    playback.select(8);
    // Ticks arrive at 3x, as a held engine delivers them once playback slowed to 3x, while
    // 480 ticks wait unplayed: the old estimate called this lag and never let go of it.
    const perTick = 1000 / (TICKS_PER_SECOND * 3);
    for (let i = 0; i < TICKS_PER_SECOND * 3 * 3; i += 1) {
      playback.noteArrival(i * perTick, i, 480);
    }
  });
  assert.equal(playback.noticeShown, false);
  assert.equal(playback.effective, 8);

  // The same rate with the store running dry is lag.
  quiet(() => {
    const perTick = 1000 / (TICKS_PER_SECOND * 3);
    const start = TICKS_PER_SECOND * 3 * 3;
    for (let i = start; i < start + TICKS_PER_SECOND * 3; i += 1) {
      playback.noteArrival(i * perTick, i, 20);
    }
  });
  assert.equal(playback.noticeShown, true);
  assert.equal(playback.effective, 3);
});
