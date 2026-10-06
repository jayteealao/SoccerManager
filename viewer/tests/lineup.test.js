// Lineup legality: every illegal case names its reason, and a legal lineup enables kick-off.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { checkLineup, fitWord, fitnessWord, lineupMessage } from '../src/lib/lineup.js';

/// A squad of 22 in the shipped team order: goalkeepers at 0 and 11.
function squad() {
  const positions = [
    'GK', 'LB', 'CB', 'CB', 'RB', 'LW', 'CM', 'CM', 'RW', 'ST', 'ST',
    'GK', 'CB', 'LB', 'RB', 'DM', 'CM', 'AM', 'LW', 'RW', 'ST', 'ST',
  ];
  return positions.map((position, i) => ({
    'player.id': `p-${i}`,
    'player.name': `Player ${i}`,
    'player.shirt': i + 1,
    'player.position': position,
    'player.natural_fitness': 140,
    role_fit: [100],
  }));
}

const ELEVEN = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

test('fewer than eleven starters is illegal and counts them', () => {
  const slots = [...ELEVEN];
  slots[4] = null;
  const verdict = checkLineup({ slots, bench: [], squad: squad(), benchSize: 7 });
  assert.deepEqual(verdict, { legal: false, reason: 'Ten starters; a match needs eleven.' });
  const one = checkLineup({
    slots: [0, ...Array(10).fill(null)],
    bench: [],
    squad: squad(),
    benchSize: 7,
  });
  assert.equal(one.reason, 'One starter; a match needs eleven.');
});

test('an outfield player in the goalkeeper slot is illegal', () => {
  const slots = [...ELEVEN];
  slots[0] = 12;
  const verdict = checkLineup({ slots, bench: [], squad: squad(), benchSize: 7 });
  assert.equal(verdict.legal, false);
  assert.equal(verdict.reason, "The goalkeeper's slot holds Player 12, who is not a goalkeeper.");
});

test('a player placed twice is illegal, on the pitch or on the bench', () => {
  const slots = [...ELEVEN];
  slots[10] = 9;
  const twice = checkLineup({ slots, bench: [], squad: squad(), benchSize: 7 });
  assert.deepEqual(twice, { legal: false, reason: 'Player 9 is placed twice.' });
  const benched = checkLineup({ slots: ELEVEN, bench: [11, 3], squad: squad(), benchSize: 7 });
  assert.equal(benched.reason, 'Player 3 is placed twice.');
});

test('a bench larger than its size is illegal', () => {
  const verdict = checkLineup({
    slots: ELEVEN,
    bench: [11, 12, 13, 14, 15, 16, 17, 18],
    squad: squad(),
    benchSize: 7,
  });
  assert.deepEqual(verdict, { legal: false, reason: 'Eight substitutes; the bench holds 7.' });
});

test('the reasons come in a fixed order: starters first', () => {
  const slots = [12, 1, 2, 3, 4, 5, 6, 7, 8, 9, null];
  const verdict = checkLineup({ slots, bench: [], squad: squad(), benchSize: 7 });
  assert.equal(verdict.reason, 'Ten starters; a match needs eleven.');
});

test('a legal lineup enables kick-off, with or without a bench', () => {
  const full = checkLineup({
    slots: ELEVEN,
    bench: [11, 12, 13, null, 15, 16, 17],
    squad: squad(),
    benchSize: 7,
  });
  assert.deepEqual(full, { legal: true, reason: null });
  const empty = checkLineup({ slots: ELEVEN, bench: [], squad: squad(), benchSize: 7 });
  assert.deepEqual(empty, { legal: true, reason: null });
});

test('the lineup message drops empty bench places and the word bands name every figure', () => {
  assert.deepEqual(lineupMessage(ELEVEN, [11, null, 13]), { lineup: ELEVEN, bench: [11, 13] });
  // The bands read the whole number 1 to 20 a screen shows.
  assert.deepEqual([20, 15, 14, 12, 11, 9, 8, 1].map(fitWord), [
    'Strong',
    'Strong',
    'Good',
    'Good',
    'Fair',
    'Fair',
    'Poor',
    'Poor',
  ]);
  assert.deepEqual([20, 14, 13, 10, 9, 1].map(fitnessWord), [
    'High',
    'High',
    'Medium',
    'Medium',
    'Low',
    'Low',
  ]);
});
