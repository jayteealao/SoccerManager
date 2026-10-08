// The squad table's columns: each with an id, a header label, a menu name, a group, whether
// it is built or LATER, how it sorts and what its cell shows. Pure: no DOM.
//
// Every 1-20 value goes through `wholeOf` and `bandOf`, so no cell, title or label shows a
// decimal or a value over 20. Every hidden value goes through `hidden-words.js`, so none shows
// as a number. A value the hello does not carry (an older protocol, a converted team file
// with no body) shows as "—" and sorts last.

import { BUILD_TEXT, HIDDEN_NAMES, hiddenWord } from './hidden-words.js';
import { BAND_WORDS, bandOf, ratingBand, ratingText, wholeOf } from './scale.js';

/// The visible attributes by group, as `content/attributes.json` groups them, in file order.
/// A test checks this list against the file, so the two never drift apart.
export const ATTRIBUTE_GROUPS = Object.freeze({
  technical: ['passing', 'dribbling', 'first_touch', 'finishing', 'crossing', 'heading', 'long_shots', 'tackling', 'technique'],
  mental: ['positioning', 'vision', 'decisions', 'composure', 'anticipation', 'work_rate', 'aggression', 'concentration', 'teamwork'],
  physical: ['pace', 'acceleration', 'stamina', 'strength', 'agility', 'balance', 'jumping', 'natural_fitness'],
  goalkeeping: ['handling', 'reflexes', 'aerial_reach', 'one_on_ones', 'kicking', 'throwing', 'command_of_area', 'communication', 'rushing_out'],
});

/// The header label of each attribute: three letters, as the genre writes them.
const SHORT = Object.freeze({
  passing: 'Pas', dribbling: 'Dri', first_touch: 'Fir', finishing: 'Fin', crossing: 'Cro',
  heading: 'Hea', long_shots: 'Lon', tackling: 'Tck', technique: 'Tec', positioning: 'Pos',
  vision: 'Vis', decisions: 'Dec', composure: 'Cmp', anticipation: 'Ant', work_rate: 'Wor',
  aggression: 'Agg', concentration: 'Cnc', teamwork: 'Tea', pace: 'Pac', acceleration: 'Acc',
  stamina: 'Sta', strength: 'Str', agility: 'Agi', balance: 'Bal', jumping: 'Jum',
  natural_fitness: 'Fit', handling: 'Han', reflexes: 'Ref', aerial_reach: 'Aer',
  one_on_ones: '1v1', kicking: 'Kic', throwing: 'Thr', command_of_area: 'Cmd',
  communication: 'Com', rushing_out: 'Rus',
});

/// The menu groups, in menu order.
export const MENU_GROUPS = Object.freeze([
  { id: 'player', label: 'Player' },
  { id: 'body', label: 'Body' },
  { id: 'technical', label: 'Technical' },
  { id: 'mental', label: 'Mental' },
  { id: 'physical', label: 'Physical' },
  { id: 'goalkeeping', label: 'Goalkeeping' },
  { id: 'hidden', label: 'Hidden, in words' },
  { id: 'form', label: 'Form' },
]);

/// An attribute name in words: `first_touch` → "First Touch".
export function attributeName(name) {
  return String(name)
    .split('_')
    .map((w) => (w ? w[0].toUpperCase() + w.slice(1) : w))
    .join(' ');
}

const DASH = '—';
const blank = () => ({ text: DASH, muted: true });

/// A 1-20 cell: the whole number, its band class and its band word.
function scaleCell(tenths) {
  if (typeof tenths !== 'number') {
    return blank();
  }
  const whole = wholeOf(tenths);
  const band = bandOf(whole);
  return { text: String(whole), band, label: `${whole}, ${BAND_WORDS[band]}` };
}

/// The attribute columns: one per visible attribute, in group order.
function attributeColumns() {
  return Object.entries(ATTRIBUTE_GROUPS).flatMap(([group, names]) =>
    names.map((name) => ({
      id: `attr:${name}`,
      label: SHORT[name] ?? attributeName(name).slice(0, 3),
      name: attributeName(name),
      group,
      built: true,
      align: 'c',
      kind: 'scale',
      sortFirst: 'down',
      value: (p) => {
        const t = p['player.attributes']?.[name];
        return typeof t === 'number' ? wholeOf(t) : null;
      },
      cell: (p) => scaleCell(p['player.attributes']?.[name]),
    }))
  );
}

/// A hidden value's column: the word, never a number; sorted by the word's band, best first,
/// and "not yet known" last.
function hiddenColumn(name, label) {
  return {
    id: name,
    label,
    name: `${HIDDEN_NAMES[name]} (words)`,
    group: 'hidden',
    built: true,
    align: 'l',
    kind: 'word',
    wide: true,
    sortFirst: 'down',
    value: (p) => hiddenWord(name, p[`player.${name}`], p['player.matches_at_club']).order,
    cell: (p) => {
      const w = hiddenWord(name, p[`player.${name}`], p['player.matches_at_club']);
      return {
        text: w.text,
        note: w.known && w.confidence === 'tentative' ? 'tentative' : '',
        muted: !w.known,
        hidden: name,
        label: w.known ? `${w.text}, ${w.line}` : w.full,
      };
    },
  };
}

/// The rating figures of a player's stored match ratings (oldest first): the last, and the
/// averages of the last 3 and the last 10, each over the matches that exist.
export function ratingFigures(list) {
  const values = (list ?? []).map((r) => Number(r.rating)).filter((r) => Number.isFinite(r));
  if (values.length === 0) {
    return null;
  }
  const mean = (n) => {
    const take = values.slice(-n);
    return { value: Math.round((take.reduce((a, b) => a + b, 0) / take.length) * 10) / 10, of: take.length };
  };
  return { last: values.at(-1), avg3: mean(3), avg10: mean(10), count: values.length };
}

/// A match-rating chip: one decimal, unlike attributes, which show whole numbers.
export function ratingChip(value) {
  return { text: ratingText(value), band: ratingBand(value) };
}

/// Every column the table knows: the fixed No and Player first, then the built columns a
/// view may show, then the LATER columns.
export function allColumns() {
  return [
    {
      id: 'no',
      label: 'No',
      name: 'Shirt number',
      group: 'player',
      fixed: true,
      built: true,
      align: 'c',
      kind: 'number',
      sortFirst: 'up',
      value: (p) => p['player.shirt'] ?? null,
      cell: (p) => ({ text: String(p['player.shirt'] ?? DASH), strong: true }),
    },
    {
      id: 'player',
      label: 'Player',
      name: 'Name',
      group: 'player',
      fixed: true,
      built: true,
      align: 'l',
      kind: 'text',
      sortFirst: 'up',
      value: (p) => p['player.name'] ?? null,
      cell: (p) => ({ text: p['player.name'] ?? DASH }),
    },
    {
      id: 'position',
      label: 'Pos',
      name: 'Position',
      group: 'player',
      built: true,
      align: 'l',
      kind: 'text',
      sortFirst: 'up',
      value: (p) => p['player.position'] ?? null,
      cell: (p) => ({ text: p['player.position'] ?? DASH, display: true }),
    },
    {
      id: 'age',
      label: 'Age',
      name: 'Age',
      group: 'player',
      built: true,
      align: 'r',
      kind: 'number',
      sortFirst: 'down',
      value: (p) => p['player.age'] ?? null,
      cell: (p) => (p['player.age'] == null ? blank() : { text: String(p['player.age']) }),
    },
    {
      id: 'nat',
      label: 'Nat',
      name: 'Nationality',
      group: 'player',
      built: true,
      align: 'l',
      kind: 'text',
      sortFirst: 'up',
      value: (p) => p['player.nationality'] ?? null,
      cell: (p) => (p['player.nationality'] ? { text: p['player.nationality'], display: true } : blank()),
    },
    {
      id: 'height',
      label: 'Height',
      name: 'Height',
      group: 'body',
      built: true,
      align: 'r',
      kind: 'number',
      sortFirst: 'down',
      value: (p) => p['player.height'] ?? null,
      cell: (p) => (p['player.height'] == null ? blank() : { text: `${p['player.height']} cm` }),
    },
    {
      id: 'build',
      label: 'Build',
      name: 'Build',
      group: 'body',
      built: true,
      align: 'l',
      kind: 'word',
      sortFirst: 'down',
      value: (p) => (p['player.build'] ? Object.keys(BUILD_TEXT).indexOf(p['player.build']) : null),
      cell: (p) => (p['player.build'] ? { text: BUILD_TEXT[p['player.build']] ?? p['player.build'] } : blank()),
    },
    {
      id: 'cond',
      label: 'Cond',
      name: 'Condition',
      group: 'body',
      built: true,
      align: 'r',
      kind: 'percent',
      sortFirst: 'down',
      value: (p) => p['player.condition'] ?? null,
      cell: (p) => percentCell(p['player.condition']),
    },
    {
      id: 'sharp',
      label: 'Sharp',
      name: 'Sharpness',
      group: 'body',
      built: true,
      align: 'r',
      kind: 'percent',
      sortFirst: 'down',
      value: (p) => p['player.sharpness'] ?? null,
      cell: (p) => percentCell(p['player.sharpness']),
    },
    {
      id: 'level',
      label: 'Level',
      name: 'Level when fresh',
      group: 'player',
      built: true,
      align: 'c',
      kind: 'scale',
      sortFirst: 'down',
      value: (p) => (typeof p['player.level'] === 'number' ? wholeOf(p['player.level']) : null),
      cell: (p) => scaleCell(p['player.level']),
    },
    ...attributeColumns(),
    hiddenColumn('consistency', 'Consistency'),
    hiddenColumn('injury_proneness', 'Injury proneness'),
    {
      id: 'rating',
      label: 'Rating 3 · 10',
      name: 'Match rating · last 3 and 10',
      group: 'form',
      built: true,
      align: 'c',
      kind: 'ratings',
      sortFirst: 'down',
      value: (p, ctx) => ratingFigures(ctx?.ratings?.get(p['player.id']))?.avg3.value ?? null,
      cell: (p, ctx) => {
        const figures = ratingFigures(ctx?.ratings?.get(p['player.id']));
        if (!figures) {
          return { ...blank(), label: 'No match rating yet' };
        }
        return {
          chips: [ratingChip(figures.avg3.value), ratingChip(figures.avg10.value)],
          label: `Average ${ratingText(figures.avg3.value)} over the last ${figures.avg3.of}, ${ratingText(figures.avg10.value)} over the last ${figures.avg10.of}`,
        };
      },
    },
    ...LATER_COLUMNS.map((c) => ({ ...c, built: false, group: 'later', align: 'l', kind: 'later' })),
  ];
}

/// A percent cell (condition, sharpness) in the sketch's ring chip; "—" with no figure.
function percentCell(value) {
  if (typeof value !== 'number') {
    return blank();
  }
  return { text: `${value}%`, ring: value >= 90 ? 'ok' : value >= 75 ? 'lo' : 'vl' };
}

/// The columns the engine has no model for yet: drawn in the menu's greyed Later group, never
/// added.
export const LATER_COLUMNS = Object.freeze([
  { id: 'competence', label: 'Competence', name: 'Competence by position' },
  { id: 'feet', label: 'Feet', name: 'Feet' },
  { id: 'load', label: 'Load', name: 'Load' },
  { id: 'morale', label: 'Morale', name: 'Morale' },
  { id: 'status', label: 'Status', name: 'Squad status' },
  { id: 'contract', label: 'Contract', name: 'Contract' },
  { id: 'lists', label: 'Lists', name: 'Lists' },
  { id: 'tactic', label: 'Tactic', name: 'Tactic grasp' },
]);

/// The LATER columns the table draws at its right edge, as the board does.
export const TRAILING_LATER = Object.freeze(['morale', 'contract']);

/// The default view's columns, after No and Player (board 1).
export const DEFAULT_COLUMNS = Object.freeze([
  'age',
  'nat',
  'height',
  'cond',
  'sharp',
  'attr:pace',
  'attr:acceleration',
  'attr:passing',
  'attr:technique',
  'attr:decisions',
  'attr:stamina',
  'consistency',
  'rating',
]);

/// The columns by id.
export function columnIndex(columns = allColumns()) {
  return new Map(columns.map((c) => [c.id, c]));
}

/// The ids a view may list: built and not fixed.
export function choosable(columns = allColumns()) {
  return columns.filter((c) => c.built && !c.fixed);
}

/// The columns a view shows, in order: No and Player, then the view's ids that are built
/// columns (an id this page does not know is dropped).
export function shownColumns(view, columns = allColumns()) {
  const index = columnIndex(columns);
  const fixed = columns.filter((c) => c.fixed);
  const chosen = (view?.columns ?? [])
    .map((id) => index.get(id))
    .filter((c) => c && c.built && !c.fixed);
  return [...fixed, ...chosen];
}

/// Rows in table order. With no sort, squad order. With `sort` (`{ column, direction }`,
/// `down` is highest or best first), a stable sort by the column's value; a row with no value
/// sorts last either way.
export function sortRows(rows, sort, ctx, columns = allColumns()) {
  const column = sort ? columnIndex(columns).get(sort.column) : null;
  if (!column) {
    return [...rows];
  }
  const sign = sort.direction === 'up' ? 1 : -1;
  return rows.toSorted((a, b) => {
    const va = column.value(a.player, ctx);
    const vb = column.value(b.player, ctx);
    const na = va === null || va === undefined;
    const nb = vb === null || vb === undefined;
    if (na || nb) {
      return na === nb ? 0 : na ? 1 : -1;
    }
    if (typeof va === 'string' || typeof vb === 'string') {
      return sign * String(va).localeCompare(String(vb));
    }
    return sign * (va - vb);
  });
}

/// The next sort after a select on `columnId`'s header: the column's first direction, or the
/// reverse of the current one when it is already sorted by it.
export function nextSort(sort, columnId, columns = allColumns()) {
  const column = columnIndex(columns).get(columnId);
  if (!column) {
    return sort ?? null;
  }
  if (sort?.column === columnId) {
    return { column: columnId, direction: sort.direction === 'down' ? 'up' : 'down' };
  }
  return { column: columnId, direction: column.sortFirst ?? 'down' };
}

/// The table rows of a squad: each player with his squad index.
export function squadRows(squad) {
  return (squad ?? []).map((player, index) => ({ index, player }));
}
