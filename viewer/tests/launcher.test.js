// The page's launcher client: a refusal hands its reason on, and a resumed match's earlier
// events come back as a list. A fake fetch stands in for the launcher.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { fetchEarlierEvents, newMatch, readViews, resume, saveViews } from '../src/lib/launcher.js';

function answering(status, body) {
  return async () => ({
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
    text: async () => body,
  });
}

test('a refused new match hands the launcher reason on and answers null', async () => {
  let reason = null;
  const answer = await newMatch(answering(400, 'a match is already running\n'), { home: 'a', away: 'b' }, (text) => {
    reason = text;
  });
  assert.equal(answer, null);
  assert.equal(reason, 'a match is already running');
});

test('a refused resume hands the reason on; one with no reason hands on an empty string', async () => {
  let reason = null;
  assert.equal(await resume(answering(400, 'no saved match yet'), (text) => (reason = text)), null);
  assert.equal(reason, 'no saved match yet');
  const bare = async () => ({ ok: false, status: 400, json: async () => null });
  assert.equal(await resume(bare, (text) => (reason = text)), null);
  assert.equal(reason, '');
});

test('the earlier events are the launcher list, and empty when nothing answers', async () => {
  const rows = [{ tick: 0, 'event.type': 'kick-off' }];
  assert.deepEqual(await fetchEarlierEvents(answering(200, rows)), rows);
  assert.deepEqual(await fetchEarlierEvents(answering(404, 'not found')), []);
  assert.deepEqual(
    await fetchEarlierEvents(async () => {
      throw new Error('offline');
    }),
    [],
  );
});

test('the squad views read by club and a refused save hands its reason on', async () => {
  const seen = [];
  const fetcher = async (url, init = {}) => {
    seen.push([url, init.method ?? 'GET', init.body ?? null]);
    return { ok: true, status: 200, json: async () => ({ active: 'Default', views: [], ratings: {} }) };
  };
  assert.deepEqual(await readViews(fetcher, 'north end'), { active: 'Default', views: [], ratings: {} });
  await saveViews(fetcher, 'north end', { active: 'Mine', views: [] });
  assert.deepEqual(seen[0].slice(0, 2), ['engine/views?club=north%20end', 'GET']);
  assert.deepEqual(seen[1].slice(0, 2), ['engine/views', 'POST']);
  assert.deepEqual(JSON.parse(seen[1][2]), { club: 'north end', active: 'Mine', views: [] });
  let reason = null;
  assert.equal(await saveViews(answering(400, 'too many views\n'), 'a', { views: [] }, (t) => (reason = t)), null);
  assert.equal(reason, 'too many views');
  assert.equal(await readViews(answering(400, 'no'), 'a'), null);
});
