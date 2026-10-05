// The match feed's logic: which rows are new this frame, the words of each row, and whether
// the list follows the newest row. Pure: the Commentary component renders what these return.
//
// At eight times speed a burst of events can land inside one frame. The batcher hands the
// renderer every entry released since the last frame, in tick order, as one batch, so the
// list changes once per frame however many rows arrive, and no row is dropped.

import { HIGHLIGHT_WORDS, KIND } from './match-state.js';

/// The empty state, before the first event is released.
export const EMPTY_TEXT = 'No events yet. The feed fills as the match plays.';

/// Only these kinds are announced to a screen reader. Announcing every row at 8x would
/// flood it.
export const ANNOUNCED = new Set([KIND.goal, KIND.card]);

export class FeedBatcher {
  constructor() {
    this.source = null;
    this.shown = 0;
  }

  /// The entries to insert this frame. `reset` is true when the rows already on screen no
  /// longer describe `entries` (a rewind), and the batch then holds every entry.
  take(entries) {
    if (entries !== this.source || entries.length < this.shown) {
      this.source = entries;
      this.shown = entries.length;
      return { reset: true, batch: entries.slice() };
    }
    const batch = entries.slice(this.shown);
    this.shown = entries.length;
    return { reset: false, batch };
  }
}

/// `23'`, or `45+2'` in added time.
export function minuteStamp(event) {
  const added = event['minute.added'];
  return added ? `${event.minute}+${added}'` : `${event.minute}'`;
}

/// The words of one feed row, from the event alone. `teamNames` maps `team.id` to the club
/// name, for the one kind that carries no commentary line.
export function feedRow(event, teamNames = new Map()) {
  const kind = event['event.type'];
  let text = event.commentary;
  if (!text) {
    const team = teamNames.get(event['team.id']);
    const words = kind === KIND.tacticsChange ? changeWords(event) : kindWords(kind);
    text = team ? `${words} — ${team}` : words;
  }
  return {
    tick: event.tick,
    kind,
    minute: minuteStamp(event),
    text,
    word: HIGHLIGHT_WORDS[kind] ?? null,
    announce: ANNOUNCED.has(kind),
  };
}

/// A change row names its verdict: applied, refused with the engine's reason, or queued.
function changeWords(event) {
  const substitution = event['change.kind'] === 'substitution';
  switch (event['change.state']) {
    case 'applied':
      return substitution ? 'Substitution applied' : 'Tactical change applied';
    case 'rejected': {
      const reason = event['change.rejected_reason'];
      return reason ? `Change rejected: ${reason}` : 'Change rejected';
    }
    case 'queued':
      return substitution ? 'Substitution queued' : 'Tactical change queued';
    default:
      return 'Tactics change';
  }
}

/// `free-kick` → `Free kick`.
function kindWords(kind) {
  const spaced = String(kind ?? 'event').replaceAll('-', ' ');
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

/// Pixels from the bottom within which the feed counts as reading the newest row.
export const FOLLOW_SLACK_PX = 24;

/// Whether the feed follows the newest row. Only the manager scrolling up stops it: a panel
/// that shrinks beside the feed moves the bottom away without moving the list, and leaves it
/// following. Pure: the caller reports where it scrolled the list, and the list's geometry on
/// each scroll event.
export class FeedFollow {
  constructor() {
    this.following = true;
    /// Where the feed last scrolled itself.
    this.autoTop = 0;
  }

  scrolledTo(top) {
    this.autoTop = top;
    this.following = true;
  }

  onScroll({ scrollHeight, scrollTop, clientHeight }) {
    const atBottom = scrollHeight - scrollTop - clientHeight < FOLLOW_SLACK_PX;
    this.following = atBottom || scrollTop >= this.autoTop;
  }
}
