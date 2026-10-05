// The contrast check (WCAG 2.2 AA). Reads a skin's tokens.css and a pair list, computes each
// pair's contrast ratio from the token values, and exits 1 when any text pair is under 4.5:1,
// any large-text pair under 3:1, or any boundary pair under 3:1. An `inert` pair (the text of
// an inactive stub, which WCAG exempts) is reported and never fails.
//
// The ratio is compared unrounded: the tightest shipped pair passes at 4.5019:1, and a
// rounded comparison would let a 4.495:1 pair through. Two decimals appear in the report only.
//
// A token written in OKLCH is read as the sRGB hex it renders as, so a skin may use either.
//
// Usage: node scripts/contrast.mjs [tokens.css] [pairs.json]
// Defaults: every shipped skin that has a contrast-pairs.json, each against its own tokens.css.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { oklchToHex } from '../src/lib/colour.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SKINS_DIR = path.join(HERE, '../src/skins');

/// The skins the build ships, read from the loader's list (the loader imports CSS, which
/// node cannot load).
export function shippedSkins() {
  const loader = fs.readFileSync(path.join(SKINS_DIR, 'index.js'), 'utf8');
  const list = /export const SKINS = \[([^\]]*)\]/.exec(loader);
  if (!list) {
    throw new Error('src/skins/index.js has no SKINS list');
  }
  return [...list[1].matchAll(/'([^']+)'/g)].map(([, name]) => name);
}

export const FLOORS = { text: 4.5, 'large-text': 3, boundary: 3 };

/// Every `--name: #hex;` and opaque `--name: oklch(L C H);` in a stylesheet, as hex.
export function readTokens(css) {
  const tokens = new Map();
  for (const [, name, value] of css.matchAll(/(--[a-z0-9-]+)\s*:\s*(#[0-9a-fA-F]{3,8})\s*;/g)) {
    tokens.set(name, value.toLowerCase());
  }
  const oklch = /(--[a-z0-9-]+)\s*:\s*oklch\(\s*([\d.]+)\s+([\d.]+)\s+([\d.]+)\s*\)\s*;/g;
  for (const [, name, l, c, h] of css.matchAll(oklch)) {
    tokens.set(name, oklchToHex({ l: Number(l), c: Number(c), h: Number(h) }));
  }
  return tokens;
}

function channels(hex) {
  const digits = hex.slice(1);
  const full =
    digits.length === 3
      ? [...digits].map((d) => d + d).join('')
      : digits.length === 6
        ? digits
        : null;
  if (!full) {
    throw new Error(`${hex} is not an opaque #rgb or #rrggbb colour`);
  }
  return [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16) / 255);
}

/// WCAG relative luminance.
export function luminance(hex) {
  const [r, g, b] = channels(hex).map((v) =>
    v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4
  );
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/// The contrast ratio of two colours, from 1 to 21, unrounded.
export function ratio(a, b) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

/// Checks every pair; returns the report lines and the failures.
export function check(tokens, pairs) {
  const lines = [];
  const failures = [];
  for (const pair of pairs) {
    const value = (name) => (name.startsWith('#') ? name : tokens.get(name));
    const fg = value(pair.fg);
    const bg = value(pair.bg);
    const label = `${pair.fg} on ${pair.bg} (${pair.kind}, ${pair.where})`;
    if (!fg || !bg) {
      failures.push(`${label}: token not defined`);
      continue;
    }
    const r = ratio(fg, bg);
    const floor = FLOORS[pair.kind];
    if (pair.kind === 'inert') {
      lines.push(`inert ${r.toFixed(2)}:1  ${label}`);
    } else if (floor === undefined) {
      failures.push(`${label}: unknown kind`);
    } else if (r < floor) {
      failures.push(`${label}: ${r.toFixed(2)}:1 is under ${floor}:1`);
    } else {
      lines.push(`pass  ${r.toFixed(2)}:1  ${label}`);
    }
  }
  return { lines, failures };
}

/// Checks one skin's tokens.css against one pair list; prints the report and returns the
/// number of failing pairs.
function report(cssPath, pairsPath, name) {
  const tokens = readTokens(fs.readFileSync(cssPath, 'utf8'));
  const pairs = JSON.parse(fs.readFileSync(pairsPath, 'utf8'));
  const { lines, failures } = check(tokens, pairs);
  for (const line of lines) {
    console.log(line);
  }
  for (const failure of failures) {
    console.error(`FAIL  ${failure}`);
  }
  console.log(`${name ? `${name}: ` : ''}${pairs.length} pairs, ${failures.length} failing`);
  return failures.length;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  let failing = 0;
  if (process.argv[2]) {
    const cssPath = process.argv[2];
    failing = report(cssPath, process.argv[3] ?? path.join(path.dirname(cssPath), 'contrast-pairs.json'));
  } else {
    for (const skin of shippedSkins()) {
      const pairsPath = path.join(SKINS_DIR, skin, 'contrast-pairs.json');
      if (fs.existsSync(pairsPath)) {
        failing += report(path.join(SKINS_DIR, skin, 'tokens.css'), pairsPath, skin);
      }
    }
  }
  process.exitCode = failing === 0 ? 0 : 1;
}
