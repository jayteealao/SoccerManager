// A sent-off player is parked beside the pitch and is not drawn; every drawn position is a
// fixed projection of the engine's metres into the match screen's pitch box.

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import { BOX, LENGTH, WIDTH, isParkingSpot, projection } from '../src/lib/pitch.js';
import { REPO_ROOT } from './helpers.js';

/// The engine's own parking spots: a Rust test in `crates/engine/src/pitch.rs` fails when
/// this file and `parking_spot` disagree, so the viewer is tested against the engine.
const { spots } = JSON.parse(
  readFileSync(path.join(REPO_ROOT, 'viewer/tests/data/parking-spots.json'), 'utf8'),
);

test('every one of the 22 parking spots is hidden', () => {
  assert.equal(spots.length, 22);
  for (const { team, slot, x_cm: x, y_cm: y } of spots) {
    assert.ok(isParkingSpot(x / 100, y / 100), `team ${team} slot ${slot}`);
  }
});

test('points on the pitch and on the touchline are drawn', () => {
  for (const [x, y] of [
    [0, 0],
    [15, -34],
    [15, 34],
    [-52.5, -34],
    [5, -37],
    [25, -37],
    [15, -36.9],
  ]) {
    assert.equal(isParkingSpot(x, y), false, `${x}, ${y}`);
  }
});

test('the ground fills the 742 by 312 box, with one scale per axis', () => {
  const map = projection();
  assert.deepEqual([BOX.width, BOX.height], [742, 312]);
  assert.equal(map.x(-LENGTH / 2), 1);
  assert.equal(map.x(LENGTH / 2), 741);
  assert.equal(map.y(-WIDTH / 2), 1);
  assert.ok(Math.abs(map.y(WIDTH / 2) - 311) < 1e-9);
  assert.equal(map.x(0), 371);
  assert.equal(map.y(0), 156);
  // The tilt is one fixed factor: a metre across is shorter on screen than a metre along.
  assert.ok(map.tilt > 0.6 && map.tilt < 0.7, `tilt ${map.tilt}`);
  // Equal steps in metres are equal steps on screen: the projection is linear.
  assert.ok(Math.abs(map.x(10) - map.x(0) - (map.x(20) - map.x(10))) < 1e-9);
  assert.ok(Math.abs(map.y(10) - map.y(0) - (map.y(20) - map.y(10))) < 1e-9);
});
