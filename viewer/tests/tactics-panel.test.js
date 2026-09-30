// The tactics panel's pure half: the change detail the engine reads, the chip words, and the
// pre-match patch that kick-off sends.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import {
  applyPatch,
  detailFor,
  instructionNames,
  labelFor,
  prematchPatch,
  roleForSlot,
  tacticsOf,
} from '../src/lib/tactics-panel.js';
import { REPO_ROOT } from './helpers.js';

const SCHEMA = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/tactics.json'), 'utf8'));

const SETUP = {
  formation: 0,
  mentality: 2,
  instructions: [1, 1, 1, 1, 1, 0],
  roles: Array.from({ length: 11 }, (_, slot) => ({ role: slot === 0 ? 0 : 1, duty: 1 })),
};
const LINEUP = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

test('a mentality edit is a patch with the mentality alone', () => {
  assert.deepEqual(detailFor({ mentality: 4 }), { patch: { mentality: 4 } });
});

test('one instruction edit sets one level and leaves the other five null', () => {
  assert.deepEqual(detailFor({ instruction: 0, level: 2 }), {
    patch: { instructions: [2, null, null, null, null, null] },
  });
});

test('a role edit names the player by squad index', () => {
  assert.deepEqual(detailFor({ role: { squad: 14, role: 9, duty: 2 } }), {
    patch: { roles: [{ squad: 14, role: 9, duty: 2 }] },
  });
});

test('the instructions follow the tactics file and every chip names its change in words', () => {
  assert.deepEqual(instructionNames(SCHEMA), [
    'pressing',
    'width',
    'tempo',
    'line_height',
    'passing_directness',
    'time_wasting',
  ]);
  // The hello names the order, because a JSON object's keys carry none on the wire.
  const sorted = { ...SCHEMA, instruction_order: ['width', 'pressing'] };
  assert.deepEqual(instructionNames(sorted), ['width', 'pressing']);
  assert.equal(labelFor({ mentality: 4 }, SCHEMA), 'Mentality: Attacking');
  assert.equal(labelFor({ instruction: 3, level: 2 }, SCHEMA), 'Line height: High');
  assert.equal(
    labelFor({ role: { squad: 9, role: 11, duty: 2 } }, SCHEMA, () => 'Sam Ode'),
    'Sam Ode: Striker, Attack'
  );
});

test('a formation change keeps suitable roles and resets the rest, as the engine does', () => {
  assert.equal(roleForSlot(SCHEMA, 'CB', 1), 1);
  assert.equal(roleForSlot(SCHEMA, 'DM', 1), 5, 'the first role that suits a DM');
  const four33 = applyPatch(SETUP, { formation: 1 }, LINEUP, SCHEMA);
  assert.equal(four33.formation, 1);
  assert.equal(four33.roles[6].role, 5);
});

test('the pre-match patch carries only what changed, and nothing when nothing did', () => {
  const base = tacticsOf(SETUP);
  assert.equal(prematchPatch(base, tacticsOf(SETUP), LINEUP), null);
  const draft = tacticsOf(SETUP);
  draft.mentality = 3;
  draft.instructions[5] = 1;
  draft.roles[9] = { role: 12, duty: 2 };
  assert.deepEqual(prematchPatch(base, draft, LINEUP), {
    mentality: 3,
    instructions: [null, null, null, null, null, 1],
    roles: [{ squad: 9, role: 12, duty: 2 }],
  });
});
