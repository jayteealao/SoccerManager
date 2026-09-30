// The contrast check (WCAG 2.2 AA). Reads a skin's tokens.css and a pair list, computes each
// pair's contrast ratio from the token values, and exits 1 when any text pair is under 4.5:1,
// any large-text pair under 3:1, or any boundary pair under 3:1. An `inert` pair (the text of
// an inactive stub, which WCAG exempts) is reported and never fails.
//
// The ratio is compared unrounded: the tightest shipped pair passes at 4.5019:1, and a
// rounded comparison would let a 4.495:1 pair through. Two decimals appear in the report only.
//
// Usage: node scripts/contrast.mjs [tokens.css] [pairs.json]
// Defaults: the broadcast-blue skin's tokens.css and contrast-pairs.json.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SKIN = path.join(HERE, '../src/skins/broadcast-blue');

export const FLOORS = { text: 4.5, 'large-text': 3, boundary: 3 };

/// Every `--name: #hex;` in a stylesheet.
export function readTokens(css) {
  const tokens = new Map();
  for (const [, name, value] of css.matchAll(/(--[a-z0-9-]+)\s*:\s*(#[0-9a-fA-F]{3,8})\s*;/g)) {
    tokens.set(name, value.toLowerCase());
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

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  const cssPath = process.argv[2] ?? path.join(SKIN, 'tokens.css');
  const pairsPath = process.argv[3] ?? path.join(SKIN, 'contrast-pairs.json');
  const tokens = readTokens(fs.readFileSync(cssPath, 'utf8'));
  const pairs = JSON.parse(fs.readFileSync(pairsPath, 'utf8'));
  const { lines, failures } = check(tokens, pairs);
  for (const line of lines) {
    console.log(line);
  }
  for (const failure of failures) {
    console.error(`FAIL  ${failure}`);
  }
  console.log(`${pairs.length} pairs, ${failures.length} failing`);
  process.exit(failures.length === 0 ? 0 : 1);
}
