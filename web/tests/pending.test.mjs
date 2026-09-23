// The pending-change list: a chip mirrors the engine's queue and shows each verdict only when
// playback reaches it.

import assert from 'node:assert/strict';
import test from 'node:test';

import { APPLIED_HOLD_TICKS, STATE_WORDS, createPendingList } from '../pending.mjs';
import { captured, eventMessage } from './helpers.mjs';

const HOME = 'club-a';

function ack(id, tick) {
  return {
    type: 'ack',
    command: 'queue-change',
    'change.queue_id': id,
    'change.queued_tick': tick,
    state: 'queued',
  };
}

function verdict(tick, id, state, fields = {}) {
  return eventMessage(tick, 'tactics-change', {
    'team.id': HOME,
    'change.kind': 'tactics',
    'change.queue_id': id,
    'change.state': state,
    ...fields,
  });
}

test('the four state words are the engine labels', () => {
  assert.deepEqual(Object.values(STATE_WORDS), ['Queued', 'Applies now', 'Applied', 'Rejected']);
});

test('a chip is Queued, turns Applied when playback reaches the stoppage, then leaves', () => {
  const list = createPendingList({ homeTeamId: HOME });
  list.queued(ack('q-1200-0', 1200), 'Mentality: attacking', 'tactics');
  assert.equal(list.chips(1000)[0].word, 'Queued');
  assert.equal(
    list.onChangeEvent(verdict(1500, 'q-1200-0', 'applied', { 'change.applied_tick': 1500 })),
    true
  );
  // The verdict arrived early: the pitch is still at 1100.
  assert.equal(list.chips(1100)[0].state, 'queued');
  const shown = list.chips(1500)[0];
  assert.equal(shown.state, 'applied');
  assert.equal(shown.word, 'Applied');
  assert.equal(shown.applied_tick, 1500);
  assert.equal(shown.queued_tick, 1200);
  assert.equal(list.chips(1500 + APPLIED_HOLD_TICKS).length, 0, 'the Applied chip leaves');
  assert.equal(list.all(1500 + APPLIED_HOLD_TICKS).length, 1, 'the record keeps it');
});

test('a change the socket refused is Rejected at once and stays until dismissed', () => {
  const list = createPendingList({ homeTeamId: HOME });
  const id = list.rejectedAtQueue(
    { type: 'reject', command: 'queue-change', reason: 'cannot read the tactics change' },
    'Mentality: attacking',
    'tactics'
  );
  const [chip] = list.chips(0);
  assert.equal(chip.word, 'Rejected');
  assert.equal(chip.reason, 'cannot read the tactics change');
  assert.equal(list.chips(99_999).length, 1);
  list.dismiss(id);
  assert.equal(list.chips(99_999).length, 0);
});

test('a stoppage rejection shows the engine reason verbatim', () => {
  const list = createPendingList({ homeTeamId: HOME });
  list.queued(ack('q-9000-5', 9000), 'Substitution: 9 off, 14 on', 'substitution');
  list.onChangeEvent(
    verdict(9100, 'q-9000-5', 'rejected', {
      'change.kind': 'substitution',
      'change.rejected_reason': 'substitution limit reached (5 of 5)',
    })
  );
  const [chip] = list.chips(9100);
  assert.equal(chip.word, 'Rejected');
  assert.equal(chip.reason, 'substitution limit reached (5 of 5)');
  assert.equal(chip.applied_tick, null);
  assert.equal(list.chips(90_000).length, 1, 'a rejection stays');
});

test('an unknown identifier changes nothing and is logged; other rows are ignored', () => {
  const list = createPendingList({ homeTeamId: HOME });
  list.queued(ack('q-1-0', 1), 'Mentality: positive', 'tactics');
  const rows = captured(() => {
    assert.equal(list.onChangeEvent(verdict(50, 'q-77-7', 'applied')), false);
  });
  assert.equal(rows.length, 1);
  assert.equal(rows[0].signal, 'viewer.change_unknown');
  assert.equal(rows[0]['change.queue_id'], 'q-77-7');
  // The socket's own record of the queued change, and the other club's verdicts.
  assert.equal(list.onChangeEvent(verdict(1, 'q-1-0', 'queued', { 'team.id': undefined })), false);
  assert.equal(
    list.onChangeEvent(verdict(60, 'q-1-0', 'applied', { 'team.id': 'club-b' })),
    false
  );
  assert.equal(list.chips(60)[0].state, 'queued');
});

test('two chips resolve out of order, each at its own tick', () => {
  const list = createPendingList({ homeTeamId: HOME });
  list.queued(ack('q-10-0', 10), 'Pressing: high', 'tactics');
  list.queued(ack('q-11-1', 11), 'Substitution: 9 off, 14 on', 'substitution');
  list.onChangeEvent(verdict(400, 'q-11-1', 'applied'));
  list.onChangeEvent(verdict(900, 'q-10-0', 'rejected', { 'change.rejected_reason': 'no' }));
  const at500 = list.chips(500);
  assert.deepEqual(
    at500.map((c) => [c.queue_id, c.state]),
    [
      ['q-10-0', 'queued'],
      ['q-11-1', 'applied'],
    ]
  );
  // By 900 the Applied chip has left and the Rejected one has arrived.
  const at900 = list.chips(900);
  assert.deepEqual(
    at900.map((c) => [c.queue_id, c.state]),
    [['q-10-0', 'rejected']]
  );
  assert.deepEqual(
    list.applied().map((c) => c.queue_id),
    ['q-11-1']
  );
});
