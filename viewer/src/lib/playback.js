// The playback controls, and the estimator behind the lag notice.
//
// No server message announces lag. The producer blocks and writes one line on its own
// side; nothing about it reaches a client, and a browser cannot see its own inbound queue
// either, because `WebSocket.bufferedAmount` counts outgoing bytes only. So the page
// measures: it compares the rate ticks actually arrive at against the rate the manager
// asked for, and when the stream cannot keep up it says so, in words and in digits.

import { TICKS_PER_SECOND } from './schedule.js';
import { signal } from './signal.js';

/// The speeds the selector offers.
export const SPEEDS = [1, 2, 4, 8];

/// A sustained rate this far below the requested one raises the notice. Ten percent is
/// wide enough that ordinary jitter never trips it.
export const LAG_TOLERANCE = 0.9;

/// Seconds of unplayed match, at the requested speed, below which a slow arrival rate is lag.
/// A live engine held near the drawn tick delivers ticks exactly as fast as playback uses
/// them, so a low rate with a healthy lead is the engine waiting, not falling behind; and
/// read as lag, it would slow playback, which slows the engine, and never recover.
export const STARVING_S = 0.5;

/// Seconds of arrivals the estimate is built from.
const WINDOW_S = 3;

export class SustainedRate {
  constructor() {
    this.arrivals = [];
  }

  /// Records one tick arrival and returns the rate, in times real time, over the window.
  note(nowMs) {
    this.arrivals.push(nowMs);
    const cutoff = nowMs - WINDOW_S * 1000;
    while (this.arrivals.length > 0 && this.arrivals[0] < cutoff) {
      this.arrivals.shift();
    }
    const span = nowMs - this.arrivals[0];
    if (this.arrivals.length < TICKS_PER_SECOND || span <= 0) {
      return null;
    }
    return (this.arrivals.length - 1) / (span / 1000) / TICKS_PER_SECOND;
  }
}

/// Rounds a measured rate to the nearest speed a manager can select, so the notice names
/// a speed rather than a measurement: "3x", never "2.97x".
export function nameRate(rate) {
  return Math.max(1, Math.round(rate));
}

const WORDS = {
  1: 'one',
  2: 'two',
  3: 'three',
  4: 'four',
  5: 'five',
  6: 'six',
  7: 'seven',
  8: 'eight',
};

/// The notice text. The rate appears in digits and in words, so the warning colour is
/// never the only thing carrying the meaning.
export function noticeText(sustained) {
  const named = nameRate(sustained);
  return `Playing at ${named}x — the engine sustains ${WORDS[named] ?? named} times real time`;
}

export class Playback {
  constructor({ scheduler, onSpeed, onNotice }) {
    this.scheduler = scheduler;
    this.onSpeed = onSpeed;
    this.onNotice = onNotice;
    this.requested = 1;
    this.effective = 1;
    this.rate = new SustainedRate();
    this.noticeShown = false;
    this.sustained = null;
  }

  /// The speed the manager asked for. Playback may settle lower; the notice says so.
  /// It applies at once: after the stream ends no arrival will recompute it.
  select(speed) {
    this.requested = speed;
    this.effective = this.noticeShown ? Math.min(speed, this.sustained) : speed;
    if (this.noticeShown && speed <= this.sustained) {
      this.noticeShown = false;
      signal('viewer.lag', {
        requested_speed: speed,
        sustained_speed: speed,
        measured_speed: this.sustained,
        tick: null,
        notice_shown: false,
      });
      this.onNotice(null);
    }
    this.apply();
  }

  /// One tick arrived. The estimate updates and the notice follows it. `lead` is the stored
  /// ticks not yet played; left out, the rate alone decides.
  noteArrival(nowMs, tick, lead = 0) {
    const measured = this.rate.note(nowMs);
    if (measured === null) {
      return;
    }
    const sustained = measured;
    const starving = lead < STARVING_S * TICKS_PER_SECOND * this.requested;
    const behind = starving && sustained < this.requested * LAG_TOLERANCE;
    const wasShown = this.noticeShown;
    const wasEffective = this.effective;
    this.effective = behind ? nameRate(sustained) : this.requested;
    this.noticeShown = behind;
    if (behind) {
      this.sustained = nameRate(sustained);
    }
    // Only speak when something changed. This runs fifty times a second at 1x and four
    // hundred at 8x; announcing every arrival would rewrite the page's live region that
    // often and make it unreadable to the one reader it exists for.
    if (this.effective !== wasEffective) {
      this.apply();
    }
    if (behind !== wasShown) {
      signal('viewer.lag', {
        requested_speed: this.requested,
        sustained_speed: behind ? nameRate(sustained) : this.requested,
        measured_speed: Math.round(sustained * 100) / 100,
        tick,
        notice_shown: behind,
      });
      this.onNotice(behind ? noticeText(sustained) : null);
    }
  }

  apply() {
    this.scheduler.setSpeed(this.effective);
    this.onSpeed(this.requested, this.effective);
  }
}
