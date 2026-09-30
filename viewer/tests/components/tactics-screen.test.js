// @vitest-environment jsdom
// The Tactics screen in its two phases, in jsdom: before kick-off the squad list fills the
// right column and KICK OFF waits for a legal lineup, with the reason in words; during play
// the roles table, the picker and the queue show. Every live control is focusable and named,
// and every stub is inert, hidden and holds nothing that takes focus.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, test } from 'vitest';

import { MatchSession } from '../../src/lib/match-session.svelte.js';
import { clearSignals } from '../../src/lib/signal.js';
import MatchScreen from '../../src/screens/MatchScreen.svelte';
import { REPO_ROOT, roster } from '../helpers.js';

const SCHEMA = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/tactics.json'), 'utf8'));

class FakeSocket {
  static OPEN = 1;
  static made = [];
  constructor() {
    this.readyState = 1;
    this.sent = [];
    this.listeners = {};
    FakeSocket.made.push(this);
  }
  addEventListener(type, fn) {
    (this.listeners[type] ??= []).push(fn);
  }
  send(text) {
    this.sent.push(JSON.parse(text));
  }
  close() {}
  deliver(message) {
    for (const fn of this.listeners.message ?? []) {
      fn({ data: JSON.stringify(message) });
    }
  }
}

const POSITIONS = [
  'GK', 'LB', 'CB', 'CB', 'RB', 'LW', 'CM', 'CM', 'RW', 'ST', 'ST',
  'GK', 'CB', 'LB', 'RB', 'DM', 'CM', 'AM', 'LW', 'RW', 'ST', 'ST',
];

const squad = (team) =>
  POSITIONS.map((position, i) => ({
    'player.id': `p-${team}-${i + 1}`,
    'player.name': `Player ${team}-${i + 1}`,
    'player.shirt': i + 1,
    'player.position': position,
    'player.natural_fitness': 70,
    'player.injury_resistance': 60,
    role_fit: SCHEMA.roles.map(() => 50),
  }));

const SETUP = {
  lineup: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
  bench: [11, 12, 13, 14, 15, 16, 17],
  formation: 0,
  mentality: 2,
  instructions: [1, 1, 1, 1, 1, 0],
  roles: [0, 3, 1, 1, 3, 9, 6, 6, 9, 11, 11].map((role) => ({ role, duty: 1 })),
};

const HELLO = {
  type: 'hello',
  'match.id': 'm',
  'protocol.version': 3,
  'engine.version': '0.3.0',
  ticks_expected: 270000,
  tactics: SCHEMA,
  substitutions: { limit: 5, windows: 3 },
  teams: [
    {
      'team.id': 'a',
      'team.name': 'Ashford Rovers',
      'team.kit.primary': '#c8102e',
      'team.kit.secondary': '#000000',
      roster: roster(0),
      squad: squad(0),
      setup: SETUP,
    },
    {
      'team.id': 'b',
      'team.name': 'Port Varrow',
      'team.kit.primary': '#6a0dad',
      'team.kit.secondary': '#ff6a13',
      roster: roster(1),
    },
  ],
};

const VIEW = {
  substitutions: [],
  energy: null,
  cards: new Map(),
  sentOff: new Set(),
  injuries: new Set(),
  fullTime: false,
};

const RUNNING = { 'engine.state': 'running', 'socket.port': 7001, 'protocol.version': 3, launcher: true };

async function opened() {
  const s = new MatchSession({
    fetcher: async () => ({ ok: true, json: async () => RUNNING }),
    timers: { setTimeout: () => 0, clearTimeout: () => {} },
    raf: null,
    now: () => 0,
  });
  render(MatchScreen, { session: s });
  await s.start();
  const socket = FakeSocket.made.at(-1);
  socket.deliver(HELLO);
  await tick();
  return { s, socket };
}

/// Every stub is inert and hidden, and holds nothing that takes focus. Returns what breaks it.
function stubFaults(root) {
  const faults = [];
  for (const stub of root.querySelectorAll('[data-stub]')) {
    if (!stub.hasAttribute('inert')) {
      faults.push(`${stub.dataset.stub}: not inert`);
    }
    if (stub.getAttribute('aria-hidden') !== 'true') {
      faults.push(`${stub.dataset.stub}: not hidden`);
    }
    const focusable = stub.querySelector('a[href], button, input, select, textarea, [tabindex]');
    if (focusable) {
      faults.push(`${stub.dataset.stub}: ${focusable.tagName.toLowerCase()} takes focus`);
    }
  }
  return faults;
}

/// The Tactics screen's own root: the match screen stays mounted, hidden, beside it.
const tactics = () => document.querySelector('[data-screen="tactics"]').closest('.app');
const action = () => tactics().querySelector('header button.cont');
const verdict = () => tactics().querySelector('[role="status"] .tagc').textContent;

let realSocket;
let realInfo;
beforeEach(() => {
  realSocket = globalThis.WebSocket;
  realInfo = console.info;
  console.info = () => {};
  globalThis.WebSocket = FakeSocket;
  FakeSocket.made = [];
  clearSignals();
});
afterEach(() => {
  globalThis.WebSocket = realSocket;
  console.info = realInfo;
  document.body.innerHTML = '';
});

test('before kick-off the Tactics screen opens with the squad list and eleven slot buttons', async () => {
  await opened();
  const root = tactics();
  assert.equal(root.querySelector('.t b').textContent, 'Tactics');
  assert.equal(root.querySelectorAll('button[data-slot]').length, 11);
  const slot = root.querySelector('button[data-slot="0"]');
  assert.match(
    slot.getAttribute('aria-label'),
    /^Slot 1, GK: Player 0-1, GK, role fit 50 Fair, fitness 70/
  );
  assert.equal(root.querySelectorAll('button[data-squad]').length, 22);
  assert.equal(verdict(), 'READY');
  assert.equal(action().textContent.trim(), 'Kick off');
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
