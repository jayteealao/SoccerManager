// The attribute hover's copy against the attribute catalogue: the page's table equals the one
// derived from `content/attributes.json` by the hover rule (main and shown supports, mains
// first, catalogue action order; the job's action for an attribute no stage names); every
// visible attribute drives at least one action; every action has words; no minor weight is
// listed unless the same attribute is main or a shown support of that action.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import { ACTION_WORDS, ATTRIBUTE_ACTIONS, actionsOf } from '../src/lib/attribute-actions.js';
import { ATTRIBUTE_GROUPS } from '../src/lib/columns.js';
import { REPO_ROOT } from './helpers.js';

const CATALOGUE = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/attributes.json'), 'utf8'));
const VISIBLE = CATALOGUE.attributes.filter((a) => !a.hidden);
const stages = (action) => Object.values(CATALOGUE.actions[action]).filter((t) => t && typeof t === 'object' && t.main);

/// The hover rule over the catalogue file.
function derive(name) {
  const main = [];
  const support = [];
  for (const action of Object.keys(CATALOGUE.actions)) {
    const tables = stages(action);
    if (tables.some((t) => t.main.attribute === name)) {
      main.push([action, 'main']);
    } else if (tables.some((t) => (t.supports ?? []).some((s) => s.attribute === name))) {
      support.push([action, 'support']);
    }
  }
  if (main.length === 0 && support.length === 0) {
    main.push([CATALOGUE.attributes.find((a) => a.name === name).job.action, 'main']);
  }
  return [...main, ...support];
}

test('the hover copy equals the derivation from the attribute catalogue', () => {
  assert.deepEqual(Object.keys(ATTRIBUTE_ACTIONS), VISIBLE.map((a) => a.name));
  for (const { name } of VISIBLE) {
    assert.deepEqual(ATTRIBUTE_ACTIONS[name], derive(name), name);
  }
  // The panel shows every attribute of the groups; each has a hover.
  for (const name of Object.values(ATTRIBUTE_GROUPS).flat()) {
    assert.ok(actionsOf(name).length >= 1, name);
  }
});

test('every catalogue action has words, in catalogue order, with no digit', () => {
  assert.deepEqual(Object.keys(ACTION_WORDS), Object.keys(CATALOGUE.actions));
  for (const words of Object.values(ACTION_WORDS)) {
    assert.ok(words && !/\d/.test(words), words);
  }
});

test('a minor weight is never listed', () => {
  const minors = Object.entries(CATALOGUE.actions).flatMap(([action, table]) =>
    Object.values(table)
      .filter((t) => t && typeof t === 'object')
      .flatMap((t) => (t.minor ?? []).map((m) => ({ action, attribute: m.attribute })))
  );
  assert.ok(minors.length >= 5, `${minors.length} minor weights`);
  for (const { action, attribute } of minors) {
    const shown = stages(action).some(
      (t) => t.main.attribute === attribute || (t.supports ?? []).some((s) => s.attribute === attribute)
    );
    const listed = actionsOf(attribute).some((r) => r.action === action);
    assert.equal(listed, shown, `${attribute} on ${action}`);
  }
  assert.ok(!actionsOf('technique').some((r) => r.action === 'penalty'), 'technique is a minor weight on the penalty');
});

test('crossing lists the cross as main and its two set-piece deliveries as supports', () => {
  assert.deepEqual(actionsOf('crossing'), [
    { action: 'cross', words: 'Cross', role: 'main' },
    { action: 'corner', words: 'Corner delivery', role: 'support' },
    { action: 'free_kick', words: 'Free kick delivery', role: 'support' },
  ]);
  // Pace is in no stage table: it lists the action of its job.
  assert.deepEqual(actionsOf('pace'), [{ action: 'sprint', words: 'Sprint', role: 'main' }]);
  assert.deepEqual(actionsOf('no_such_attribute'), []);
});
