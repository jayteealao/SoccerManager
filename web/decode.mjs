// The wire decoder, mirroring `crates/protocol/src/codec.rs` component for component.
//
// Every constant below is ported, not guessed. `docs/reference/protocol.md` is the written
// source and `crates/protocol/tests/document.rs` holds that document to the Rust code, so
// the three stay in step. A wrong offset here draws a match that never happened.

import { signal } from './signal.mjs';

/// Frame kind tags, from `frame.rs`.
export const KIND_KEYFRAME = 0x01;
export const KIND_DELTA = 0x02;
export const KIND_RESTART = 0x03;

/// Payload sizes, from `codec.rs`. The tag byte is not counted.
export const KEYFRAME_BYTES = 98;
export const DELTA_BYTES = 47;

export const PLAYER_COUNT = 22;

/// Ball x, y, height, then two components per player: 47 signed centimetre values.
export const COMPONENT_COUNT = 3 + PLAYER_COUNT * 2;

/// One decoded tick. `components` is the wire order, so a copy into the history is one
/// `set` call rather than a loop over named fields.
export function newFrame() {
  return { tick: 0, components: new Int16Array(COMPONENT_COUNT) };
}

/// Decodes one frame into `out`, using `prev` as the base for a delta.
///
/// Returns the kind and the tick, or `null` when the frame cannot be read. A frame that
/// cannot be read is named through `viewer.decode_refused` rather than swallowed, because
/// a silently dropped frame reads as a rendering stutter and sends a reader to the wrong
/// code.
export function decodeInto(buffer, prev, out) {
  const bytes = new Uint8Array(buffer);
  if (bytes.length === 0) {
    signal('viewer.decode_refused', {
      reason: 'empty frame',
      tick: prev.tick,
      bytes: 0,
      kind: null,
    });
    return null;
  }
  const kind = bytes[0];
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);

  if (kind === KIND_KEYFRAME || kind === KIND_RESTART) {
    if (bytes.length !== 1 + KEYFRAME_BYTES) {
      return refuse('keyframe length', prev.tick, bytes.length, kind);
    }
    out.tick = view.getUint32(1, true);
    for (let i = 0; i < COMPONENT_COUNT; i += 1) {
      out.components[i] = view.getInt16(5 + i * 2, true);
    }
    return { kind: kind === KIND_RESTART ? 'restart' : 'keyframe', tick: out.tick };
  }

  if (kind === KIND_DELTA) {
    if (bytes.length !== 1 + DELTA_BYTES) {
      return refuse('delta length', prev.tick, bytes.length, kind);
    }
    // A delta carries no tick number: it is the tick after the frame before it.
    out.tick = prev.tick + 1;
    for (let i = 0; i < COMPONENT_COUNT; i += 1) {
      out.components[i] = prev.components[i] + view.getInt8(1 + i);
    }
    return { kind: 'delta', tick: out.tick };
  }

  return refuse('unknown frame kind', prev.tick, bytes.length, kind);
}

function refuse(reason, tick, bytes, kind) {
  signal('viewer.decode_refused', { reason, tick, bytes, kind });
  return null;
}
