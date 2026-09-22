// Where play stops, merged from the two sources that already mark it.
//
// The restart tag on a tick frame marks every tick that restarts play, and the event
// messages carry their own ticks. Merging two sources the protocol already sends beats
// adding a third. A goal and the restart that follows it are one stoppage to a manager,
// not two, so entries within half a second of each other collapse to the earlier.

/// Two marks closer than this are the same stoppage. 25 ticks is half a second.
export const COLLAPSE_TICKS = 25;

export class Stoppages {
  constructor() {
    this.ticks = [];
  }

  /// Adds one mark. Marks arrive in tick order, so the collapse only looks back one entry.
  add(tick) {
    const last = this.ticks[this.ticks.length - 1];
    if (last !== undefined && tick - last <= COLLAPSE_TICKS) {
      return;
    }
    this.ticks.push(tick);
  }

  /// The first stoppage after `fromTick`, or null when there is none.
  next(fromTick) {
    for (const tick of this.ticks) {
      if (tick > fromTick) {
        return tick;
      }
    }
    return null;
  }

  /// The last stoppage before `fromTick`, or null when there is none.
  prev(fromTick) {
    let found = null;
    for (const tick of this.ticks) {
      if (tick < fromTick) {
        found = tick;
      } else {
        break;
      }
    }
    return found;
  }

  get length() {
    return this.ticks.length;
  }
}
