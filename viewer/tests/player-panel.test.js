// The player panel model: attributes by group as whole numbers in their bands, "Plays between"
// in whole numbers (and "Plays at" when equal), the hidden words for 0, 9 and 31 matches at the
// club, the match ratings with one decimal, the body words against the engine's references,
// and an older hello's missing fields read as not known.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import {
  abilityAxis,
  ageWords,
  assistantView,
  BODY_REFERENCE,
  feetModel,
  footWord,
  formWord,
  heightWords,
  overrideLine,
  panelModel,
  playsBetween,
  positionChips,
  progressWord,
  ratingLines,
  seasonModel,
  stateRows,
  TRAIT_ORDER,
  traitRows,
} from '../src/lib/player-panel.js';
import { PERSONALITY_NAMES } from '../src/lib/hidden-words.js';
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

// ---- The person (protocol 7) ----------------------------------------------------------------

const TRAITS = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/traits.json'), 'utf8'));

/// A personality of word keys at `confidence`, every value `middling` unless named.
function personality(keys = {}, confidence = 'firm') {
  return Object.fromEntries(PERSONALITY_NAMES.map((n) => [n, { word: keys[n] ?? 'middling', confidence }]));
}

/// A protocol 7 squad entry: an AM rated 20 there, 31 matches at the club, driven and calm.
function person(over = {}) {
  return player(31, {
    'player.positions': { GK: 10, CB: 40, LB: 50, RB: 50, DM: 90, CM: 160, AM: 200, LW: 120, RW: 140, ST: 130 },
    'player.strong_foot': 'right',
    'player.weaker_foot': 64,
    'player.traits': [
      { name: 'tries_tricks', strength: 'moderate' },
      { name: 'tries_killer_balls', strength: 'strong' },
      { name: 'switches_play', strength: 'strong' },
    ],
    'player.personality': personality({ professionalism: 'high', ambition: 'high', temperament: 'high' }),
    'player.mood': 'lifted',
    'player.sharpness': 90,
    ...over,
  });
}

const SEASON = {
  starts: 12,
  minutes: 1010,
  goals: 5,
  assists: 6,
  clean_sheets: 0,
  red_cards: 0,
  ratings: [6.9, 7.1, 7.6, 7.2, 7.8, 7.4, 7.7].map((rating, i) => ({ match: `m-${i}`, rating })),
  goal: { kind: 'assists', target: 10 },
  form: 7.5,
  progress: 100,
};

test('the positions pitch has ten chips, the natural one highest, and a whole 1 reads "—"', () => {
  const p = positionChips(person()['player.positions'], 'AM');
  assert.deepEqual(p.chips.map((c) => c.code), ['GK', 'CB', 'LB', 'RB', 'DM', 'CM', 'AM', 'LW', 'RW', 'ST']);
  const am = p.chips.find((c) => c.code === 'AM');
  assert.deepEqual([am.text, am.band, am.natural], ['20', 'v4', true]);
  assert.ok(p.chips.every((c) => c.natural || (c.whole ?? 0) < am.whole));
  const gk = p.chips.find((c) => c.code === 'GK');
  assert.deepEqual([gk.text, gk.band, gk.label], ['—', null, 'GK not rated']);
  assert.equal(p.note, 'Natural: AM (C). Away from it he loses some familiarity; his versatility sets how much.');
  assert.equal(positionChips({}, 'AM'), null, 'an older hello has no ratings');
  assert.equal(positionChips(undefined, 'AM'), null);
  // A converted player is rated at his listed position only.
  const converted = positionChips({ GK: 10, CB: 10, LB: 150, RB: 10, DM: 10, CM: 10, AM: 10, LW: 10, RW: 10, ST: 10 }, 'LB');
  assert.deepEqual(converted.chips.filter((c) => c.whole !== null).map((c) => c.code), ['LB']);
  assert.match(converted.note, /^Natural: LB\. /);
});

test('the feet read the strong foot at 100 and the weaker as a percent with its word', () => {
  const feet = feetModel('right', 64);
  assert.deepEqual(feet.rows.map((r) => [r.text, r.value, r.band]), [
    ['Right (strong)', '100', 'v4'],
    ['Left · decent', '64%', 'v3'],
  ]);
  assert.equal(feet.note, 'Costs him only on the left side or with no time to shift it. Opponents show him left.');
  assert.equal(feetModel('left', 50).rows[1].text, 'Right · decent');
  assert.equal(feetModel('left', 50).rows[1].band, 'v2');
  assert.deepEqual([footWord(20), footWord(34), footWord(35), footWord(79), footWord(80)], [
    'one-footed', 'one-footed', 'decent', 'decent', 'two-footed',
  ]);
  assert.equal(feetModel(undefined, 64), null);
  assert.equal(feetModel('right', undefined), null);
});

test('a known player reads a summary word, standouts with sureness, and the assistant view', () => {
  const m = panelModel(person(), [], { season: SEASON, played: 'AM' });
  assert.equal(m.personality.summary, 'Driven');
  assert.equal(m.personality.line, 'professional, ambitious, calm · sure · 31 matches here');
  assert.deepEqual(m.assistant, [
    'Model pro — first in, last out, and he wants to win things.',
    'Doesn’t bite when fouled.',
    'In good spirits after his last match.',
    'Chasing his assists target.',
  ]);
  assert.deepEqual([m.mood.text, m.mood.mark, m.mood.token], ['Lifted', '▲', '--good']);
  assert.deepEqual(m.traits.map((t) => [t.words, t.strength]), [
    ['Tries killer balls', 'Strong'],
    ['Switches play', 'Strong'],
    ['Tries tricks', 'Moderate'],
  ]);
  assert.equal(m.override, null, 'the override count comes after a match');
});

test('a new player reads not yet known and too early to say, settling in', () => {
  const m = panelModel(
    person({
      'player.matches_at_club': 0,
      'player.personality': personality({}, 'not_yet_known'),
      'player.mood': 'steady',
    }),
    [],
    { season: { ...SEASON, ratings: [], starts: 0, minutes: 0, goals: 0, assists: 0, form: null, progress: null } }
  );
  assert.equal(m.personality.summary, 'Not yet known');
  assert.equal(m.personality.line, 'no match for us yet');
  assert.equal(m.personality.token, '--ink-3');
  assert.deepEqual(m.assistant, ['Too early to say what he is like.', 'Quiet in his first week, a bit unsure of himself.']);
  assert.equal(m.mood.text, 'Settling in');
  assert.equal(m.season.text, 'No matches this season yet.');
  assert.deepEqual(m.season.rows.map((r) => [r.label, r.text]), [
    ['Goal', 'Make 10 assists · 0 of 10'],
    ['Form', 'Not yet known'],
  ]);
});

test('the assistant names money motivation when the goal has no progress, and the goal when behind', () => {
  const p = personality({ temperament: 'low', money_motivation: 'low' });
  assert.deepEqual(assistantView({ personality: p, mood: 'low', season: null, matches: 9 }), [
    'Plays on the edge and can boil over.',
    'Can lose his head when fouled.',
    'Low after his last match.',
    'Money matters to him only a little.',
  ]);
  const behind = assistantView({
    personality: personality({ money_motivation: 'high' }),
    mood: 'steady',
    season: { goal: { kind: 'goals', target: 12 }, progress: 40 },
    matches: 9,
  });
  assert.deepEqual(behind, ['No worries about his character.', 'Behind on his goals target and knows it.']);
});

test('traits keep catalogue order, at most eight; no traits read none; an older hello reads not known', () => {
  assert.deepEqual(TRAIT_ORDER, TRAITS.traits.map((t) => t.name));
  const many = [...TRAIT_ORDER].reverse().slice(0, 10).map((name) => ({ name, strength: 'moderate' }));
  const rows = traitRows(many);
  assert.equal(rows.length, 8);
  assert.deepEqual(rows.map((r) => r.name), [...many].reverse().slice(0, 8).map((t) => t.name));
  assert.deepEqual(panelModel(person({ 'player.traits': [] }), []).traits, []);
  const { 'player.traits': _, ...noField } = person();
  assert.deepEqual(panelModel(noField, []).traits, [], 'protocol 7 omits an empty list');
  const old = player(31);
  assert.equal(panelModel(old, []).traits, null);
  assert.equal(overrideLine(0), null);
  assert.equal(overrideLine(undefined), null);
  assert.equal(overrideLine(1, 'a killer ball against "Shorter passing"').count, 1);
});

test('the season block reads so far, story, goal and form, and its empty states', () => {
  const s = seasonModel(SEASON);
  assert.deepEqual(s.points, [7.1, 7.6, 7.2, 7.8, 7.4, 7.7], 'the last six ratings');
  assert.deepEqual(s.rows.map((r) => [r.label, r.text]), [
    ['So far', '7 apps · 5 goals · 6 assists · avg 7.4'],
    ['Story', 'Three good games in a row.'],
    ['Goal', 'Make 10 assists · 6 of 10, on track'],
    ['Form', 'In form'],
  ]);
  assert.equal(s.rows.at(-1).mark, '▲');
  assert.equal(seasonModel(null).none, true);
  assert.equal(seasonModel({}).text, 'No matches this season yet.');
  assert.deepEqual([formWord(7.2).text, formWord(7.1).text, formWord(6.3).text, formWord(null).text], [
    'In form', 'Steady form', 'Out of form', 'Not yet known',
  ]);
  assert.deepEqual([progressWord(110), progressWord(90), progressWord(89), progressWord(null)], ['ahead', 'on track', 'behind', null]);
  const keeper = seasonModel({ ...SEASON, clean_sheets: 3, goal: { kind: 'clean_sheets', target: 12 }, progress: 120 });
  assert.match(keeper.rows[0].text, /3 clean sheets/);
  assert.equal(keeper.rows[2].text, 'Keep 12 clean sheets · 3 of 12, ahead');
  const rating = seasonModel({ ...SEASON, goal: { kind: 'average_rating', target: 7 }, progress: 105 });
  assert.equal(rating.rows[2].text, 'Average a rating of 7.0 · 7.4 so far, on track');
  const discipline = seasonModel({ ...SEASON, goal: { kind: 'no_own_fault_absence', target: 0 }, progress: 100 });
  assert.equal(discipline.rows[2].text, 'Miss no match through his own fault · none missed, on track');
});

test('the state rows read mind, body, familiarity by the rating gap, and surroundings', () => {
  const positions = person()['player.positions'];
  const rows = (over) =>
    Object.fromEntries(
      stateRows({ mood: null, form: null, condition: 100, sharpness: 90, natural: 'AM', played: 'AM', positions, ...over }).map(
        (r) => [r.key, r]
      )
    );
  assert.equal(rows({}).familiarity.text, 'Natural position — knows the role');
  assert.equal(rows({ played: 'ST' }).familiarity.text, 'Out of position at ST — costs him some');
  assert.equal(rows({ played: 'CM' }).familiarity.text, 'Out of position at CM — costs him some');
  assert.equal(rows({ played: 'RW' }).familiarity.text, 'Out of position at RW — costs him some');
  assert.equal(rows({ played: 'CB' }).familiarity.text, 'Out of position at CB — costs him a lot');
  assert.equal(rows({ played: 'ST', positions: { ...positions, ST: 190 } }).familiarity.text, 'Out of position at ST — costs him a little');
  assert.equal(rows({ played: 'ST' }).familiarity.out, true);
  assert.equal(rows({ played: null }).familiarity.text, 'Not in the eleven');
  assert.equal(rows({ played: 'ST', positions: undefined }).familiarity.text, 'Out of position at ST');
  assert.equal(rows({}).body.text, 'Fresh and sharp');
  assert.equal(rows({ sharpness: 70 }).body.text, 'Short of sharpness');
  assert.equal(rows({ condition: 70 }).body.text, 'Tired');
  assert.equal(rows({ condition: undefined }).body.text, 'Not yet known');
  assert.equal(rows({}).surroundings.text, 'No effect today');
});

test('no personality, mood, form or assistant text carries a digit, and none takes a band', () => {
  const cases = [
    panelModel(person(), [], { season: SEASON, played: 'ST' }),
    panelModel(person({ 'player.mood': 'rattled', 'player.personality': personality({ temperament: 'low', controversy: 'high' }) }), [], {
      season: { ...SEASON, form: 5.9, progress: 40 },
    }),
    panelModel(person({ 'player.matches_at_club': 0, 'player.personality': personality({}, 'not_yet_known') }), []),
  ];
  for (const m of cases) {
    const mind = m.state.find((r) => r.key === 'mind');
    const texts = [m.personality.summary, ...m.personality.standouts, ...m.assistant, m.mood.text, mind.mood.text, mind.form.text];
    for (const text of texts) {
      assert.ok(!/\d/.test(text), text);
    }
    for (const part of [m.personality, m.mood, mind.mood, mind.form]) {
      assert.ok(!('band' in part), JSON.stringify(part));
    }
  }
});
