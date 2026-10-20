// Skip to the next stoppage: one index merged from the two marks the protocol already
// sends, with a goal and the restart that follows it counted once.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { COLLAPSE_TICKS, STOPS_PLAY, Stoppages, stopsPlay } from '../src/lib/stoppages.js';

test('a goal and its restart collapse to one entry at the earlier tick', () => {
  const stoppages = new Stoppages();
  stoppages.add(1000);
  stoppages.add(1012);
  assert.equal(stoppages.length, 1);
  assert.equal(stoppages.next(0), 1000);
});

test('a restart with no event of its own stands alone', () => {
  const stoppages = new Stoppages();
  stoppages.add(1000);
  stoppages.add(1000 + COLLAPSE_TICKS + 1);
  assert.equal(stoppages.length, 2);
  assert.equal(stoppages.next(1000), 1026);
});

test('a restart tag seconds after the stop event closes it and adds no mark', () => {
  const stoppages = new Stoppages();
  stoppages.stop(1000);
  stoppages.restart(1600);
  assert.equal(stoppages.length, 1);
  assert.equal(stoppages.next(0), 1000);
  assert.equal(stoppages.next(1000), null);
});

test('a lone restart tag with no stoppage open still marks', () => {
  const stoppages = new Stoppages();
  stoppages.restart(1);
  stoppages.stop(5000);
  stoppages.restart(5600);
  stoppages.restart(9000);
  assert.deepEqual(stoppages.ticks, [1, 5000, 9000]);
});

test('a quick restart tagged before its event is one mark', () => {
  // The tick frame reaches the page before that tick's events.
  const stoppages = new Stoppages();
  stoppages.restart(3000);
  stoppages.stop(3000);
  assert.deepEqual(stoppages.ticks, [3000]);
});

test('a resumed match forgets the open stoppage', () => {
  const stoppages = new Stoppages();
  stoppages.stop(1000);
  stoppages.truncate(900);
  stoppages.restart(1500);
  assert.deepEqual(stoppages.ticks, [1500]);
});

test('the next stoppage past the last one is null', () => {
  const stoppages = new Stoppages();
  stoppages.add(1);
  stoppages.add(500);
  assert.equal(stoppages.next(500), null);
  assert.equal(stoppages.next(9999), null);
});

test('the previous stoppage walks backwards and runs out at the first', () => {
  const stoppages = new Stoppages();
  for (const tick of [1, 500, 2000, 9000]) {
    stoppages.add(tick);
  }
  assert.equal(stoppages.prev(9000), 2000);
  assert.equal(stoppages.prev(2001), 2000);
  assert.equal(stoppages.prev(2000), 500);
  assert.equal(stoppages.prev(1), null);
});

const event = (type, extra = {}) => ({ type: 'event', tick: 10, 'event.type': type, ...extra });

test('every event that stops play is a mark', () => {
  for (const type of STOPS_PLAY) {
    assert.equal(stopsPlay(event(type)), true, type);
  }
  assert.equal(stopsPlay(event('foul')), true);
  assert.equal(stopsPlay(event('foul', { 'foul.advantage': false })), true);
});

test('advantage, a card, and a tactical change are not marks', () => {
  assert.equal(stopsPlay(event('foul', { 'foul.advantage': true })), false);
  assert.equal(stopsPlay(event('card', { 'card.kind': 'yellow' })), false);
  assert.equal(stopsPlay(event('tactics-change')), false);
});

test('an injury is a mark; a substitution and an AI choice are not', () => {
  assert.equal(stopsPlay(event('injury')), true);
  assert.equal(stopsPlay(event('substitution')), false);
  assert.equal(stopsPlay(event('ai-decision', { 'ai.decision': 'sub-injury' })), false);
});

test('a message that is not an event is not a mark', () => {
  assert.equal(stopsPlay({ type: 'stats', tick: 10 }), false);
  assert.equal(stopsPlay({ type: 'command-ack', tick: 10, 'event.type': 'goal' }), false);
});

test('a resumed match drops the marks after its stoppage', () => {
  const stoppages = new Stoppages();
  for (const tick of [100, 500, 900]) {
    stoppages.add(tick);
  }
  stoppages.truncate(500);
  assert.equal(stoppages.length, 2);
  assert.equal(stoppages.next(500), null);
  stoppages.add(700);
  assert.equal(stoppages.next(500), 700);
});
