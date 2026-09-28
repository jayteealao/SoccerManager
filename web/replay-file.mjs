// Replay files: the wire bytes of one match, kept as they arrived and saved in the engine's
// own fixture layout (`.smfx`, `crates/stream/src/record.rs`).
//
// The page never re-encodes a position. Every frame is stored exactly as the socket handed
// it over, so a saved file is the recorded stream byte for byte, and the engine's own
// `replay` command reads it. Layout, little-endian:
// - Header, 32 bytes: magic `SMFX`, the format version (u16), the frames' protocol version
//   (u16; zero in format 3, whose format field is the protocol version), the match stamp in
//   milliseconds (u64), the frame count (u32), the tick count (u32), and the seed (u64).
// - One entry each: kind (u8), tick (u32), length (u32), then the payload. Kinds 0 (binary
//   tick frame) and 1 (text frame) are the frames; a text frame carries the tick of the tick
//   frame before it. Format 4 adds kind 2, one input file of the match (name length u16, the
//   UTF-8 name, the bytes), written before the frames, and kind 3, the record (one JSON
//   document), written last.
// - Trailer, 16 bytes: magic `SMFE`, the frame count (u32), the first six bytes of the
//   SHA-256 over every payload in file order, and two zero bytes.
//
// The page saves format 3: it receives frames only. It reads both formats, and a format-4
// file read and written back is the same file byte for byte.

/// The protocol version of the frames this page reads.
export const PROTOCOL_VERSION = 3;
/// The file format that holds frames only; its format field is the protocol version.
export const LEGACY_VERSION = 3;
/// The file format that also holds the match's inputs and its record.
export const FORMAT_VERSION = 4;

const MAGIC = [0x53, 0x4d, 0x46, 0x58]; // SMFX
const TRAILER_MAGIC = [0x53, 0x4d, 0x46, 0x45]; // SMFE
export const HEADER_BYTES = 32;
export const TRAILER_BYTES = 16;
const ENTRY_BYTES = 9;
const ENTRY_BINARY = 0;
const ENTRY_TEXT = 1;
const ENTRY_INPUT = 2;
const ENTRY_RECORD = 3;
const KIND_DELTA = 0x02;

const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8', { fatal: true });

/// Every frame of one match, in arrival order, as bytes. Grows by doubling, like the history.
export class FrameStore {
  constructor(capacityBytes = 1 << 20, capacityFrames = 4096) {
    this.bytes = new Uint8Array(capacityBytes);
    this.kinds = new Uint8Array(capacityFrames);
    this.ticks = new Uint32Array(capacityFrames);
    this.offsets = new Uint32Array(capacityFrames + 1);
    this.count = 0;
    this.tickFrames = 0;
    this.lastTick = 0;
    this.helloStored = false;
  }

  /// Bytes of payload held.
  get used() {
    return this.offsets[this.count];
  }

  /// Stores one text frame. Only the first hello is kept: a reconnect repeats it, and a
  /// file that opened with two would not be the stream a replay sends. An acknowledgement
  /// or a refusal answers this page's own command and is never part of the match.
  addText(text, type) {
    if (type === 'ack' || type === 'reject') {
      return false;
    }
    if (type === 'hello') {
      if (this.helloStored) {
        return false;
      }
      this.helloStored = true;
    }
    this.push(ENTRY_TEXT, this.lastTick, encoder.encode(text));
    return true;
  }

  /// Stores one binary tick frame. A keyframe names its tick; a delta is the next one.
  addBinary(payload) {
    const bytes = payload instanceof Uint8Array ? payload : new Uint8Array(payload);
    let tick = this.lastTick + 1;
    if (bytes.length >= 5 && bytes[0] !== KIND_DELTA) {
      tick = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(1, true);
    }
    this.lastTick = tick;
    this.tickFrames += 1;
    this.push(ENTRY_BINARY, tick, bytes);
    return tick;
  }

  push(kind, tick, payload) {
    if (this.count === this.kinds.length) {
      this.kinds = grown(this.kinds, this.kinds.length * 2);
      this.ticks = grown(this.ticks, this.ticks.length * 2);
      this.offsets = grown(this.offsets, this.offsets.length * 2);
    }
    const at = this.used;
    if (at + payload.length > this.bytes.length) {
      let size = this.bytes.length * 2;
      while (size < at + payload.length) {
        size *= 2;
      }
      this.bytes = grown(this.bytes, size);
    }
    this.bytes.set(payload, at);
    this.kinds[this.count] = kind;
    this.ticks[this.count] = tick;
    this.count += 1;
    this.offsets[this.count] = at + payload.length;
  }

  /// Drops every frame after `tick`. A resumed match plays those ticks again.
  truncate(tick) {
    let keep = this.count;
    while (keep > 0 && this.ticks[keep - 1] > tick) {
      keep -= 1;
    }
    let tickFrames = 0;
    for (let i = 0; i < keep; i += 1) {
      if (this.kinds[i] === ENTRY_BINARY) {
        tickFrames += 1;
      }
    }
    this.count = keep;
    this.tickFrames = tickFrames;
    this.lastTick = Math.min(this.lastTick, tick);
  }

  /// Frame `i`: `{ text, tick, payload }`, where `payload` is a view into the store.
  frame(i) {
    const payload = this.bytes.subarray(this.offsets[i], this.offsets[i + 1]);
    return { text: this.kinds[i] === ENTRY_TEXT, tick: this.ticks[i], payload };
  }
}

function grown(array, size) {
  const next = new array.constructor(size);
  next.set(array);
  return next;
}

/// The seed and the stamp in a `match.id` (`{seed:016x}-{millis}`). Both are 64-bit, which a
/// JavaScript number cannot hold exactly, so both are BigInts.
export function matchIdentity(matchId) {
  const match = /^([0-9a-f]{16})-(\d+)$/.exec(matchId ?? '');
  if (!match) {
    return { seed: 0n, millis: 0n };
  }
  return { seed: BigInt(`0x${match[1]}`), millis: BigInt(match[2]) };
}

async function sha256(bytes) {
  return new Uint8Array(await globalThis.crypto.subtle.digest('SHA-256', bytes));
}

const hex = (bytes) => Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');

/// The payload of an input entry: the name's length (u16), the UTF-8 name, the bytes.
function inputPayload({ name, bytes }) {
  const encoded = encoder.encode(name);
  const payload = new Uint8Array(2 + encoded.length + bytes.length);
  new DataView(payload.buffer).setUint16(0, encoded.length, true);
  payload.set(encoded, 2);
  payload.set(bytes, 2 + encoded.length);
  return payload;
}

/// The whole file for `store`. `matchId` names the seed and the stamp for the header.
/// Without `record` the file is version 3, frames only, as the page saves a match it
/// watched. With `record` (as `readReplay` returns it) the file is version 4, in the engine's
/// entry order: the inputs, the frames, then the record, kept as its original bytes, so a
/// file the engine wrote is written back byte for byte.
export async function writeReplay(store, { matchId, version = PROTOCOL_VERSION, record = null }) {
  const { seed, millis } = matchIdentity(matchId);
  const inputs = record ? record.inputs.map(inputPayload) : [];
  const tail = record ? [record.raw] : [];
  const extra = [...inputs, ...tail];
  const extraBytes = extra.reduce((sum, p) => sum + ENTRY_BYTES + p.length, 0);
  const total = HEADER_BYTES + store.count * ENTRY_BYTES + store.used + extraBytes + TRAILER_BYTES;
  const out = new Uint8Array(total);
  const view = new DataView(out.buffer);
  out.set(MAGIC, 0);
  if (record) {
    view.setUint16(4, FORMAT_VERSION, true);
    view.setUint16(6, version, true);
  } else {
    view.setUint16(4, version, true);
  }
  view.setBigUint64(8, millis, true);
  view.setUint32(16, store.count, true);
  view.setUint32(20, store.tickFrames, true);
  view.setBigUint64(24, seed, true);
  let at = HEADER_BYTES;
  const entry = (kind, tick, payload) => {
    out[at] = kind;
    view.setUint32(at + 1, tick, true);
    view.setUint32(at + 5, payload.length, true);
    out.set(payload, at + ENTRY_BYTES);
    at += ENTRY_BYTES + payload.length;
  };
  for (const payload of inputs) {
    entry(ENTRY_INPUT, 0, payload);
  }
  for (let i = 0; i < store.count; i += 1) {
    const { text, tick, payload } = store.frame(i);
    entry(text ? ENTRY_TEXT : ENTRY_BINARY, tick, payload);
  }
  for (const payload of tail) {
    entry(ENTRY_RECORD, 0, payload);
  }
  const digest = await sha256(concat([...inputs, store.bytes.subarray(0, store.used), ...tail]));
  out.set(TRAILER_MAGIC, at);
  view.setUint32(at + 4, store.count, true);
  out.set(digest.subarray(0, 6), at + 8);
  return { bytes: out, hash: hex(digest.subarray(0, 6)) };
}

function concat(parts) {
  const out = new Uint8Array(parts.reduce((sum, p) => sum + p.length, 0));
  let at = 0;
  for (const part of parts) {
    out.set(part, at);
    at += part.length;
  }
  return out;
}

/// A refusal that names the check that failed.
export class ReplayRefused extends Error {
  constructor(reason) {
    super(reason);
    this.name = 'ReplayRefused';
    this.reason = reason;
  }
}

const same = (bytes, at, magic) => magic.every((b, i) => bytes[at + i] === b);

/// Reads a replay file into a new store. Fails closed, naming the check: the magic, the
/// format and protocol versions, the trailer, the counts, a truncated entry, an entry out of
/// place, the hash, the inputs against the record's list, and a file with no hello. A
/// version-4 file's inputs and record stay out of the store; `record` returns them
/// (`{ inputs: [{ role, name, bytes }], meta, raw }`), and is null for a version-3 file.
export async function readReplay(input) {
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
  if (bytes.length < HEADER_BYTES + TRAILER_BYTES) {
    throw new ReplayRefused('the file is shorter than a replay header and trailer');
  }
  if (!same(bytes, 0, MAGIC)) {
    throw new ReplayRefused('bad magic: this is not a replay file');
  }
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const version = view.getUint16(4, true);
  if (version !== LEGACY_VERSION && version !== FORMAT_VERSION) {
    throw new ReplayRefused(
      `format ${version}; this page reads formats ${LEGACY_VERSION} and ${FORMAT_VERSION} (read as a version-3 file: protocol version ${version}; this page reads ${PROTOCOL_VERSION})`
    );
  }
  const protocol = version === FORMAT_VERSION ? view.getUint16(6, true) : version;
  if (protocol !== PROTOCOL_VERSION) {
    throw new ReplayRefused(
      `frames protocol version ${protocol}; this page reads ${PROTOCOL_VERSION}`
    );
  }
  const frames = view.getUint32(16, true);
  const ticks = view.getUint32(20, true);
  const trailerAt = bytes.length - TRAILER_BYTES;
  if (!same(bytes, trailerAt, TRAILER_MAGIC)) {
    throw new ReplayRefused('missing trailer: the file is incomplete');
  }
  if (view.getUint32(trailerAt + 4, true) !== frames) {
    throw new ReplayRefused('frame count mismatch between the header and the trailer');
  }
  const store = new FrameStore(Math.max(1024, bytes.length), Math.max(16, frames));
  const inputs = [];
  let raw = null;
  let at = HEADER_BYTES;
  while (at < trailerAt) {
    if (at + ENTRY_BYTES > trailerAt) {
      throw new ReplayRefused('a frame entry is truncated');
    }
    const kind = bytes[at];
    const tick = view.getUint32(at + 1, true);
    const length = view.getUint32(at + 5, true);
    at += ENTRY_BYTES;
    if (at + length > trailerAt) {
      throw new ReplayRefused(`the frame at tick ${tick} runs past the end of the file`);
    }
    const payload = bytes.subarray(at, at + length);
    at += length;
    if (raw) {
      throw new ReplayRefused(
        kind === ENTRY_RECORD
          ? 'the file holds two record entries'
          : `an entry of kind ${kind} follows the record entry`
      );
    }
    if (version === LEGACY_VERSION && (kind === ENTRY_INPUT || kind === ENTRY_RECORD)) {
      throw new ReplayRefused(`entry kind ${kind} in a version-3 file, which holds frames only`);
    }
    if (kind === ENTRY_BINARY) {
      store.lastTick = tick - 1;
      store.addBinary(payload);
    } else if (kind === ENTRY_TEXT) {
      store.push(ENTRY_TEXT, tick, payload);
    } else if (kind === ENTRY_INPUT) {
      if (store.count > 0) {
        throw new ReplayRefused('an input entry follows the frames');
      }
      if (payload.length < 2) {
        throw new ReplayRefused('an input entry is truncated');
      }
      const nameLength = new DataView(payload.buffer, payload.byteOffset, 2).getUint16(0, true);
      if (payload.length < 2 + nameLength) {
        throw new ReplayRefused("an input entry's name is truncated");
      }
      inputs.push({
        payload,
        name: decoder.decode(payload.subarray(2, 2 + nameLength)),
        bytes: payload.slice(2 + nameLength),
      });
    } else if (kind === ENTRY_RECORD) {
      raw = payload.slice();
    } else {
      throw new ReplayRefused(`unknown entry kind ${kind} at tick ${tick}`);
    }
  }
  if (store.count !== frames || store.tickFrames !== ticks) {
    throw new ReplayRefused(
      `frame count mismatch: the header says ${frames} frames and ${ticks} ticks, the file holds ${store.count} and ${store.tickFrames}`
    );
  }
  const digest = await sha256(
    concat([...inputs.map((i) => i.payload), store.bytes.subarray(0, store.used), ...(raw ? [raw] : [])])
  );
  if (!digest.subarray(0, 6).every((b, i) => b === bytes[trailerAt + 8 + i])) {
    throw new ReplayRefused('hash mismatch: the frame bytes do not match the trailer');
  }
  let record = null;
  if (version === FORMAT_VERSION) {
    if (!raw) {
      throw new ReplayRefused('a version-4 file must end with its record entry; this one has none');
    }
    record = await matchedRecord(raw, inputs);
  }
  const first = store.count > 0 ? store.frame(0) : null;
  let hello = null;
  if (first && first.text) {
    try {
      const message = JSON.parse(decoder.decode(first.payload));
      hello = message.type === 'hello' ? message : null;
    } catch {
      hello = null;
    }
  }
  if (!hello) {
    throw new ReplayRefused('the file does not open with a hello');
  }
  store.helloStored = true;
  return { store, hello, version, frames, ticks, record, hash: hex(digest.subarray(0, 6)) };
}

/// The record of a version-4 file, when every input entry is the one its list names at its
/// place, with the same size and SHA-256.
async function matchedRecord(raw, inputs) {
  let meta;
  try {
    meta = JSON.parse(decoder.decode(raw));
  } catch (error) {
    throw new ReplayRefused(`the record entry is not a record: ${error.message}`);
  }
  const listed = Array.isArray(meta?.inputs) ? meta.inputs : [];
  if (listed.length !== inputs.length) {
    throw new ReplayRefused(
      `the file holds ${inputs.length} input entries and the record lists ${listed.length}`
    );
  }
  const out = [];
  for (let i = 0; i < inputs.length; i += 1) {
    const { name, bytes } = inputs[i];
    const digest = hex(await sha256(bytes));
    const entry = listed[i];
    if (entry.name !== name || entry.sha256 !== digest || entry.bytes !== bytes.length) {
      throw new ReplayRefused(
        `input ${name} (SHA-256 ${digest}) does not match the record's ${entry.role} ${entry.name} (SHA-256 ${entry.sha256})`
      );
    }
    out.push({ role: entry.role, name, bytes });
  }
  return { inputs: out, meta, raw };
}

/// The text of a stored text frame.
export function frameText(payload) {
  return decoder.decode(payload);
}
