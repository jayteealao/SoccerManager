// The decoder reads the same bytes the Rust producer writes, offset for offset.

import assert from 'node:assert/strict';
import test from 'node:test';

import {
  COMPONENT_COUNT,
  KEYFRAME_BYTES,
  PLAYER_COUNT,
  decodeInto,
  newFrame,
} from '../decode.mjs';
import {
  FIXTURE_MISSING,
  captured,
  encodeDelta,
  encodeKeyframe,
  fixtureExists,
  quiet,
  readFixture,
} from './helpers.mjs';

const components = (seed) => {
  const out = new Int16Array(COMPONENT_COUNT);
  for (let i = 0; i < COMPONENT_COUNT; i += 1) {
    out[i] = ((seed * 37 + i * 13) % 5000) - 2500;
  }
  return out;
};

test('the wire constants match the Rust codec', () => {
  assert.equal(PLAYER_COUNT, 22);
  assert.equal(KEYFRAME_BYTES, 98);
  assert.equal(COMPONENT_COUNT, 47);
});

test('a keyframe round trips', () => {
  const want = components(1);
  const out = newFrame();
  const result = decodeInto(encodeKeyframe(4242, want), newFrame(), out);
  assert.deepEqual(result, { kind: 'keyframe', tick: 4242 });
  assert.equal(out.tick, 4242);
  assert.deepEqual([...out.components], [...want]);
});

test('a fifty-frame cycle reconstructs every absolute position exactly', () => {
  // One keyframe then 49 deltas, exactly as the producer's keyframe interval writes them.
  const frames = [];
  let previous = components(0);
  frames.push({ bytes: encodeKeyframe(100, previous), want: previous });
  for (let i = 1; i < 50; i += 1) {
    const next = new Int16Array(previous);
    for (let c = 0; c < COMPONENT_COUNT; c += 1) {
      next[c] = previous[c] + ((i + c) % 9) - 4;
    }
    const bytes = encodeDelta(previous, next);
    assert.ok(bytes, 'every step fits in a signed byte');
    frames.push({ bytes, want: next });
    previous = next;
  }

  let prev = newFrame();
  let out = newFrame();
  for (const [i, frame] of frames.entries()) {
    const result = decodeInto(frame.bytes, prev, out);
    assert.ok(result, `frame ${i} decodes`);
    assert.equal(out.tick, 100 + i);
    assert.deepEqual([...out.components], [...frame.want], `frame ${i}`);
    [prev, out] = [out, prev];
  }
});

test('a restart frame is reported as a restart', () => {
  const out = newFrame();
  const result = decodeInto(encodeKeyframe(7, components(2), true), newFrame(), out);
  assert.deepEqual(result, { kind: 'restart', tick: 7 });
});

test('a truncated frame is refused and named', () => {
  const rows = captured(() => {
    const short = encodeKeyframe(9, components(3)).slice(0, 40);
    assert.equal(decodeInto(short, newFrame(), newFrame()), null);
    assert.equal(decodeInto(new Uint8Array(0), newFrame(), newFrame()), null);
    assert.equal(decodeInto(new Uint8Array([0x09, 1, 2]), newFrame(), newFrame()), null);
  });
  assert.equal(rows.length, 3);
  for (const row of rows) {
    assert.equal(row.signal, 'viewer.decode_refused');
    assert.equal(row['record.kind'], 'viewer-event');
  }
  assert.equal(rows[0].reason, 'keyframe length');
  assert.equal(rows[1].reason, 'empty frame');
  assert.equal(rows[2].reason, 'unknown frame kind');
});

test('the recorded fixture decodes from its first keyframe forward', (t) => {
  if (!fixtureExists()) {
    t.skip(FIXTURE_MISSING);
    return;
  }
  const { header, entries } = readFixture(400);
  assert.equal(header.protocolVersion, 1);
  assert.equal(header.seed, 7n);

  let prev = newFrame();
  let out = newFrame();
  let decoded = 0;
  let expectedTick = null;
  quiet(() => {
    for (const entry of entries) {
      if (!entry.bytes) {
        continue;
      }
      const result = decodeInto(entry.bytes, prev, out);
      assert.ok(result, `the producer's own bytes decode at tick ${entry.tick}`);
      // The fixture stamps every entry with its tick, so the decoder's idea of the tick
      // is checked against the recorder's, not against itself.
      assert.equal(out.tick, entry.tick);
      if (expectedTick !== null) {
        assert.equal(out.tick, expectedTick);
      }
      expectedTick = out.tick + 1;
      // No position may leave the pitch: 52.5 m and 34 m in centimetres, plus a margin.
      for (let i = 0; i < COMPONENT_COUNT; i += 1) {
        assert.ok(Math.abs(out.components[i]) <= 6000, `component ${i} is ${out.components[i]}`);
      }
      decoded += 1;
      [prev, out] = [out, prev];
    }
  });
  assert.ok(decoded > 300, `decoded ${decoded} frames`);
});
