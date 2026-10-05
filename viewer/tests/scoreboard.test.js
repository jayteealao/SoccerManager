// The score strip's words: the score, the fixture title, the crest capitals, and the scorers
// released at the rendered tick.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { fixtureTitle, initials, scoreText, scorerLines } from '../src/lib/scoreboard.js';
import { eventMessage, roster } from './helpers.js';

const TEAMS = [
  { 'team.id': 'club-a', 'team.name': 'Ashford Rovers', roster: roster(0) },
  { 'team.id': 'club-b', 'team.name': 'Port Varrow', roster: roster(1) },
];

test('the score and the fixture read as the strip and the header write them', () => {
  assert.equal(scoreText(1, 0), '1 – 0');
  assert.equal(fixtureTitle(null), 'Touchline');
  assert.equal(fixtureTitle(['Ashford Rovers', 'Port Varrow']), 'Ashford Rovers v Port Varrow');
  assert.equal(fixtureTitle(['Ashford Rovers', 'Port Varrow'], [1, 1]), 'Ashford Rovers 1–1 Port Varrow');
  assert.equal(initials('Ashford Rovers'), 'AR');
  assert.equal(initials('Dunmore FC of the North'), 'DFO');
});

test('each side lists its scorers by surname and minute, in order', () => {
  const goals = [
    eventMessage(69000, 'goal', { minute: 23, 'team.id': 'club-a', 'player.id': 'p-0-9' }),
    eventMessage(174000, 'goal', { minute: 58, 'team.id': 'club-b', 'player.id': 'p-1-11' }),
    eventMessage(180000, 'goal', { minute: 60, 'team.id': 'club-a', 'player.id': 'unknown' }),
  ];
  assert.deepEqual(scorerLines(goals, TEAMS), ["0-9 23', 60'", "1-11 58'"]);
  assert.deepEqual(scorerLines([], TEAMS), ['', '']);
  assert.deepEqual(scorerLines(goals, null), ['', '']);
});
