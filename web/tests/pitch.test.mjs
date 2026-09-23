// A sent-off player is parked beside the pitch and is not drawn.

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { isParkingSpot } from '../pitch.mjs';

/// The engine's own parking spots: a Rust test in `crates/engine/src/pitch.rs` fails when
/// this file and `parking_spot` disagree, so the viewer is tested against the engine.
const { spots } = JSON.parse(
  readFileSync(new URL('./data/parking-spots.json', import.meta.url), 'utf8'),
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
