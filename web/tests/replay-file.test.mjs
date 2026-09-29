// The page's replay file is the engine's fixture layout, byte for byte. The version-3 golden
// file was written by `engine-cli record --seed 42 --minutes 1` before version 4 existed; the
// version-4 file by `engine-cli record --seed 42 --minutes 1 --script-pack
// content/scripts/sample --changes <file>`, with a home substitution (slot 9, bench 0) and a
// home mentality of 4 at tick 0. Regenerate them whenever the protocol version changes.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';

import {
  FORMAT_VERSION,
  FrameStore,
  LEGACY_VERSION,
  PROTOCOL_VERSION,
  matchIdentity,
  readReplay,
  writeReplay,
} from '../replay-file.mjs';
import { createHash } from 'node:crypto';
import { REPO_ROOT, encodeDelta, encodeKeyframe } from './helpers.mjs';

const golden = () =>
  new Uint8Array(fs.readFileSync(path.join(REPO_ROOT, 'web/tests/data/one-minute.smfx')));
const goldenV4 = () =>
  new Uint8Array(fs.readFileSync(path.join(REPO_ROOT, 'web/tests/data/one-minute-v4.smfx')));
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

test('the golden file reads and writes back byte for byte', async () => {
  const bytes = golden();
  const read = await readReplay(bytes);
  assert.equal(read.version, PROTOCOL_VERSION);
  assert.equal(read.ticks, 3000);
  assert.equal(read.frames, read.store.count);
  assert.equal(read.hello.type, 'hello');
  const written = await writeReplay(read.store, { matchId: read.hello['match.id'] });
  assert.equal(written.bytes.length, bytes.length);
  assert.deepEqual(written.bytes, bytes);
  assert.equal(written.hash, read.hash);
});

test('a store fed frame by frame writes the same file as the engine', async () => {
  const read = await readReplay(golden());
  const store = new FrameStore(64, 4);
  for (let i = 0; i < read.store.count; i += 1) {
    const { text, payload } = read.store.frame(i);
    if (text) {
      const message = JSON.parse(new TextDecoder().decode(payload));
      store.addText(new TextDecoder().decode(payload), message.type);
    } else {
      store.addBinary(payload.slice());
    }
  }
  // A second hello and a command answer never reach the file.
  assert.equal(store.addText('{"type":"hello"}', 'hello'), false);
  assert.equal(store.addText('{"type":"ack"}', 'ack'), false);
  const written = await writeReplay(store, { matchId: read.hello['match.id'] });
  assert.deepEqual(written.bytes, golden());
});

test('each corruption is refused by name', async () => {
  const cases = [
    ['bad magic', (b) => (b[0] = 0x00)],
    ['protocol version', (b) => (b[4] = 9)],
    ['hash mismatch', (b) => (b[200] ^= 0xff)],
    ['missing trailer', (b) => (b[b.length - 16] = 0x00)],
  ];
  for (const [name, corrupt] of cases) {
    const bytes = golden();
    corrupt(bytes);
    await assert.rejects(readReplay(bytes), (error) => error.reason.includes(name), name);
  }
  const cut = golden().subarray(0, 5000);
  await assert.rejects(readReplay(cut), (error) => error.name === 'ReplayRefused');
});

test('truncating the store drops every frame after the tick', () => {
  const store = new FrameStore(16, 2);
  const zero = new Int16Array(47);
  store.addText('{"type":"hello"}', 'hello');
  store.addBinary(encodeKeyframe(1, zero));
  store.addBinary(encodeDelta(zero, zero));
  store.addText('{"type":"event","tick":2}', 'event');
  store.addBinary(encodeDelta(zero, zero));
  assert.equal(store.lastTick, 3);
  store.truncate(2);
  assert.equal(store.count, 4);
  assert.equal(store.tickFrames, 2);
  assert.equal(store.lastTick, 2);
  store.addBinary(encodeKeyframe(3, zero));
  assert.equal(store.frame(4).tick, 3);
});

test('the seed and the stamp come from the match identifier exactly', () => {
  const { seed, millis } = matchIdentity('ffffffffffffffff-1700000000123');
  assert.equal(seed, 0xffffffffffffffffn);
  assert.equal(millis, 1700000000123n);
});

test('a version-4 file from the engine and a version-3 file both load with their frames', async () => {
  const legacy = await readReplay(golden());
  assert.equal(legacy.version, LEGACY_VERSION);
  assert.equal(legacy.record, null);
  const read = await readReplay(goldenV4());
  assert.equal(read.version, FORMAT_VERSION);
  assert.equal(read.ticks, 3000);
  assert.equal(read.frames, read.store.count);
  assert.equal(read.store.tickFrames, 3000);
  assert.equal(read.hello.type, 'hello');
  // The inputs and the record stay out of the frames.
  for (let i = 0; i < read.store.count; i += 1) {
    const { text, payload } = read.store.frame(i);
    if (text) {
      assert.ok(JSON.parse(new TextDecoder().decode(payload)).type, 'a text frame is a message');
    }
  }
});

test('a version-4 file read and written back is byte-identical and keeps every field', async () => {
  const bytes = goldenV4();
  const read = await readReplay(bytes);
  const written = await writeReplay(read.store, {
    matchId: read.hello['match.id'],
    record: read.record,
  });
  assert.equal(written.bytes.length, bytes.length);
  assert.deepEqual(written.bytes, bytes);
  assert.equal(written.hash, read.hash);
  const again = await readReplay(written.bytes);
  assert.deepEqual(again.record.meta, read.record.meta);
  assert.deepEqual(again.record.inputs, read.record.inputs);
});

test('the record holds the engine identity, every input, and every applied change', async () => {
  const { record } = await readReplay(goldenV4());
  const { meta } = record;
  assert.deepEqual(Object.keys(meta).sort(), [
    'changes',
    'engine',
    'inputs',
    'inputs_bytes',
    'settings',
    'watchdog',
  ]);
  assert.deepEqual(Object.keys(meta.engine).sort(), [
    'build',
    'commit',
    'crate_version',
    'dirty',
    'executable_sha256',
    'maths',
    'scheme',
  ]);
  assert.match(meta.engine.commit, /^[0-9a-f]{40}$/);
  assert.match(meta.engine.executable_sha256, /^[0-9a-f]{64}$/);
  assert.equal(typeof meta.engine.dirty, 'boolean');
  assert.equal(meta.engine.scheme, 1);
  assert.deepEqual(meta.settings, {
    seed: '42',
    minutes: 1,
    knockout: false,
    managers: ['human', 'ai'],
  });
  assert.deepEqual(
    record.inputs.map((i) => i.role),
    ['attributes', 'tuning', 'rules', 'tactics', 'commentary', 'team_a', 'team_b', 'pack_manifest', 'pack_script']
  );
  assert.equal(
    meta.inputs_bytes,
    record.inputs.reduce((sum, i) => sum + i.bytes.length, 0)
  );
  for (const [i, input] of record.inputs.entries()) {
    assert.equal(sha256(input.bytes), meta.inputs[i].sha256, input.name);
    assert.equal(input.bytes.length, meta.inputs[i].bytes, input.name);
  }
  assert.equal(meta.changes.length, 2);
  for (const [order, change] of meta.changes.entries()) {
    assert.deepEqual(Object.keys(change).sort(), [
      'change',
      'order',
      'queue_number',
      'queued_tick',
      'source',
      'stoppage',
      'team',
      'tick',
    ]);
    assert.equal(change.order, order);
    assert.equal(change.source, 'manager');
  }
  assert.ok(meta.changes[0].change.substitution);
  assert.equal(meta.changes[1].change.tactics.mentality, 4);
  assert.equal(meta.changes[0].tick, meta.changes[1].tick);
  assert.deepEqual(meta.watchdog, { slow_calls: 0, invalid: null });
});

test('a changed input byte and frames of another protocol are refused by name', async () => {
  const changed = goldenV4();
  // The first input entry's first file byte: header, entry head, name length, name.
  const nameLength = new DataView(changed.buffer).getUint16(32 + 9, true);
  changed[32 + 9 + 2 + nameLength] ^= 0x01;
  await assert.rejects(readReplay(changed), (error) => error.reason.includes('hash mismatch'));

  const protocol = goldenV4();
  protocol[6] = 9;
  await assert.rejects(
    readReplay(protocol),
    (error) => error.reason.includes('frames protocol version 9')
  );

  // The same change with a matching trailer: the record's list names the file.
  const read = await readReplay(goldenV4());
  read.record.inputs[1].bytes[0] ^= 0x01;
  const tampered = await writeReplay(read.store, {
    matchId: read.hello['match.id'],
    record: read.record,
  });
  await assert.rejects(readReplay(tampered.bytes), (error) =>
    error.reason.includes('input tuning.json') && error.reason.includes('does not match')
  );
});

test('a loaded version-4 file saved the way the viewer saves it keeps its protocol version, inputs and record', async () => {
  const read = await readReplay(goldenV4());
  // The viewer passes the hello's protocol version and the loaded record.
  const saved = await writeReplay(read.store, {
    matchId: read.hello['match.id'],
    version: read.hello['protocol.version'],
    record: read.record,
  });
  const again = await readReplay(saved.bytes);
  assert.equal(again.version, FORMAT_VERSION);
  assert.equal(again.hello['protocol.version'], PROTOCOL_VERSION);
  assert.ok(read.record.inputs.length > 0);
  assert.deepEqual(again.record.inputs, read.record.inputs);
  assert.deepEqual(again.record.meta, read.record.meta);
});

test('writing refuses a protocol version that is not the one this page reads', async () => {
  const read = await readReplay(golden());
  for (const version of [FORMAT_VERSION, 0, 9]) {
    await assert.rejects(
      writeReplay(read.store, { matchId: read.hello['match.id'], version }),
      (error) => error.reason.includes('protocol version')
    );
  }
});
