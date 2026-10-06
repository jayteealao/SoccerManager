// The position chips: the team-file position first, then each position whose best role fit
// reaches the threshold.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import { MOST_CHIPS, POSITION_FIT, positionsFor } from '../src/lib/positions.js';
import { REPO_ROOT } from './helpers.js';

const SCHEMA = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/tactics.json'), 'utf8'));
const ROLES = SCHEMA.roles;
const role = (name) => ROLES.findIndex((r) => r.name === name);

/// A fit of `base` tenths to every role, with `fits` naming the exceptions by role name.
function fitsWith(fits, base = 40) {
  const out = ROLES.map(() => base);
  for (const [name, fit] of Object.entries(fits)) {
    out[role(name)] = fit;
  }
  return out;
}

test('a position shows at the threshold and not one point under it', () => {
  // The threshold is the whole number 1 to 20 a screen shows; fits arrive in tenths.
  assert.equal(POSITION_FIT, 14);
  const at = positionsFor(fitsWith({ striker: POSITION_FIT * 10 }), ROLES, 'CB');
  assert.deepEqual(at, ['CB', 'ST']);
  // 13.5 shows as 14, so it reaches the threshold; 13.4 shows as 13.
  assert.deepEqual(positionsFor(fitsWith({ striker: 135 }), ROLES, 'CB'), ['CB', 'ST']);
  assert.deepEqual(positionsFor(fitsWith({ striker: 134 }), ROLES, 'CB'), ['CB']);
  const under = positionsFor(fitsWith({ striker: (POSITION_FIT - 1) * 10 }), ROLES, 'CB');
  assert.deepEqual(under, ['CB']);
});

test('the team-file position comes first even below the threshold, then the best fit', () => {
  const fits = fitsWith({ central_midfielder: 144, defensive_midfielder: 176 });
  // central_midfielder suits CM, DM and AM (14.4); defensive_midfielder suits DM and CM (17.6).
  assert.deepEqual(positionsFor(fits, ROLES, 'LB'), ['LB', 'DM', 'CM']);
  assert.equal(positionsFor(fits, ROLES, 'LB').length, MOST_CHIPS);
});

test('a position takes its best role, and ties keep the team-sheet order', () => {
  const fits = fitsWith({ full_back: 160 });
  // full_back suits LB and RB at 16.0 each: team-sheet order, left before right.
  assert.deepEqual(positionsFor(fits, ROLES, 'GK'), ['GK', 'LB', 'RB']);
});

test('a player who suits only his own position has one chip', () => {
  const fits = fitsWith({ goalkeeper: 190 });
  assert.deepEqual(positionsFor(fits, ROLES, 'GK'), ['GK']);
  assert.deepEqual(positionsFor([], ROLES, 'ST'), ['ST']);
});
