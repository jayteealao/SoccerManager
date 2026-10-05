// Every page-side signal is one JSON Lines row on console.info, with the engine's envelope,
// plus a short ring a test or a drive reads. The ported modules log through it.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { RING_LIMIT, clearSignals, signal, signals } from '../src/lib/signal.js';
import { captured, quiet } from './helpers.js';

test('a signal writes one row with the envelope and its fields', () => {
  clearSignals();
  const rows = captured(() => signal('viewer.test', { 'match.id': 'm1' }));
  assert.equal(rows.length, 1);
  const [row] = rows;
  assert.equal(row['record.kind'], 'viewer-event');
  assert.equal(row['schema.version'], '1');
  assert.equal(row.service, 'touchline-viewer');
  assert.equal(row.operation, 'view');
  assert.equal(row.signal, 'viewer.test');
  assert.equal(row['match.id'], 'm1');
  assert.ok(!Number.isNaN(Date.parse(row.ts)));
  assert.deepEqual(signals(), rows);
});

test('the ring keeps the newest rows up to its limit', () => {
  clearSignals();
  quiet(() => {
    for (let i = 0; i < RING_LIMIT + 5; i += 1) {
      signal('viewer.test', { i });
    }
  });
  const kept = signals();
  assert.equal(kept.length, RING_LIMIT);
  assert.equal(kept[0].i, 5);
  assert.equal(kept.at(-1).i, RING_LIMIT + 4);
  clearSignals();
  assert.deepEqual(signals(), []);
});
