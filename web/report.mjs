// The half-time and full-time reports. Each is counted from the page's one event list, the
// same list the feed shows, so a report can never disagree with the feed.

import { minuteStamp } from './feed.mjs';
import { KIND } from './match-state.mjs';

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

  /// Starts again for another match.
  reset() {
    this.shown.clear();
  }
}

/// The report dialog. It fills the `<dialog>` from a model and opens it modal, so focus is
/// held inside and Escape closes it.
export class ReportDialog {
  constructor({ dialog, title, score, table, moments, actions }) {
    Object.assign(this, { dialog, title, score, table, moments, actions });
    this.kind = null;
    this.model = null;
  }

  show(kind, model) {
    const doc = this.dialog.ownerDocument;
    this.kind = kind;
    this.model = model;
    this.dialog.dataset.kind = kind;
    this.title.textContent = kind === 'half-time' ? 'Half-time report' : 'Full-time report';
    this.score.textContent = `${model.teams[0]} ${model.score[0]} – ${model.score[1]} ${model.teams[1]}`;

    const head = doc.createElement('tr');
    for (const text of ['', model.teams[0], model.teams[1]]) {
      const th = doc.createElement('th');
      th.scope = 'col';
      th.textContent = text;
      head.append(th);
    }
    const body = model.rows.map((row) => {
      const tr = doc.createElement('tr');
      tr.dataset.row = row.id;
      const th = doc.createElement('th');
      th.scope = 'row';
      th.textContent = row.label;
      tr.append(th);
      row.counts.forEach((count, side) => {
        const td = doc.createElement('td');
        td.className = 'tl-num';
        td.dataset.count = String(count);
        td.dataset.side = side === 0 ? 'home' : 'away';
        td.textContent = String(count);
        tr.append(td);
      });
      return tr;
    });
    const thead = doc.createElement('thead');
    thead.append(head);
    const tbody = doc.createElement('tbody');
    tbody.append(...body);
    this.table.replaceChildren(thead, tbody);

    this.moments.replaceChildren(
      ...model.moments.map((m) => {
        const li = doc.createElement('li');
        const minute = doc.createElement('span');
        minute.className = 'report__minute tl-num';
        minute.textContent = m.minute;
        const word = doc.createElement('span');
        word.className = 'report__word';
        word.textContent = m.kind;
        const text = doc.createElement('span');
        text.textContent = m.text || (m.side >= 0 ? model.teams[m.side] : '');
        li.append(minute, word, text);
        return li;
      })
    );
    if (model.moments.length === 0) {
      const li = doc.createElement('li');
      li.className = 'report__none';
      li.textContent = 'No goals or cards.';
      this.moments.append(li);
    }
    for (const button of this.actions.querySelectorAll('[data-for]')) {
      button.hidden = button.dataset.for !== kind;
    }
    if (!this.dialog.open) {
      this.dialog.showModal();
    }
  }

  get open() {
    return this.dialog.open;
  }

  close() {
    if (this.dialog.open) {
      this.dialog.close();
    }
  }
}
