// The touchline mark is drawn, never loaded. Its geometry comes from the tile size alone,
// so the same size always gives the same tile and a crest cannot drift from the pitch.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import {
  LINE_AT,
  MONO,
  PITCHSIDE,
  PRIMARY,
  RADIUS_AT,
  REVERSED,
  colours,
  markGeometry,
  markSvg,
} from '../src/lib/mark.js';

test('the geometry is deterministic for a given size', () => {
  for (const size of [16, 28, 32, 96, 256]) {
    assert.deepEqual(markGeometry(size), markGeometry(size));
  }
});

test('the line sits at 58 percent and the radius at 21 percent of the tile', () => {
  const g = markGeometry(100);
  assert.equal(LINE_AT, 0.58);
  assert.equal(RADIUS_AT, 0.21);
  // Binary floating point puts `size * 0.58` a fraction below 58, which is a pixel
  // position and not an identity, so the check is a tolerance and not equality.
  assert.ok(Math.abs(g.lineY - 58) < 1e-9, String(g.lineY));
  assert.ok(Math.abs(g.radius - 21) < 1e-9, String(g.radius));
});

test('the marker, the ball and the marking all sit on the field, above the line', () => {
  for (const size of [16, 32, 256]) {
    const g = markGeometry(size);
    for (const [name, o] of [
      ['marker', g.marker],
      ['ball', g.ball],
    ]) {
      assert.ok(o.y + o.r <= g.lineY, `${name} crosses the line at size ${size}`);
      assert.ok(o.x - o.r >= 0 && o.x + o.r <= size, `${name} leaves the tile at ${size}`);
    }
    assert.ok(g.arc.r > 0, 'the faint pitch marking has a radius');
  }
});

test('the four variants are distinct and none is pure black or pure white', () => {
  const variants = { PRIMARY, REVERSED, PITCHSIDE, MONO };
  const seen = new Set();
  for (const [name, variant] of Object.entries(variants)) {
    const key = `${variant.field}|${variant.line}|${variant.band}`;
    assert.ok(!seen.has(key), `${name} repeats another variant`);
    seen.add(key);
    for (const value of Object.values(variant)) {
      assert.match(value, /^oklch\(/, `${name} uses a token value`);
      assert.doesNotMatch(value, /^oklch\(\s*[01](\s|\))/, `${name} reaches an extreme`);
    }
  }
});

test('the mark renders as an SVG document and never as an image file', () => {
  const svg = markSvg(32);
  assert.ok(svg.startsWith('<svg '), svg.slice(0, 40));
  assert.ok(svg.includes('viewBox="0 0 32 32"'));
  assert.equal(markSvg(32), markSvg(32), 'the same inputs give the same document');
  assert.notEqual(markSvg(32), markSvg(32, MONO));
  assert.doesNotMatch(svg, /<image|href="[^"]*\.(png|jpg|svg)"/, 'no image file is loaded');
});

test('without a document the mark falls back to the primary variant', () => {
  assert.deepEqual(colours(undefined), PRIMARY);
});
