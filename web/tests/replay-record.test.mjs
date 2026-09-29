// The viewer's check of a version-4 record, against the two lists the engine's reader is
// checked against too (`crates/stream/tests/replay_record_shape.rs`):
// `data/record-paths.json` names every leaf of a full record, and `data/damaged-records.json`
// lists edits to the committed file's record that both readers must refuse.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';

import { matchedRecord, readReplay, requireRecord } from '../replay-file.mjs';
import { REPO_ROOT } from './helpers.mjs';

const data = (name) => path.join(REPO_ROOT, 'web/tests/data', name);
const load = (name) => JSON.parse(fs.readFileSync(data(name), 'utf8'));
const fixture = () => new Uint8Array(fs.readFileSync(data('one-minute-v4.smfx')));
const clone = (value) => JSON.parse(JSON.stringify(value));
const encoder = new TextEncoder();

/// The fixture's record with one role added to its tactics change, so every leaf exists.
async function fullRecord() {
  const { record } = await readReplay(fixture());
  const meta = clone(record.meta);
  meta.changes[1].change.tactics.roles = [{ squad: 1, role: 2, duty: 3 }];
  return { meta, inputs: record.inputs };
}

/// Every leaf of `value` as a path, with `[]` for a list item.
function leaves(value, at, out) {
  if (Array.isArray(value)) {
    value.forEach((item) => leaves(item, `${at}[]`, out));
  } else if (value !== null && typeof value === 'object') {
    for (const [key, inner] of Object.entries(value)) {
      leaves(inner, at ? `${at}.${key}` : key, out);
    }
  } else {
    out.add(at);
  }
  return out;
}

/// Deletes the leaf at `route` (`a.b[].c`) on the first list item that holds it, and returns
/// the concrete path it deleted (`a.b[1].c`), or null. A route ending in `[]` drops the last
/// item of the list and returns the list's path.
function deleteAt(value, route, prefix = '') {
  const [head, ...rest] = route.split('.');
  const rest_ = rest.join('.');
  if (head.endsWith('[]')) {
    const key = head.slice(0, -2);
    const items = value?.[key];
    const base = `${prefix}${key}`;
    if (!Array.isArray(items)) return null;
    if (!rest_) {
      items.pop();
      return base;
    }
    for (const [i, item] of items.entries()) {
      const done = deleteAt(item, rest_, `${base}[${i}].`);
      if (done) return done;
    }
    return null;
  }
  if (!rest_) {
    if (value === null || typeof value !== 'object' || !Object.hasOwn(value, head)) return null;
    delete value[head];
    return `${prefix}${head}`;
  }
  return deleteAt(value?.[head], rest_, `${prefix}${head}.`);
}

/// Applies one edit of the shared list: a path of keys and indices, set or delete.
function apply(record, { path: steps, op, value }) {
  const parent = steps.slice(0, -1).reduce((at, step) => at[step], record);
  const last = steps.at(-1);
  if (op === 'delete') {
    delete parent[last];
  } else {
    parent[last] = clone(value);
  }
}

test('the fixture record holds exactly the leaves the shared list names', async () => {
  const { meta } = await fullRecord();
  const found = [...leaves(meta, '', new Set())].sort();
  assert.deepEqual(found, load('record-paths.json'));
});

test('a record without any listed leaf is refused, naming it', async () => {
  const { meta } = await fullRecord();
  requireRecord(meta);
  for (const route of load('record-paths.json')) {
    const copy = clone(meta);
    const concrete = deleteAt(copy, route);
    assert.ok(concrete, `${route} is not in the record`);
    assert.throws(
      () => requireRecord(copy),
      (error) => error.reason.includes(concrete),
      `without ${route}`
    );
  }
});

test('every damaged record of the shared list is refused by the reader', async () => {
  const { inputs } = await fullRecord();
  const base = (await readReplay(fixture())).record.meta;
  await matchedRecord(encoder.encode(JSON.stringify(base)), inputs);
  const damaged = load('damaged-records.json');
  assert.ok(damaged.length > 30);
  for (const { name, edits } of damaged) {
    const record = clone(base);
    edits.forEach((edit) => apply(record, edit));
    await assert.rejects(
      matchedRecord(encoder.encode(JSON.stringify(record)), inputs),
      (error) => error.name === 'ReplayRefused' && error.reason.length > 0,
      `the reader took a record with ${name}`
    );
  }
});

test('the refusals name the path and the reason', async () => {
  const { inputs } = await fullRecord();
  const base = (await readReplay(fixture())).record.meta;
  const cases = [
    [[['extra'], 1], 'the record has an unknown field extra'],
    [[['changes', 0, 'change', 'substitution', 'extra'], 1], 'unknown field changes[0].change.substitution.extra'],
    [[['engine', 'dirty'], 'yes'], 'the record field engine.dirty is not true or false'],
    [[['settings', 'managers', 0], 'cyborg'], 'the record field settings.managers[0] is not one of ai, human'],
    [[['inputs_bytes'], 1], `the inputs hold ${base.inputs_bytes} bytes and the record says 1`],
  ];
  for (const [[steps, value], reason] of cases) {
    const record = clone(base);
    apply(record, { path: steps, op: 'set', value });
    await assert.rejects(
      matchedRecord(encoder.encode(JSON.stringify(record)), inputs),
      (error) => error.reason.includes(reason),
      reason
    );
  }
});
