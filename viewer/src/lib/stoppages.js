// Where play stops, merged from the two sources that already mark it.
//
// The restart tag on a tick frame marks every tick that restarts play, and the event
// messages carry their own ticks. Merging two sources the protocol already sends beats
// adding a third. A goal and the restart that follows it are one stoppage to a manager,
// not two, so entries within half a second of each other collapse to the earlier. Only an
// event that stops play is a mark: a foul played on with advantage, a card shown at a later
// stoppage, a queued tactical change, a substitution, and the AI manager's choice are not
// (they happen at a stoppage another event already marks). An injury stops play for a
// dropped ball.

/// Two marks closer than this are the same stoppage. 25 ticks is half a second.
export const COLLAPSE_TICKS = 25;

/// The event types that stop play.
export const STOPS_PLAY = new Set([
  'kick-off',
  'goal',
  'offside',
  'throw-in',
  'corner',
  'goal-kick',
  'free-kick',
  'penalty',
  'half-time',
  'full-time',
  'injury',
]);

/// `true` when an event message stops play. A foul stops play unless the referee played
/// advantage; a foul that stops play is followed by its free kick or penalty, which is the
/// mark either way.
export function stopsPlay(message) {
  if (message.type !== 'event') {
    return false;
  }
  const type = message['event.type'];
  if (type === 'foul') {
    return message['foul.advantage'] !== true;
  }
  return STOPS_PLAY.has(type);
}

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

  /// Drops every mark after `tick`, where a resumed match continues.
  truncate(tick) {
    this.ticks = this.ticks.filter((t) => t <= tick);
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
