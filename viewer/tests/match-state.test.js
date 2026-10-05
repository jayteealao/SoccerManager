// The match at the rendered tick: nothing shows before the pitch reaches it.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { HIDDEN_KINDS, KIND, MatchState, newestAtOrBefore } from '../src/lib/match-state.js';
import { eventMessage, statsMessage } from './helpers.js';

test('kick-off with no events reads 0 to 0 with no entries and no statistics', () => {
  const state = new MatchState();
  const view = state.at(0);
  assert.equal(view.home, 0);
  assert.equal(view.away, 0);
  assert.equal(view.entries.length, 0);
  assert.equal(view.stats, null);
  assert.equal(view.energy, null);
});

test('a goal at tick T shows at T and not at T - 1', () => {
  const state = new MatchState();
  state.add(eventMessage(1, KIND.kickOff));
  state.add(eventMessage(900, KIND.goal, { 'home.score': 1, 'team.id': 'club-a' }));
  let view = state.at(899);
  assert.deepEqual([view.home, view.away], [0, 0]);
  assert.equal(view.goals.length, 0);
  view = state.at(900);
  assert.deepEqual([view.home, view.away], [1, 0]);
  assert.equal(view.goals.length, 1);
  assert.equal(view.entries[view.entries.length - 1]['event.type'], KIND.goal);
});

test('a goal that arrives 400 ticks early stays hidden until the rendered tick reaches it', () => {
  const state = new MatchState();
  state.at(1000);
  // The stream leads playback: the goal at 1400 is stored while the pitch draws 1000.
  state.add(eventMessage(1400, KIND.goal, { 'home.score': 1 }));
  state.add(statsMessage(1400));
  const early = state.at(1000);
  assert.equal(early.home, 0);
  assert.equal(early.entries.length, 0);
  assert.equal(early.stats, null);
  const later = state.at(1400);
  assert.equal(later.home, 1);
  assert.equal(later.stats.tick, 1400);
});

test('a seek backwards removes later entries and returns the earlier statistics and energy', () => {
  const state = new MatchState();
  state.add(eventMessage(1, KIND.kickOff));
  state.add(statsMessage(50));
  state.add({ type: 'condition', tick: 50, energy: new Array(22).fill(0.99) });
  state.add(eventMessage(80, KIND.goal, { 'away.score': 1 }));
  state.add(statsMessage(100, { 'away.score': 1 }));
  state.add({ type: 'condition', tick: 100, energy: new Array(22).fill(0.95) });
  const forward = state.at(120);
  assert.equal(forward.entries.length, 2);
  assert.equal(forward.stats.tick, 100);
  assert.equal(forward.energy[0], 0.95);
  const back = state.at(60);
  assert.equal(back.entries.length, 1);
  assert.equal(back.away, 0);
  assert.equal(back.goals.length, 0);
  assert.equal(back.stats.tick, 50);
  assert.equal(back.energy[0], 0.99);
  assert.equal(back.energyTick, 50);
  assert.equal(state.at(0).entries.length, 0);
});

test('the computer manager reasoning stays out of the feed, its outcome does not', () => {
  const state = new MatchState();
  assert.ok(HIDDEN_KINDS.has(KIND.aiDecision));
  state.add(eventMessage(10, KIND.aiDecision, { 'ai.decision': 'sub-fatigue' }));
  state.add(eventMessage(10, KIND.substitution, { 'player.id': 'a', 'player.secondary_id': 'b', 'team.id': 't' }));
  const view = state.at(10);
  assert.deepEqual(view.entries.map((e) => e['event.type']), [KIND.substitution]);
  assert.deepEqual(view.substitutions, [{ tick: 10, team: 't', off: 'a', on: 'b' }]);
});

test('a second yellow is a red and sends the player off; an injury is remembered', () => {
  const state = new MatchState();
  state.add(eventMessage(5, KIND.card, { 'player.id': 'p', 'card.kind': 'yellow' }));
  state.add(eventMessage(9, KIND.injury, { 'player.id': 'q' }));
  assert.equal(state.at(6).cards.get('p'), 'yellow');
  state.add(eventMessage(20, KIND.card, { 'player.id': 'p', 'card.kind': 'second-yellow' }));
  const view = state.at(20);
  assert.equal(view.cards.get('p'), 'red');
  assert.ok(view.sentOff.has('p'));
  assert.ok(view.injuries.has('q'));
});

test('an event stored behind the shown tick is folded into the view', () => {
  const state = new MatchState();
  state.at(500);
  state.add(eventMessage(300, KIND.goal, { 'home.score': 1 }));
  assert.equal(state.at(500).home, 1);
});

test('the newest message at or before a tick is found by binary search', () => {
  const list = [{ tick: 50 }, { tick: 100 }, { tick: 150 }];
  assert.equal(newestAtOrBefore(list, 0), -1);
  assert.equal(newestAtOrBefore(list, 50), 0);
  assert.equal(newestAtOrBefore(list, 149), 1);
  assert.equal(newestAtOrBefore(list, 1e9), 2);
});

test('full time is known once its event is stored, before playback reaches it', () => {
  const state = new MatchState();
  assert.equal(state.fullTimeTick, null);
  state.add(eventMessage(292900, KIND.fullTime));
  assert.equal(state.fullTimeTick, 292900);
  assert.equal(state.at(100).fullTime, false);
});

test('the full-time tick follows a rewind past it and a cleared match', () => {
  const state = new MatchState();
  state.add(eventMessage(1, KIND.kickOff));
  state.add(eventMessage(4500, KIND.fullTime));
  assert.equal(state.fullTimeTick, 4500);
  state.truncate(4000);
  assert.equal(state.fullTimeTick, null);
  state.add(eventMessage(4600, KIND.fullTime));
  assert.equal(state.fullTimeTick, 4600);
  state.clear();
  assert.equal(state.fullTimeTick, null);
});
