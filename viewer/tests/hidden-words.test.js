// The hidden values in words: every word key the engine bands has its text; the three states
// read "not yet known", a tentative word and a firm word with the matches seen; no result
// carries a number for the value itself; the build words.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import {
  BUILD_TEXT,
  buildWord,
  HIDDEN_TEXT,
  hiddenWord,
  keyWords,
  MOOD_TEXT,
  moodWord,
  PERSONALITY_NAMES,
  PERSONALITY_TEXT,
  standouts,
  summaryWord,
  sureness,
} from '../src/lib/hidden-words.js';
import { REPO_ROOT } from './helpers.js';

const TUNING = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/tuning.json'), 'utf8'));
const ATTRIBUTES = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/attributes.json'), 'utf8'));
// The personality values have word bands too, but the panel composes their words into its
// summary word, so this page's hidden-value text covers the other hidden values only.
const PERSONALITY = new Set(ATTRIBUTES.attributes.filter((a) => a.group === 'personality').map((a) => a.name));

test('every word key in the tuning file has its text, in the same order, best first', () => {
  for (const [name, bands] of Object.entries(TUNING.hidden.words).filter(([n]) => !PERSONALITY.has(n))) {
    const keys = bands.map((b) => b.word);
    const text = Object.keys(HIDDEN_TEXT[name]);
    // The tuning lists bands from the highest rating down; for injury proneness the highest
    // rating is the worst word, so the page's best-first order is the reverse.
    assert.deepEqual([...text].sort(), [...keys].sort(), name);
  }
  const build = TUNING.engine.contract.body.build;
  assert.ok(build.slight_below < build.powerful_from);
  assert.deepEqual(Object.keys(BUILD_TEXT), ['slight', 'athletic', 'powerful']);
});

test('not yet known, tentative and firm read as the boards draw them', () => {
  const none = hiddenWord('consistency', { confidence: 'not_yet_known' }, 0);
  assert.equal(none.full, 'Not yet known — no match for us yet');
  assert.equal(none.token, '--ink-3');
  assert.equal(none.known, false);
  assert.equal(none.order, null);
  // A word sent with not yet known is never shown.
  assert.equal(hiddenWord('consistency', { word: 'steady', confidence: 'not_yet_known' }, 0).known, false);
  const tentative = hiddenWord('injury_proneness', { word: 'hardly_ever_injured', confidence: 'tentative' }, 9);
  assert.equal(tentative.full, '“Hardly ever injured” — 9 matches here, tentative');
  assert.equal(tentative.token, '--ink-2');
  const firm = hiddenWord('consistency', { word: 'rarely_off', confidence: 'firm' }, 31);
  assert.equal(firm.full, '“Rarely has an off day” — 31 matches here, sure');
  assert.equal(hiddenWord('consistency', { word: 'steady', confidence: 'tentative' }, 1).line, '1 match here, tentative');
  assert.equal(hiddenWord('consistency', { word: 'steady', confidence: 'firm' }).line, 'sure');
});

test('the best word orders highest and an unknown key reads in words, never as a number', () => {
  const order = (word) => hiddenWord('consistency', { word, confidence: 'firm' }, 20).order;
  assert.ok(order('rarely_off') > order('steady'));
  assert.ok(order('has_off_days') > order('erratic'));
  const injury = (word) => hiddenWord('injury_proneness', { word, confidence: 'firm' }, 20).order;
  assert.ok(injury('hardly_ever_injured') > injury('injury_prone'));
  const odd = hiddenWord('consistency', { word: 'new_word_later', confidence: 'firm' }, 20);
  assert.equal(odd.text, 'New word later');
  assert.equal(keyWords('has_off_days'), 'Has off days');
  for (const [name, words] of Object.entries(HIDDEN_TEXT)) {
    for (const word of Object.keys(words)) {
      const w = hiddenWord(name, { word, confidence: 'firm' }, 31);
      assert.ok(!/\d/.test(w.text), `${name} ${word}: ${w.text}`);
    }
  }
  assert.equal(buildWord('powerful'), 'powerful');
  assert.equal(buildWord(null), null);
});

/// A personality of word keys, every value `middling` unless named, at `confidence`.
function person(keys = {}, confidence = 'firm') {
  return Object.fromEntries(
    PERSONALITY_NAMES.map((name) => [name, { word: keys[name] ?? 'middling', confidence }])
  );
}

test('the personality words cover the eleven values and their word keys', () => {
  assert.deepEqual([...PERSONALITY_NAMES].sort(), [...PERSONALITY].sort());
  for (const name of PERSONALITY_NAMES) {
    assert.deepEqual(TUNING.hidden.words[name].map((b) => b.word), ['high', 'middling', 'low'], name);
    assert.deepEqual(Object.keys(PERSONALITY_TEXT[name]).sort(), ['high', 'low'], name);
  }
});

test('the summary word follows the rule table in order, and Balanced by default', () => {
  const rows = [
    [{ professionalism: 'high', ambition: 'high', temperament: 'high' }, 'Driven'],
    [{ professionalism: 'high', temperament: 'high' }, 'Model professional'],
    [{ leadership: 'high', temperament: 'high' }, 'Natural leader'],
    [{ temperament: 'low', controversy: 'high' }, 'Volatile'],
    [{ temperament: 'low' }, 'Fiery'],
    [{ professionalism: 'low', pressure_handling: 'high' }, 'Casual'],
    [{ pressure_handling: 'high', ambition: 'high' }, 'Big-game player'],
    [{ ambition: 'high', loyalty: 'high' }, 'Ambitious'],
    [{ loyalty: 'high' }, 'Loyal'],
    [{ adaptability: 'high' }, 'Adaptable'],
    [{ versatility: 'high' }, 'Adaptable'],
    [{ sportsmanship: 'low', money_motivation: 'high' }, 'Balanced'],
    [{}, 'Balanced'],
  ];
  for (const [keys, word] of rows) {
    assert.equal(summaryWord(person(keys)), word, JSON.stringify(keys));
  }
  assert.equal(summaryWord(person({ professionalism: 'high', ambition: 'high' }, 'not_yet_known')), 'Not yet known');
  assert.equal(summaryWord(null), 'Not yet known');
  assert.equal(summaryWord({}), 'Not yet known');
});

test('standouts take the first three high or low values in the fixed order', () => {
  const p = person({ controversy: 'high', professionalism: 'high', ambition: 'high', temperament: 'high', loyalty: 'low' });
  assert.deepEqual(standouts(p), ['professional', 'ambitious', 'calm']);
  assert.deepEqual(standouts(person({ loyalty: 'low', versatility: 'high' })), ['restless', 'versatile']);
  assert.deepEqual(standouts(person()), []);
  assert.deepEqual(standouts(person({ professionalism: 'high' }, 'not_yet_known')), []);
});

test('sureness reads sure or tentative with the matches here, and nothing known before a match', () => {
  assert.equal(sureness(person({}, 'not_yet_known'), 0), 'no match for us yet');
  assert.equal(sureness(person({}, 'tentative'), 9), 'tentative · 9 matches here');
  assert.equal(sureness(person({}, 'firm'), 31), 'sure · 31 matches here');
  assert.equal(sureness(person({}, 'tentative'), 1), 'tentative · 1 match here');
});

test('the five mood words carry their marks; a steady player new at the club is settling in', () => {
  assert.deepEqual(Object.keys(MOOD_TEXT), ['confident', 'lifted', 'steady', 'low', 'rattled']);
  const marks = Object.keys(MOOD_TEXT).map((k) => [moodWord(k, 9).text, moodWord(k, 9).mark, moodWord(k, 9).token]);
  assert.deepEqual(marks, [
    ['Confident', '▲', '--good'],
    ['Lifted', '▲', '--good'],
    ['Steady', '=', null],
    ['Low', '▼', '--warn'],
    ['Rattled', '▼', '--warn'],
  ]);
  assert.equal(moodWord('steady', 0).text, 'Settling in');
  assert.equal(moodWord('lifted', 0).text, 'Lifted');
  assert.equal(moodWord('steady', null).text, 'Steady');
  const none = moodWord(undefined, 3);
  assert.deepEqual([none.known, none.text, none.mark], [false, 'Not yet known', null]);
});

test('no personality or mood word carries a digit', () => {
  const texts = [
    ...Object.values(PERSONALITY_TEXT).flatMap((w) => Object.values(w)),
    ...Object.values(MOOD_TEXT).map((m) => m.text),
    summaryWord(person({ professionalism: 'high', ambition: 'high' })),
    ...standouts(person({ professionalism: 'low', temperament: 'low', controversy: 'high' })),
  ];
  for (const text of texts) {
    assert.ok(!/\d/.test(text), text);
  }
});
