// Shared test helpers. No package, no install: Node 22 ships `node:test` and `node:assert`.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { COMPONENT_COUNT, KIND_DELTA, KIND_KEYFRAME } from '../decode.mjs';

export const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

/// Runs `fn` with `console.info` silenced. The page writes one JSON Lines row per signal,
/// which is the point in a browser and noise in a test.
export function quiet(fn) {
  const info = console.info;
  console.info = () => {};
  try {
    return fn();
  } finally {
    console.info = info;
  }
}

/// Collects the rows a block of code writes to `console.info`, parsed.
export function captured(fn) {
  const info = console.info;
  const rows = [];
  console.info = (line) => rows.push(JSON.parse(line));
  try {
    fn();
  } finally {
    console.info = info;
  }
  return rows;
}

/// Encodes one keyframe exactly as `crates/protocol/src/codec.rs` does.
export function encodeKeyframe(tick, components, restart = false) {
  const bytes = new Uint8Array(1 + 98);
  const view = new DataView(bytes.buffer);
  bytes[0] = restart ? 0x03 : KIND_KEYFRAME;
  view.setUint32(1, tick, true);
  for (let i = 0; i < COMPONENT_COUNT; i += 1) {
    view.setInt16(5 + i * 2, components[i], true);
  }
  return bytes;
}

/// Encodes one delta, or returns null when a step is wider than a signed byte, exactly as
/// the Rust producer does before it falls back to a keyframe.
export function encodeDelta(prev, components) {
  const bytes = new Uint8Array(1 + 47);
  const view = new DataView(bytes.buffer);
  bytes[0] = KIND_DELTA;
  for (let i = 0; i < COMPONENT_COUNT; i += 1) {
    const step = components[i] - prev[i];
    if (step < -128 || step > 127) {
      return null;
    }
    view.setInt8(1 + i, step);
  }
  return bytes;
}

/// The recorded fixture, when one has been recorded. Fixtures are gitignored, so a test
/// that needs one reports a skip with this reason rather than passing quietly.
export const FIXTURE_PATH = path.join(REPO_ROOT, 'fixture.smfx');
export const FIXTURE_MISSING =
  'no fixture.smfx at the repository root; record one with: ' +
  'target/release/engine-cli.exe record --seed 7 --out fixture.smfx --minutes 90';

export function fixtureExists() {
  return fs.existsSync(FIXTURE_PATH);
}

/// Reads a fixture file into its header and its entries, per `crates/stream/src/record.rs`.
export function readFixture(limit = Infinity) {
  const raw = fs.readFileSync(FIXTURE_PATH);
  if (raw.subarray(0, 4).toString('latin1') !== 'SMFX') {
    throw new Error('not a fixture file');
  }
  const header = {
    protocolVersion: raw.readUInt16LE(4),
    matchMillis: raw.readBigUInt64LE(8),
    frames: raw.readUInt32LE(16),
    ticks: raw.readUInt32LE(20),
    seed: raw.readBigUInt64LE(24),
  };
  const entries = [];
  const end = raw.length - 16;
  let at = 32;
  while (at < end && entries.length < limit) {
    const kind = raw[at];
    const tick = raw.readUInt32LE(at + 1);
    const length = raw.readUInt32LE(at + 5);
    const payload = raw.subarray(at + 9, at + 9 + length);
    at += 9 + length;
    entries.push(
      kind === 1
        ? { text: payload.toString('utf8'), tick }
        : { bytes: new Uint8Array(payload), tick }
    );
  }
  return { header, entries };
}
