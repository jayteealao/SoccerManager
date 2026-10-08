// The squad table's column model: the default view, sorting (stable, words by their band,
// "not yet known" and missing values last, a second select reverses), the band thresholds, the
// attribute groups against the content file, and a scan that no 1-20 cell carries a decimal or
// a value over 20.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import {
  allColumns,
  ATTRIBUTE_GROUPS,
  choosable,
  DEFAULT_COLUMNS,
  nextSort,
  ratingFigures,
  shownColumns,
  sortRows,
  squadRows,
} from '../src/lib/columns.js';
import { bandOf } from '../src/lib/scale.js';
import { REPO_ROOT } from './helpers.js';

const ATTRIBUTES = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/attributes.json'), 'utf8'));

/// A protocol 6 squad entry.
function player(i, over = {}) {
  const attributes = {};
  for (const names of Object.values(ATTRIBUTE_GROUPS)) {
    for (const name of names) {
      attributes[name] = 100 + i * 10;
    }
  }
  return {
    'player.id': `p-${i}`,
    'player.name': `Player ${String.fromCharCode(65 + i)}`,
    'player.shirt': i + 1,
    'player.position': 'CM',
    'player.attributes': attributes,
    'player.height': 175 + i,
    'player.age': 20 + i,
    'player.nationality': 'ENG',
    'player.build': 'athletic',
    'player.condition': 100 - i,
    'player.consistency': { confidence: 'not_yet_known' },
    'player.injury_proneness': { confidence: 'not_yet_known' },
    'player.level': 100 + i * 10,
    'player.plays_between': [95 + i * 10, 100 + i * 10],
    ...over,
  };
}

test('the attribute groups match the content file, hidden values left out', () => {
  const visible = ATTRIBUTES.attributes.filter((a) => !a.hidden);
  const byGroup = {};
  for (const a of visible) {
    (byGroup[a.group] ??= []).push(a.name);
  }
  assert.deepEqual(byGroup, ATTRIBUTE_GROUPS);
  const ids = new Set(allColumns().map((c) => c.id));
  for (const a of ATTRIBUTES.attributes) {
    assert.equal(ids.has(`attr:${a.name}`), !a.hidden, a.name);
  }
});

test('the default view is board 1: No and Player, then its thirteen columns', () => {
  const shown = shownColumns({ columns: [...DEFAULT_COLUMNS] }).map((c) => c.id);
  assert.deepEqual(shown, ['no', 'player', ...DEFAULT_COLUMNS]);
  assert.equal(DEFAULT_COLUMNS.length, 13);
  // An id this page does not know is dropped, and a LATER column is never shown.
  const odd = shownColumns({ columns: ['age', 'retired-column', 'morale'] }).map((c) => c.id);
  assert.deepEqual(odd, ['no', 'player', 'age']);
  assert.ok(choosable().every((c) => c.built && !c.fixed));
});

test('the bands are 15, 11 and 7 on the shown whole number', () => {
  assert.equal(bandOf(15), 'v4');
  assert.equal(bandOf(14), 'v3');
  assert.equal(bandOf(11), 'v3');
  assert.equal(bandOf(10), 'v2');
  assert.equal(bandOf(7), 'v2');
  assert.equal(bandOf(6), 'v1');
  assert.equal(bandOf(1), 'v1');
  const pace = allColumns().find((c) => c.id === 'attr:pace');
  // 14.5 shows as 15 and takes the good band: the number and its colour never disagree.
  const cell = pace.cell({ 'player.attributes': { pace: 145 } });
  assert.deepEqual([cell.text, cell.band], ['15', 'v4']);
  assert.equal(pace.cell({ 'player.attributes': { pace: 144 } }).band, 'v3');
});

test('a sort is stable, reverses on a second select, and puts missing values last', () => {
  const rows = squadRows([player(0), player(1), player(2, { 'player.height': undefined }), player(3)]);
  const first = nextSort(null, 'height');
  assert.deepEqual(first, { column: 'height', direction: 'down' });
  assert.deepEqual(
    sortRows(rows, first, {}).map((r) => r.index),
    [3, 1, 0, 2]
  );
  const second = nextSort(first, 'height');
  assert.deepEqual(second, { column: 'height', direction: 'up' });
  assert.deepEqual(
    sortRows(rows, second, {}).map((r) => r.index),
    [0, 1, 3, 2],
    'missing still last'
  );
  // Text sorts A to Z first.
  assert.deepEqual(nextSort(null, 'player'), { column: 'player', direction: 'up' });
  // Equal values keep squad order.
  const same = squadRows([player(0), player(1), player(2)].map((p) => ({ ...p, 'player.nationality': 'ESP' })));
  assert.deepEqual(sortRows(same, { column: 'nat', direction: 'up' }, {}).map((r) => r.index), [0, 1, 2]);
  // No sort: squad order.
  assert.deepEqual(sortRows(rows, null, {}).map((r) => r.index), [0, 1, 2, 3]);
});

test('hidden words sort by their band, best first, and not yet known last', () => {
  const words = ['steady', null, 'erratic', 'rarely_off'];
  const squad = words.map((word, i) =>
    player(i, {
      'player.consistency': word ? { word, confidence: 'firm' } : { confidence: 'not_yet_known' },
      'player.matches_at_club': word ? 30 : 0,
    })
  );
  const down = sortRows(squadRows(squad), { column: 'consistency', direction: 'down' }, {});
  assert.deepEqual(down.map((r) => r.index), [3, 0, 2, 1]);
  const up = sortRows(squadRows(squad), { column: 'consistency', direction: 'up' }, {});
  assert.deepEqual(up.map((r) => r.index), [2, 0, 3, 1]);
  const cell = allColumns().find((c) => c.id === 'consistency').cell(squad[1]);
  assert.equal(cell.text, 'Not yet known');
  assert.ok(cell.muted);
});

test('the rating column averages the last 3 and 10 with one decimal, over what exists', () => {
  assert.equal(ratingFigures([]), null);
  const one = ratingFigures([{ match: 'm-1', rating: 7.4 }]);
  assert.deepEqual([one.last, one.avg3, one.avg10], [7.4, { value: 7.4, of: 1 }, { value: 7.4, of: 1 }]);
  const list = [6.0, 6.5, 7.0, 8.0, 7.5, 6.8, 7.2, 6.9, 7.1, 7.6, 8.2].map((rating, i) => ({ match: `m-${i}`, rating }));
  const many = ratingFigures(list);
  assert.equal(many.avg3.value, 7.6);
  assert.equal(many.avg3.of, 3);
  assert.equal(many.avg10.of, 10);
  const column = allColumns().find((c) => c.id === 'rating');
  const ctx = { ratings: new Map([['p-0', list]]) };
  const cell = column.cell(player(0), ctx);
  assert.deepEqual(cell.chips.map((c) => c.text), ['7.6', '7.3']);
  assert.equal(column.cell(player(1), ctx).text, '—');
});

test('no 1-20 cell, title or label carries a decimal or a value over 20', () => {
  const squad = [player(0, { 'player.attributes': { pace: 200, passing: 2, technique: 147 } }), player(9), player(1)];
  const scale = allColumns().filter((c) => c.kind === 'scale');
  assert.ok(scale.length > 30);
  for (const p of squad) {
    for (const column of scale) {
      const cell = column.cell(p);
      for (const text of [cell.text, cell.label ?? '']) {
        assert.ok(!/\d\.\d/.test(text), `${column.id}: ${text}`);
        for (const n of text.match(/\d+/g) ?? []) {
          assert.ok(Number(n) >= 1 && Number(n) <= 20, `${column.id}: ${text}`);
        }
      }
    }
  }
});
