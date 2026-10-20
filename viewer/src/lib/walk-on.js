// The walk-on: before the first tick of each half the engine sends the players' walk from the
// tunnel on the near touchline to the places that tick starts from (a `walk-on` message,
// protocol 8). It is not match time, so no tick carries it: the page draws it as an interlude
// before the half's first tick, at the playback speed, and the clock does not move.
//
// A seek, a scrub, a skip or any jump that passes a half's first tick passes its walk-on too:
// only playback that runs into the tick plays it.

import { TICKS_PER_SECOND } from './schedule.js';

/// The widest step of the playback cursor that still plays a walk-on. A wider step is a jump.
export const WALK_ON_MAX_STEP = 25;

/// How long a walk-on lasts, in seconds of match time.
export function walkOnSeconds(message, ticksPerSecond = TICKS_PER_SECOND) {
  return message.ticks / ticksPerSecond;
}

/// Where each player stands `seconds` into the walk-on, as `[slot, x, y]` in metres: on the
/// straight line from the tunnel to his place, at the walk's speed, standing once he arrives.
export function walkOnAt(message, seconds) {
  const walked = Math.max(0, seconds) * message.speed;
  return message.paths.map(({ player, from, to }) => {
    const dx = to[0] - from[0];
    const dy = to[1] - from[1];
    const length = Math.hypot(dx, dy);
    if (length <= 0 || walked >= length) {
      return [player, to[0], to[1]];
    }
    const f = walked / length;
    return [player, from[0] + dx * f, from[1] + dy * f];
  });
}

/// Writes the walk-on `seconds` in into `out`, a wire-order component array: the ball and every
/// player who does not walk as in `base` (the half's first tick), each walking player at his
/// place on the walk, in centimetres.
export function drawWalkOn(message, seconds, base, out) {
  out.set(base);
  for (const [player, x, y] of walkOnAt(message, seconds)) {
    out[3 + player * 2] = Math.round(x * 100);
    out[4 + player * 2] = Math.round(y * 100);
  }
  return out;
}

/// The walk-on the playback cursor runs into when it moves from tick `before` to tick `now`,
/// or null: a half's first tick inside the step, and a step no wider than `WALK_ON_MAX_STEP`.
export function walkOnDue(walkOns, before, now) {
  if (now - before > WALK_ON_MAX_STEP) {
    return null;
  }
  for (const [tick, message] of walkOns) {
    if (before < tick && tick <= now) {
      return message;
    }
  }
  return null;
}
