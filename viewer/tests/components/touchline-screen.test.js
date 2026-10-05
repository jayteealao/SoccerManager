// @vitest-environment jsdom
// The Touchline, in jsdom: the queue shows each change with its words and its actions; a
// substitution with a new shape queues both, in that order; the assistant's pick shows at
// its tick and Accept queues it; with no pick open the block says so; with no substitute
// left the picker says why; every stub is inert and hidden.

import assert from 'node:assert/strict';
import { tick } from 'svelte';
import { test } from 'vitest';

import { HELLO, VIEW, opened, stubFaults, useFakes } from './harness.js';

useFakes();

const page = () => document.querySelector('[data-screen="touchline"]')?.closest('.app') ?? null;
const button = (text) => [...page().querySelectorAll('button')].find((b) => b.textContent.trim() === text);

/// Kicks off, lets the engine accept the lineup, and opens the Touchline during play.
async function touchline(hello = HELLO) {
  const run = await opened(hello);
  run.s.act();
  run.s.act();
  run.socket.deliver({ type: 'ack', command: 'set-lineup' });
  await tick();
  run.s.dugout.update(10, VIEW);
  run.s.show('touchline');
  await tick();
  return run;
}

test('the Touchline shows its strip, the player state and the other bench during play', async () => {
  const { s } = await touchline();
  assert.equal(s.view, 'touchline');
  const root = page();
  const strip = root.querySelector('.strip').textContent;
  assert.match(strip, /Ball in play|Play stopped/);
  assert.match(strip, /0 of 5/);
  assert.equal(root.querySelectorAll('section[aria-label="Player state"] tbody tr').length, 11);
  assert.match(root.querySelector('section[aria-label="Port Varrow bench"]').textContent, /Their substitutes used/);
});

test('a substitution with a new shape queues both, the substitution first, and each chip has its words and actions', async () => {
  const { socket } = await touchline();
  const root = page();
  const select = [...root.querySelectorAll('label')].find((l) => l.textContent.trim().startsWith('New shape')).querySelector('select');
  assert.match(select.options[0].textContent, /^Keep /);
  select.value = select.options[1].value;
  select.dispatchEvent(new Event('change', { bubbles: true }));
  await tick();
  button('Queue sub').click();
  await tick();
  const queued = socket.sent.filter((m) => m.type === 'queue-change');
  assert.deepEqual(queued.map((m) => m['change.kind']), ['substitution', 'tactics']);
  assert.equal(queued[1].detail.patch.formation, Number(select.options[1].value));
  socket.deliver({ type: 'ack', command: 'queue-change', 'change.queue_id': 'q-1' });
  socket.deliver({ type: 'ack', command: 'queue-change', 'change.queue_id': 'q-2' });
  await tick();
  assert.match(root.querySelector('[data-queue-id="q-1"]').textContent, /Substitution:/);
  assert.match(root.querySelector('[data-queue-id="q-2"]').textContent, /Shape: /);
  assert.ok(root.querySelector('button[aria-label^="Cancel: Substitution"]'));
  assert.ok(root.querySelector('button[aria-label^="Edit: Substitution"]'));
});

test("the assistant's pick shows at its tick; Accept queues it and it reads Queued", async () => {
  const { s, socket } = await touchline();
  assert.match(page().querySelector('section[aria-label="Assistant\'s proposals"]').textContent, /No pick open/);
  socket.deliver({ type: 'advice', tick: 20, minute: 0, picks: [{ code: 'sub-fatigue', kind: 'substitution', off: 7, on: 14 }] });
  await tick();
  assert.equal(page().querySelector('[data-pick]'), null, 'not before its tick');
  s.dugout.update(20, VIEW);
  await tick();
  const accept = page().querySelector('button[aria-label^="Accept: "]');
  assert.match(accept.getAttribute('aria-label'), /tired player$/);
  accept.click();
  await tick();
  assert.deepEqual(socket.sent.at(-1), { type: 'queue-change', 'change.kind': 'substitution', detail: { off: 7, on: 14 } });
  socket.deliver({ type: 'ack', command: 'queue-change', 'change.queue_id': 'q-3' });
  await tick();
  assert.match(page().querySelector('[data-pick]').textContent, /Queued ✓/);
  assert.match(page().querySelector('[data-queue-id="q-3"]').textContent, /Substitution: Player 0-8 off, Player 0-15 on/);
});

test('with no substitute left the picker is disabled and says why', async () => {
  const { s } = await touchline({ ...HELLO, substitutions: { limit: 0, windows: 3 } });
  assert.equal(button('Queue sub').disabled, true);
  assert.ok(s.dugout.picker.block, 'the picker names a reason');
  assert.equal(page().querySelector('p.g').textContent.trim(), s.dugout.picker.block);
});

test('every Touchline stub is inert and hidden, and a planted focusable stub fails the check', async () => {
  await touchline();
  const root = page();
  for (const note of ['shouts', 'game-state plans', 'analyst reads', 'what each change did', 'warming up', 'sprint reserve', 'strip fact: Concussion substitutes open']) {
    assert.ok(root.querySelector(`[data-stub="${note}"]`), note);
  }
  assert.deepEqual(stubFaults(root), []);
  root.querySelector('[data-stub="shouts"]').append(document.createElement('button'));
  assert.deepEqual(stubFaults(root), ['shouts: button takes focus']);
});
