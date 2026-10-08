// The hidden values in words: every word key the engine bands has its text; the three states
// read "not yet known", a tentative word and a firm word with the matches seen; no result
// carries a number for the value itself; the build words.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import { BUILD_TEXT, buildWord, HIDDEN_TEXT, hiddenWord, keyWords } from '../src/lib/hidden-words.js';
import { REPO_ROOT } from './helpers.js';

const TUNING = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/tuning.json'), 'utf8'));

test('every word key in the tuning file has its text, in the same order, best first', () => {
  for (const [name, bands] of Object.entries(TUNING.hidden.words)) {
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
