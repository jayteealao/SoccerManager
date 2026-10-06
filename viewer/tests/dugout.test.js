// The dugout: the commands the viewer sends before kick-off and during play, and how each chip
// moves only on the engine's word. A fake `send` records the commands; the test answers them.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { beforeEach, test } from 'vitest';

import { Dugout } from '../src/lib/dugout.svelte.js';
import { clearSignals, signals } from '../src/lib/signal.js';
import {
  pickerBlock,
  pickerPlayers,
  remaining,
  remainingText,
} from '../src/lib/substitution-picker.js';
import { detailFor } from '../src/lib/tactics-panel.js';
import { REPO_ROOT } from './helpers.js';

const SCHEMA = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/tactics.json'), 'utf8'));

const POSITIONS = [
  'GK', 'LB', 'CB', 'CB', 'RB', 'LW', 'CM', 'CM', 'RW', 'ST', 'ST',
  'GK', 'CB', 'LB', 'RB', 'DM', 'CM', 'AM', 'LW', 'RW', 'ST', 'ST',
];

function squad(team) {
  return POSITIONS.map((position, i) => ({
    'player.id': `p-${team}-${i}`,
    'player.name': `Player ${team}-${i}`,
    'player.shirt': i + 1,
    'player.position': position,
    'player.natural_fitness': 120,
    'player.injury_resistance': 120,
    role_fit: SCHEMA.roles.map(() => 100),
  }));
}

const SETUP = {
  lineup: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
  bench: [11, 12, 13, 14, 15, 16, 17],
  formation: 0,
  mentality: 2,
  instructions: [1, 1, 1, 1, 1, 0],
  roles: Array.from({ length: 11 }, () => ({ role: 0, duty: 1 })),
};

function helloWith(setup = SETUP) {
  const team = (n, id) => ({
    'team.id': id,
    'team.name': id,
    squad: squad(n),
    setup: setup ? { ...setup } : undefined,
    roster: squad(n)
      .slice(0, 18)
      .map((p, i) => ({ ...p, 'player.squad_index': i })),
  });
  return {
    type: 'hello',
    tactics: SCHEMA,
    substitutions: { limit: 5, windows: 3 },
    teams: [team(0, 'club-a'), team(1, 'club-b')],
  };
}

const VIEW = () => ({
  substitutions: [],
  energy: null,
  cards: new Map(),
  sentOff: new Set(),
  injuries: new Set(),
  fullTime: false,
});

/// A dugout whose `send` records every command; `started` kicks it off and answers the ack.
function dugout({ refuse = false } = {}) {
  const sent = [];
  const starts = [];
  const d = new Dugout({
    send: (c) => (refuse ? false : (sent.push(c), true)),
    onStart: (message, patch) => starts.push({ message, patch }),
  });
  return { d, sent, starts };
}

function live() {
  const out = dugout();
  out.d.begin(helloWith());
  out.d.kickOff();
  out.d.answer({ type: 'ack', command: 'set-lineup' });
  out.d.update(10, VIEW());
  out.sent.length = 0;
  return out;
}

/// Queues a mentality change and acknowledges it as `queueId`.
function queued(d, queueId, mentality = 3) {
  d.editTactics({ mentality });
  d.answer({ type: 'ack', command: 'queue-change', 'change.queue_id': queueId });
}

beforeEach(() => clearSignals());

test('kick-off sends the lineup alone when the draft is the computer pick, and starts on the ack', () => {
  const { d, sent, starts } = dugout();
  d.begin(helloWith());
  assert.equal(d.phase, 'pre-match');
  assert.equal(d.kickOff(), true);
  assert.equal(d.phase, 'kicking-off');
  assert.deepEqual(sent, [
    { type: 'set-lineup', lineup: SETUP.lineup, bench: SETUP.bench },
  ]);
  assert.equal(starts.length, 0, 'nothing starts before the engine answers');
  d.answer({ type: 'ack', command: 'set-lineup' });
  assert.equal(d.phase, 'live');
  assert.equal(starts.length, 1);
  assert.equal(starts[0].patch, null);
  assert.ok(signals().some((s) => s.signal === 'viewer.kick_off'));
});

test('a changed draft adds its patch to the lineup, and edits stop once kick-off is asked', () => {
  const { d, sent } = dugout();
  d.begin(helloWith());
  d.editTactics({ mentality: 4 });
  d.lineupAction((e) => e.drop(18, 9));
  d.kickOff();
  assert.equal(sent[0].lineup[9], 18);
  assert.equal(sent[0].patch.mentality, 4);
  d.lineupAction((e) => e.drop(19, 10));
  assert.equal(d.editor.slots[10], 10, 'the lineup is fixed while kick-off waits');
});

test('a refused lineup returns to the editor with the engine reason', () => {
  const { d } = dugout();
  d.begin(helloWith());
  d.kickOff();
  d.answer({ type: 'reject', command: 'set-lineup', reason: 'slot 0 needs a goalkeeper' });
  assert.equal(d.phase, 'pre-match');
  assert.equal(d.editor.reason, 'slot 0 needs a goalkeeper');
  assert.equal(d.lineupView().kickOffDisabled, true);
});

test('no socket leaves the editor open and says so', () => {
  const { d } = dugout({ refuse: true });
  d.begin(helloWith());
  assert.equal(d.kickOff(), false);
  assert.equal(d.phase, 'pre-match');
  assert.equal(d.editor.reason, 'The engine is not connected.');
});

test('a stored match and a hello with no setup take no change', () => {
  const stored = dugout();
  stored.d.begin(helloWith(), { stored: true });
  assert.equal(stored.d.phase, 'stored');
  assert.equal(stored.d.kickOff(), false);
  const bare = dugout();
  bare.d.begin(helloWith(null));
  assert.equal(bare.d.phase, 'stored');
  bare.d.editTactics({ mentality: 3 });
  assert.equal(bare.sent.length, 0);
});

test('a live tactics edit queues the detail the former page sent, and the chip waits for the ack', () => {
  const { d, sent } = live();
  d.editTactics({ instruction: 2, level: 0 });
  assert.deepEqual(sent, [
    { type: 'queue-change', 'change.kind': 'tactics', detail: detailFor({ instruction: 2, level: 0 }) },
  ]);
  assert.equal(d.chips.length, 0, 'no chip before the engine acknowledges');
  d.answer({ type: 'ack', command: 'queue-change', 'change.queue_id': 'q-9-0' });
  assert.equal(d.chips.length, 1);
  assert.equal(d.chips[0].state, 'queued');
  assert.equal(d.chips[0].editable, true);
});

test('the picker queues a substitution as off and on squad indices', () => {
  const { d, sent } = live();
  assert.equal(d.picker.left, 5);
  assert.equal(d.picker.block, null);
  d.substitute(9, 11);
  assert.deepEqual(sent, [
    { type: 'queue-change', 'change.kind': 'substitution', detail: { off: 9, on: 11 } },
  ]);
  d.answer({ type: 'ack', command: 'queue-change', 'change.queue_id': 'q-9-0' });
  assert.equal(d.chips[0].label, 'Substitution: Player 0-9 off, Player 0-11 on');
});

test('Cancel withdraws a chip on the ack and keeps it, with the reason, on a refusal', () => {
  const { d, sent } = live();
  queued(d, 'q-9-0');
  sent.length = 0;
  assert.equal(d.cancel('q-9-0'), true);
  assert.deepEqual(sent, [{ type: 'cancel-change', 'change.queue_id': 'q-9-0' }]);
  assert.equal(d.chips.length, 1, 'the chip stays until the engine answers');
  d.answer({ type: 'ack', command: 'cancel-change', 'change.queue_id': 'q-9-0' });
  assert.equal(d.chips.length, 0);
  assert.equal(d.pending.all(10)[0].cancelled, true);

  queued(d, 'q-9-1', 1);
  d.cancel('q-9-1');
  d.answer({ type: 'reject', command: 'cancel-change', reason: 'change q-9-1 has already applied' });
  assert.equal(d.chips.length, 1);
  assert.equal(d.cancelRefused, 'change q-9-1 has already applied');
});

test('Edit sends the withdrawal, then the edited change only on its ack', () => {
  const { d, sent } = live();
  queued(d, 'q-9-0');
  sent.length = 0;
  d.startEdit('q-9-0');
  assert.equal(d.editing.queue_id, 'q-9-0');
  d.editTactics({ mentality: 0 });
  assert.deepEqual(sent, [{ type: 'cancel-change', 'change.queue_id': 'q-9-0' }]);
  d.answer({ type: 'ack', command: 'cancel-change', 'change.queue_id': 'q-9-0' });
  assert.equal(d.editing, null);
  assert.deepEqual(sent[1], {
    type: 'queue-change',
    'change.kind': 'tactics',
    detail: detailFor({ mentality: 0 }),
  });
});

test('the engine turns a chip to Applies now, and it can no longer be withdrawn', () => {
  const { d, sent } = live();
  queued(d, 'q-9-0');
  sent.length = 0;
  d.onChangeState({ type: 'change-state', 'change.queue_id': 'q-9-0', state: 'applies-now', tick: 40 });
  assert.equal(d.chips[0].state, 'applies-now');
  assert.equal(d.chips[0].editable, false);
  assert.equal(d.cancel('q-9-0'), false);
  assert.equal(sent.length, 0);
});

test('full time takes no further change', () => {
  const { d, sent } = live();
  d.update(20, { ...VIEW(), fullTime: true });
  assert.equal(d.live, false);
  d.editTactics({ mentality: 3 });
  d.substitute(9, 11);
  assert.equal(sent.length, 0);
  assert.equal(d.picker.block, 'Changes need a live match.');
});

test('the picker words: the count left, and why it cannot queue', () => {
  assert.equal(remaining(5, [{ team: 'a' }, { team: 'b' }, { team: 'a' }], 'a'), 3);
  assert.equal(remaining(1, [{ team: 'a' }, { team: 'a' }], 'a'), 0);
  assert.equal(remainingText(3, 5), '3 of 5 substitutions left');
  const players = pickerPlayers(
    [
      { squad: 1, shirt: 2, name: 'A', position: 'LB', sentOff: false },
      { squad: 2, shirt: 3, name: 'B', position: 'CB', sentOff: true },
    ],
    [{ 'player.squad_index': 11, 'player.shirt': 12, 'player.name': 'C', 'player.position': 'GK' }]
  );
  assert.deepEqual(players.off.map((p) => p.squad), [1], 'a player sent off cannot come off');
  assert.deepEqual(players.on.map((p) => p.squad), [11]);
  assert.equal(pickerBlock({ live: true, left: 0, ...players }), 'No substitutions left.');
  assert.equal(pickerBlock({ live: true, left: 2, off: [], on: players.on }), 'No player to change.');
  assert.equal(pickerBlock({ live: true, left: 2, ...players }), null);
});

// ---- The Touchline: the assistant's picks and a shape queued with a substitution -----------

test('no lineup is sent before KICK OFF, whatever the editor does', () => {
  const { d, sent } = dugout();
  d.begin(helloWith());
  d.lineupAction((e) => e.pickRow(3));
  d.setFormation(1);
  d.editTactics({ mentality: 3 });
  assert.deepEqual(sent, [], 'editing before kick-off sends nothing');
  d.kickOff();
  assert.equal(sent.length, 1);
  assert.equal(sent[0].type, 'set-lineup');
});

test('a pick shows once playback reaches its tick, and a newer advice replaces it', () => {
  const { d } = live();
  const pick = { code: 'sub-fatigue', kind: 'substitution', off: 7, on: 14 };
  d.onAdvice({ type: 'advice', tick: 100, minute: 56, picks: [pick] });
  assert.equal(d.advice, null, 'not before the pitch reaches tick 100');
  d.update(100, VIEW());
  assert.equal(d.proposals().length, 1);
  assert.equal(d.proposals()[0].reason, 'tired player');
  d.onAdvice({ type: 'advice', tick: 150, minute: 57, picks: [] });
  d.update(149, VIEW());
  assert.equal(d.proposals().length, 1);
  d.update(150, VIEW());
  assert.equal(d.proposals().length, 0, 'the newer advice has no pick open');
  d.update(120, VIEW());
  assert.equal(d.proposals().length, 1, 'a rewind shows the advice of its tick again');
});

test('Accept queues the pick’s own change and the pick reads Queued; a refusal frees it', () => {
  const { d, sent } = live();
  const pick = { code: 'sub-fatigue', kind: 'substitution', off: 7, on: 14 };
  d.onAdvice({ type: 'advice', tick: 5, minute: 56, picks: [pick] });
  d.update(10, VIEW());
  assert.equal(d.accept(pick), true);
  assert.deepEqual(sent, [
    { type: 'queue-change', 'change.kind': 'substitution', detail: { off: 7, on: 14 } },
  ]);
  assert.equal(d.proposals()[0].accepted, true);
  assert.equal(d.accept(pick), false, 'an accepted pick is not sent twice');
  d.answer({ type: 'ack', command: 'queue-change', 'change.queue_id': 'q-10-0' });
  assert.equal(d.chips[0].label, 'Substitution: Player 0-7 off, Player 0-14 on');
  const tactics = { code: 'mentality-up-trailing', kind: 'tactics', patch: { mentality: 3 } };
  d.onAdvice({ type: 'advice', tick: 6, minute: 70, picks: [tactics] });
  d.update(10, VIEW());
  sent.length = 0;
  d.accept(tactics);
  assert.deepEqual(sent, [
    { type: 'queue-change', 'change.kind': 'tactics', detail: { patch: { mentality: 3 } } },
  ]);
  d.answer({ type: 'reject', command: 'queue-change', reason: 'no' });
  assert.equal(d.proposals()[0].accepted, false, 'a refused pick can be accepted again');
});

test('a substitution with a new shape sends the substitution, then the shape', () => {
  const { d, sent } = live();
  const off = d.picker.off[3].squad;
  const on = d.picker.on[0].squad;
  d.substitute(off, on, 1);
  assert.deepEqual(sent, [
    { type: 'queue-change', 'change.kind': 'substitution', detail: { off, on } },
    { type: 'queue-change', 'change.kind': 'tactics', detail: { patch: { formation: 1 } } },
  ]);
  sent.length = 0;
  d.substitute(off, on, d.tactics.formation);
  assert.equal(sent.length, 1, 'keeping the shape queues the substitution alone');
});
