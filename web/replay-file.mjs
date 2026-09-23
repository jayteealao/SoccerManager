// Replay files: the wire bytes of one match, kept as they arrived and saved in the engine's
// own fixture layout (`.smfx`, `crates/stream/src/record.rs`).
//
// The page never re-encodes a position. Every frame is stored exactly as the socket handed
// it over, so a saved file is the recorded stream byte for byte, and the engine's own
// `replay` command reads it. Layout, little-endian:
// - Header, 32 bytes: magic `SMFX`, protocol version (u16), two zero bytes, the match stamp
//   in milliseconds (u64), the frame count (u32), the tick count (u32), and the seed (u64).
// - One entry per frame: kind (u8, 0 binary and 1 text), tick (u32), length (u32), then the
//   payload. A text frame carries the tick of the tick frame before it.
// - Trailer, 16 bytes: magic `SMFE`, the frame count (u32), the first six bytes of the
//   SHA-256 over every payload in order, and two zero bytes.

/// The protocol version this page reads.
export const PROTOCOL_VERSION = 3;

const MAGIC = [0x53, 0x4d, 0x46, 0x58]; // SMFX
const TRAILER_MAGIC = [0x53, 0x4d, 0x46, 0x45]; // SMFE
export const HEADER_BYTES = 32;
export const TRAILER_BYTES = 16;
const ENTRY_BYTES = 9;
const ENTRY_BINARY = 0;
const ENTRY_TEXT = 1;
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

/// The whole file for `store`. `matchId` names the seed and the stamp for the header.
export async function writeReplay(store, { matchId, version = PROTOCOL_VERSION }) {
  const { seed, millis } = matchIdentity(matchId);
  const total = HEADER_BYTES + store.count * ENTRY_BYTES + store.used + TRAILER_BYTES;
  const out = new Uint8Array(total);
  const view = new DataView(out.buffer);
  out.set(MAGIC, 0);
  view.setUint16(4, version, true);
  view.setBigUint64(8, millis, true);
  view.setUint32(16, store.count, true);
  view.setUint32(20, store.tickFrames, true);
  view.setBigUint64(24, seed, true);
  let at = HEADER_BYTES;
  for (let i = 0; i < store.count; i += 1) {
    const { text, tick, payload } = store.frame(i);
    out[at] = text ? ENTRY_TEXT : ENTRY_BINARY;
    view.setUint32(at + 1, tick, true);
    view.setUint32(at + 5, payload.length, true);
    out.set(payload, at + ENTRY_BYTES);
    at += ENTRY_BYTES + payload.length;
  }
  const digest = await sha256(store.bytes.slice(0, store.used));
  out.set(TRAILER_MAGIC, at);
  view.setUint32(at + 4, store.count, true);
  out.set(digest.subarray(0, 6), at + 8);
  return { bytes: out, hash: hex(digest.subarray(0, 6)) };
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
/// version, the trailer, the counts, a truncated entry, the hash, and a file with no hello.
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
  if (version !== PROTOCOL_VERSION) {
    throw new ReplayRefused(`protocol version ${version}; this page reads ${PROTOCOL_VERSION}`);
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
    if (kind === ENTRY_BINARY) {
      store.lastTick = tick - 1;
      store.addBinary(payload);
    } else if (kind === ENTRY_TEXT) {
      store.push(ENTRY_TEXT, tick, payload);
    } else {
      throw new ReplayRefused(`unknown entry kind ${kind} at tick ${tick}`);
    }
  }
  if (store.count !== frames || store.tickFrames !== ticks) {
    throw new ReplayRefused(
      `frame count mismatch: the header says ${frames} frames and ${ticks} ticks, the file holds ${store.count} and ${store.tickFrames}`
    );
  }
  const digest = await sha256(store.bytes.slice(0, store.used));
  if (!digest.subarray(0, 6).every((b, i) => b === bytes[trailerAt + 8 + i])) {
    throw new ReplayRefused('hash mismatch: the frame bytes do not match the trailer');
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
  return { store, hello, version, frames, ticks, hash: hex(digest.subarray(0, 6)) };
}

/// The text of a stored text frame.
export function frameText(payload) {
  return decoder.decode(payload);
}
