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

/// A fit of `base` to every role, with `fits` naming the exceptions by role name.
function fitsWith(fits, base = 20) {
  const out = ROLES.map(() => base);
  for (const [name, fit] of Object.entries(fits)) {
    out[role(name)] = fit;
  }
  return out;
}

test('a position shows at the threshold and not one point under it', () => {
  assert.equal(POSITION_FIT, 70);
  const at = positionsFor(fitsWith({ striker: POSITION_FIT }), ROLES, 'CB');
  assert.deepEqual(at, ['CB', 'ST']);
  const under = positionsFor(fitsWith({ striker: POSITION_FIT - 1 }), ROLES, 'CB');
  assert.deepEqual(under, ['CB']);
});

test('the team-file position comes first even below the threshold, then the best fit', () => {
  const fits = fitsWith({ central_midfielder: 72, defensive_midfielder: 88 });
  // central_midfielder suits CM, DM and AM (72); defensive_midfielder suits DM and CM (88).
  assert.deepEqual(positionsFor(fits, ROLES, 'LB'), ['LB', 'DM', 'CM']);
  assert.equal(positionsFor(fits, ROLES, 'LB').length, MOST_CHIPS);
});

test('a position takes its best role, and ties keep the team-sheet order', () => {
  const fits = fitsWith({ full_back: 80 });
  // full_back suits LB and RB at 80 each: team-sheet order, left before right.
  assert.deepEqual(positionsFor(fits, ROLES, 'GK'), ['GK', 'LB', 'RB']);
});

test('a player who suits only his own position has one chip', () => {
  const fits = fitsWith({ goalkeeper: 95 });
  assert.deepEqual(positionsFor(fits, ROLES, 'GK'), ['GK']);
  assert.deepEqual(positionsFor([], ROLES, 'ST'), ['ST']);
});
