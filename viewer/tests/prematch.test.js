// The read-only Pre-match line-ups: the risk word, the kick-off places, the two sheets and the
// rule-pack checks, all read from the hello and the lineup set on Tactics.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import {
  formationName,
  formationSlots,
  kickOffDots,
  kickOffSheet,
  prematchLead,
  riskWord,
  rosterSheet,
  rulePackRows,
  squadSheet,
} from '../src/lib/prematch.js';
import { REPO_ROOT } from './helpers.js';

const SCHEMA = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/tactics.json'), 'utf8'));

test('the risk word follows injury resistance: High below 40, Raised to 59, Low from 60', () => {
  assert.deepEqual(riskWord(0), { word: 'High', tone: 'bad' });
  assert.deepEqual(riskWord(39), { word: 'High', tone: 'bad' });
  assert.deepEqual(riskWord(40), { word: 'Raised', tone: 'warn' });
  assert.deepEqual(riskWord(59), { word: 'Raised', tone: 'warn' });
  assert.deepEqual(riskWord(60), { word: 'Low', tone: 'good' });
  assert.deepEqual(riskWord(100), { word: 'Low', tone: 'good' });
  assert.equal(riskWord(undefined).word, 'High', 'no figure reads as the worst case');
});

/// The engine's kick-off places (crates/engine/src/rules/restart.rs, kick_off_position) in
/// metres from the home goal line and metres across, worked by hand from the tactics file: a
/// slot deeper than 1 m short of halfway keeps its place, any other stands 1 m inside its own
/// half, and the away side's length is mirrored.
const ENGINE = {
  '4-4-2': {
    home: [[5, 0], [25, -22], [25, -8], [25, 8], [25, 22], [45, -24], [45, -8], [45, 8], [45, 24], [51.5, -8], [51.5, 8]],
    away: [[100, 0], [80, -22], [80, -8], [80, 8], [80, 22], [60, -24], [60, -8], [60, 8], [60, 24], [53.5, -8], [53.5, 8]],
  },
  '4-3-3': {
    home: [[5, 0], [25, -22], [25, -8], [25, 8], [25, 22], [43, -15], [38, 0], [43, 15], [51.5, -22], [51.5, 22], [51.5, 0]],
    away: [[100, 0], [80, -22], [80, -8], [80, 8], [80, 22], [62, -15], [67, 0], [62, 15], [53.5, -22], [53.5, 22], [53.5, 0]],
  },
};

test('kick-off places match the engine rule for 4-4-2 and 4-3-3 on both sides', () => {
  for (const [name, sides] of Object.entries(ENGINE)) {
    const slots = formationSlots(SCHEMA, name);
    for (const [side, expected] of [[0, sides.home], [1, sides.away]]) {
      const dots = kickOffDots(slots, side);
      dots.forEach((dot, i) => {
        const [x, y] = expected[i];
        assert.ok(Math.abs(dot.left - (x / 105) * 100) < 1e-9, `${name} side ${side} slot ${i} x`);
        assert.ok(Math.abs(dot.top - ((y + 34) / 68) * 100) < 1e-9, `${name} side ${side} slot ${i} y`);
      });
      // No dot stands in the other half.
      assert.ok(dots.every((d) => (side === 0 ? d.left < 50 : d.left > 50)), `${name} side ${side}`);
    }
  }
});

function squad() {
  return Array.from({ length: 20 }, (_, i) => ({
    'player.id': `h-${i}`,
    'player.name': `Home Player${i}`,
    'player.shirt': i + 1,
    'player.position': i === 0 ? 'GK' : 'CM',
    'player.injury_resistance': [70, 50, 20][i % 3],
  }));
}

test('the home sheet is the lineup set on Tactics, with each starter’s risk word', () => {
  const sheet = squadSheet(squad(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 19], [10, 11]);
  assert.equal(sheet.eleven.length, 11);
  assert.equal(sheet.eleven[10].name, 'Home Player19', 'the manager’s pick, not the hello roster');
  assert.equal(sheet.eleven[10].surname, 'Player19');
  assert.deepEqual(sheet.eleven.slice(0, 3).map((r) => r.risk.word), ['Low', 'Raised', 'High']);
  assert.deepEqual(sheet.bench.map((r) => r.squad), [10, 11]);
});

test('the other sheet reads the hello roster and carries no risk', () => {
  const roster = Array.from({ length: 14 }, (_, i) => ({
    'player.id': `a-${i}`,
    'player.name': `Away ${i}`,
    'player.shirt': i + 1,
    'player.position': 'CB',
    'player.squad_index': i,
  }));
  const sheet = rosterSheet({ roster });
  assert.equal(sheet.eleven.length, 11);
  assert.equal(sheet.bench.length, 3);
  assert.ok(sheet.eleven.every((r) => r.risk === null));
});

test('the kick-off sheet places both elevens by their named formations', () => {
  const eleven = squadSheet(squad(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10], []).eleven;
  const [home, away] = kickOffSheet(SCHEMA, [
    { formation: '4-4-2', eleven },
    { formation: '4-3-3', eleven },
  ]);
  assert.equal(home.length, 11);
  assert.equal(away.length, 11);
  assert.ok(home[0].keeper && away[0].keeper);
  assert.equal(home[9].surname, 'Player9');
  assert.deepEqual(kickOffSheet(SCHEMA, [{ formation: 'W-M', eleven }])[0], [], 'an unknown shape draws none');
  assert.deepEqual(kickOffSheet(SCHEMA, [{ formation: '4-4-2', eleven: eleven.slice(0, 5) }])[0], []);
});

test('a formation name comes from the hello, or from the home setup when an older engine sent none', () => {
  assert.equal(formationName({ formation: '4-3-3' }, SCHEMA), '4-3-3');
  assert.equal(formationName({ setup: { formation: 2 } }, SCHEMA), SCHEMA.formations[2].name);
  assert.equal(formationName({}, SCHEMA), '');
});

const HELLO = {
  teams: [{ 'team.name': 'Ashford' }, { 'team.name': 'Varrow' }],
  tactics: { ai: { bench_size: 7 } },
  substitutions: {
    limit: 5,
    windows: 3,
    extra_substitutions: 1,
    extra_windows: 1,
    windows_exempt: ['half_time'],
  },
};

test('the rule-pack checks read the hello’s substitution rules, bench size and knockout flag', () => {
  const rows = rulePackRows(HELLO);
  assert.deepEqual(
    rows.live.map((r) => r.text),
    [
      '5 substitutes in 3 windows; half-time uses none',
      'One more substitute and window in extra time',
      '7 on the bench',
    ]
  );
  assert.equal(rows.level, 'The match ends level');
  assert.equal(rulePackRows({ ...HELLO, knockout: true }).level, 'Extra time, then penalties');
  assert.equal(rows.later.length, 4, 'the rows with no engine rule are LATER');
  const plain = rulePackRows({ substitutions: { limit: 3, windows: 1 } });
  assert.deepEqual(plain.live.map((r) => r.text), ['3 substitutes in 1 window']);
});

test('the strip lead names the fixture and what a level score means', () => {
  assert.deepEqual(prematchLead(HELLO), {
    value: 'Ashford v Varrow',
    label: 'One match: it can end level',
  });
  assert.equal(prematchLead({ ...HELLO, knockout: true }).label, 'Knockout: extra time and penalties if level');
});
