// The position chips beside a player's name. The team file names one position per player,
// so the others come from how well the player fits the roles each position takes: a
// position shows when the player's best fit among its roles is at least `POSITION_FIT`.
// Pure: no DOM.

/// The best role fit, out of 100, at which a position shows as a chip.
export const POSITION_FIT = 70;

/// The most chips a squad row shows; the row is 20 px tall and shares its width.
export const MOST_CHIPS = 3;

/// Every position the tactics file names, in the order a team sheet reads: goal outwards.
export const POSITION_ORDER = Object.freeze([
  'GK',
  'LB',
  'CB',
  'RB',
  'DM',
  'CM',
  'AM',
  'LW',
  'RW',
  'ST',
]);

/// The player's positions: the team-file `position` first, then every other position whose
/// best fit among `roles` (the tactics file's roles, each with its `positions`) reaches
/// `POSITION_FIT`, best fit first and then in team-sheet order. `roleFit` holds one fit per
/// role, in the order of `roles`.
export function positionsFor(roleFit, roles, position) {
  const best = new Map();
  (roles ?? []).forEach((role, r) => {
    const fit = roleFit?.[r] ?? 0;
    for (const p of role.positions ?? []) {
      best.set(p, Math.max(best.get(p) ?? 0, fit));
    }
  });
  const order = (p) => {
    const i = POSITION_ORDER.indexOf(p);
    return i < 0 ? POSITION_ORDER.length : i;
  };
  const others = [...best.entries()]
    .filter(([p, fit]) => p !== position && fit >= POSITION_FIT)
    .sort((a, b) => b[1] - a[1] || order(a[0]) - order(b[0]))
    .map(([p]) => p);
  const out = position ? [position, ...others] : others;
  return out.slice(0, MOST_CHIPS);
}
