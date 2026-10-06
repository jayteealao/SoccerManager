// The rating scale: tenths of 1 to 20 shown as whole numbers, and an old hello (protocol 3,
// ratings on 1 to 100) lifted to tenths before any screen reads it.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import { readReplay } from '../src/lib/replay-file.js';
import { liftHello, TENTHS_PROTOCOL, wholeOf } from '../src/lib/scale.js';
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
  assert.equal(keeper['player.injury_resistance'], 128);
  assert.deepEqual(keeper.role_fit, [160, 24]);
  assert.deepEqual(back.role_fit, [8, 194]);
  // The lifted copy keeps the protocol its frames were written in.
  assert.equal(lifted['protocol.version'], 3);
  for (const p of lifted.teams[0].squad) {
    for (const v of [p['player.natural_fitness'], p['player.injury_resistance'], ...p.role_fit]) {
      const shown = wholeOf(v);
      assert.ok(Number.isInteger(shown) && shown >= 1 && shown <= 20, `${v} shows as ${shown}`);
    }
  }
  assert.equal(wholeOf(keeper['player.natural_fitness']), 14);
  assert.equal(wholeOf(back['player.natural_fitness']), 1);
});

test('a protocol 4 hello is already in tenths and is returned as it is', () => {
  const hello = JSON.parse(fs.readFileSync(OLD_HELLO, 'utf8'));
  hello['protocol.version'] = TENTHS_PROTOCOL;
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
