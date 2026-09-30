// The viewer's replay migration chain, the same as the engine's: the committed version-4
// file lifts through two test-only later formats with every frame, input, and record field
// kept; each broken file or broken chain is refused by name, with no default filled; and the
// steps match the shared step list that the engine's reader is checked against too.
//
// The two later formats exist only in this file and in the engine's
// `crates/stream/tests/replay_migration.rs`. Format 5 moves the engine identity out of the
// record into its own entry (kind 4), first in the file. Format 6 merges the input entries
// into one entry (kind 5) and renames the listed `bytes` field to `size`.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import {
  FORMAT_VERSION,
  PROTOCOL_VERSION,
  REPLAY_STEPS,
  ReplayRefused,
  joinReplay,
  matchedRecord,
  productionChain,
  readReplay,
  splitInput,
  splitReplay,
  storeFrames,
} from '../src/lib/replay-file.js';
import { REPO_ROOT } from './helpers.js';

const ENTRY_INPUT = 2;
const ENTRY_RECORD = 3;
const ENTRY_ENGINE = 4;
const ENTRY_INPUTS = 5;

const encoder = new TextEncoder();
const decoder = new TextDecoder();
const data = (name) => path.join(REPO_ROOT, 'web/tests/data', name);
const fixture = () => new Uint8Array(fs.readFileSync(data('one-minute-v4.smfx')));
const json = (payload) => JSON.parse(decoder.decode(payload));
const bytesOf = (value) => encoder.encode(JSON.stringify(value));
const recordAt = (raw) => raw.entries.findIndex((e) => e.kind === ENTRY_RECORD);

/// Step 4→5: the record's `engine` object becomes a new first entry.
function engineEntry(raw) {
  const at = recordAt(raw);
  const record = json(raw.entries[at].payload);
  if (!Object.hasOwn(record, 'engine')) {
    throw new ReplayRefused('the record has no field engine');
  }
  const { engine, ...rest } = record;
  const entries = raw.entries.slice();
  entries[at] = { ...entries[at], payload: bytesOf(rest) };
  entries.unshift({ kind: ENTRY_ENGINE, tick: 0, payload: bytesOf(engine) });
  return { ...raw, format: 5, entries };
}

/// Renames `from` to `to` in each listed input. A missing field stays missing.
function renameInputs(record, from, to) {
  for (const input of record.inputs ?? []) {
    if (Object.hasOwn(input, from)) {
      input[to] = input[from];
      delete input[from];
    }
  }
}

/// Step 5→6: the input entries become one entry (a u16 count, then per file a u16 name
/// length, the name, a u32 byte length, and the bytes), and `bytes` becomes `size`.
function inputBundle(raw) {
  const files = raw.entries.filter((e) => e.kind === ENTRY_INPUT).map((e) => splitInput(e.payload));
  const parts = [];
  const count = new Uint8Array(2);
  new DataView(count.buffer).setUint16(0, files.length, true);
  parts.push(count);
  for (const { name, bytes } of files) {
    const encoded = encoder.encode(name);
    const head = new Uint8Array(2 + encoded.length + 4);
    const view = new DataView(head.buffer);
    view.setUint16(0, encoded.length, true);
    head.set(encoded, 2);
    view.setUint32(2 + encoded.length, bytes.length, true);
    parts.push(head, bytes);
  }
  const payload = new Uint8Array(parts.reduce((sum, p) => sum + p.length, 0));
  let at = 0;
  for (const part of parts) {
    payload.set(part, at);
    at += part.length;
  }
  const first = raw.entries.findIndex((e) => e.kind === ENTRY_INPUT);
  const entries = raw.entries.filter((e) => e.kind !== ENTRY_INPUT);
  if (first >= 0) {
    entries.splice(first, 0, { kind: ENTRY_INPUTS, tick: 0, payload });
  }
  const lifted = { ...raw, format: 6, entries };
  const r = recordAt(lifted);
  const record = json(entries[r].payload);
  renameInputs(record, 'bytes', 'size');
  entries[r] = { ...entries[r], payload: bytesOf(record) };
  return lifted;
}

/// The files of a format-6 input entry.
function unbundle(payload) {
  const view = new DataView(payload.buffer, payload.byteOffset, payload.byteLength);
  const need = (at, n) => {
    if (at + n > payload.length) {
      throw new ReplayRefused('the input bundle is truncated');
    }
  };
  need(0, 2);
  const count = view.getUint16(0, true);
  const files = [];
  let at = 2;
  for (let i = 0; i < count; i += 1) {
    need(at, 2);
    const length = view.getUint16(at, true);
    need(at + 2, length + 4);
    const name = decoder.decode(payload.subarray(at + 2, at + 2 + length));
    const size = view.getUint32(at + 2 + length, true);
    at += 2 + length + 4;
    need(at, size);
    files.push({ name, bytes: payload.slice(at, at + size) });
    at += size;
  }
  return files;
}

/// Decodes a format-6 file through the reader's exported pieces, so every check of the
/// production decode applies: the frames, the required record fields, and the inputs.
async function decodeV6(raw) {
  assert.equal(raw.format, 6);
  if (raw.protocol !== PROTOCOL_VERSION) {
    throw new ReplayRefused(`frames protocol version ${raw.protocol}`);
  }
  let engine = null;
  let files = [];
  let record = null;
  const frames = [];
  for (const entry of raw.entries) {
    if (entry.kind === ENTRY_ENGINE) {
      engine = json(entry.payload);
    } else if (entry.kind === ENTRY_INPUTS) {
      files = unbundle(entry.payload);
    } else if (entry.kind === ENTRY_RECORD) {
      record = json(entry.payload);
    } else if (entry.kind === ENTRY_INPUT) {
      throw new ReplayRefused('an input entry in a version-6 file');
    } else {
      frames.push(entry);
    }
  }
  if (!engine || !record) {
    throw new ReplayRefused('a version-6 file needs its engine and record entries');
  }
  renameInputs(record, 'size', 'bytes');
  const store = storeFrames(raw, frames);
  return { store, record: await matchedRecord(bytesOf({ engine, ...record }), files) };
}

const TEST_STEPS = [
  { from: 4, to: 5, name: 'engine-entry', lift: engineEntry },
  { from: 5, to: 6, name: 'input-bundle', lift: inputBundle },
];
const testChain = { current: 6, steps: TEST_STEPS, decode: decodeV6 };

/// The frames of a store as `{ text, tick, bytes }`.
const framesOf = (store) =>
  Array.from({ length: store.count }, (_, i) => {
    const { text, tick, payload } = store.frame(i);
    return { text, tick, bytes: Array.from(payload) };
  });

/// Reads `bytes` through both chains and checks that the lifted read keeps every frame,
/// input, and record field of the production read.
async function liftsIntact(bytes) {
  const production = await readReplay(bytes);
  const lifted = await readReplay(bytes, { chain: testChain });
  assert.equal(lifted.version, 6);
  assert.deepEqual(lifted.migrated, { from: 4, to: 6, steps: 2 });
  assert.equal(production.migrated, null);
  assert.deepEqual(framesOf(lifted.store), framesOf(production.store));
  assert.deepEqual(lifted.record.inputs, production.record.inputs);
  assert.deepEqual(lifted.record.meta, production.record.meta);
  for (const key of ['frames', 'ticks', 'hash', 'hello']) {
    assert.deepEqual(lifted[key], production[key], key);
  }
  return production;
}

const refusedWith = (text) => (error) =>
  error instanceof ReplayRefused && error.reason.includes(text) ? true : assert.fail(error.reason);

/// The fixture with its record JSON edited by `edit`, written with a matching trailer.
async function editedRecord(edit) {
  const raw = await splitReplay(fixture());
  const at = recordAt(raw);
  const record = json(raw.entries[at].payload);
  edit(record);
  raw.entries[at] = { ...raw.entries[at], payload: bytesOf(record) };
  return joinReplay(raw);
}

test('the committed file holds what the migration tests rely on', async () => {
  const read = await readReplay(fixture());
  assert.equal(read.version, FORMAT_VERSION);
  assert.equal(read.record.inputs.length, 9);
  assert.equal(read.record.meta.changes.length, 2);
  assert.equal(read.store.tickFrames, 3000);
  assert.equal(read.store.count - read.store.tickFrames, 129);
  assert.equal(read.migrated, null);
});

test('the committed file lifts through two later formats with every field kept', async () => {
  const raw = await splitReplay(fixture());
  const v5 = engineEntry(raw);
  assert.equal(v5.entries[0].kind, ENTRY_ENGINE);
  assert.equal(Object.hasOwn(json(v5.entries[recordAt(v5)].payload), 'engine'), false);
  const v6 = inputBundle(v5);
  assert.equal(v6.entries.filter((e) => e.kind === ENTRY_INPUT).length, 0);
  assert.equal(v6.entries.filter((e) => e.kind === ENTRY_INPUTS).length, 1);
  for (const input of json(v6.entries[recordAt(v6)].payload).inputs) {
    assert.ok(Object.hasOwn(input, 'size') && !Object.hasOwn(input, 'bytes'));
  }

  await liftsIntact(fixture());

  // A non-empty watchdog mark survives too.
  const marked = await editedRecord((record) => {
    record.watchdog = { slow_calls: 3, invalid: 'slow script' };
  });
  const read = await liftsIntact(marked);
  assert.deepEqual(read.record.meta.watchdog, { slow_calls: 3, invalid: 'slow script' });
});

test('a newer format is refused by both chains', async () => {
  const newer = fixture();
  new DataView(newer.buffer).setUint16(4, 7, true);
  await assert.rejects(
    readReplay(newer),
    refusedWith('format 7 is newer than this reader knows; this page reads formats 3 and 4')
  );
  await assert.rejects(
    readReplay(newer, { chain: testChain }),
    refusedWith('format 7 is newer than this reader knows; this page reads formats 3 to 6')
  );
  // Control: the same bytes with format 4 read.
  new DataView(newer.buffer).setUint16(4, 4, true);
  await readReplay(newer);
  await readReplay(newer, { chain: testChain });
});

test('a chain with a missing step is refused', async () => {
  const gap = { current: 7, steps: [TEST_STEPS[0], { from: 6, to: 7, name: 'later', lift: (r) => r }], decode: decodeV6 };
  await assert.rejects(
    readReplay(fixture(), { chain: gap }),
    refusedWith('no step from format 5 (it holds 4→5, 6→7)')
  );
  // Control: the contiguous chain reads.
  await readReplay(fixture(), { chain: testChain });
});

test('a step that returns the wrong format is refused', async () => {
  const skipping = { ...TEST_STEPS[0], lift: (raw) => ({ ...engineEntry(raw), format: 6 }) };
  const wrong = { current: 6, steps: [skipping, TEST_STEPS[1]], decode: decodeV6 };
  await assert.rejects(
    readReplay(fixture(), { chain: wrong }),
    refusedWith('migration step 4→5 (engine-entry) returned format 6')
  );
  // Control: the correct step reads.
  await readReplay(fixture(), { chain: testChain });
});

test('a missing record field is refused by name and never filled', async () => {
  const cases = [
    [(r) => delete r.watchdog.invalid, 'the record has no field watchdog.invalid'],
    [(r) => delete r.changes[1].change.tactics.formation, 'the record has no field changes[1].change.tactics.formation'],
    [(r) => delete r.engine.scheme, 'the record has no field engine.scheme'],
  ];
  for (const [remove, named] of cases) {
    const bytes = await editedRecord((record) => assert.equal(remove(record), true));
    for (const chain of [productionChain, testChain]) {
      await assert.rejects(readReplay(bytes, { chain }), refusedWith(named));
    }
  }
  // Control: the same edit that removes nothing reads, through both chains.
  const untouched = await editedRecord(() => {});
  const read = await readReplay(untouched);
  assert.deepEqual(read.record.meta, (await readReplay(fixture())).record.meta);
  await readReplay(untouched, { chain: testChain });
});

test('a corrupted inputs section is refused by name', async () => {
  const raw = await splitReplay(fixture());
  const inputs = raw.entries.flatMap((e, i) => (e.kind === ENTRY_INPUT ? [i] : []));

  // One data byte of the second input, with a trailer hash that matches.
  const flippedEntries = raw.entries.slice();
  const payload = flippedEntries[inputs[1]].payload.slice();
  payload[payload.length - 1] ^= 0x01;
  flippedEntries[inputs[1]] = { ...flippedEntries[inputs[1]], payload };
  const flipped = await joinReplay({ ...raw, entries: flippedEntries });

  // The first input's name length runs past its payload.
  const cutEntries = raw.entries.slice();
  const named = cutEntries[inputs[0]].payload.slice();
  new DataView(named.buffer).setUint16(0, named.length, true);
  cutEntries[inputs[0]] = { ...cutEntries[inputs[0]], payload: named };
  const cut = await joinReplay({ ...raw, entries: cutEntries });

  for (const chain of [productionChain, testChain]) {
    await assert.rejects(readReplay(flipped, { chain }), (error) =>
      error.reason.includes('input tuning.json') && error.reason.includes('does not match')
        ? true
        : assert.fail(error.reason)
    );
    await assert.rejects(readReplay(cut, { chain }), refusedWith('name is truncated'));
  }
  // Control: an unchanged round trip is the same file, and it reads.
  const again = await joinReplay(raw);
  assert.deepEqual(again, fixture());
  await readReplay(again);
  await readReplay(again, { chain: testChain });
});

test('both chains match the shared step list', () => {
  const list = JSON.parse(fs.readFileSync(data('replay-steps.json'), 'utf8'));
  const listed = (steps) => steps.map(({ from, to, name }) => ({ from, to, name }));
  assert.deepEqual(listed(REPLAY_STEPS), list.production);
  assert.deepEqual(listed(TEST_STEPS), list.test);
  // Control: the same steps in the other order do not match.
  assert.notDeepEqual(listed([TEST_STEPS[1], TEST_STEPS[0]]), list.test);
});
