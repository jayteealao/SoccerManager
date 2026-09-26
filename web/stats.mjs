// The statistics panel: nine rows, home and away, straight from the released stats message.
//
// The values are the engine's own, rounded by the engine exactly as the saved match record
// rounds them. The page only formats: expected goals to two decimals, shares to one decimal
// with a percent sign, counts as integers. It never recomputes a figure.

/// The rows, in the order the match screen lists them. `key` is the stats message field.
export const STAT_ROWS = Object.freeze([
  { key: 'stats.possession_pct', label: 'Possession', format: 'pct' },
  { key: 'stats.shots', label: 'Shots', format: 'int' },
  { key: 'stats.shots_on_target', label: 'On target', format: 'int' },
  { key: 'stats.xg', label: 'Expected goals', format: 'xg' },
  { key: 'stats.passes', label: 'Passes', format: 'int' },
  { key: 'stats.pass_accuracy_pct', label: 'Pass accuracy', format: 'pct' },
  { key: 'stats.fouls', label: 'Fouls', format: 'int' },
  { key: 'stats.corners', label: 'Corners', format: 'int' },
  { key: 'stats.offsides', label: 'Offsides', format: 'int' },
]);

export function formatStat(value, format) {
  switch (format) {
    case 'pct':
      return `${Number(value).toFixed(1)}%`;
    case 'xg':
      return Number(value).toFixed(2);
    default:
      return String(Math.round(Number(value)));
  }
}

/// The panel model for one stats message, or the all-zero model before the first one.
export function toPanel(message, names = ['Home', 'Away']) {
  return STAT_ROWS.map(({ key, label, format }) => {
    const pair = message && Array.isArray(message[key]) ? message[key] : [0, 0];
    const [home, away] = pair;
    const homeText = formatStat(home, format);
    const awayText = formatStat(away, format);
    return {
      key,
      label,
      home,
      away,
      homeText,
      awayText,
      aria: `${label}: ${names[0]} ${homeText}, ${names[1]} ${awayText}`,
    };
  });
}

/// The DOM side: two small tables (rows one to five, then six to nine) so the nine rows fit
/// under the control strip without scrolling at 1280 by 800.
export class StatsPanel {
  constructor(root) {
    this.root = root;
    this.names = ['Home', 'Away'];
    this.message = undefined;
    this.cells = new Map();
    this.build();
  }

  setTeams(names) {
    this.names = names;
    for (const th of this.root.querySelectorAll('[data-team]')) {
      th.textContent = names[Number(th.dataset.team)];
    }
    this.message = undefined;
  }

  build() {
    const doc = this.root.ownerDocument;
    this.root.replaceChildren();
    const halves = [STAT_ROWS.slice(0, 5), STAT_ROWS.slice(5)];
    for (const rows of halves) {
      const table = doc.createElement('table');
      table.className = 'stats__table';
      const head = doc.createElement('thead');
      const hr = doc.createElement('tr');
      for (const [text, team] of [['Statistic', null], ['Home', 0], ['Away', 1]]) {
        const th = doc.createElement('th');
        th.scope = 'col';
        th.textContent = team === null ? text : this.names[team];
        if (team === null) {
          th.className = 'visually-hidden-cell';
        } else {
          th.dataset.team = String(team);
        }
        hr.append(th);
      }
      head.append(hr);
      table.append(head);
      const body = doc.createElement('tbody');
      for (const { key, label } of rows) {
        const tr = doc.createElement('tr');
        tr.dataset.key = key;
        const th = doc.createElement('th');
        th.scope = 'row';
        th.textContent = label;
        const home = doc.createElement('td');
        const away = doc.createElement('td');
        home.className = 'stats__value tl-num';
        away.className = 'stats__value tl-num';
        tr.append(th, home, away);
        body.append(tr);
        this.cells.set(key, { row: tr, home, away });
      }
      table.append(body);
      this.root.append(table);
    }
  }

  /// Writes the panel when the released message changed. Returns the model it shows.
  update(message) {
    if (message === this.message) {
      return null;
    }
    this.message = message;
    const model = toPanel(message, this.names);
    for (const row of model) {
      const cells = this.cells.get(row.key);
      if (cells.home.textContent !== row.homeText) {
        cells.home.textContent = row.homeText;
      }
      if (cells.away.textContent !== row.awayText) {
        cells.away.textContent = row.awayText;
      }
      cells.row.setAttribute('aria-label', row.aria);
    }
    return model;
  }
}
