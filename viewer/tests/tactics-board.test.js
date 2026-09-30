// The Tactics board's words: role and duty tags, the duty colour tokens, and which card each
// team instruction sits in.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import {
  dutyTag,
  dutyToken,
  instructionRows,
  LIMITS,
  markerTag,
  roleTag,
  shapeLine,
  WEIGHTS,
} from '../src/lib/tactics-board.js';
import { REPO_ROOT } from './helpers.js';

const SCHEMA = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/tactics.json'), 'utf8'));

test('each role takes a short tag, and no two roles share one', () => {
  assert.equal(roleTag('central_defender'), 'CD');
  assert.equal(roleTag('ball_playing_defender'), 'BPD');
  assert.equal(roleTag('goalkeeper'), 'GK');
  assert.equal(roleTag('winger'), 'W');
  const tags = SCHEMA.roles.map((r) => roleTag(r.name));
  assert.equal(new Set(tags).size, tags.length, tags.join(' '));
});

test('a duty takes two letters and its own colour token', () => {
  assert.deepEqual(
    SCHEMA.duties.map((d) => dutyTag(d.name)),
    ['De', 'Su', 'At']
  );
  assert.equal(dutyToken('attack'), '--duty-attack');
  assert.equal(dutyToken('unknown'), '--ink');
  const tag = markerTag(SCHEMA, { roles: [{ role: 1, duty: 0 }] }, 0);
  assert.deepEqual(tag, { tag: 'CD·De', duty: 'defend', token: '--duty-defend' });
});

test('every instruction sits in one card, in the engine order', () => {
  const t = [0, 1, 2, 0, 1, 1];
  const limits = instructionRows(SCHEMA, t, 'limits');
  const weights = instructionRows(SCHEMA, t, 'weights');
  assert.deepEqual(
    limits.map((r) => r.name),
    LIMITS.filter((n) => n in SCHEMA.instructions).sort(
      (a, b) => Object.keys(SCHEMA.instructions).indexOf(a) - Object.keys(SCHEMA.instructions).indexOf(b)
    )
  );
  assert.equal(limits.length + weights.length, Object.keys(SCHEMA.instructions).length);
  for (const name of WEIGHTS) {
    assert.ok(weights.some((r) => r.name === name), name);
  }
  const width = limits.find((r) => r.name === 'width');
  assert.equal(width.value, t[width.index]);
  assert.equal(width.label, 'Width');
  assert.ok(width.levels.length >= 2);
});

test('the shape line names the formation and the mentality', () => {
  assert.equal(
    shapeLine(SCHEMA, { formation: 2, mentality: 3 }),
    `Base shape ${SCHEMA.formations[2].name} · Mentality ${SCHEMA.mentalities[3].name.charAt(0).toUpperCase()}${SCHEMA.mentalities[3].name.slice(1)}`
  );
  assert.equal(shapeLine(null, null), '');
});
