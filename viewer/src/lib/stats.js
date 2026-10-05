// The statistics: nine rows, home and away, straight from the released stats message, plus
// the three facts the score strip shows.
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

/// The dash a strip fact shows before the first stats message.
export const NO_VALUE = '—';

/// The score strip's three facts: expected goals and shots as home – away, and the home
/// side's share of possession, rounded to a whole percent as the sketch writes it.
export function stripFacts(message) {
  const pair = (key) => (message && Array.isArray(message[key]) ? message[key] : null);
  const xg = pair('stats.xg');
  const shots = pair('stats.shots');
  const possession = pair('stats.possession_pct');
  return [
    { label: 'xG', value: xg ? `${formatStat(xg[0], 'xg')} – ${formatStat(xg[1], 'xg')}` : NO_VALUE },
    {
      label: 'Possession',
      value: possession ? `${Math.round(Number(possession[0]))}%` : NO_VALUE,
    },
    {
      label: 'Shots',
      value: shots ? `${formatStat(shots[0], 'int')} – ${formatStat(shots[1], 'int')}` : NO_VALUE,
    },
  ];
}
