// The whole match in memory, so a rewind is a lookup rather than a replay.
//
// The alternative is keeping the wire form and walking forward from the last keyframe on
// every scrub, which costs up to 49 delta applications per drag step. The decoded form
// costs 94 bytes a tick: a 90-minute match is 25,380,000 bytes, about a twelfth of the
// 300-megabyte budget. Buying an O(1) rewind with a twelfth of the budget is the trade.

import { COMPONENT_COUNT, PLAYER_COUNT } from './decode.mjs';
import { signal } from './signal.mjs';

/// The budget the whole page is held to, in bytes. 300 MB.
export const BUDGET_BYTES = 314_572_800;

/// Ball x, y and height.
const BALL_COMPONENTS = 3;

export class History {
  /// `ticksExpected` comes from the hello. Stoppage time can run past it, so the buffers
  /// grow rather than refuse.
  constructor(ticksExpected) {
    const capacity = Math.max(1, ticksExpected);
    this.capacity = capacity;
    this.count = 0;
    this.firstTick = 0;
    this.ball = new Int16Array(capacity * BALL_COMPONENTS);
    this.players = new Int16Array(capacity * PLAYER_COUNT * 2);
    this.pageBytes = null;
    this.pageBytesReason = 'not measured yet';
    this.measuring = false;
  }

  /// The exact bytes the history holds. Synchronous and never an estimate.
  bytes() {
    return this.ball.byteLength + this.players.byteLength;
  }

  /// The newest tick stored, or 0 when the history is empty.
  get newestTick() {
    return this.count === 0 ? 0 : this.firstTick + this.count - 1;
  }

  /// Stores one decoded tick. Ticks arrive in order.
  append(tick, components) {
    if (this.count === 0) {
      this.firstTick = tick;
    }
    if (this.count === this.capacity) {
      this.grow();
    }
    const i = this.count;
    this.ball[i * BALL_COMPONENTS] = components[0];
    this.ball[i * BALL_COMPONENTS + 1] = components[1];
    this.ball[i * BALL_COMPONENTS + 2] = components[2];
    this.players.set(components.subarray(BALL_COMPONENTS), i * PLAYER_COUNT * 2);
    this.count += 1;
  }

  /// Writes the stored tick into `out`, in wire component order. Returns false when the
  /// tick was never stored.
  tickAt(tick, out) {
    const i = tick - this.firstTick;
    if (i < 0 || i >= this.count) {
      return false;
    }
    out[0] = this.ball[i * BALL_COMPONENTS];
    out[1] = this.ball[i * BALL_COMPONENTS + 1];
    out[2] = this.ball[i * BALL_COMPONENTS + 2];
    const from = i * PLAYER_COUNT * 2;
    out.set(this.players.subarray(from, from + PLAYER_COUNT * 2), BALL_COMPONENTS);
    return true;
  }

  /// Doubles both buffers, keeping every stored tick.
  grow() {
    this.capacity *= 2;
    const ball = new Int16Array(this.capacity * BALL_COMPONENTS);
    ball.set(this.ball);
    this.ball = ball;
    const players = new Int16Array(this.capacity * PLAYER_COUNT * 2);
    players.set(this.players);
    this.players = players;
  }

  /// One `viewer.history` row. The caller sends it after each keyframe.
  ///
  /// `history_bytes` is exact and synchronous. `page_bytes` is the browser's own figure
  /// and is null until it resolves. A null always carries `page_bytes_reason`, so a
  /// missing figure is a named shortfall rather than a silent pass.
  report() {
    return signal('viewer.history', {
      ticks_stored: this.count,
      history_bytes: this.bytes(),
      page_bytes: this.pageBytes,
      page_bytes_reason: this.pageBytes === null ? this.pageBytesReason : null,
      budget_bytes: BUDGET_BYTES,
    });
  }

  /// Asks the browser what the whole page costs. Never blocks a frame: the answer lands
  /// on a later turn and is reported with the next keyframe.
  ///
  /// The call is asked once and not again while one is outstanding, because the browser
  /// coalesces requests and a request per keyframe would queue fifty a second.
  measurePage() {
    if (this.measuring) {
      return;
    }
    if (typeof performance === 'undefined' || !performance.measureUserAgentSpecificMemory) {
      this.pageBytesReason = 'the browser has no measureUserAgentSpecificMemory';
      return;
    }
    this.measuring = true;
    try {
      performance
        .measureUserAgentSpecificMemory()
        .then((result) => {
          this.pageBytes = result.bytes;
          this.pageBytesReason = null;
          this.measuring = false;
        })
        .catch((error) => {
          this.pageBytes = null;
          this.pageBytesReason = `${error.name}: ${error.message}`;
          this.measuring = false;
        });
    } catch (error) {
      // Chrome refuses synchronously with a SecurityError in an embedded browser, even
      // where the page is cross-origin isolated, a secure context, and the function is
      // present. repro: crates/engine-cli serves the page with both isolation headers,
      // `crossOriginIsolated` reads true, and the call still throws
      // "performance.measureUserAgentSpecificMemory is not available".
      this.pageBytes = null;
      this.pageBytesReason = `${error.name}: ${error.message}`;
      this.measuring = false;
    }
  }
}

export { COMPONENT_COUNT };
