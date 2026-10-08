// Named views of the squad table and the match ratings each club keeps, and where they are
// stored. Pure operations plus one adapter chosen at start: the launcher's `views.json` in
// the data folder when the launcher serves the page, the page's memory when the engine serves
// it or a replay is open (views then last for the session). No DOM.
//
// A view is `{ name, columns: [<column id>], sort: { column, direction } | null }`. The store
// keeps column ids as text, so a later build may add or retire a column without a new file
// format; an id this page does not know is dropped when the view is read.

import { choosable, columnIndex, DEFAULT_COLUMNS } from './columns.js';
import { readViews, saveViews } from './launcher.js';

/// The name of the view a club starts with.
export const DEFAULT_VIEW = 'Default';
/// The most views a club keeps, as the launcher's store allows.
export const MAX_VIEWS = 20;
/// The match ratings kept per player: the newest ten.
export const KEPT_RATINGS = 10;

/// The default view (board 1).
export function defaultView(name = DEFAULT_VIEW) {
  return { name, columns: [...DEFAULT_COLUMNS], sort: null };
}

/// `view` with every column id this page does not know, or cannot choose, dropped, and a
/// sort on such a column cleared. A view left with no column takes the default columns.
export function cleanView(view) {
  const known = new Set(choosable().map((c) => c.id));
  const columns = (view?.columns ?? []).filter((id, i, all) => known.has(id) && all.indexOf(id) === i);
  const index = columnIndex();
  const sort =
    view?.sort && index.has(view.sort.column) && ['up', 'down'].includes(view.sort.direction)
      ? { column: view.sort.column, direction: view.sort.direction }
      : null;
  return {
    name: String(view?.name ?? DEFAULT_VIEW),
    columns: columns.length ? columns : [...DEFAULT_COLUMNS],
    sort,
  };
}

/// `view` with column `id` added at the end; unchanged when it is shown or not choosable.
export function addColumn(view, id) {
  const known = new Set(choosable().map((c) => c.id));
  if (!known.has(id) || view.columns.includes(id)) {
    return view;
  }
  return { ...view, columns: [...view.columns, id] };
}

/// `view` without column `id`. The last column stays: a view always shows one. A sort on the
/// removed column is cleared.
export function removeColumn(view, id) {
  if (!view.columns.includes(id) || view.columns.length <= 1) {
    return view;
  }
  return {
    ...view,
    columns: view.columns.filter((c) => c !== id),
    sort: view.sort?.column === id ? null : view.sort,
  };
}

/// `view` with column `id` moved `by` places (−1 up, +1 down), kept inside the list.
export function moveColumn(view, id, by) {
  const from = view.columns.indexOf(id);
  if (from < 0) {
    return view;
  }
  return moveTo(view, from, from + by);
}

/// `view` with the column at `from` moved to `to` (a drag), kept inside the list.
export function moveTo(view, from, to) {
  const last = view.columns.length - 1;
  const target = Math.max(0, Math.min(last, to));
  if (from < 0 || from > last || target === from) {
    return view;
  }
  const columns = [...view.columns];
  const [id] = columns.splice(from, 1);
  columns.splice(target, 0, id);
  return { ...view, columns };
}

/// `view` back to the default columns and no sort, keeping its name.
export function resetView(view) {
  return { ...defaultView(view.name) };
}

/// Whether two views show the same columns in the same order with the same sort.
export function sameView(a, b) {
  return JSON.stringify([a?.columns, a?.sort ?? null]) === JSON.stringify([b?.columns, b?.sort ?? null]);
}

/// The first free name "View N" among `views`.
export function freeName(views) {
  const taken = new Set(views.map((v) => v.name));
  let n = 1;
  while (taken.has(`View ${n}`)) {
    n += 1;
  }
  return `View ${n}`;
}

/// One club's stored part as the page reads it: `{ active, views, ratings }`, cleaned. A club
/// with no views gets the default view, active.
export function readPart(part) {
  const views = (Array.isArray(part?.views) ? part.views : []).slice(0, MAX_VIEWS).map(cleanView);
  const unique = views.filter((v, i) => views.findIndex((w) => w.name === v.name) === i);
  const list = unique.length ? unique : [defaultView()];
  const active = list.some((v) => v.name === part?.active) ? part.active : list[0].name;
  const ratings = {};
  for (const [id, rows] of Object.entries(part?.ratings ?? {})) {
    if (Array.isArray(rows)) {
      ratings[id] = rows
        .filter((r) => r && typeof r.match === 'string' && Number.isFinite(Number(r.rating)))
        .map((r) => ({ match: r.match, rating: Number(r.rating) }));
    }
  }
  return { active, views: list, ratings };
}

/// The stored ratings merged with one match's ratings as they arrived at full time
/// (`{ match, ratings: [{ 'player.id', rating }] }`): one entry per match id, oldest first,
/// the newest ten per player. A match the store already holds is not counted twice.
export function mergeRatings(stored, live) {
  const out = new Map();
  for (const [id, rows] of Object.entries(stored ?? {})) {
    out.set(id, [...rows]);
  }
  for (const r of live?.ratings ?? []) {
    const id = r['player.id'];
    const rows = out.get(id) ?? [];
    if (!rows.some((x) => x.match === live.match)) {
      rows.push({ match: live.match, rating: Number(r.rating) });
    }
    out.set(id, rows.slice(-KEPT_RATINGS));
  }
  return out;
}

/// The page's memory: views last for the session. Used when the engine serves the page (it
/// keeps no store) and for a replay.
export function memoryAdapter() {
  const clubs = new Map();
  return {
    kind: 'memory',
    async read(club) {
      return clubs.get(club) ?? { views: [], ratings: {} };
    },
    async save(club, { active, views }) {
      const before = clubs.get(club) ?? { ratings: {} };
      clubs.set(club, { ...before, active, views });
      return true;
    },
  };
}

/// The launcher's store, `views.json` in the data folder. A read that fails gives an empty
/// part; a save that fails answers false and says why through `onRefused`.
export function launcherAdapter(fetcher, onRefused = undefined) {
  return {
    kind: 'launcher',
    async read(club) {
      return (await readViews(fetcher, club)) ?? { views: [], ratings: {} };
    },
    async save(club, body) {
      return (await saveViews(fetcher, club, body, onRefused)) !== null;
    },
  };
}

/// The adapter for a page: the launcher's store when the launcher serves it
/// (`status.launcher`), otherwise the page's memory.
export function adapterFor(status, fetcher) {
  return status?.launcher === true ? launcherAdapter(fetcher) : memoryAdapter();
}
