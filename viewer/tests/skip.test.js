// Skip to the result: the steps, the decision's facts, the NOT LIVE rule and the hatched
// span, as the four skip surfaces render them.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import {
  TICKS_PER_MINUTE,
  decisionFacts,
  hatchSpan,
  minuteOf,
  notLive,
  skipSteps,
  skippedWord,
  totalMinutes,
} from '../src/lib/skip.js';

const AT_67_12 = 67 * TICKS_PER_MINUTE + 12 * 50;

test('the minute and the match length follow the clock', () => {
  assert.equal(minuteOf(90_000), 30);
  assert.equal(minuteOf(AT_67_12), 67);
  assert.equal(minuteOf(-5), 0);
  assert.equal(totalMinutes(270_000), 90);
  assert.equal(totalMinutes(30_000), 10, 'a ten-minute test match');
  assert.equal(totalMinutes(400_000, true), 120, 'a knockout match plays to 120');
});

test('while the engine plays, step 2 is current and names the newest minute of 90', () => {
  const steps = skipSteps(AT_67_12, 82 * TICKS_PER_MINUTE, 90, 'playing');
  assert.deepEqual(
    steps.map((s) => [s.label, s.state]),
    [
      ['Freeze the match at 67:12', 'done'],
      ['Play 67:12 to full time', 'current'],
      ['Write the report', 'pending'],
      ['Store the whole match', 'pending'],
    ]
  );
  assert.equal(steps[1].word, "82' of 90 · at full speed");
  assert.ok(steps[1].progress > 60 && steps[1].progress < 70, `${steps[1].progress}`);
  assert.equal(steps[0].word, 'The state and the random draws are kept');
});

test('a knockout match counts to 120, and the minute never passes the length', () => {
  assert.equal(skipSteps(90_000, 100 * TICKS_PER_MINUTE, 120, 'playing')[1].word, "100' of 120 · at full speed");
  assert.equal(skipSteps(90_000, 95 * TICKS_PER_MINUTE, 90, 'playing')[1].word, "90' of 90 · at full speed");
});

test('storing makes step 4 current; ready makes every step done', () => {
  assert.deepEqual(
    skipSteps(90_000, 270_000, 90, 'storing').map((s) => s.state),
    ['done', 'done', 'done', 'current']
  );
  assert.deepEqual(
    skipSteps(90_000, 270_000, 90, 'ready').map((s) => s.state),
    ['done', 'done', 'done', 'done']
  );
});

test('a moment is NOT LIVE only after the skip point', () => {
  assert.equal(notLive({ tick: 90_001 }, 90_000), true);
  assert.equal(notLive({ tick: 90_000 }, 90_000), false, 'the skip tick itself was watched');
  assert.equal(notLive({ tick: 10 }, 90_000), false);
  assert.equal(notLive({ tick: 200_000 }, null), false, 'with no skip nothing is marked');
});

test('the hatched span runs from the skip point to the end of the timeline', () => {
  assert.deepEqual(hatchSpan(0, 270_000, 1140, 10), { x: 10, width: 1140 });
  assert.deepEqual(hatchSpan(135_000, 270_000, 1140, 10), { x: 580, width: 570 });
  assert.deepEqual(hatchSpan(400_000, 270_000, 1140, 10), { x: 1150, width: 0 }, 'past the end');
  assert.equal(hatchSpan(null, 270_000, 1140, 10), null);
  assert.equal(skippedWord(AT_67_12), 'skipped from 67:12');
});

test('the decision names the paused clock, the score, the substitutes and the queue', () => {
  const base = {
    from: AT_67_12,
    score: [1, 1],
    scorers: ["Oduya 23'", "Hask 58'"],
    subsUsed: 2,
    limit: 5,
    queued: ['Hart → Aydin'],
    engineVersion: '0.3.0',
    period: '2nd half',
    mentality: 'Positive',
  };
  const facts = decisionFacts(base);
  assert.deepEqual(
    facts.strip.map((f) => [f.value, f.label]),
    [
      ['Play paused · 67:12', 'You chose Skip to result'],
      ["1 – 1 · 67'", "Oduya 23' · Hask 58'"],
      ['2 of 5', 'Substitutes used'],
      ['1 queued', 'Hart → Aydin at the next stoppage'],
      ['Engine 0.3.0', 'Plays the rest at full speed'],
    ]
  );
  assert.deepEqual(facts.rows, [
    { key: 'Clock', value: '67:12 · 2nd half' },
    { key: 'Queued change', value: 'Hart → Aydin' },
    { key: 'Mentality', value: 'Positive' },
  ]);
  const none = decisionFacts({ ...base, queued: [], scorers: ['', ''], score: [0, 0] });
  assert.deepEqual(none.strip[3], { value: 'Nothing queued', label: 'No change waits for a stoppage' });
  assert.equal(none.strip[1].label, 'No goals yet');
  assert.equal(none.rows[1].value, 'None');
  const two = decisionFacts({ ...base, queued: ['Hart → Aydin', 'Mentality: Attacking'] });
  assert.equal(two.strip[3].label, 'Hart → Aydin and 1 more at the next stoppage');
});
