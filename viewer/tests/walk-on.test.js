// The walk-on before each half: the players walk from the tunnel to the places of the half's
// first tick, never faster than the walk, and a seek that passes the tick passes the walk too.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { COMPONENT_COUNT } from '../src/lib/decode.js';
import { WALK_ON_MAX_STEP, drawWalkOn, walkOnAt, walkOnDue, walkOnSeconds } from '../src/lib/walk-on.js';

const walkOn = {
  type: 'walk-on',
  period: 0,
  first_tick: 1,
  speed: 2,
  ticks: 1500,
  paths: [
    { player: 0, from: [-1, -34], to: [-48, 0] },
    { player: 11, from: [1, -34], to: [9, -5] },
    { player: 5, from: [-2.5, -34], to: [-2.5, -34] },
  ],
};

test('every player starts at the tunnel and ends at his place', () => {
  for (const [player, x, y] of walkOnAt(walkOn, 0)) {
    const path = walkOn.paths.find((p) => p.player === player);
    assert.deepEqual([x, y], path.from);
  }
  for (const [player, x, y] of walkOnAt(walkOn, walkOnSeconds(walkOn))) {
    const path = walkOn.paths.find((p) => p.player === player);
    assert.deepEqual([x, y], path.to);
  }
  assert.equal(walkOnSeconds(walkOn), 30);
});

test('no player walks faster than the walk, and one who arrives stands', () => {
  const step = 0.02;
  let before = walkOnAt(walkOn, 0);
  for (let t = step; t <= walkOnSeconds(walkOn) + step; t += step) {
    const now = walkOnAt(walkOn, t);
    for (let i = 0; i < now.length; i += 1) {
      const moved = Math.hypot(now[i][1] - before[i][1], now[i][2] - before[i][2]);
      assert.ok(moved <= walkOn.speed * step + 1e-9, `player ${now[i][0]} at ${t}`);
    }
    before = now;
  }
  // The near player (30.1 m away) has stood at his place since about 15 s.
  assert.deepEqual(walkOnAt(walkOn, 16)[1].slice(1), [9, -5]);
});

test('the drawn frame keeps the ball and every player who does not walk', () => {
  const base = new Int16Array(COMPONENT_COUNT).fill(7);
  const out = drawWalkOn(walkOn, 0, base, new Int16Array(COMPONENT_COUNT));
  assert.equal(out[0], 7);
  assert.equal(out[3 + 0 * 2], -100);
  assert.equal(out[4 + 0 * 2], -3400);
  assert.equal(out[3 + 1 * 2], 7, 'player 1 sends no walk');
  assert.equal(out[3 + 11 * 2], 100);
});

test('playback that runs into the first tick plays the walk-on; a seek past it does not', () => {
  const walkOns = new Map([[1, walkOn], [135_001, { ...walkOn, period: 1, first_tick: 135_001 }]]);
  assert.equal(walkOnDue(walkOns, 0, 1), walkOn);
  assert.equal(walkOnDue(walkOns, 134_998, 135_002).period, 1);
  assert.equal(walkOnDue(walkOns, 1, 2), null, 'once the tick is drawn');
  assert.equal(walkOnDue(walkOns, 100_000, 140_000), null, 'a seek passes it');
  assert.equal(walkOnDue(walkOns, 135_000 - WALK_ON_MAX_STEP, 135_001 + 1), null, 'a wide step');
  assert.equal(walkOnDue(new Map(), 0, 1), null, 'a replay with no walk-on');
});
