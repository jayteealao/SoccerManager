// @vitest-environment jsdom
// The Skip decision, in jsdom: it draws the paused frame on its own canvas; its strip names
// the paused clock, the score, the substitutes and the queue; both options are radios with
// the first checked; Confirm and Keep watching call the session, and Escape is Keep
// watching; focus starts on Confirm; every stub is inert and hidden, and a planted focusable
// stub fails the check.

import assert from 'node:assert/strict';
import { tick } from 'svelte';
import { test } from 'vitest';

import { COMPONENT_COUNT } from '../../src/lib/decode.js';
import { encodeKeyframe } from '../helpers.js';
import { opened, stubFaults, useFakes } from './harness.js';

useFakes();

const page = () => document.querySelector('[data-screen="skip"]')?.closest('.app') ?? null;
const button = (text) =>
  [...page().querySelectorAll('button')].find((b) => b.textContent.trim() === text) ?? null;

/// A kicked-off session paused at tick 600 (0:12) and showing the Skip decision.
async function deciding() {
  const run = await opened();
  run.s.act();
  run.s.act();
  run.socket.deliver({ type: 'ack', command: 'set-lineup' });
  for (let t = 1; t <= 900; t += 1) {
    const data = encodeKeyframe(t, new Array(COMPONENT_COUNT).fill(t % 100));
    for (const fn of run.socket.listeners.message ?? []) {
      fn({ data });
    }
  }
  run.s.rewind(600);
  assert.equal(run.s.openSkip(), true);
  await tick();
  return run;
}

test('the decision draws the paused frame on its own canvas and names where the match stands', async () => {
  const { s } = await deciding();
  assert.equal(s.view, 'skip');
  const root = page();
  const canvas = root.querySelector('canvas');
  assert.equal(canvas.dataset.width, '310');
  assert.equal(canvas.dataset.height, '170');
  assert.equal(s.canvases.skip, canvas);
  assert.match(root.querySelector('.hd').textContent, /Skip to result/);
  assert.match(root.querySelector('.hd').textContent, /Ashford Rovers 0–0 Port Varrow · 0'/);
  assert.match(root.querySelector('.hd').textContent, /PAUSED/);
  const strip = [...root.querySelectorAll('.strip .cell')].map((c) => c.textContent.replace(/\s+/g, ' ').trim());
  assert.deepEqual(strip, [
    'Play paused · 00:12 You chose Skip to result',
    "0 – 0 · 0' No goals yet",
    '0 of 5 Substitutes used',
    'Nothing queued No change waits for a stoppage',
    'Engine 0.3.0 Plays the rest at full speed',
  ]);
  assert.match(root.textContent, /PAUSED · 00:12/);
  assert.match(root.textContent, /Ashford Rovers 0 – 0 Port Varrow/);
});

test('both options are radios with the first checked, and focus starts on Confirm', async () => {
  await deciding();
  const radios = [...page().querySelectorAll('[role="radiogroup"] [role="radio"]')];
  assert.deepEqual(
    radios.map((r) => [r.textContent.replace(/\s+/g, ' ').trim().split('.')[0], r.getAttribute('aria-checked')]),
    [
      ['Skip to the final whistle The report opens at full time', 'true'],
      ['Keep watching Back to the live match at 00:12, paused', 'false'],
    ]
  );
  assert.equal(document.activeElement, button('Confirm: skip to result'));
  radios[1].click();
  await tick();
  assert.equal(radios[1].getAttribute('aria-checked'), 'true');
  assert.ok(button('Confirm: keep watching'), 'Confirm follows the choice');
});

test('Confirm sends the skip and Keep watching goes back with nothing sent', async () => {
  const { s, socket } = await deciding();
  button('Keep watching').click();
  await tick();
  assert.equal(s.view, 'match');
  assert.equal(s.renderedTick, 600);
  assert.ok(!socket.sent.some((c) => c.type === 'skip'));

  s.openSkip();
  await tick();
  button('Confirm: skip to result').click();
  await tick();
  assert.deepEqual(
    socket.sent.filter((c) => c.type === 'skip'),
    [{ type: 'skip' }]
  );
  assert.equal(s.view, 'report');
  assert.equal(s.report.state, 'playing-rest');
});

test('Escape is Keep watching, and RESUME goes back playing', async () => {
  const { s } = await deciding();
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
  await tick();
  assert.equal(s.view, 'match');
  assert.equal(s.playing, false);
  s.openSkip();
  await tick();
  page().querySelector('.cont').click();
  await tick();
  assert.equal(s.view, 'match');
  assert.equal(s.playing, true);
});

test('every decision stub is inert and hidden, and a planted focusable stub fails the check', async () => {
  await deciding();
  const root = page();
  for (const note of ['tab: squad', 'tab: other-grounds']) {
    assert.ok(root.querySelector(`[data-stub="${note}"]`), note);
  }
  assert.deepEqual(stubFaults(root), []);
  assert.equal(root.querySelector('[data-stub="other grounds finish"]'), null, 'the line is live');
  root.querySelector('[data-stub="tab: squad"]').append(document.createElement('button'));
  assert.deepEqual(stubFaults(root), ['tab: squad: button takes focus']);
});

test('the Other grounds step shows with fixtures and is left out with none', async () => {
  const run = await deciding();
  assert.doesNotMatch(page().textContent, /They finish too/, 'no other match: no line');
  run.socket.deliver({
    type: 'matchday',
    round: 1,
    fixtures: [
      {
        fixture: 0,
        home: { 'team.id': 'cu', 'team.name': 'Castlemere United', 'team.kit.primary': '#0f5c63', 'team.kit.secondary': '#ffffff', roster: [] },
        away: { 'team.id': 'gw', 'team.name': 'Greywater', 'team.kit.primary': '#0f5c63', 'team.kit.secondary': '#ffffff', roster: [] },
      },
    ],
  });
  await tick();
  const step = [...page().querySelectorAll('.next .step')].find((s) => /Other grounds/.test(s.textContent));
  assert.match(step.textContent, /They finish too, and their results show in the report\./);
  assert.equal(step.closest('[data-stub]'), null);
});
