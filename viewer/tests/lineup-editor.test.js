// The lineup editor's state: one squad list with every player's place, swaps by click and by
// drag between any two rows, and the reason a clash gives.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import { LineupEditor, slotPlace, uprightPlace } from '../src/lib/lineup-editor.js';
import { REPO_ROOT } from './helpers.js';

const SCHEMA = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/tactics.json'), 'utf8'));

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
    'player.natural_fitness': 80 + 2 * i,
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

const editor = () => new LineupEditor({ schema: SCHEMA, squad: squad(), setup: SETUP, benchSize: 7 });

test('the squad list holds every player once, in the eleven, the bench, then not picked', () => {
  const rows = editor().squadRows();
  assert.deepEqual(
    rows.eleven.map((r) => r.index),
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
  );
  assert.deepEqual(
    rows.bench.map((r) => r.index),
    [11, 12, 13, 14, 15, 16, 17]
  );
  assert.deepEqual(
    rows.out.map((r) => r.index),
    [18, 19, 20, 21]
  );
  const keeper = rows.eleven[0];
  assert.equal(keeper.chip, 'GK');
  assert.equal(keeper.chipKind, 'eleven');
  assert.equal(rows.bench[0].chip, 'S1');
  assert.equal(rows.bench[0].chipKind, 'bench');
  assert.equal(rows.out[0].chip, '—');
  assert.equal(rows.out[0].chipKind, 'none');
  // 8.0 in tenths shows as 8.
  assert.equal(keeper.fitness, 8);
  assert.equal(keeper.fitnessWord, 'Low');
  assert.match(keeper.label, /^1 Player 0, GK, in the eleven at GK, fitness 8 Low$/);
  assert.match(rows.out[0].label, /not picked/);
});

test('two rows clicked in turn swap places, eleven with bench and bench with not picked', () => {
  const e = editor();
  e.pickRow(6);
  assert.equal(e.selection.kind, 'squad');
  assert.deepEqual(e.selection.place, { kind: 'slot', n: 6 });
  e.pickRow(16);
  assert.equal(e.selection, null);
  assert.equal(e.slots[6], 16);
  assert.equal(e.bench[5], 6);
  e.pickRow(12);
  e.pickRow(20);
  assert.equal(e.bench[1], 20, 'a player not picked takes the bench place');
  assert.ok(!e.bench.includes(12), 'and the benched player is no longer picked');
  assert.equal(e.verdict().legal, true);
});

test('a drag from any row onto any row is the same swap, and onto a slot places the player', () => {
  const e = editor();
  e.drop(19, 9);
  assert.equal(e.slots[9], 19);
  assert.ok(!e.slots.includes(9) && !e.bench.includes(9));
  e.drop(21, { kind: 'slot', n: 10 });
  assert.equal(e.slots[10], 21);
  e.drop(3, 3);
  assert.equal(e.slots[3], 3, 'a row dropped on itself changes nothing');
});

test('a player named twice is a clash: its chip shows both places and the reason names him', () => {
  const e = editor();
  e.pickPlace({ kind: 'bench', n: 6 });
  e.pickRow(12);
  const rows = e.squadRows();
  const clash = rows.bench.find((r) => r.index === 12);
  assert.equal(clash.clash, true);
  assert.equal(clash.chipKind, 'clash');
  assert.equal(clash.chip, 'S2+S7');
  assert.match(clash.label, /named twice/);
  assert.equal(rows.bench.filter((r) => r.index === 12).length, 1, 'one row per player');
  assert.deepEqual(e.verdict(), { legal: false, reason: 'Player 12 is placed twice.' });
  assert.equal(e.ready, false);
});

test('Empty the picked slot empties the picked place or the picked row, and the reason follows', () => {
  const e = editor();
  assert.equal(e.canEmpty, false);
  e.pickRow(4);
  assert.equal(e.pickedText(), 'Player 4 · RB picked.');
  assert.equal(e.canEmpty, true);
  e.emptyPicked();
  assert.equal(e.slots[4], null);
  assert.equal(e.reason, 'Ten starters; a match needs eleven.');
  e.pickPlace({ kind: 'bench', n: 0 });
  e.emptyPicked();
  assert.equal(e.bench[0], null);
  e.pickRow(20);
  assert.equal(e.canEmpty, false, 'a row not picked holds no place to empty');
});

test('two slots picked in turn swap, and a slot then a row places the row', () => {
  const e = editor();
  e.pickPlace({ kind: 'slot', n: 1 });
  e.pickPlace({ kind: 'slot', n: 4 });
  assert.deepEqual([e.slots[1], e.slots[4]], [4, 1]);
  e.pickPlace({ kind: 'slot', n: 0 });
  e.pickRow(11);
  assert.equal(e.slots[0], 11);
  assert.equal(e.verdict().legal, false, 'goalkeeper 11 now also sits on the bench');
});

test('an engine refusal shows until the lineup changes', () => {
  const e = editor();
  e.refused('slot 0 needs a goalkeeper');
  assert.equal(e.reason, 'slot 0 needs a goalkeeper');
  assert.equal(e.ready, false);
  e.drop(18, 17);
  assert.equal(e.reason, null);
  assert.equal(e.ready, true);
});

test('each slot button names its player, role fit and fitness, on an upright pitch', () => {
  const rows = editor().slotRows(() => 0);
  assert.equal(rows.length, 11);
  assert.match(rows[0].label, /^Slot 1, GK: Player 0, GK, role fit 10 Fair, fitness 8 Low$/);
  const spot = SCHEMA.formations[0].slots[0];
  const flat = slotPlace(spot.x, spot.y);
  assert.deepEqual(uprightPlace(spot.x, spot.y), { left: flat.top, top: 100 - flat.left });
  assert.ok(rows[0].place.top > 80, 'the goalkeeper stands at the bottom');
});

test('an emptied place keeps a row in the list, and a click or a drop fills it', () => {
  const e = editor();
  e.emptyPlace({ kind: 'slot', n: 3 });
  e.emptyPlace({ kind: 'bench', n: 2 });
  const list = e.squadList();
  const slot = list.eleven[3];
  assert.equal(slot.empty, true);
  assert.equal(slot.chip, 'CB');
  assert.equal(slot.label, 'Empty place: slot 4, CB');
  assert.equal(list.bench[2].label, 'Empty place: substitute 3');
  assert.deepEqual(
    list.out.map((r) => r.index),
    [3, 13, 18, 19, 20, 21]
  );
  e.pickPlace(slot.place);
  assert.equal(e.squadList().eleven[3].picked, true);
  e.pickRow(20);
  assert.equal(e.slots[3], 20);
  e.drop(3, { kind: 'bench', n: 2 });
  assert.equal(e.bench[2], 3);
  assert.equal(e.verdict().legal, true);
});
