// The other grounds on the player's clock: what the list shows at the rendered tick through
// a pause, a speed change, a rewind and a skip, the 8-second outline of a new goal, the late
// and unavailable words, full time elsewhere, a ground behind the clock, and the report's
// final summary. A planted control shows the reveal check can fail.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import {
  HIGHLIGHT_TICKS,
  addEvent,
  addProgress,
  allEnded,
  eventStamp,
  finalSummary,
  groundsAt,
  headline,
  minuteLabel,
  newMatchday,
} from '../src/lib/matchday.js';
import { TICKS_PER_MINUTE } from '../src/lib/skip.js';

const M = TICKS_PER_MINUTE;
const team = (id, name) => ({ 'team.id': id, 'team.name': name, 'team.kit.primary': '#0f5c63', 'team.kit.secondary': '#ffffff', roster: [] });

const MESSAGE = {
  type: 'matchday',
  round: 1,
  fixtures: [
    { fixture: 0, home: team('cu', 'Castlemere United'), away: team('gw', 'Greywater') },
    { fixture: 1, home: team('ka', 'Kelder Athletic'), away: team('mt', 'Millbridge Town') },
    { fixture: 2, home: team('df', 'Dunmore FC'), away: team('oc', 'Oldfield City') },
    { fixture: 3, home: team('hv', 'Harlow Vale'), away: team('sv', 'Sporting Veira') },
  ],
};

const goal = (fixture, minute, side, score, scorer, extra = {}) => ({
  type: 'ground-event',
  fixture,
  tick: minute * M + 100,
  kind: 'goal',
  minute,
  side,
  scorer,
  score,
  ...extra,
});
const period = (fixture, tick, kind, minute, score, extra = {}) => ({
  type: 'ground-event',
  fixture,
  tick,
  kind,
  minute,
  score,
  ...extra,
});

/// A recorded matchday: goals at 12', 40', 55' and 67', half time at 45+1, full time at
/// 90+2 in fixture 0 and 90+4 in fixture 2.
function recorded() {
  let day = newMatchday(MESSAGE);
  const events = [
    goal(0, 12, 'home', [1, 0], 'Tomas Okafor'),
    goal(2, 40, 'away', [0, 1], 'Lew Brand'),
    period(0, 45 * M + 3_200, 'half-time', 45, [1, 0], { added: 2 }),
    period(1, 45 * M + 3_200, 'half-time', 45, [0, 0], { added: 2 }),
    period(2, 45 * M + 3_200, 'half-time', 45, [0, 1], { added: 2 }),
    period(3, 45 * M + 3_200, 'half-time', 45, [0, 0], { added: 2 }),
    period(0, 46 * M + 3_200, 'second-half', 45, [1, 0]),
    period(1, 46 * M + 3_200, 'second-half', 45, [0, 0]),
    period(2, 46 * M + 3_200, 'second-half', 45, [0, 1]),
    period(3, 46 * M + 3_200, 'second-half', 45, [0, 0]),
    goal(1, 55, 'home', [1, 0], 'Ade Harrow'),
    goal(0, 67, 'away', [1, 1], 'Kit Mells'),
    period(0, 93 * M, 'full-time', 90, [1, 1], { added: 2 }),
    period(1, 93 * M + 500, 'full-time', 90, [1, 0], { added: 3 }),
    period(2, 95 * M, 'full-time', 90, [0, 1], { added: 4 }),
    period(3, 93 * M + 900, 'full-time', 90, [0, 0], { added: 3 }),
  ];
  for (const e of events) {
    day = addEvent(day, e, 0);
  }
  return addProgress(day, { tick: 0, reached: [99 * M, 99 * M, 99 * M, 99 * M] });
}

/// Every shown event of the list at `tick` sits at or before `tick`.
function shownTicks(day, tick, opts) {
  const grounds = groundsAt(day, tick, opts);
  return grounds.rows.map((row, i) => {
    const shown = day.events[i].filter((e) => e.tick <= tick);
    return { row, last: shown.at(-1) ?? null };
  });
}

test('before kick-off every fixture shows 0-0 KO, and an empty round says so', () => {
  const day = recorded();
  const at = groundsAt(day, 0);
  assert.equal(at.state, 'rows');
  assert.equal(at.rows.length, 4);
  for (const row of at.rows) {
    assert.deepEqual(row.score, [0, 0]);
    assert.equal(row.minute, 'KO');
    assert.equal(row.started, false);
  }
  const none = groundsAt(newMatchday({ round: 1, fixtures: [] }), 1000);
  assert.equal(none.state, 'none');
  assert.equal(none.words, 'No other matches this matchday.');
  const replay = groundsAt(day, 1000, { stored: true });
  assert.equal(replay.state, 'none');
  assert.match(replay.words, /replay keeps no other grounds/);
});

test('a pause holds the list: the same rendered tick shows the same rows', () => {
  const day = recorded();
  const held = 30 * M;
  const first = groundsAt(day, held);
  for (let i = 0; i < 5; i += 1) {
    assert.deepEqual(groundsAt(day, held), first);
  }
  assert.deepEqual(first.rows[0].score, [1, 0]);
  assert.deepEqual(first.rows[2].score, [0, 0], 'the 40th-minute goal waits for the clock');
});

test('at 8x the rendered tick steps 8 ticks a frame and no event shows before its tick', () => {
  const day = recorded();
  for (let tick = 0; tick <= 96 * M; tick += 8 * 50) {
    for (const { row, last } of shownTicks(day, tick)) {
      if (last) {
        assert.ok(last.tick <= tick);
        if (!row.unavailable) {
          assert.deepEqual(row.score, last.score);
        }
      } else {
        assert.deepEqual(row.score, [0, 0]);
      }
    }
  }
});

test('planted control: an event one tick past the rendered tick stays hidden, and drawing a tick late would show it', () => {
  const day = recorded();
  const scored = 12 * M + 100;
  assert.deepEqual(groundsAt(day, scored - 1).rows[0].score, [0, 0]);
  // The control: the check above fails if the list is drawn one tick ahead.
  assert.notDeepEqual(groundsAt(day, scored - 1 + 1).rows[0].score, [0, 0]);
});

test('a new goal is outlined for 8 seconds of the clock with its words, then settles', () => {
  const day = recorded();
  const scored = 67 * M + 100;
  const at = groundsAt(day, scored).rows[0];
  assert.equal(at.flag, 'new');
  assert.equal(at.tag, 'GOAL');
  assert.equal(at.words, "Mells 67'. Level at 1–1.");
  assert.equal(groundsAt(day, scored + HIGHLIGHT_TICKS - 1).rows[0].flag, 'new');
  assert.equal(groundsAt(day, scored + HIGHLIGHT_TICKS).rows[0].flag, null);
  const lead = groundsAt(day, 12 * M + 150).rows[0];
  assert.equal(lead.words, "Okafor 12'. Castlemere lead.");
});

test('a rewind from 67 to 40 hides later goals; forward again they show with no outline', () => {
  const day = recorded();
  const seeks = [{ lo: 40 * M, hi: 67 * M + 600, n: 1 }];
  const back = groundsAt(day, 40 * M, { seeks });
  assert.deepEqual(back.rows[0].score, [1, 0]);
  assert.deepEqual(back.rows[1].score, [0, 0], 'the 55th-minute goal is hidden');
  assert.equal(back.rows[0].minute, "40'");
  const again = groundsAt(day, 55 * M + 150, { seeks });
  assert.deepEqual(again.rows[1].score, [1, 0]);
  assert.equal(again.rows[1].flag, null, 'no outline on a goal shown before');
  // A seek forward over a goal never outlines it.
  const over = groundsAt(day, 55 * M + 200, { seeks: [{ lo: 50 * M, hi: 55 * M + 200, n: 1 }] });
  assert.equal(over.rows[1].flag, null);
  // A short rewind that leaves the goal shown keeps its outline.
  const short = groundsAt(day, 55 * M + 200, { seeks: [{ lo: 55 * M + 200, hi: 55 * M + 300, n: 1 }] });
  assert.equal(short.rows[1].flag, 'new');
});

test('a skip holds the rows at the skip point while it is decided and plays; the report shows all final', () => {
  const day = recorded();
  const from = 50 * M;
  for (const state of ['deciding', 'playing', 'storing']) {
    const held = groundsAt(day, 80 * M, { skip: { state, from } });
    assert.deepEqual(held.rows[1].score, [0, 0], `${state}: the 55th-minute goal is not shown`);
    assert.equal(held.rows[0].minute, "47'", "the ground's own clock at the skip point");
  }
  const report = groundsAt(day, Number.MAX_SAFE_INTEGER, { skip: { state: 'ready', from }, final: true });
  assert.deepEqual(
    report.rows.map((r) => r.minute),
    ['FT', 'FT', 'FT', 'FT']
  );
  assert.ok(report.rows.every((r) => r.flag === null));
  assert.equal(finalSummary(day), '4 of 4 final');
  assert.equal(allEnded(day), true);
  assert.equal(headline(day), 'Castlemere 1–1');
});

test('the minute follows each ground: half time, the second half, added time, full time elsewhere', () => {
  const day = recorded();
  assert.equal(groundsAt(day, 45 * M + 3_300).rows[0].minute, 'HT');
  assert.equal(groundsAt(day, 45 * M + 1_000).rows[0].minute, "45+1'");
  assert.equal(groundsAt(day, 60 * M).rows[0].minute, "57'", 'the second half kicked off 2 minutes after the 45th');
  const late = groundsAt(day, 94 * M);
  assert.equal(late.rows[0].minute, 'FT', 'less added time elsewhere ends first');
  assert.equal(late.rows[0].ended, true);
  assert.equal(late.rows[2].minute, "90+2'");
  assert.equal(late.rows[2].ended, false);
  assert.equal(minuteLabel(10 * M, [], 90), "10'");
  assert.equal(minuteLabel(46 * M, [], 90), "45+2'");
});

test('a ground behind the clock shows its own minute, and a late goal says when it showed', () => {
  let day = newMatchday(MESSAGE);
  day = addEvent(day, period(1, 46 * M + 3_200, 'second-half', 45, [0, 0]), 0);
  day = addProgress(day, { tick: 61 * M, reached: [61 * M, 58 * M, 61 * M, 61 * M] });
  const behind = groundsAt(day, 61 * M);
  assert.equal(behind.rows[1].behind, true);
  assert.equal(behind.rows[1].minute, "55'", 'its own clock at the tick it reached');
  day = addEvent(day, goal(1, 55, 'home', [1, 0], 'Ade Harrow', { late: true }), 61 * M);
  const shown = groundsAt(day, 61 * M + 50).rows[1];
  assert.equal(shown.flag, 'late');
  assert.equal(shown.tag, 'GOAL · LATE');
  assert.equal(shown.words, "Harrow 55', shown at your 61'.");
  assert.equal(groundsAt(day, 61 * M + HIGHLIGHT_TICKS + 1).rows[1].flag, null);
});

test('a failed ground shows its mark and words; the others play on', () => {
  let day = newMatchday(MESSAGE);
  day = addEvent(day, period(2, 34 * M + 10, 'unavailable', 34, [0, 0]), 0);
  day = addEvent(day, goal(1, 50, 'home', [1, 0], 'Ade Harrow'), 0);
  const at = groundsAt(day, 52 * M);
  assert.equal(at.rows[2].unavailable, true);
  assert.equal(at.rows[2].minute, '!');
  assert.equal(at.rows[2].score, null);
  assert.equal(at.rows[2].failedAt, "34'");
  assert.deepEqual(at.rows[1].score, [1, 0]);
  assert.equal(finalSummary(day), '0 of 4 final');
  assert.equal(allEnded(day), false);
});

test('a catch-up repeat is dropped, and an event for no fixture is ignored', () => {
  let day = recorded();
  const before = day;
  day = addEvent(day, goal(0, 12, 'home', [1, 0], 'Tomas Okafor'), 50 * M);
  assert.equal(day, before);
  day = addEvent(day, goal(9, 12, 'home', [1, 0], 'Nobody'), 0);
  assert.equal(day, before);
  assert.equal(eventStamp({ minute: 45, added: 2 }), "45+2'");
  assert.equal(finalSummary(null), 'none');
});
