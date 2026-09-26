// Keeps a live engine close to the tick the pitch is drawing. Pure: no DOM, no socket.
//
// The page reads the socket as fast as it can and stores every tick, so the engine's own
// bounded buffer never fills and a live match would be simulated to full time within
// seconds. A change the manager queued at the 20th minute would then reach an engine that
// had already finished. The page therefore pulls: it sends `pause` when it holds more than
// a few seconds of unplayed match, and `start` when playback has caught up.

import { TICKS_PER_SECOND } from './schedule.mjs';

/// Seconds of unplayed match, at the playback speed, before the page pauses the engine.
export const PAUSE_AFTER_S = 5;
/// Seconds of unplayed match, at the playback speed, below which the page restarts it.
export const RESUME_BELOW_S = 2;

export class LeadControl {
  constructor() {
    /// `true` while the page has the engine paused.
    this.holding = false;
    this.pauses = 0;
  }

  /// The command to send, or `null`, for `lead` stored ticks ahead of the rendered tick at
  /// playback `speed`. After full time the engine has nothing left to produce.
  next(lead, speed = 1, fullTime = false) {
    if (fullTime) {
      this.holding = false;
      return null;
    }
    const perSecond = TICKS_PER_SECOND * Math.max(1, speed);
    if (!this.holding && lead > PAUSE_AFTER_S * perSecond) {
      this.holding = true;
      this.pauses += 1;
      return 'pause';
    }
    if (this.holding && lead < RESUME_BELOW_S * perSecond) {
      this.holding = false;
      return 'start';
    }
    return null;
  }
}

/// The fewest milliseconds between two `seen` reports.
export const SEEN_EVERY_MS = 100;

/// Decides when the page tells the engine which tick it has drawn. A `serve` engine that
/// hears it stays within its buffer bound of that tick, which `pause` and `start` alone
/// cannot promise: the engine simulates thousands of ticks in the time a pause takes to
/// arrive. Pure: the caller sends the tick this returns.
export class SeenReport {
  constructor() {
    this.sentTick = null;
    this.sentAt = -Infinity;
  }

  /// The tick to report for drawn tick `tick` at time `now` in milliseconds, or `null`.
  next(tick, now) {
    if (tick === this.sentTick || now - this.sentAt < SEEN_EVERY_MS) {
      return null;
    }
    this.sentTick = tick;
    this.sentAt = now;
    return tick;
  }
}
