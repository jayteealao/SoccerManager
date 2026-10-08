// Named views and their stores: add, remove, move and reset; a stored id this page does not
// know is dropped; both adapters read back what they saved; the launcher adapter sends the
// club's views and reads them by club; the live ratings merge with the stored ones by match.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { DEFAULT_COLUMNS } from '../src/lib/columns.js';
import {
  addColumn,
  adapterFor,
  cleanView,
  defaultView,
  freeName,
  launcherAdapter,
  memoryAdapter,
  mergeRatings,
  moveColumn,
  moveTo,
  readPart,
  removeColumn,
  resetView,
  sameView,
} from '../src/lib/views.js';

test('a view adds, removes and moves columns, and resets to the default', () => {
  let view = defaultView('Mine');
  view = addColumn(view, 'build');
  assert.equal(view.columns.at(-1), 'build');
  assert.equal(addColumn(view, 'build'), view, 'a shown column is not added twice');
  assert.equal(addColumn(view, 'morale'), view, 'a LATER column is never added');
  view = moveColumn(view, 'build', -1);
  assert.equal(view.columns.at(-2), 'build');
  view = moveTo(view, view.columns.indexOf('build'), 0);
  assert.equal(view.columns[0], 'build');
  assert.equal(moveColumn(view, 'build', -1).columns[0], 'build', 'kept inside the list');
  view = { ...view, sort: { column: 'cond', direction: 'down' } };
  view = removeColumn(view, 'cond');
  assert.ok(!view.columns.includes('cond'));
  assert.equal(view.sort, null, 'a sort on a removed column is cleared');
  const single = { name: 'One', columns: ['age'], sort: null };
  assert.equal(removeColumn(single, 'age'), single, 'a view keeps one column');
  const reset = resetView(view);
  assert.equal(reset.name, 'Mine');
  assert.deepEqual(reset.columns, [...DEFAULT_COLUMNS]);
  assert.ok(sameView(reset, defaultView('Other')));
  assert.equal(freeName([{ name: 'View 1' }, { name: 'Default' }]), 'View 2');
});

test('a stored view drops ids this page does not know and keeps the rest in order', () => {
  const view = cleanView({ name: 'Old', columns: ['age', 'retired', 'height', 'age'], sort: { column: 'retired', direction: 'down' } });
  assert.deepEqual(view.columns, ['age', 'height']);
  assert.equal(view.sort, null);
  assert.deepEqual(cleanView({ name: 'Empty', columns: ['retired'] }).columns, [...DEFAULT_COLUMNS]);
  const part = readPart({ active: 'Nope', views: [view, { ...view }], ratings: { 'p-1': [{ match: 'm', rating: 7 }, { bad: 1 }] } });
  assert.equal(part.views.length, 1, 'one view per name');
  assert.equal(part.active, 'Old', 'an unknown active view falls back to the first');
  assert.deepEqual(part.ratings['p-1'], [{ match: 'm', rating: 7 }]);
  const empty = readPart(null);
  assert.deepEqual(empty.views, [defaultView()]);
  assert.equal(empty.active, 'Default');
});

test('the memory adapter keeps views for the session and reads them back', async () => {
  const memory = memoryAdapter();
  assert.deepEqual(await memory.read('club-a'), { views: [], ratings: {} });
  const views = [defaultView('Mine')];
  assert.equal(await memory.save('club-a', { active: 'Mine', views }), true);
  const back = readPart(await memory.read('club-a'));
  assert.equal(back.active, 'Mine');
  assert.deepEqual(back.views, views);
  assert.deepEqual(await memory.read('club-b'), { views: [], ratings: {} });
  assert.equal(adapterFor({ launcher: false }).kind, 'memory');
  assert.equal(adapterFor(null).kind, 'memory');
  assert.equal(adapterFor({ launcher: true }).kind, 'launcher');
});

test('the launcher adapter reads by club and saves the club views', async () => {
  const store = new Map();
  const calls = [];
  const fetcher = async (url, init = {}) => {
    calls.push([url, init.method ?? 'GET']);
    if (init.method === 'POST') {
      const body = JSON.parse(init.body);
      store.set(body.club, { active: body.active, views: body.views, ratings: {} });
      return { ok: true, json: async () => ({ launcher: true }) };
    }
    const club = decodeURIComponent(url.split('club=')[1]);
    return { ok: true, json: async () => store.get(club) ?? { views: [], ratings: {} } };
  };
  const adapter = launcherAdapter(fetcher);
  const views = [defaultView('Before Kelder')];
  assert.equal(await adapter.save('club a', { active: 'Before Kelder', views }), true);
  const back = readPart(await adapter.read('club a'));
  assert.equal(back.active, 'Before Kelder');
  assert.deepEqual(calls, [
    ['engine/views', 'POST'],
    ['engine/views?club=club%20a', 'GET'],
  ]);
  const refused = launcherAdapter(async () => ({ ok: false, status: 400, text: async () => 'no' }));
  assert.equal(await refused.save('club a', { active: 'x', views }), false);
  assert.deepEqual(await refused.read('club a'), { views: [], ratings: {} });
});

test('the live ratings merge with the stored ones by match id, newest ten kept', () => {
  const stored = { 'p-1': Array.from({ length: 10 }, (_, i) => ({ match: `m-${i}`, rating: 6 + i / 10 })) };
  const merged = mergeRatings(stored, { match: 'm-new', ratings: [{ 'player.id': 'p-1', rating: 8.1 }, { 'player.id': 'p-2', rating: 6.4 }] });
  assert.equal(merged.get('p-1').length, 10);
  assert.equal(merged.get('p-1').at(-1).match, 'm-new');
  assert.equal(merged.get('p-1')[0].match, 'm-1');
  assert.deepEqual(merged.get('p-2'), [{ match: 'm-new', rating: 6.4 }]);
  // The launcher ingested the same match already: it is not counted twice.
  const again = mergeRatings({ 'p-2': [{ match: 'm-new', rating: 6.4 }] }, { match: 'm-new', ratings: [{ 'player.id': 'p-2', rating: 6.4 }] });
  assert.equal(again.get('p-2').length, 1);
  assert.equal(mergeRatings(stored, null).get('p-1').length, 10);
});
