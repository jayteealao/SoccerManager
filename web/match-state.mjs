// The match as the manager should see it at the tick the pitch is drawing.
//
// The engine streams up to 500 ticks ahead of playback, so an event, a statistics line, or an
// energy reading usually arrives well before the pitch reaches it. Showing a message when it
// arrives would announce a goal before the ball crosses the line. Every message is stored by
// its tick instead, and `at(tick)` answers "what is true at the rendered tick?". Pure: no DOM.

/// Every event kind the match-day panels read, in one table. A later rename of an event kind
/// changes this table and `STOPS_PLAY` in `stoppages.mjs`, and nothing else.
export const KIND = Object.freeze({
  kickOff: 'kick-off',
  goal: 'goal',
  card: 'card',
  substitution: 'substitution',
  injury: 'injury',
  tacticsChange: 'tactics-change',
  aiDecision: 'ai-decision',
  halfTime: 'half-time',
  fullTime: 'full-time',
});

/// The feed rows that carry a highlight and a state word, never colour alone.
export const HIGHLIGHT_WORDS = Object.freeze({
  [KIND.goal]: 'Goal',
  [KIND.card]: 'Card',
  [KIND.substitution]: 'Substitution',
  [KIND.injury]: 'Injury',
});

/// Event kinds kept out of the feed. The computer manager's reasoning is not a match event:
/// its visible outcome arrives as its own tactics-change or substitution row.
export const HIDDEN_KINDS = new Set([KIND.aiDecision]);

/// The index of the newest item in `sorted` (ascending by `.tick`) at or before `tick`, or -1.
export function newestAtOrBefore(sorted, tick) {
  let lo = 0;
  let hi = sorted.length - 1;
  let found = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (sorted[mid].tick <= tick) {
      found = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return found;
}

/// Inserts `item` keeping `list` sorted by tick. Messages arrive in tick order, so this is an
/// append in practice; equal ticks keep their arrival order.
function insertByTick(list, item) {
  let i = list.length;
  while (i > 0 && list[i - 1].tick > item.tick) {
    i -= 1;
  }
  list.splice(i, 0, item);
}

function emptyView() {
  return {
    tick: -1,
    home: 0,
    away: 0,
    /// Released feed entries (every event except the hidden kinds), in tick order.
    entries: [],
    /// `player.id` → `yellow` or `red`. A second yellow is a red.
    cards: new Map(),
    injuries: new Set(),
    sentOff: new Set(),
    /// `{ tick, team, off, on }` in tick order.
    substitutions: [],
    goals: [],
    fullTime: false,
    stats: null,
    energy: null,
    energyTick: null,
  };
}

export class MatchState {
  constructor() {
    this.events = [];
    this.stats = [];
    this.conditions = [];
    this.view = emptyView();
    /// How many stored events the view has consumed.
    this.consumed = 0;
  }

  /// Stores one server message. Anything other than an event, statistics, or condition
  /// message is not match state and is ignored.
  add(message) {
    switch (message.type) {
      case 'event':
        insertByTick(this.events, message);
        // An event stored at or before the tick already shown changes that past view.
        if (message.tick <= this.view.tick) {
          this.reset();
        }
        return true;
      case 'stats':
        insertByTick(this.stats, message);
        return true;
      case 'condition':
        insertByTick(this.conditions, message);
        return true;
      default:
        return false;
    }
  }

  reset() {
    this.view = emptyView();
    this.consumed = 0;
  }

  /// The match at `tick`: every message with a tick at or before it, and nothing later.
  /// Forward steps are incremental; a step backwards recomputes from kick-off.
  at(tick) {
    if (tick < this.view.tick) {
      this.reset();
    }
    const view = this.view;
    while (this.consumed < this.events.length && this.events[this.consumed].tick <= tick) {
      apply(view, this.events[this.consumed]);
      this.consumed += 1;
    }
    view.tick = tick;
    const s = newestAtOrBefore(this.stats, tick);
    view.stats = s < 0 ? null : this.stats[s];
    const c = newestAtOrBefore(this.conditions, tick);
    view.energy = c < 0 ? null : this.conditions[c].energy;
    view.energyTick = c < 0 ? null : this.conditions[c].tick;
    return view;
  }

  /// The tick of the newest stored full-time event, or null.
  get fullTimeTick() {
    for (let i = this.events.length - 1; i >= 0; i -= 1) {
      if (this.events[i]['event.type'] === KIND.fullTime) {
        return this.events[i].tick;
      }
    }
    return null;
  }
}

function apply(view, event) {
  const kind = event['event.type'];
  view.home = event['home.score'];
  view.away = event['away.score'];
  if (!HIDDEN_KINDS.has(kind)) {
    view.entries.push(event);
  }
  const player = event['player.id'];
  switch (kind) {
    case KIND.goal:
      view.goals.push(event);
      break;
    case KIND.card: {
      const card = event['card.kind'];
      if (player) {
        const red = card === 'red' || card === 'second-yellow';
        view.cards.set(player, red ? 'red' : 'yellow');
        if (red) {
          view.sentOff.add(player);
        }
      }
      break;
    }
    case KIND.injury:
      if (player) {
        view.injuries.add(player);
      }
      break;
    case KIND.substitution:
      view.substitutions.push({
        tick: event.tick,
        team: event['team.id'] ?? null,
        off: player ?? null,
        on: event['player.secondary_id'] ?? null,
      });
      break;
    case KIND.fullTime:
      view.fullTime = true;
      break;
    default:
      break;
  }
}
