// The statistics panel shows every field of the stats message, equal to the stream value.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { NO_VALUE, STAT_ROWS, formatStat, stripFacts, toPanel } from '../src/lib/stats.js';
import { CAPTURED_STATS, statsMessage } from './helpers.js';

const KEYS = [
  'stats.possession_pct',
  'stats.shots',
  'stats.shots_on_target',
  'stats.xg',
  'stats.passes',
  'stats.pass_accuracy_pct',
  'stats.fouls',
  'stats.corners',
  'stats.offsides',
];

function assertPanelEquals(message) {
  const model = toPanel(message, ['A', 'B']);
  assert.deepEqual(
    model.map((r) => r.key),
    KEYS,
    'nine rows, in the order the screen lists them'
  );
  for (const row of model) {
    const [home, away] = message[row.key];
    assert.equal(row.home, home, row.key);
    assert.equal(row.away, away, row.key);
    // The shown text reads back as the same number.
    assert.equal(Number.parseFloat(row.homeText), home, `${row.key} ${row.homeText}`);
    assert.equal(Number.parseFloat(row.awayText), away, `${row.key} ${row.awayText}`);
    assert.ok(row.aria.includes(row.label) && row.aria.includes('A') && row.aria.includes('B'));
  }
}

test('every field of a factory stats message appears with an equal value', () => {
  assertPanelEquals(statsMessage());
});

test('every field of a stats line captured from a recorded match appears with an equal value', () => {
  assertPanelEquals(CAPTURED_STATS);
});

test('before the first stats message every row reads zero', () => {
  const model = toPanel(null);
  assert.equal(model.length, 9);
  for (const row of model) {
    assert.equal(row.home, 0);
    assert.equal(row.away, 0);
  }
  const text = Object.fromEntries(model.map((r) => [r.key, r.homeText]));
  assert.equal(text['stats.possession_pct'], '0.0%');
  assert.equal(text['stats.xg'], '0.00');
  assert.equal(text['stats.shots'], '0');
});

test('shares keep one decimal and a percent sign, expected goals two decimals', () => {
  assert.equal(formatStat(59.5, 'pct'), '59.5%');
  assert.equal(formatStat(60, 'pct'), '60.0%');
  assert.equal(formatStat(0.2, 'xg'), '0.20');
  assert.equal(formatStat(848, 'int'), '848');
  assert.equal(STAT_ROWS.length, 9);
});

test('the strip facts read the released message, and a dash before the first one', () => {
  const facts = stripFacts(statsMessage());
  assert.deepEqual(
    facts.map((f) => [f.label, f.value]),
    [
      ['xG', '1.42 – 0.87'],
      ['Possession', '52%'],
      ['Shots', '12 – 9'],
    ]
  );
  assert.deepEqual(
    stripFacts(null).map((f) => f.value),
    [NO_VALUE, NO_VALUE, NO_VALUE]
  );
});
