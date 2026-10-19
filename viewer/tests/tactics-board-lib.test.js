// The tactics board's pure half: every primitive's words and its one place on the Phases
// board, the out-of-possession card names, the style presets, the game plans and the live
// fold, against the content folder's own tactics and styles files.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import { BLOCK, CARDS, TOP_BAR, controlFor, placeOf, placements } from '../src/lib/board-cards.js';
import { blockWord, levelWord, PRIMITIVE_WORDS, primitiveWords, reachWords, reasonSentence, STYLE_WORDS, styleWords } from '../src/lib/board-words.js';
import { fold } from '../src/lib/board-live.js';
import { rowModel } from '../src/lib/board-model.js';
import {
  changeWords,
  conditionKinds,
  conditionWords,
  fullWords,
  maxPlans,
  moved,
  newCondition,
  planCount,
  planRows,
  ruleProblem,
  timeline,
} from '../src/lib/game-plans.js';
import { changesFrom, initialPick, presetGameState, presetLevels } from '../src/lib/presets.js';
import {
  defaultLevels,
  fromWire,
  levelNames,
  levelWire,
  merge,
  prematchLevels,
  primitiveIndex,
  primitivesOf,
  withLevel,
} from '../src/lib/primitives.js';
import { REPO_ROOT } from './helpers.js';

const SCHEMA = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/tactics.json'), 'utf8'));
const STYLES = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/styles.json'), 'utf8'));
/// The presets as the hello carries them: by name, the formations and the game plans.
const PRESETS = STYLES.styles.map((s) => ({
  name: s.name,
  primitives: s.primitives,
  formations: s.formations,
  rules: s.rules ?? [],
  player_instructions: s.player_instructions ?? [],
}));
const BANNED = ['High press', 'Mid block', 'Low block'];
/// An engine id: a lower-case word joined to another by an underscore.
const ENGINE_ID = /\b[a-z]+_[a-z_]+\b/;

test('the tactics file has primitives, and every one has its own words', () => {
  const all = primitivesOf(SCHEMA);
  assert.ok(all.length >= 20, `${all.length} primitives`);
  for (const p of all) {
    const names = levelNames(SCHEMA, p);
    assert.ok(PRIMITIVE_WORDS[p.name], `no words for ${p.name}`);
    const w = primitiveWords(p.name, names);
    assert.equal(w.levels.length, names.length, `${p.name}: one word per level`);
    for (const text of [w.label, w.full, ...w.levels]) {
      assert.doesNotMatch(text, ENGINE_ID, `${p.name}: "${text}" reads as an engine id`);
    }
  }
});

test('every primitive sits in exactly one place on the board', () => {
  const placed = placements().map((p) => p.primitive);
  for (const p of primitivesOf(SCHEMA)) {
    assert.equal(placed.filter((n) => n === p.name).length, 1, `${p.name} placed ${placed.filter((n) => n === p.name).length} times`);
  }
  for (const name of placed) {
    assert.ok(primitiveIndex(SCHEMA, name) >= 0, `the board places ${name}, which the tactics file lacks`);
  }
  assert.deepEqual(TOP_BAR.map((r) => r.primitive), ['mentality', 'time_wasting']);
  assert.deepEqual(BLOCK.map((r) => r.primitive), ['line_of_engagement', 'line_height']);
});

test('the out-of-possession cards are Pressing, Midfield, Defending the box and Ball lost', () => {
  assert.deepEqual(
    CARDS.filter((c) => c.side === 'out').map((c) => c.title),
    ['Pressing', 'Midfield', 'Defending the box', 'Ball lost']
  );
  assert.equal(CARDS.filter((c) => c.side === 'in').length, 4);
});

test('no card, row or style is named High press, Mid block or Low block', () => {
  const texts = [
    ...CARDS.flatMap((c) => [c.title, c.sub, ...c.rows.map((r) => r.sub ?? '')]),
    ...Object.values(PRIMITIVE_WORDS).flatMap((w) => [w.label, w.full, ...w.levels]),
    ...Object.values(STYLE_WORDS).map((s) => s.name),
    ...PRESETS.map((p) => styleWords(p.name).name),
  ];
  for (const text of texts) {
    for (const banned of BANNED) {
      assert.ok(!String(text).toLowerCase().includes(banned.toLowerCase()), `"${text}" holds "${banned}"`);
    }
  }
});

test('the block chip is read from where the team presses from', () => {
  assert.deepEqual([0, 1, 2, 3, 4].map(blockWord), ['Low', 'Low', 'Mid', 'High', 'High']);
});

test('every preset has player words', () => {
  for (const p of PRESETS) {
    assert.ok(STYLE_WORDS[p.name], `no words for ${p.name}`);
  }
});

test('a control is picked for every primitive and only from the known kinds', () => {
  for (const p of primitivesOf(SCHEMA)) {
    const control = controlFor(p, levelNames(SCHEMA, p).length);
    assert.ok(['pips', 'select', 'segments', 'stack', 'switch'].includes(control), `${p.name}: ${control}`);
    assert.equal(control, placeOf(p.name).control);
  }
});

test('a level goes to the wire by name and comes back', () => {
  primitivesOf(SCHEMA).forEach((p, i) => {
    const names = levelNames(SCHEMA, p);
    const last = names.length - 1;
    const wire = levelWire(SCHEMA, i, last);
    assert.deepEqual(wire, { primitive: p.name, level: names[last] });
    assert.deepEqual(fromWire(SCHEMA, wire), [i, last]);
  });
  assert.equal(fromWire(SCHEMA, { primitive: 'no_such', level: 'x' }), null);
  assert.equal(fromWire(SCHEMA, { primitive: 'tempo', level: 'no_such' }), null);
});

test('the mentality and the team instructions move in the tactics, the rest in the levels', () => {
  const levels = defaultLevels(SCHEMA);
  const tactics = { formation: 0, mentality: 2, instructions: [1, 1, 1, 1, 1, 0], roles: [] };
  const merged = merge(SCHEMA, tactics, []);
  assert.equal(merged.length, levels.length);
  const m = primitiveIndex(SCHEMA, 'mentality');
  const moved1 = withLevel(SCHEMA, tactics, merged, m, 3);
  assert.equal(moved1.tactics.mentality, 3);
  assert.equal(tactics.mentality, 2, 'the old tactics are not changed');
  const press = primitiveIndex(SCHEMA, 'line_of_engagement');
  const other = (merged[press] + 1) % levelNames(SCHEMA, primitivesOf(SCHEMA)[press]).length;
  const moved2 = withLevel(SCHEMA, tactics, merged, press, other);
  assert.deepEqual(moved2.tactics, tactics);
  assert.equal(moved2.levels[press], other);
  // Before kick-off only the settings ride the new field; the mentality rides the patch.
  assert.deepEqual(prematchLevels(SCHEMA, merged, moved1.levels), []);
  assert.deepEqual(prematchLevels(SCHEMA, merged, moved2.levels), [levelWire(SCHEMA, press, other)]);
});

test('a preset gives every level, the picker starts on an exact match, else Custom', () => {
  const first = PRESETS[0];
  const levels = presetLevels(SCHEMA, first);
  assert.deepEqual(changesFrom(SCHEMA, levels, first), []);
  assert.equal(initialPick(SCHEMA, levels, PRESETS), first.name);
  const tempo = primitiveIndex(SCHEMA, 'tempo');
  const changed = [...levels];
  changed[tempo] = (changed[tempo] + 2) % levelNames(SCHEMA, primitivesOf(SCHEMA)[tempo]).length;
  assert.deepEqual(changesFrom(SCHEMA, changed, first), [tempo]);
  // A control: a level no preset holds picks none.
  const odd = [...changed];
  assert.ok(initialPick(SCHEMA, odd, PRESETS) === null || changesFrom(SCHEMA, odd, PRESETS.find((p) => p.name === initialPick(SCHEMA, odd, PRESETS))).length === 0);
});

test('picking a preset takes its game plans and keeps the player instructions', () => {
  const current = { rules: [], instructions: [{ slot: 9, primitive: 'forward_runs', level: 'often' }] };
  const state = presetGameState(PRESETS[0], current);
  assert.deepEqual(state.rules, PRESETS[0].rules);
  assert.deepEqual(state.instructions, current.instructions);
  assert.notEqual(state.instructions, current.instructions, 'a copy, not the same list');
});

test('game plans read in player words, move, and stop at the tactics file limit', () => {
  const state = { rules: PRESETS[0].rules, instructions: [] };
  const rows = planRows(state, SCHEMA);
  assert.equal(rows.length, state.rules.length);
  for (const row of rows) {
    for (const text of [...row.when.map((w) => w.text), ...row.set.flatMap((s) => [s.label, s.word])]) {
      assert.doesNotMatch(text, ENGINE_ID, `"${text}" reads as an engine id`);
    }
  }
  assert.equal(conditionWords({ kind: 'score', state: 'trailing', by: 1 }, SCHEMA).text, 'Losing by 1');
  assert.equal(changeWords({ formation: '4-4-2' }, SCHEMA).waits, true);
  assert.equal(maxPlans(SCHEMA), SCHEMA.game_state?.max_rules ?? 6);
  assert.equal(planCount(state, SCHEMA), `${state.rules.length} of ${maxPlans(SCHEMA)}`);
  assert.match(fullWords(SCHEMA), /remove one to add another/);
  const two = [{ when: [], set: [] }, { when: [{ kind: 'x' }], set: [] }];
  assert.deepEqual(moved(two, 0, 1), [two[1], two[0]]);
  assert.equal(moved(two, 0, -1), two);
});

test('a plan needs a condition and a change, each once', () => {
  const score = newCondition('score', SCHEMA);
  const time = newCondition('time_left', SCHEMA);
  const set = [{ primitive: 'mentality', level: 'attacking' }];
  assert.equal(ruleProblem({ when: [], set }), 'Add a condition.');
  assert.equal(ruleProblem({ when: [score], set: [] }), 'Add a change.');
  assert.match(ruleProblem({ when: [score, score], set }), /once/);
  assert.match(ruleProblem({ when: [score], set: [...set, ...set] }), /once/);
  assert.equal(ruleProblem({ when: [score, time], set }), null);
  for (const kind of conditionKinds(SCHEMA)) {
    assert.equal(newCondition(kind, SCHEMA).kind, kind);
  }
});

test('the timeline puts a plan with a time condition on a lane from its minute', () => {
  const rules = [
    { when: [{ kind: 'score', state: 'trailing', by: 1 }, { kind: 'time_left', minutes: 20 }], set: [{ primitive: 'mentality', level: 'attacking' }] },
    { when: [{ kind: 'red_cards_ours', at_least: 1 }], set: [{ primitive: 'mentality', level: 'defensive' }] },
  ];
  const t = timeline({ rules, instructions: [] }, SCHEMA);
  assert.equal(t.lanes.length, 1);
  assert.equal(t.lanes[0].from, 70);
  assert.equal(t.anyTime.length, 1);
});

test('the board shows a level change only once the engine applied it', () => {
  const tactics = { formation: 0, mentality: 2, instructions: [1, 1, 1, 1, 1, 0], roles: [] };
  const levels = merge(SCHEMA, tactics, []);
  const start = { tactics, levels, gameState: { rules: [], instructions: [] } };
  const press = primitiveIndex(SCHEMA, 'line_of_engagement');
  const want = (levels[press] + 1) % levelNames(SCHEMA, primitivesOf(SCHEMA)[press]).length;
  const chip = (verdict) => ({
    kind: 'instructions',
    detail: { levels: [levelWire(SCHEMA, press, want)] },
    engine_verdict: verdict,
  });
  assert.equal(fold(SCHEMA, start, [chip(null)], null).levels[press], levels[press]);
  assert.equal(fold(SCHEMA, start, [chip('rejected')], null).levels[press], levels[press]);
  assert.equal(fold(SCHEMA, start, [chip('applied')], null).levels[press], want);
});

test('a row reads its words, its default and whether it moved from the style', () => {
  const levels = defaultLevels(SCHEMA);
  const row = rowModel(SCHEMA, 'tempo', { levels });
  assert.equal(row.primitive, 'tempo');
  assert.equal(row.changed, false);
  assert.doesNotMatch(row.label, ENGINE_ID);
});

test('reach words and reasons read as sentences', () => {
  assert.equal(reachWords(['ST', 'LW', 'RW']), 'wingers and strikers only');
  assert.equal(reasonSentence('the player instruction does not reach the position of slot 3'), 'That instruction does not suit his position.');
  assert.equal(reasonSentence('anything else'), 'The engine did not take it.');
  assert.equal(levelWord('no_such', 0, ['fast_one']), 'Fast one');
});
