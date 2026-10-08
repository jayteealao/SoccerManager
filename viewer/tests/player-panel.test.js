// The player panel model: attributes by group as whole numbers in their bands, "Plays between"
// in whole numbers (and "Plays at" when equal), the hidden words for 0, 9 and 31 matches at the
// club, the match ratings with one decimal, the body words against the engine's references,
// and an older hello's missing fields read as not known.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import { abilityAxis, ageWords, BODY_REFERENCE, heightWords, panelModel, playsBetween, ratingLines } from '../src/lib/player-panel.js';
import { REPO_ROOT } from './helpers.js';

const TUNING = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/tuning.json'), 'utf8'));

function player(matches, over = {}) {
  return {
    'player.id': 'p-10',
    'player.name': 'Nico Ferraz',
    'player.shirt': 10,
    'player.position': 'AM',
    'player.attributes': { passing: 171, technique: 165, decisions: 158, pace: 139, handling: 20 },
    'player.height': 178,
    'player.age': 23,
    'player.nationality': 'POR',
    'player.build': 'slight',
    'player.condition': 95,
    'player.matches_at_club': matches,
    'player.consistency':
      matches === 0 ? { confidence: 'not_yet_known' } : { word: 'rarely_off', confidence: matches >= 20 ? 'firm' : 'tentative' },
    'player.injury_proneness':
      matches === 0 ? { confidence: 'not_yet_known' } : { word: 'hardly_ever_injured', confidence: matches >= 20 ? 'firm' : 'tentative' },
    'player.level': 172,
    'player.plays_between': [158, 172],
    ...over,
  };
}

test('the body references match the engine tuning', () => {
  const body = TUNING.engine.contract.body;
  assert.equal(BODY_REFERENCE.heightCm, body.height.reference_cm);
  assert.equal(BODY_REFERENCE.age, body.age.reference);
});

test('a new signing reads not yet known, with no match rating yet', () => {
  const m = panelModel(player(0), []);
  assert.deepEqual(m.groups.map((g) => g.id), ['technical', 'mental', 'physical']);
  assert.deepEqual(m.groups[0].rows.map((r) => [r.label, r.value, r.band]), [
    ['Passing', 17, 'v4'],
    ['Technique', 17, 'v4'],
  ]);
  for (const h of m.hidden) {
    assert.equal(h.full, 'Not yet known — no match for us yet');
  }
  assert.equal(m.ratings.text, 'No match rating yet');
  assert.equal(m.nation.value, 'New at the club');
  assert.match(m.sub, /no match rating yet/);
  assert.deepEqual(m.plays, { lo: 16, hi: 17, text: 'Plays between 16 and 17' });
  assert.equal(m.body.build, 'Build: slight, from his strength and balance.');
});

test('9 matches read tentative and 31 firm, with the match ratings in one decimal', () => {
  const tentative = panelModel(player(9), [{ match: 'm-1', rating: 7.94 }]);
  assert.equal(tentative.hidden[0].full, '“Rarely has an off day” — 9 matches here, tentative');
  assert.equal(tentative.ratings.last, '7.9');
  assert.equal(tentative.ratings.avg3.of, '1 match');
  const firm = panelModel(player(31), [6.5, 7.0, 7.5, 8.0].map((rating, i) => ({ match: `m-${i}`, rating })));
  assert.equal(firm.hidden[1].full, '“Hardly ever injured” — 31 matches here, sure');
  assert.equal(firm.nation.value, 'Settled');
  assert.equal(firm.nation.label, 'POR · 31 matches here');
  assert.equal(firm.ratings.avg3.text, '7.5');
  assert.equal(firm.ratings.avg10.text, '7.3');
  assert.equal(firm.ratings.avg10.of, '4 matches');
  assert.deepEqual(firm.ratings.chips.map((c) => c.text), ['6.5', '7.0', '7.5', '8.0']);
});

test('plays between rounds to whole numbers, and reads plays at when they meet', () => {
  assert.equal(playsBetween([158, 172]).text, 'Plays between 16 and 17');
  assert.equal(playsBetween([166, 172]).text, 'Plays at 17');
  assert.equal(playsBetween(undefined), null);
  assert.equal(ratingLines([]).none, true);
});

test('a keeper shows his goalkeeping group; an older hello reads not known', () => {
  const keeper = panelModel(player(31, { 'player.position': 'GK' }), []);
  assert.deepEqual(keeper.groups.map((g) => g.id), ['goalkeeping', 'mental', 'physical']);
  const old = panelModel(
    {
      'player.id': 'p-1',
      'player.name': 'Old Hello',
      'player.shirt': 1,
      'player.position': 'CB',
      'player.consistency': { confidence: 'not_yet_known' },
      'player.injury_proneness': { confidence: 'not_yet_known' },
    },
    []
  );
  assert.deepEqual(old.groups, []);
  assert.equal(old.plays, null);
  assert.equal(old.age, 'Age not known');
  assert.equal(old.body.height, null);
  assert.equal(old.nation.value, 'Not known');
});

test('height and age read against the references', () => {
  assert.match(heightWords(191), /reaches higher/);
  assert.match(heightWords(172), /turns a little quicker/);
  assert.match(heightWords(181), /average/);
  assert.match(ageWords(23), /no late-match fade/);
  assert.match(ageWords(30), /a little more slowly/);
  assert.match(ageWords(34), /fades late/);
  assert.equal(heightWords(undefined), null);
});

test('the effective-ability scale reaches below 12 for a player who plays below it', () => {
  assert.deepEqual(abilityAxis(14, 16), { from: 12, ticks: [12, 14, 16, 18, 20] });
  assert.deepEqual(abilityAxis(9, 11), { from: 8, ticks: [8, 11, 14, 17, 20] });
  assert.deepEqual(abilityAxis(1, null), { from: 0, ticks: [0, 5, 10, 15, 20] });
  assert.deepEqual(abilityAxis(undefined, null), { from: 12, ticks: [12, 14, 16, 18, 20] });
});
