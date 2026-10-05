// AC-c: at eight times speed the viewer skips ticks and never renders past the newest
// tick it has received. Driven with a synthetic stream and an injected clock, so the
// property is proved rather than sampled.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { Scheduler, TICKS_PER_SECOND } from '../src/lib/schedule.js';
import { captured, quiet } from './helpers.js';

/// Runs a stream for `seconds` of wall time at `fps`, with ticks arriving at `speed`.
function run({ speed, fps, seconds, stallAt = null }) {
  const scheduler = new Scheduler();
  scheduler.setSpeed(speed);
  const frameMs = 1000 / fps;
  let timestamp = 0;
  let consumed = 0;
  let lastFrom = 0;
  let passedNewest = false;
  let extrapolated = false;

  for (let frame = 0; frame < fps * seconds; frame += 1) {
    timestamp += stallAt !== null && frame === stallAt ? 16 + frameMs : frameMs;
    // The producer is ahead of the clock by exactly the speed it is asked for.
    const newest = Math.floor((timestamp / 1000) * TICKS_PER_SECOND * speed);
    const step = scheduler.advance(timestamp, newest);
    if (step.to > newest) {
      passedNewest = true;
    }
    if (step.fraction < 0 || step.fraction > 1) {
      extrapolated = true;
    }
    consumed += step.from - lastFrom;
    lastFrom = step.from;
  }
  return { scheduler, consumed, passedNewest, extrapolated, seconds };
}

test('at eight times speed the scheduler consumes four hundred ticks a second', () => {
  const { consumed, seconds, passedNewest, extrapolated } = quiet(() =>
    run({ speed: 8, fps: 60, seconds: 5 })
  );
  const perSecond = consumed / seconds;
  assert.ok(Math.abs(perSecond - 400) < 8, `consumed ${perSecond} ticks a second`);
  assert.equal(passedNewest, false, 'the cursor never passes the newest received tick');
  assert.equal(extrapolated, false);
});

test('at eight times speed ticks are skipped and counted', () => {
  const rows = captured(() => run({ speed: 8, fps: 60, seconds: 3 }));
  const skips = rows.filter((r) => r.signal === 'viewer.tick_skipped');
  assert.ok(skips.length >= 2, `expected a row a second, saw ${skips.length}`);
  assert.ok(skips[0].skipped > 0);
  assert.equal(skips[0].speed, 8);
});

test('at one times speed no tick is skipped', () => {
  let result;
  const rows = captured(() => {
    result = run({ speed: 1, fps: 60, seconds: 3 });
  });
  assert.equal(
    rows.filter((r) => r.signal === 'viewer.tick_skipped').length,
    0,
    'fifty ticks a second against sixty frames a second skips nothing'
  );
  const perSecond = result.consumed / result.seconds;
  assert.ok(Math.abs(perSecond - TICKS_PER_SECOND) < 2, `consumed ${perSecond}`);
});

test('at one times speed a stalled frame delays playback instead of skipping', () => {
  const scheduler = new Scheduler();
  scheduler.setSpeed(1);
  const rows = captured(() => {
    let timestamp = 0;
    let lastFrom = 0;
    for (let frame = 0; frame < 120; frame += 1) {
      // Frame 3 stalls for 100 milliseconds, as the first stream burst decodes.
      timestamp += frame === 3 ? 100 : 1000 / 60;
      const step = scheduler.advance(timestamp, 100000);
      if (frame === 3) {
        assert.equal(step.skipped, 0, 'the stalled frame skips no tick');
        assert.ok(step.from - lastFrom <= 1, `stepped from ${lastFrom} to ${step.from}`);
      }
      assert.ok(step.fraction >= 0 && step.fraction <= 1);
      lastFrom = step.from;
    }
  });
  assert.equal(rows.filter((r) => r.signal === 'viewer.tick_skipped').length, 0);
});

test('a sixteen millisecond stall does not cause an extrapolation', () => {
  const { passedNewest, extrapolated } = quiet(() =>
    run({ speed: 4, fps: 60, seconds: 2, stallAt: 40 })
  );
  assert.equal(passedNewest, false);
  assert.equal(extrapolated, false);
});

test('a cursor asked to run past the newest tick stops at it', () => {
  const scheduler = new Scheduler();
  scheduler.setSpeed(8);
  quiet(() => {
    // The producer has stopped at tick 120; a whole second of frames must not pass it.
    let timestamp = 0;
    for (let frame = 0; frame < 60; frame += 1) {
      timestamp += 1000 / 60;
      const step = scheduler.advance(timestamp, 120);
      assert.ok(step.to <= 120, `drew towards ${step.to} with 120 received`);
      assert.ok(step.fraction >= 0 && step.fraction <= 1);
    }
  });
});

test('the frame rate is derived from timestamp deltas and carries the refresh rate', () => {
  const scheduler = new Scheduler();
  quiet(() => {
    // A 144 hertz panel that delivers every frame.
    let timestamp = 0;
    for (let frame = 0; frame < 300; frame += 1) {
      timestamp += 1000 / 144;
      scheduler.advance(timestamp, 100000);
    }
  });
  const budget = scheduler.budget();
  assert.equal(budget.refresh_hz, 144, 'the panel rate is reported beside the frame rate');
  assert.ok(Math.abs(budget.fps_median - 144) < 1);
  assert.equal(budget.dropped_frames, 0);
  assert.ok(budget.frame_ms_p95 < 16.6);
});

test('a dropped frame shows as a lower rate against an unchanged refresh rate', () => {
  const scheduler = new Scheduler();
  quiet(() => {
    let timestamp = 0;
    for (let frame = 0; frame < 300; frame += 1) {
      // Every other frame is missed: the panel still runs at 60, the page draws at 30.
      timestamp += frame % 2 === 0 ? 1000 / 60 : 2000 / 60;
      scheduler.advance(timestamp, 100000);
    }
  });
  const budget = scheduler.budget();
  assert.equal(budget.refresh_hz, 60);
  assert.ok(budget.fps_median < 60, `median ${budget.fps_median}`);
  assert.ok(budget.dropped_frames > 100, `dropped ${budget.dropped_frames}`);
});
