// The half-time and full-time reports. Each is counted from the page's one event list, the
// same list the feed shows, so a report can never disagree with the feed. Pure: the Report
// screen renders what these return.

import { minuteStamp } from './feed.js';
import { KIND } from './match-state.js';

const type = (e) => e['event.type'];

/// The rows a report shows, in order. Every row counts events of one kind per club.
export const ROWS = Object.freeze([
  { id: 'goals', label: 'Goals', counts: (e) => type(e) === KIND.goal },
  { id: 'yellow', label: 'Yellow cards', counts: (e) => type(e) === KIND.card && e['card.kind'] === 'yellow' },
  {
    id: 'red',
    label: 'Red cards',
    counts: (e) => type(e) === KIND.card && (e['card.kind'] === 'red' || e['card.kind'] === 'second-yellow'),
  },
  { id: 'fouls', label: 'Fouls', counts: (e) => type(e) === 'foul' },
  { id: 'corners', label: 'Corners', counts: (e) => type(e) === 'corner' },
  { id: 'offsides', label: 'Offsides', counts: (e) => type(e) === 'offside' },
  { id: 'throw-ins', label: 'Throw-ins', counts: (e) => type(e) === 'throw-in' },
  { id: 'goal-kicks', label: 'Goal kicks', counts: (e) => type(e) === 'goal-kick' },
  { id: 'free-kicks', label: 'Free kicks', counts: (e) => type(e) === 'free-kick' },
  { id: 'penalties', label: 'Penalties', counts: (e) => type(e) === 'penalty' },
]);

/// The report at `uptoTick`: per club, every row's count, the score, and the goals and cards
/// with their minutes. `teams` are the hello's two clubs, home first.
export function reportModel(events, uptoTick, teams) {
  const ids = teams.map((t) => t['team.id']);
  const upto = events.filter((e) => e.tick <= uptoTick);
  const rows = ROWS.map((row) => {
    const counts = [0, 0];
    for (const event of upto) {
      const side = ids.indexOf(event['team.id']);
      if (side >= 0 && row.counts(event)) {
        counts[side] += 1;
      }
    }
    return { id: row.id, label: row.label, counts };
  });
  const last = upto[upto.length - 1];
  const moments = upto
    .filter((e) => type(e) === KIND.goal || type(e) === KIND.card)
    .map((e) => ({
      tick: e.tick,
      minute: minuteStamp(e),
      kind: type(e) === KIND.goal ? 'Goal' : e['card.kind'] === 'yellow' ? 'Yellow card' : 'Red card',
      side: ids.indexOf(e['team.id']),
      text: e.commentary ?? '',
    }));
  return {
    tick: uptoTick,
    teams: teams.map((t) => t['team.name']),
    score: last ? [last['home.score'], last['away.score']] : [0, 0],
    rows,
    moments,
  };
}

/// Which report the rendered tick has reached, once each: `half-time` or `full-time`, or null.
export class ReportClock {
  constructor() {
    this.shown = new Set();
  }

  /// The report to open now, given the break events in the list and the rendered tick.
  due(events, renderedTick) {
    for (const kind of [KIND.halfTime, KIND.fullTime]) {
      if (this.shown.has(kind)) {
        continue;
      }
      const at = events.find((e) => type(e) === kind);
      if (at && at.tick <= renderedTick) {
        this.shown.add(kind);
        return { kind, tick: at.tick };
      }
    }
    return null;
  }

  /// Marks every break at or before `tick` as shown without opening it: a scrub passes a
  /// break, and the report stays closed.
  pass(events, tick) {
    for (const kind of [KIND.halfTime, KIND.fullTime]) {
      const at = events.find((e) => type(e) === kind);
      if (at && at.tick <= tick) {
        this.shown.add(kind);
      }
    }
  }

  /// Starts again for another match.
  reset() {
    this.shown.clear();
  }
}
