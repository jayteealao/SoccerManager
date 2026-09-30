// @vitest-environment jsdom
// The replay, in jsdom: it draws the stored match on its own 752 × 290 canvas; its playback
// controls have names and call the session (Back 10 seconds, play and pause, Forward 10
// seconds, the speeds); CONTINUE goes back to the view it came from; every stub is inert and
// hidden, and a planted focusable stub fails the check.

import assert from 'node:assert/strict';
import { tick } from 'svelte';
import { test } from 'vitest';

import { COMPONENT_COUNT } from '../../src/lib/decode.js';
import { encodeKeyframe } from '../helpers.js';
import { opened, stubFaults, useFakes } from './harness.js';

useFakes();

const page = () => document.querySelector('[data-screen="replay"]')?.closest('.app') ?? null;
const named = (name) => page().querySelector(`button[aria-label="${name}"]`);

/// A session holding 1,000 stored ticks, shown in the replay view.
async function replay() {
  const run = await opened();
  for (let t = 1; t <= 1000; t += 1) {
    const data = encodeKeyframe(t, new Array(COMPONENT_COUNT).fill(t % 100));
    for (const fn of run.socket.listeners.message ?? []) {
      fn({ data });
    }
  }
  run.s.showReplay();
  await tick();
  return run;
}

test('the replay draws on its own canvas at 752 × 290', async () => {
  const { s } = await replay();
  assert.equal(s.view, 'replay');
  const canvas = page().querySelector('canvas');
  assert.equal(canvas.dataset.width, '752');
  assert.equal(canvas.dataset.height, '290');
  assert.equal(s.canvases.replay, canvas);
  assert.match(page().querySelector('.hd').textContent, /Match Analysis/);
});

test('the playback controls have names and call the session', async () => {
  const { s } = await replay();
  const calls = [];
  s.step = (seconds) => calls.push(['step', seconds]);
  s.setPlaying = (playing) => calls.push(['play', playing]);
  s.selectSpeed = (speed) => calls.push(['speed', speed]);
  named('Back 10 seconds').click();
  named('Forward 10 seconds').click();
  (named('Pause') ?? named('Play')).click();
  named('4x').click();
  assert.deepEqual(calls, [
    ['step', -10],
    ['step', 10],
    ['play', !s.playing],
    ['speed', 4],
  ]);
  assert.ok(page().querySelector('[role="slider"], input[type="range"]'), 'the timeline rewinds');
});

test('CONTINUE on the replay goes back to the match it came from', async () => {
  const { s } = await replay();
  page().querySelector('.cont').click();
  await tick();
  assert.equal(s.view, 'match');
  assert.equal(page(), null);
});

test('every replay stub is inert and hidden, and a planted focusable stub fails the check', async () => {
  await replay();
  const root = page();
  for (const note of ['replay view toggles', 'slow motion speeds', 'clip this moment and video session', 'your screen', 'catalogue', 'analyst depth', 'traced figure', 'tab: catalogue']) {
    assert.ok(root.querySelector(`[data-stub="${note}"]`), note);
  }
  assert.deepEqual(stubFaults(root), []);
  root.querySelector('[data-stub="catalogue"]').append(document.createElement('button'));
  assert.deepEqual(stubFaults(root), ['catalogue: button takes focus']);
});
