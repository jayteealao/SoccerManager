// A sent-off player is parked beside the pitch and is not drawn; every drawn position is a
// fixed projection of the engine's metres into the match screen's pitch box.

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import { BOX, DEFAULT_GROUND, LENGTH, WIDTH, groundOf, isParkingSpot, projection } from '../src/lib/pitch.js';
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

test('the default ground keeps the numbers it was drawn with before grounds had sizes', () => {
  for (const [width, height] of [
    [742, 312],
    [752, 290],
  ]) {
    const map = projection(width, height);
    // The former projection: one scale per axis that makes 105 by 68 fill the box.
    const sx = (width - 2) / 105;
    const sy = (height - 2) / 68;
    assert.equal(map.sx, sx);
    assert.equal(map.sy, sy);
    assert.equal(map.tilt, sy / sx);
    for (const m of [-52.5, -20, 0, 13.7, 52.5]) {
      assert.equal(map.x(m), 1 + (m + 52.5) * sx, `x(${m})`);
    }
    for (const m of [-34, -5.5, 0, 21, 34]) {
      assert.equal(map.y(m), 1 + (m + 34) * sy, `y(${m})`);
    }
    assert.deepEqual({ ...map.rect }, { left: 1, top: 1, width: width - 2, height: height - 2 });
  }
});

test('a 100 by 64 ground draws smaller, centred, at the same tilt and in proportion', () => {
  const full = projection();
  const map = projection(BOX.width, BOX.height, 100, 64);
  const { rect } = map;
  assert.equal(map.sx, full.sx);
  assert.equal(map.sy, full.sy);
  assert.equal(map.tilt, full.tilt);
  assert.ok(rect.width < full.rect.width && rect.height < full.rect.height);
  assert.ok(rect.left > 1 && rect.top > 1);
  assert.ok(rect.left + rect.width < BOX.width - 1 && rect.top + rect.height < BOX.height - 1);
  // Centred: the same margin on both sides of each axis.
  assert.ok(Math.abs(rect.left - (BOX.width - rect.left - rect.width)) < 1e-9);
  assert.ok(Math.abs(rect.top - (BOX.height - rect.top - rect.height)) < 1e-9);
  // In proportion: the drawn width over height is the ground's, times the tilt.
  assert.ok(Math.abs(rect.width / rect.height - (100 / 64) / full.tilt) < 1e-9);
  // The lines sit on the ground's edges, and the centre spot in the box's centre.
  assert.equal(map.x(-50), rect.left);
  assert.ok(Math.abs(map.x(50) - (rect.left + rect.width)) < 1e-9);
  assert.equal(map.y(-32), rect.top);
  assert.ok(Math.abs(map.x(0) - BOX.width / 2) < 1e-9);
  assert.ok(Math.abs(map.y(0) - BOX.height / 2) < 1e-9);
});

test('a ground too large for the box at the default scale shrinks to fit, its tilt kept', () => {
  const full = projection();
  for (const [length, width] of [
    [120, 90],
    [120, 68],
    [105, 90],
  ]) {
    const map = projection(BOX.width, BOX.height, length, width);
    const { rect } = map;
    assert.ok(Math.abs(map.tilt - full.tilt) < 1e-12, `${length} by ${width}`);
    assert.ok(rect.left >= 1 - 1e-9 && rect.top >= 1 - 1e-9, `${length} by ${width}`);
    assert.ok(rect.left + rect.width <= BOX.width - 1 + 1e-9, `${length} by ${width}`);
    assert.ok(rect.top + rect.height <= BOX.height - 1 + 1e-9, `${length} by ${width}`);
    // One side touches the box: the ground is as large as it can be.
    const touches =
      Math.abs(rect.width - (BOX.width - 2)) < 1e-9 || Math.abs(rect.height - (BOX.height - 2)) < 1e-9;
    assert.ok(touches, `${length} by ${width}`);
  }
});

test('a parking spot sits beside the ground the match is played on', () => {
  // A 64 m ground's touchline is at 32 m, so its parking spots are at -35 m.
  assert.ok(isParkingSpot(10, -35, 64));
  assert.equal(isParkingSpot(10, -37, 64), false);
  assert.ok(isParkingSpot(10, -37));
  assert.equal(isParkingSpot(10, -35), false);
});

test('a hello names its ground, or the default 105 by 68', () => {
  assert.deepEqual({ ...groundOf({ 'ground.length': 100, 'ground.width': 64 }) }, { length: 100, width: 64 });
  assert.deepEqual({ ...groundOf({ 'ground.length': 110 }) }, { length: 110, width: 68 });
  assert.deepEqual({ ...groundOf({}) }, { ...DEFAULT_GROUND });
  assert.deepEqual({ ...groundOf(null) }, { length: LENGTH, width: WIDTH });
});
