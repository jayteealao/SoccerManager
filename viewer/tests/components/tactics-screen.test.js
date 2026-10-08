// @vitest-environment jsdom
// The Tactics screen in its two phases, in jsdom: before kick-off the squad list fills the
// right column and CONTINUE waits for a legal lineup, with the reason in words; during play
// the roles table, the picker and the queue show. Every live control is focusable and named,
// and every stub is inert, hidden and holds nothing that takes focus.

import assert from 'node:assert/strict';
import { tick } from 'svelte';
import { test } from 'vitest';

import { VIEW, opened, stubFaults, useFakes } from './harness.js';

useFakes();

/// The Tactics screen's own root: the match screen stays mounted, hidden, beside it.
const tactics = () => document.querySelector('[data-screen="tactics"]').closest('.app');
const action = () => tactics().querySelector('header button.cont');
const verdict = () => tactics().querySelector('[role="status"] .tagc').textContent;

test('before kick-off the Tactics screen opens with the squad list and eleven slot buttons', async () => {
  await opened();
  const root = tactics();
  assert.equal(root.querySelector('.t b').textContent, 'Tactics');
  assert.equal(root.querySelectorAll('button[data-slot]').length, 11);
  const slot = root.querySelector('button[data-slot="0"]');
  assert.match(
    slot.getAttribute('aria-label'),
    /^Slot 1, GK: Player 0-1, GK, role fit 10 Fair, fitness 14/
  );
  assert.equal(root.querySelectorAll('button[data-squad]').length, 22);
  assert.equal(verdict(), 'READY');
  assert.equal(action().textContent.trim(), 'Continue', 'Tactics leads on to the Pre-match line-ups');
  assert.equal(action().disabled, false);
  // The match screen stays mounted, hidden, so its canvas keeps drawing.
  assert.ok(document.querySelector('.view[hidden] canvas'));
});

test('a clash disables KICK OFF and names the reason; a swap by clicks clears it', async () => {
  const { s } = await opened();
  const root = tactics();
  root.querySelector('button[data-squad="12"]').click();
  await tick();
  assert.equal(root.querySelector('button[data-squad="12"]').getAttribute('aria-pressed'), 'true');
  root.querySelector('button[data-slot="3"]').click();
  await tick();
  assert.equal(verdict(), 'NOT READY');
  assert.match(root.querySelector('[role="status"] .why').textContent, /placed twice/);
  assert.equal(action().disabled, true);
  assert.equal(s.lineupReady, false);
  root.querySelector('button[data-squad="12"]').click();
  await tick();
  root.querySelector('button[data-squad="3"]').click();
  await tick();
  assert.equal(verdict(), 'READY');
  assert.equal(action().disabled, false);
});

test('the NOT READY reason is a link that scrolls to and focuses the clashing row', async () => {
  await opened();
  const root = tactics();
  root.querySelector('button[data-squad="12"]').click();
  await tick();
  root.querySelector('button[data-slot="3"]').click();
  await tick();
  const link = root.querySelector('[role="status"] button.why');
  assert.ok(link, 'the reason is a button');
  assert.match(link.textContent, /placed twice/);
  const row = root.querySelector('button[data-squad="12"]');
  const scrolled = [];
  row.scrollIntoView = (options) => scrolled.push(options);
  link.click();
  await tick();
  assert.equal(document.activeElement, row);
  assert.deepEqual(scrolled, [{ block: 'nearest' }]);
});

test('with an empty slot the NOT READY link focuses that empty place', async () => {
  await opened();
  const root = tactics();
  const slot = root.querySelector('button[data-slot="5"]');
  slot.dispatchEvent(new KeyboardEvent('keydown', { key: 'Delete', bubbles: true }));
  await tick();
  root.querySelector('[role="status"] button.why').click();
  await tick();
  assert.equal(document.activeElement.getAttribute('aria-label'), 'Empty place: slot 6, LW');
});

test('Delete on a slot empties it, and the empty place gets its own row', async () => {
  await opened();
  const root = tactics();
  const slot = root.querySelector('button[data-slot="5"]');
  slot.dispatchEvent(new KeyboardEvent('keydown', { key: 'Delete', bubbles: true }));
  await tick();
  assert.match(root.querySelector('[role="status"] .why').textContent, /Ten starters/);
  const names = [...root.querySelectorAll('button')].map((b) => b.getAttribute('aria-label'));
  assert.ok(names.includes('Empty place: slot 6, LW'), names.join(' | '));
});

test('during play the roles table, the picker and the queue show, and Cancel withdraws', async () => {
  const { s, socket } = await opened();
  s.act();
  assert.equal(s.view, 'prematch', 'CONTINUE opens the Pre-match line-ups');
  assert.deepEqual(socket.sent.map((m) => m.type), [], 'CONTINUE sends nothing');
  s.act();
  socket.deliver({ type: 'ack', command: 'set-lineup' });
  await tick();
  s.dugout.update(10, VIEW);
  s.show('tactics');
  await tick();
  assert.deepEqual(socket.sent.map((m) => m.type), ['set-lineup', 'seen', 'start'], s.dugout.phase);
  const root = tactics();
  assert.equal(root.querySelectorAll('button[data-slot]').length, 0, 'the lineup is fixed during play');
  assert.equal(root.querySelectorAll('select[aria-label$=": role"]').length, 11);
  const labels = [...root.querySelectorAll('label')].map((l) => l.textContent);
  assert.ok(labels.some((t) => t.includes('Coming off')));
  const queueButton = [...root.querySelectorAll('button')].find(
    (b) => b.textContent.trim() === 'Queue substitution'
  );
  assert.equal(queueButton.disabled, false);
  queueButton.click();
  await tick();
  const queued = socket.sent.at(-1);
  assert.equal(queued.type, 'queue-change');
  assert.equal(queued['change.kind'], 'substitution');
  socket.deliver({ type: 'ack', command: 'queue-change', 'change.queue_id': 'q-9-0' });
  await tick();
  const cancel = root.querySelector('button[aria-label^="Cancel: Substitution"]');
  assert.ok(cancel);
  cancel.click();
  assert.deepEqual(socket.sent.at(-1), { type: 'cancel-change', 'change.queue_id': 'q-9-0' });
  socket.deliver({ type: 'ack', command: 'cancel-change', 'change.queue_id': 'q-9-0' });
  await tick();
  assert.equal(root.querySelector('[data-queue-id="q-9-0"]'), null);
});

test('every Tactics stub is inert and hidden, and every live control is named', async () => {
  await opened();
  const root = tactics();
  for (const note of [
    'in-possession shape',
    'tactic cards',
    'per-player limits',
    'weight bars',
    'cannot play group',
    'tactic column',
    'sharpness column',
    'tab: roles',
  ]) {
    assert.ok(root.querySelector(`[data-stub="${note}"]`), note);
  }
  assert.deepEqual(stubFaults(root), []);
  for (const control of root.querySelectorAll('button, select')) {
    if (control.closest('[data-stub]')) {
      continue;
    }
    const name =
      control.getAttribute('aria-label') ||
      control.closest('label')?.textContent.trim() ||
      control.textContent.trim();
    assert.ok(name, control.outerHTML.slice(0, 120));
  }
});

test('the stub check fails when a Tactics stub holds something that takes focus', async () => {
  await opened();
  const root = tactics();
  const stub = root.querySelector('[data-stub="tactic cards"]');
  stub.append(document.createElement('button'));
  assert.deepEqual(stubFaults(root), ['tactic cards: button takes focus']);
});
