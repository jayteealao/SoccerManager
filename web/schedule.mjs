// The frame scheduler: a fixed-timestep accumulator driven by animation-frame timestamps.
//
// The load-bearing measurement rule lives here. `requestAnimationFrame` fires at the
// display's refresh rate, so counting callbacks on a 144 hertz panel reports 144 and a
// naive 60-frames-per-second assertion passes for the wrong reason. Every rate this module
// reports is derived from consecutive timestamp deltas, and `refresh_hz` is reported
// beside it, so a reader can always see whether a number is a rendering fact or a panel
// fact.
//
// Ticks are skipped only at a speed that needs more than one tick per frame at the measured
// panel rate; below that, a stalled frame delays playback by landing on the next tick.

import { signal } from './signal.mjs';

/// Engine ticks per second of match time.
export const TICKS_PER_SECOND = 50;

/// The longest gap one frame may consume. A tab that was hidden for a minute must not
/// spend a single frame consuming three thousand ticks.
const MAX_FRAME_MS = 250;

/// Frame durations kept for the rate figures.
const WINDOW = 300;
/// Frames between two measurements of the panel rate. The rate is a percentile of the whole
/// window, so sorting it on every frame would buy nothing but work.
const REMEASURE_FRAMES = 30;

const percentile = (sorted, p) =>
  sorted.length === 0 ? 0 : sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * p))];

export class Scheduler {
  constructor({ ticksPerSecond = TICKS_PER_SECOND } = {}) {
    this.ticksPerSecond = ticksPerSecond;
    this.speed = 1;
    this.playing = true;
    this.position = 0;
    this.lastTimestamp = null;
    this.durations = [];
    this.skipped = 0;
    this.skippedWindowAt = 0;
    this.budgetWindowAt = 0;
    this.lastTick = 0;
    /// The measured panel rate, and the frames recorded since it was measured.
    this.measuredHz = null;
    this.sinceMeasured = 0;
  }

  setSpeed(speed) {
    this.speed = speed;
  }

  setPlaying(playing) {
    this.playing = playing;
  }

  /// Puts the cursor on an exact tick, for a rewind. The fraction resets to zero, so the
  /// next frame drawn is the stored tick itself.
  seek(tick) {
    this.position = tick;
  }

  /// Advances by one animation frame and reports what to draw.
  ///
  /// `newestTick` is the newest tick received. The returned `to` never passes it and the
  /// returned `fraction` never leaves `[0, 1]`, so no caller can draw ahead of the engine.
  advance(timestampMs, newestTick, oldestTick = 0) {
    const previous = this.lastTimestamp;
    this.lastTimestamp = timestampMs;
    if (previous !== null) {
      this.durations.push(timestampMs - previous);
      if (this.durations.length > WINDOW) {
        this.durations.shift();
      }
      this.sinceMeasured += 1;
    }
    const elapsed = previous === null ? 0 : Math.min(MAX_FRAME_MS, timestampMs - previous);

    const before = Math.floor(this.position);
    if (this.playing) {
      this.position += (elapsed / 1000) * this.ticksPerSecond * this.speed;
      // A speed the panel can show one tick a frame never skips: a stalled frame that
      // would pass over a tick lands on the next tick instead, and playback runs late.
      if (this.speed * this.ticksPerSecond <= this.refreshHz() && this.position >= before + 2) {
        this.position = before + 1;
      }
    }
    if (this.position < oldestTick) {
      this.position = oldestTick;
    }
    if (this.position > newestTick) {
      this.position = newestTick;
    }

    const from = Math.floor(this.position);
    const to = Math.min(from + 1, newestTick);
    const fraction = to > from ? this.position - from : 0;

    // A skipped tick is one the cursor passed over without ever being the frame drawn.
    const stepped = from - before;
    if (stepped > 1) {
      this.skipped += stepped - 1;
    }
    this.lastTick = from;

    this.reportWindows(timestampMs);
    return { from, to, fraction, skipped: this.skipped };
  }

  /// The rate figures, all derived from timestamp deltas and never from a frame count.
  ///
  /// `refresh_hz` comes from the fastest typical frame, which is the panel's own period.
  /// `fps_median` comes from the median frame, which is the rate actually delivered. They
  /// differ exactly when frames are being dropped, and that difference is the reading.
  budget() {
    const sorted = [...this.durations].sort((a, b) => a - b);
    const fastest = percentile(sorted, 0.1);
    const median = percentile(sorted, 0.5);
    const p95 = percentile(sorted, 0.95);
    const dropped = fastest > 0 ? sorted.filter((d) => d > fastest * 1.5).length : 0;
    return {
      frames: this.durations.length,
      fps_median: median > 0 ? Math.round((1000 / median) * 10) / 10 : 0,
      frame_ms_p95: Math.round(p95 * 100) / 100,
      dropped_frames: dropped,
      refresh_hz: fastest > 0 ? Math.round(1000 / fastest) : 0,
    };
  }

  /// The panel rate, derived as `budget()` derives `refresh_hz`, or a nominal 60 until
  /// enough frames have been measured.
  /// Measured again every `REMEASURE_FRAMES` frames, not on every frame.
  refreshHz() {
    if (this.durations.length < 10) {
      return 60;
    }
    if (this.measuredHz === null || this.sinceMeasured >= REMEASURE_FRAMES) {
      const fastest = percentile([...this.durations].sort((a, b) => a - b), 0.1);
      this.measuredHz = fastest > 0 ? 1000 / fastest : 60;
      this.sinceMeasured = 0;
    }
    return this.measuredHz;
  }

  /// One `viewer.tick_skipped` row a second while ticks are being skipped, and one
  /// `viewer.frame_budget` row every five seconds.
  reportWindows(timestampMs) {
    if (timestampMs - this.skippedWindowAt >= 1000) {
      if (this.skipped > 0) {
        signal('viewer.tick_skipped', {
          tick: this.lastTick,
          skipped: this.skipped,
          speed: this.speed,
          window_s: (timestampMs - this.skippedWindowAt) / 1000,
        });
        this.skipped = 0;
      }
      this.skippedWindowAt = timestampMs;
    }
    if (timestampMs - this.budgetWindowAt >= 5000) {
      signal('viewer.frame_budget', {
        ...this.budget(),
        window_s: (timestampMs - this.budgetWindowAt) / 1000,
      });
      this.budgetWindowAt = timestampMs;
    }
  }
}
