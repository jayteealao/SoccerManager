// A sent-off player is parked beside the pitch and is not drawn.

import assert from 'node:assert/strict';
import test from 'node:test';

import { isParkingSpot } from '../pitch.mjs';

/// `parking_spot(team, slot)` from `crates/engine/src/pitch.rs`, in wire centimetres.
function parkingSpot(team, slot) {
  const side = team === 0 ? -1 : 1;
  return [Math.round(side * (10 + slot) * 100), Math.round(-(34 + 3) * 100)];
}

test('every one of the 22 parking spots is hidden', () => {
  for (const team of [0, 1]) {
    for (let slot = 0; slot < 11; slot += 1) {
      const [x, y] = parkingSpot(team, slot);
      assert.ok(isParkingSpot(x / 100, y / 100), `team ${team} slot ${slot}`);
    }
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
