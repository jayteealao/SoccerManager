// A report's counts are the counts of the events the feed shows, per club.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { ROWS, ReportClock, reportModel } from '../src/lib/report.js';

const teams = [
  { 'team.id': 'home', 'team.name': 'Home FC' },
  { 'team.id': 'away', 'team.name': 'Away Town' },
];

let score = [0, 0];
function event(tick, type, team, extra = {}) {
  if (type === 'goal') {
    score = team === 'home' ? [score[0] + 1, score[1]] : [score[0], score[1] + 1];
  }
  return {
    type: 'event',
    tick,
    minute: Math.floor(tick / 3000),
    'event.type': type,
    'team.id': team,
    'home.score': score[0],
    'away.score': score[1],
    ...extra,
  };
}

function match() {
  score = [0, 0];
  return [
    event(1, 'kick-off', 'home'),
    event(900, 'throw-in', 'away'),
    event(1500, 'foul', 'home', { 'foul.advantage': true }),
    event(1600, 'foul', 'away'),
    event(1601, 'free-kick', 'home'),
    event(2000, 'card', 'away', { 'card.kind': 'yellow' }),
    event(3000, 'corner', 'home'),
    event(3100, 'goal', 'home', { commentary: 'In off the post.' }),
    event(3200, 'kick-off', 'away'),
    event(4000, 'offside', 'away'),
    event(4500, 'card', 'away', { 'card.kind': 'second-yellow' }),
    event(4800, 'goal-kick', 'away'),
    event(5000, 'penalty', 'home'),
    event(5001, 'goal', 'home'),
    event(6000, 'half-time', null),
    event(7000, 'goal', 'away'),
    event(12000, 'full-time', null),
  ];
}

test('every count equals a direct filter over the same events', () => {
  const events = match();
  const report = reportModel(events, 6000, teams);
  for (const row of report.rows) {
    const rule = ROWS.find((r) => r.id === row.id).counts;
    for (const [side, id] of ['home', 'away'].entries()) {
      const direct = events.filter((e) => e.tick <= 6000 && e['team.id'] === id && rule(e)).length;
      assert.equal(row.counts[side], direct, `${row.id} ${id}`);
    }
  }
  const counts = Object.fromEntries(report.rows.map((r) => [r.id, r.counts]));
  assert.deepEqual(counts.goals, [2, 0]);
  assert.deepEqual(counts.yellow, [0, 1]);
  assert.deepEqual(counts.red, [0, 1]);
  assert.deepEqual(counts.fouls, [1, 1]);
  assert.deepEqual(report.score, [2, 0]);
  assert.deepEqual(
    report.moments.map((m) => m.kind),
    ['Yellow card', 'Goal', 'Red card', 'Goal']
  );
});

test('the full-time report counts the whole match', () => {
  const report = reportModel(match(), 12000, teams);
  assert.deepEqual(report.score, [2, 1]);
  assert.deepEqual(report.rows.find((r) => r.id === 'goals').counts, [2, 1]);
});

test('each report opens once, when the rendered tick reaches its event', () => {
  const clock = new ReportClock();
  const events = match();
  assert.equal(clock.due(events, 5999), null);
  assert.deepEqual(clock.due(events, 6000), { kind: 'half-time', tick: 6000 });
  assert.equal(clock.due(events, 6500), null);
  assert.deepEqual(clock.due(events, 12000), { kind: 'full-time', tick: 12000 });
  assert.equal(clock.due(events, 12001), null);
  clock.reset();
  assert.equal(clock.due(events, 12001).kind, 'half-time');
});

test('a scrub past a break marks it shown, so no report opens behind the scrub', () => {
  const clock = new ReportClock();
  const events = match();
  clock.pass(events, 11_000);
  assert.equal(clock.due(events, 11_500), null, 'half time was passed by the scrub');
  assert.deepEqual(clock.due(events, 12_000), { kind: 'full-time', tick: 12_000 });
});
