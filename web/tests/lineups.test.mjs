// Lineups: every state carries a word, and a substitution swaps the row to the named player.

import assert from 'node:assert/strict';
import test from 'node:test';

import { BANDS, cardWord, energyBand, lineupModel } from '../lineups.mjs';
import { KIND, MatchState } from '../match-state.mjs';
import { eventMessage, roster } from './helpers.mjs';

const TEAMS = ['club-a', 'club-b'];

test('every energy band returns a word, never a colour alone', () => {
  for (const band of BANDS) {
    assert.ok(band.word && band.word.length > 0);
    assert.ok(band.token.startsWith('--tl-'));
  }
  assert.equal(energyBand(1).word, 'Fresh');
  assert.equal(energyBand(0.7).word, 'Fresh');
  assert.equal(energyBand(0.69).word, 'Tiring');
  assert.equal(energyBand(0.4).word, 'Tiring');
  assert.equal(energyBand(0.39).word, 'Exhausted');
  assert.equal(energyBand(0).word, 'Exhausted');
  assert.equal(energyBand(null).word, 'Fresh', 'before the first condition message');
  assert.equal(cardWord('yellow'), 'Yellow');
  assert.equal(cardWord('red'), 'Red');
  assert.equal(cardWord(null), null);
});

test('the starters are the first 11 roster entries and read energy from their wire slot', () => {
  const state = new MatchState();
  const energy = Array.from({ length: 22 }, (_, i) => 1 - i * 0.04);
  state.add({ type: 'condition', tick: 50, energy });
  const model = lineupModel([roster(0), roster(1)], TEAMS, state.at(50));
  assert.equal(model[0].length, 11);
  assert.equal(model[1].length, 11);
  assert.equal(model[0][0].id, 'p-0-1');
  assert.equal(model[1][10].id, 'p-1-11');
  assert.equal(model[1][10].energy, energy[21]);
  assert.equal(model[1][10].condition, 'Exhausted');
  assert.equal(model[0][0].condition, 'Fresh');
  assert.equal(model[0][8].condition, 'Tiring');
});

test('a fatigue change moves the bar and the labelled condition', () => {
  const state = new MatchState();
  state.add({ type: 'condition', tick: 50, energy: new Array(22).fill(0.9) });
  state.add({ type: 'condition', tick: 100, energy: new Array(22).fill(0.6) });
  const before = lineupModel([roster(0), roster(1)], TEAMS, state.at(50))[0][3];
  const after = lineupModel([roster(0), roster(1)], TEAMS, state.at(100))[0][3];
  assert.equal(before.condition, 'Fresh');
  assert.equal(after.condition, 'Tiring');
  assert.notEqual(before.energy, after.energy);
});

test('a substitution swaps the row to the bench entry the event names', () => {
  const state = new MatchState();
  state.add({ type: 'condition', tick: 50, energy: new Array(22).fill(0.55) });
  state.add(
    eventMessage(60, KIND.substitution, {
      'team.id': 'club-b',
      'player.id': 'p-1-7',
      'player.secondary_id': 'p-1-14',
    })
  );
  const model = lineupModel([roster(0), roster(1)], TEAMS, state.at(60));
  const row = model[1][6];
  assert.equal(row.id, 'p-1-14');
  assert.equal(row.name, 'Player 1-14');
  assert.equal(row.wire, 17, 'the substitute takes the wire slot of the player who left');
  assert.ok(!model[1].some((r) => r.id === 'p-1-7'));
});

test('an injured player reads Injured, a sent-off player Sent off with a Red card', () => {
  const state = new MatchState();
  state.add(eventMessage(10, KIND.injury, { 'player.id': 'p-0-4' }));
  state.add(eventMessage(20, KIND.card, { 'player.id': 'p-1-2', 'card.kind': 'yellow' }));
  state.add(eventMessage(30, KIND.card, { 'player.id': 'p-1-3', 'card.kind': 'second-yellow' }));
  const model = lineupModel([roster(0), roster(1)], TEAMS, state.at(30));
  assert.equal(model[0][3].condition, 'Injured');
  assert.equal(model[1][1].cardWord, 'Yellow');
  assert.equal(model[1][1].condition, 'Fresh');
  assert.equal(model[1][2].condition, 'Sent off');
  assert.equal(model[1][2].cardWord, 'Red');
});
