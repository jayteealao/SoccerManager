// The recovery states: the loading steps, the first-run panel, and the error panel for a
// crash, a refused snapshot, an abandoned match, and a page with no launcher behind it.

import assert from 'node:assert/strict';
import test from 'node:test';

import { abandon, fetchStatus, poll, restart } from '../launcher.mjs';
import {
  BUILD_INSTRUCTION,
  NO_LAUNCHER_HINT,
  backoff,
  clockAt,
  loadingSteps,
  panelModel,
  refusalReason,
} from '../recovery.mjs';

test('a corrupt snapshot names the corruption and offers abandon only', () => {
  // The engine's own refusal, as `crates/engine/src/error.rs` formats it.
  const reason =
    'snapshot refused: C:\\Users\\m\\data\\matches\\000000000000002a-1\\snapshot.smsn: checksum mismatch: the file is corrupt';
  const panel = panelModel({ 'engine.state': 'refused', 'engine.reason': reason, launcher: true });
  assert.equal(panel.kind, 'refused');
  assert.equal(panel.word, 'Error');
  assert.equal(
    panel.title,
    'The saved match could not be read: checksum mismatch: the file is corrupt'
  );
  assert.deepEqual(panel.actions, ['abandon']);
});

test('a crash under the launcher offers restart from the saved stoppage and abandon', () => {
  const panel = panelModel({
    'engine.state': 'crashed',
    'engine.code': 1,
    'snapshot.tick': 6_000,
    launcher: true,
  });
  assert.equal(panel.kind, 'crashed');
  assert.equal(panel.title, 'The engine stopped (exit code 1)');
  assert.equal(panel.restartLabel, 'Restart from 02:00');
  assert.deepEqual(panel.actions, ['restart', 'abandon']);
  assert.equal(panel.hint, null);
});

test('with no launcher the page offers abandon and says how to start one', () => {
  for (const status of [null, { 'engine.state': 'crashed', launcher: false }]) {
    const panel = panelModel(status);
    assert.equal(panel.kind, 'crashed');
    assert.deepEqual(panel.actions, ['abandon']);
    assert.equal(panel.hint, NO_LAUNCHER_HINT);
  }
});

test('a missing engine shows its path and the build instruction', () => {
  const panel = panelModel({
    'engine.state': 'not-found',
    'engine.path': 'C:/missing/engine-cli.exe',
    launcher: true,
  });
  assert.equal(panel.kind, 'first-run');
  assert.equal(panel.path, 'C:/missing/engine-cli.exe');
  assert.equal(panel.instruction, BUILD_INSTRUCTION);
  assert.deepEqual(panel.actions, ['open-replay']);
});

test('an abandoned match offers the replay, and a live one shows no panel', () => {
  assert.equal(panelModel({ 'engine.state': 'abandoned' }).kind, 'abandoned');
  for (const state of ['running', 'starting', 'finished']) {
    assert.equal(panelModel({ 'engine.state': state }), null, state);
  }
});

test('the loading steps carry a state word each', () => {
  assert.deepEqual(
    loadingSteps(1).map((s) => s.word),
    ['Done', 'In progress', 'Waiting']
  );
  assert.deepEqual(
    loadingSteps(3).map((s) => s.word),
    ['Done', 'Done', 'Done']
  );
});

test('the reconnect backoff doubles and stops at four seconds', () => {
  assert.deepEqual([0, 1, 2, 3, 4, 9].map(backoff), [250, 500, 1000, 2000, 4000, 4000]);
});

test('helpers read the clock and a plain reason as written', () => {
  assert.equal(clockAt(0), '00:00');
  assert.equal(clockAt(135_000), '45:00');
  assert.equal(refusalReason('no stoppage was saved'), 'no stoppage was saved');
});

test('the launcher client posts with no body and survives a dead server', async () => {
  const calls = [];
  const fetcher = async (url, init) => {
    calls.push([url, init?.method ?? 'GET']);
    return { ok: true, json: async () => ({ 'engine.state': 'running' }) };
  };
  assert.equal((await restart(fetcher))['engine.state'], 'running');
  assert.equal((await abandon(fetcher))['engine.state'], 'running');
  assert.deepEqual(calls, [
    ['engine/restart', 'POST'],
    ['engine/abandon', 'POST'],
  ]);
  const dead = async () => {
    throw new TypeError('network');
  };
  assert.equal(await fetchStatus(dead), null);
  const seen = await poll((s) => s === null, { fetcher: dead, every: 1, timeoutMs: 10 });
  assert.equal(seen, null);
});
