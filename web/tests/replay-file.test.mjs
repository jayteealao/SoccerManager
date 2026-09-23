// The page's replay file is the engine's fixture layout, byte for byte. The golden file was
// written by `engine-cli record --seed 42 --minutes 1`; regenerate it whenever the protocol
// version changes.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';

import { FrameStore, PROTOCOL_VERSION, matchIdentity, readReplay, writeReplay } from '../replay-file.mjs';
import { REPO_ROOT, encodeDelta, encodeKeyframe } from './helpers.mjs';

const golden = () =>
  new Uint8Array(fs.readFileSync(path.join(REPO_ROOT, 'web/tests/data/one-minute.smfx')));

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
