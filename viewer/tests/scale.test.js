// The rating scale: tenths of 1 to 20 shown as whole numbers, and an old hello (protocol 3,
// ratings on 1 to 100) lifted to tenths before any screen reads it.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import { readReplay } from '../src/lib/replay-file.js';
import {
  bandOf,
  HIDDEN_KEYS,
  LEVELS_PROTOCOL,
  liftHello,
  ratingBand,
  ratingText,
  TENTHS_PROTOCOL,
  WORDS_PROTOCOL,
  wholeOf,
} from '../src/lib/scale.js';
import { REPO_ROOT } from './helpers.js';

const OLD_HELLO = path.join(REPO_ROOT, 'viewer/tests/data/hello-protocol-3.json');

test('a rating in tenths shows as its whole number, kept to 1 to 20', () => {
  assert.equal(wholeOf(125), 13);
  assert.equal(wholeOf(124), 12);
  assert.equal(wholeOf(200), 20);
  assert.equal(wholeOf(10), 1);
  // A converted old value (0.2 to 0.8) and a missing one show as 1, never 0.
  assert.equal(wholeOf(2), 1);
  assert.equal(wholeOf(8), 1);
  assert.equal(wholeOf(undefined), 1);
  assert.equal(wholeOf(255), 20);
});

test('a protocol 3 hello is lifted to tenths, and every squad value shows on 1 to 20', () => {
  const hello = JSON.parse(fs.readFileSync(OLD_HELLO, 'utf8'));
  const before = JSON.stringify(hello);
  const lifted = liftHello(hello);
  assert.equal(JSON.stringify(hello), before, 'the hello itself is never changed');
  const [keeper, back] = lifted.teams[0].squad;
  assert.equal(keeper['player.natural_fitness'], 142);
  // The old resistance figure is dropped; the hidden values are not yet known.
  assert.ok(!('player.injury_resistance' in keeper));
  for (const key of HIDDEN_KEYS) {
    assert.deepEqual(keeper[key], { confidence: 'not_yet_known' });
  }
  assert.deepEqual(keeper.role_fit, [160, 24]);
  assert.deepEqual(back.role_fit, [8, 194]);
  // The lifted copy keeps the protocol its frames were written in.
  assert.equal(lifted['protocol.version'], 3);
  for (const p of lifted.teams[0].squad) {
    for (const v of [p['player.natural_fitness'], ...p.role_fit]) {
      const shown = wholeOf(v);
      assert.ok(Number.isInteger(shown) && shown >= 1 && shown <= 20, `${v} shows as ${shown}`);
    }
  }
  assert.equal(wholeOf(keeper['player.natural_fitness']), 14);
  assert.equal(wholeOf(back['player.natural_fitness']), 1);
});

test('a protocol 4 hello keeps its tenths and shows no number under a hidden key', () => {
  const hello = JSON.parse(fs.readFileSync(OLD_HELLO, 'utf8'));
  hello['protocol.version'] = TENTHS_PROTOCOL;
  const lifted = liftHello(hello);
  const [keeper, back] = lifted.teams[0].squad;
  // Already in tenths: nothing is doubled.
  assert.equal(keeper['player.natural_fitness'], 71);
  assert.deepEqual(back.role_fit, [4, 97]);
  for (const p of lifted.teams[0].squad) {
    assert.ok(!('player.injury_resistance' in p), 'the old figure is dropped');
    for (const key of HIDDEN_KEYS) {
      assert.deepEqual(p[key], { confidence: 'not_yet_known' }, 'no figure becomes a word');
    }
  }
});

test('a protocol 5 hello is returned as it is', () => {
  const hello = JSON.parse(fs.readFileSync(OLD_HELLO, 'utf8'));
  hello['protocol.version'] = WORDS_PROTOCOL;
  assert.equal(liftHello(hello), hello);
});

test('the committed version-4 replay opens with its protocol 3 hello and plays', async () => {
  const bytes = new Uint8Array(
    fs.readFileSync(path.join(REPO_ROOT, 'viewer/tests/data/one-minute-v4.smfx'))
  );
  const read = await readReplay(bytes);
  assert.equal(read.hello.type, 'hello');
  assert.equal(read.hello['protocol.version'], 3);
  assert.equal(read.store.protocol, 3);
  assert.equal(read.ticks, 3000);
  // Every squad value the hello carries (none for a match no page managed) shows on 1 to 20.
  for (const team of read.hello.teams) {
    for (const p of team.squad ?? []) {
      for (const v of [p['player.natural_fitness'], ...(p.role_fit ?? [])]) {
        assert.ok(wholeOf(v) >= 1 && wholeOf(v) <= 20);
      }
    }
  }
});

test('the bands and the match-rating chips follow the sketch thresholds', () => {
  assert.deepEqual([20, 15, 14, 11, 10, 7, 6, 1].map(bandOf), ['v4', 'v4', 'v3', 'v3', 'v2', 'v2', 'v1', 'v1']);
  assert.deepEqual([8.2, 7.5, 7.4, 6.8, 6.7, 6.5, 6.4].map(ratingBand), ['a', 'a', 'b', 'b', 'c', 'c', 'd']);
  assert.equal(ratingText(7), '7.0');
  assert.equal(LEVELS_PROTOCOL, 6);
});

test('a protocol 6 hello is returned as it is', () => {
  const hello = { 'protocol.version': 6, teams: [{ squad: [{ 'player.attributes': { pace: 145 }, 'player.level': 150 }] }] };
  assert.equal(liftHello(hello), hello);
});
