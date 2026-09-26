// The match feed: one row per released event, inserted once per animation frame.
//
// At eight times speed a burst of events can land inside one frame. The batcher hands the
// renderer every entry released since the last frame, in tick order, as one batch, and the
// renderer inserts that batch through one DocumentFragment: one layout invalidation per frame
// however many rows arrive, and no row dropped.

import { HIGHLIGHT_WORDS, KIND } from './match-state.mjs';

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
/// that shrinks beside the feed (a pending-change chip appearing, the roles list opening)
/// moves the bottom away without moving the list, and leaves it following. Pure: the caller
/// reports where it scrolled the list, and the list's geometry on each scroll event.
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

/// The DOM side. `list` is the scrolling list, `empty` the empty-state node, and `live` a
/// visually hidden polite live region for goals and cards.
export class Feed {
  constructor({ list, empty, live }, teamNames = new Map()) {
    this.list = list;
    this.empty = empty;
    this.live = live;
    this.teamNames = teamNames;
    this.batcher = new FeedBatcher();
    this.count = 0;
    this.last = null;
    this.lastRow = null;
    this.onRow = null;
    this.follow = new FeedFollow();
    list.addEventListener?.('scroll', () => this.follow.onScroll(list));
  }

  setTeams(teamNames) {
    this.teamNames = teamNames;
  }

  /// Applies this frame's released entries. Returns the rows inserted.
  flush(entries) {
    const { reset, batch } = this.batcher.take(entries);
    if (!reset && batch.length === 0) {
      return [];
    }
    // Follow the newest row unless the manager scrolled up to read an older one.
    const list = this.list;
    const pinned = this.follow.following;
    if (reset) {
      list.replaceChildren();
      this.count = 0;
      this.lastRow = null;
    }
    const doc = list.ownerDocument;
    const fragment = doc.createDocumentFragment();
    const rows = [];
    let spoken = null;
    for (const event of batch) {
      const row = feedRow(event, this.teamNames);
      rows.push(row);
      const li = doc.createElement('li');
      li.className = 'feed__row';
      li.dataset.kind = row.kind;
      li.dataset.tick = String(row.tick);
      if (row.word) {
        li.classList.add('feed__row--highlight');
      }
      const minute = doc.createElement('span');
      minute.className = 'feed__minute tl-num';
      minute.textContent = row.minute;
      li.append(minute);
      if (row.word) {
        const word = doc.createElement('span');
        word.className = 'feed__word';
        word.textContent = row.word;
        li.append(word);
      }
      const text = doc.createElement('span');
      text.className = 'feed__text';
      text.textContent = row.text;
      li.append(text);
      fragment.append(li);
      this.lastRow = li;
      if (row.announce && !reset) {
        spoken = `${row.minute} ${row.text}`;
      }
    }
    list.append(fragment);
    this.count += batch.length;
    this.last = rows.length ? rows[rows.length - 1] : this.last;
    if (reset && batch.length === 0) {
      this.last = null;
    }
    this.empty.hidden = this.count > 0;
    if (pinned || reset) {
      list.scrollTop = list.scrollHeight;
      this.follow.scrolledTo(list.scrollTop);
    }
    if (spoken && this.live) {
      this.live.textContent = spoken;
    }
    return rows;
  }

  /// Marks the newest goal row, so the goal moment and the feed agree on one line.
  highlightGoal(tick) {
    for (const li of this.list.querySelectorAll('.feed__row--goal-now')) {
      li.classList.remove('feed__row--goal-now');
    }
    const row = this.list.querySelector(`.feed__row[data-kind="${KIND.goal}"][data-tick="${tick}"]`);
    if (row) {
      row.classList.add('feed__row--goal-now');
    }
    return row;
  }
}
