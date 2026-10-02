// The page's launcher client: a refusal hands its reason on, and a resumed match's earlier
// events come back as a list. A fake fetch stands in for the launcher.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { fetchEarlierEvents, newMatch, resume } from '../src/lib/launcher.js';

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
