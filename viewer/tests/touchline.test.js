// The Touchline page: player state, the other bench, the assistant's picks and the strip,
// each at the rendered tick.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import { MatchState } from '../src/lib/match-state.js';
import {
  benchRows,
  NO_PICK,
  pickDetail,
  pickKey,
  playerStateRows,
  proposalRows,
  touchlineFacts,
} from '../src/lib/touchline.js';
import { REPO_ROOT } from './helpers.js';

const SCHEMA = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/tactics.json'), 'utf8'));

const row = (i, energy, extra = {}) => ({
  wire: i,
  id: `h-${i}`,
  name: `Home Player${i}`,
  shirt: i + 1,
  energy,
  band: energy >= 0.7 ? 'Fresh' : energy >= 0.4 ? 'Tiring' : 'Exhausted',
  token: '--good',
  condition: 'Fresh',
  sentOff: false,
  injured: false,
  ...extra,
});

test('player state shows the slow reserve as a whole percentage with its band, and each card with its minute', () => {
  const entries = [
    { 'event.type': 'card', 'player.id': 'h-1', 'card.kind': 'yellow', minute: 44 },
    { 'event.type': 'card', 'player.id': 'h-2', 'card.kind': 'yellow', minute: 30 },
    { 'event.type': 'card', 'player.id': 'h-2', 'card.kind': 'second-yellow', minute: 45, 'minute.added': 2 },
    { 'event.type': 'goal', 'player.id': 'h-3', minute: 50 },
  ];
  const rows = playerStateRows([row(0, 0.964), row(1, 0.381), row(2, 0.5, { sentOff: true })], entries);
  assert.equal(rows[0].reserve, 96);
  assert.equal(rows[0].band, 'Fresh');
  assert.equal(rows[0].card, null);
  assert.deepEqual(rows[1].card, { kind: 'yellow', minute: "44'", word: 'Yellow card' });
  assert.equal(rows[1].band, 'Exhausted');
  assert.deepEqual(rows[2].card, { kind: 'red', minute: "45+2'", word: 'Second yellow, sent off' });
  assert.ok(rows[2].sentOff);
});

test('the other bench lists who is left and what their manager used', () => {
  const state = new MatchState();
  state.add({ type: 'event', tick: 10, 'event.type': 'substitution', 'team.id': 'away', 'player.id': 'a-5', 'player.secondary_id': 'a-12', 'home.score': 0, 'away.score': 0 });
  state.add({ type: 'condition', tick: 50, energy: [], subs_used: [0, 1], windows_used: [0, 1] });
  const view = state.at(60);
  assert.deepEqual(view.windowsUsed, [0, 1]);
  const bench = [
    { 'player.id': 'a-13', 'player.name': 'Aaron Beck', 'player.shirt': 14, 'player.position': 'CM' },
  ];
  const out = benchRows(bench, view, 'away', 5);
  assert.equal(out.players[0].name, 'Aaron Beck');
  assert.equal(out.used, 1);
  assert.equal(out.text, '1 of 5 · 1 window');
  assert.equal(benchRows([], new MatchState().at(0), 'away', 5).text, '0 of 5', 'no windows before a condition');
});

const SQUAD = Array.from({ length: 20 }, (_, i) => ({ 'player.name': `Home Player${i}` }));

test('each pick reads as its change and its reason, and its label is the chip’s', () => {
  const picks = [
    { code: 'sub-fatigue', kind: 'substitution', off: 7, on: 18 },
    { code: 'sub-injury', kind: 'substitution', off: 3, on: 14 },
    { code: 'sub-keeper', kind: 'substitution', off: 5, on: 12 },
    { code: 'mentality-up-trailing', kind: 'tactics', patch: { mentality: 3, instructions: [2, null, null, null, null, null] } },
    { code: 'mentality-down-leading', kind: 'tactics', patch: { mentality: 1, instructions: [null, null, null, null, null, 1] } },
  ];
  const rows = proposalRows(picks, SQUAD, SCHEMA, new Set([pickKey(picks[0])]));
  assert.equal(rows[0].text, 'Player7 → Player18 · tired player');
  assert.equal(rows[0].label, 'Substitution: Home Player7 off, Home Player18 on', 'the chip names the players in full, as the picker does');
  assert.ok(rows[0].accepted);
  assert.ok(!rows[1].accepted);
  assert.equal(rows[1].reason, 'injured player');
  assert.equal(rows[2].reason, 'outfield player in goal');
  assert.equal(rows[3].text, 'Mentality Positive, Pressing high · trailing');
  assert.equal(rows[4].reason, 'leading');
  assert.match(rows[4].what, /^Mentality Cautious, Time wasting /);
  assert.deepEqual(rows.map((r) => r.n), [1, 2, 3, 4, 5]);
});

test('accepting a pick queues exactly the change the assistant named', () => {
  assert.deepEqual(pickDetail({ kind: 'substitution', off: 7, on: 18 }), { off: 7, on: 18 });
  const patch = { mentality: 3 };
  assert.deepEqual(pickDetail({ kind: 'tactics', patch }), { patch });
  assert.notEqual(pickKey({ code: 'a', kind: 'substitution', off: 1, on: 2 }), pickKey({ code: 'a', kind: 'substitution', off: 1, on: 3 }));
  assert.match(NO_PICK, /every 30 seconds/);
});

test('the strip names the ball state, the next stoppage and the counts', () => {
  const facts = touchlineFacts({
    score: [1, 1],
    clock: '60:00',
    play: { stopped: false, nextIn: 8.4 },
    subsUsed: 1,
    windowsUsed: 1,
    limit: 5,
    windows: 3,
    mentality: 'Balanced',
  });
  assert.equal(facts[0].value, 'Ball in play');
  assert.equal(facts[0].label, 'Next stoppage in 0:08 · queued changes apply there');
  assert.equal(facts[1].value, '1 – 1 · 60:00');
  assert.equal(facts[2].value, '1 of 5');
  assert.equal(facts[3].value, '1 of 3');
  assert.equal(facts[4].value, 'Balanced');
  const stopped = touchlineFacts({ score: [0, 0], clock: '10:00', play: { stopped: true, nextIn: null }, subsUsed: 0, windowsUsed: 0, limit: 5, windows: 3, mentality: 'Positive' });
  assert.equal(stopped[0].value, 'Play stopped');
  assert.equal(stopped[0].label, 'Queued changes apply at the next stoppage');
});
