// Kit colours reach the pitch only after they are made safe and measured.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';

import {
  LIGHTNESS_MAX,
  LIGHTNESS_MIN,
  RING_MIN_CONTRAST,
  TOKENS,
  clampLightness,
  contrast,
  hexToOklch,
  oklchToHex,
  safeHex,
  safeKit,
} from '../colour.mjs';
import { REPO_ROOT } from './helpers.mjs';

test('pure black is pulled into the safe band and never returned', () => {
  const safe = safeHex('#000000');
  assert.notEqual(safe, '#000000');
  const back = hexToOklch(safe);
  assert.ok(back.l >= LIGHTNESS_MIN - 0.005, `lightness ${back.l}`);
  assert.equal(clampLightness(0), LIGHTNESS_MIN);
  assert.equal(clampLightness(1), LIGHTNESS_MAX);
  assert.equal(clampLightness(0.5), 0.5);
});

test('pure white is pulled into the safe band and never returned', () => {
  const safe = safeHex('#ffffff');
  assert.notEqual(safe, '#ffffff');
  assert.ok(hexToOklch(safe).l <= LIGHTNESS_MAX + 0.005);
});

test('a kit colour already inside the band survives the round trip', () => {
  const safe = safeHex('#c8102e');
  assert.equal(safe, '#c8102e');
  assert.equal(safeHex('#6a0dad'), '#6a0dad');
  assert.equal(safeHex('#ff6a13'), '#ff6a13');
});

test('a hex colour survives a round trip through OKLCH', () => {
  for (const hex of ['#c8102e', '#6a0dad', '#ff6a13', '#123456', '#7f7f7f']) {
    assert.equal(oklchToHex(hexToOklch(hex)), hex);
  }
});

test('the two shipped kits take different rings, and both are legible on turf', () => {
  // The home club's trim colour is pure black in the team file. Clamped into the safe
  // band it is a dark grey measuring 5.2 to 1 on turf, so it is used: the clamp is what
  // makes it usable, and the raw colour never reaches the pitch.
  const home = safeKit({ primary: '#c8102e', secondary: '#000000' });
  assert.equal(home.ringIsTrim, true);
  assert.notEqual(home.ring, '#000000');
  assert.ok(contrast(home.ring, TOKENS.pitch) >= RING_MIN_CONTRAST);

  // The away club's trim colour is orange, which sits at almost exactly the turf's own
  // luminance and measures 1.2 to 1. It is refused and the pitch line carries the ring.
  const away = safeKit({ primary: '#6a0dad', secondary: '#ff6a13' });
  assert.equal(away.ringIsTrim, false);
  assert.equal(away.ring, oklchToHex(parse(TOKENS.pitchLine)));
  assert.ok(contrast(away.ring, TOKENS.pitch) >= RING_MIN_CONTRAST);

  // The two rings differ, so a reader separates the teams without reading the fills.
  assert.notEqual(home.ring, away.ring);
  assert.ok(contrast(home.ring, away.ring) >= 3);
});

test('the shirt number takes whichever of paper and ink contrasts more', () => {
  for (const primary of ['#c8102e', '#6a0dad', '#ff6a13', '#f2f2c0', '#101010']) {
    const kit = safeKit({ primary, secondary: '#ffffff' });
    const paper = oklchToHex(parse(TOKENS.onBrand));
    const ink = oklchToHex(parse(TOKENS.fg));
    const chosen = contrast(kit.number, kit.fill);
    assert.ok(chosen >= contrast(paper, kit.fill) - 1e-9);
    assert.ok(chosen >= contrast(ink, kit.fill) - 1e-9);
    assert.ok(kit.number === paper || kit.number === ink);
  }
});

test('the canvas token values match web/tokens.css', () => {
  const css = fs.readFileSync(path.join(REPO_ROOT, 'web/tokens.css'), 'utf8');
  const valueOf = (name) => {
    const found = new RegExp(`--${name}:\\s*([^;]+);`).exec(css);
    assert.ok(found, `tokens.css declares --${name}`);
    return found[1].trim();
  };
  assert.equal(valueOf('tl-pitch'), TOKENS.pitch);
  assert.equal(valueOf('tl-pitch-line'), TOKENS.pitchLine);
  assert.equal(valueOf('tl-on-brand'), TOKENS.onBrand);
  assert.equal(valueOf('tl-fg'), TOKENS.fg);
});

function parse(text) {
  const [, l, c, h] = /oklch\(\s*([\d.]+)\s+([\d.]+)\s+([\d.]+)/.exec(text);
  return { l: Number(l), c: Number(c), h: Number(h) };
}
