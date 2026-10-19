// Where each primitive sits on the Phases board: the top bar, the Block strip between the two
// rows, or one row of one of the eight phase cards. Each primitive sits in exactly one place.
// Pure: no DOM.
//
// In possession the cards follow the ball up the pitch (Build-up, Progression, Final third,
// Ball won); out of possession they follow it back (Pressing, Midfield, Defending the box,
// Ball lost). How high the team defends is not a phase: the Block strip sets it with two
// primitives, and its word is read from them.

/// The controls a row can use. `pips`: five levels; `select`: a choice list in a menu;
/// `segments`: a short choice list side by side; `stack`: segments under the label; `switch`:
/// Off and On.
export const CONTROLS = Object.freeze(['pips', 'select', 'segments', 'stack', 'switch']);

const row = (primitive, control = 'pips', extra = {}) => ({ primitive, control, ...extra });

/// The top bar's primitives, before the cards.
export const TOP_BAR = Object.freeze([row('mentality'), row('time_wasting', 'switch')]);

/// The Block strip's primitives, between the two rows.
export const BLOCK = Object.freeze([row('line_of_engagement'), row('line_height')]);

/// The eight phase cards, in board order. A card with `players` lists the player
/// instructions on its primitive and links to the Roles tab.
export const CARDS = Object.freeze([
  {
    id: 'build-up',
    side: 'in',
    title: 'Build-up',
    sub: 'from the back',
    zone: 'own',
    rows: [row('build_up'), row('passing_directness'), row('tempo'), row('width')],
  },
  {
    id: 'progression',
    side: 'in',
    title: 'Progression',
    sub: 'through midfield',
    zone: 'middle',
    rows: [
      row('attacking_channel', 'select'),
      row('forward_runs'),
      row('fullback_runs', 'segments'),
      row('fullback_run_frequency'),
      row('dribbling'),
      row('creative_freedom'),
    ],
  },
  {
    id: 'final-third',
    side: 'in',
    title: 'Final third',
    sub: 'making chances',
    zone: 'their',
    rows: [row('final_third'), row('crossing'), row('cross_type', 'stack')],
  },
  {
    id: 'ball-won',
    side: 'in',
    title: 'Ball won',
    sub: 'just after we win it',
    zone: 'transition',
    rows: [row('on_winning_the_ball')],
    players: 'on_winning_the_ball',
  },
  {
    id: 'pressing',
    side: 'out',
    title: 'Pressing',
    sub: 'ball in their third',
    zone: 'their',
    rows: [
      row('pressing'),
      row('press_on_back_pass', 'switch', { sub: 'Press when' }),
      row('press_on_poor_touch', 'switch'),
      row('press_on_facing_back', 'switch'),
      row('press_on_wide', 'switch'),
      row('stop_short_goal_kicks', 'switch'),
    ],
  },
  {
    id: 'midfield',
    side: 'out',
    title: 'Midfield',
    sub: 'ball in the middle third',
    zone: 'middle',
    rows: [row('defensive_width'), row('force_the_opponent')],
  },
  {
    id: 'defending-the-box',
    side: 'out',
    title: 'Defending the box',
    sub: 'ball in our third',
    zone: 'own',
    rows: [row('offside_trap', 'switch'), row('tackling')],
  },
  {
    id: 'ball-lost',
    side: 'out',
    title: 'Ball lost',
    sub: 'just after we lose it',
    zone: 'transition',
    rows: [row('on_losing_the_ball'), row('counter_press_length')],
    players: 'on_losing_the_ball',
  },
]);

/// Every place a primitive sits, as `{ primitive, place, control }`, in board order.
export function placements() {
  return [
    ...TOP_BAR.map((r) => ({ ...r, place: 'top-bar' })),
    ...CARDS.filter((c) => c.side === 'in').flatMap((c) => c.rows.map((r) => ({ ...r, place: c.id }))),
    ...BLOCK.map((r) => ({ ...r, place: 'block' })),
    ...CARDS.filter((c) => c.side === 'out').flatMap((c) => c.rows.map((r) => ({ ...r, place: c.id }))),
  ];
}

/// The place primitive `name` sits, or null.
export function placeOf(name) {
  return placements().find((p) => p.primitive === name) ?? null;
}

/// The control for primitive `p` when the board places it nowhere: pips for five levels, a
/// switch for two, segments for a short choice list and a select for a long one.
export function controlFor(p, count) {
  const placed = placeOf(p?.name);
  if (placed) {
    return placed.control;
  }
  if (count === 2) {
    return 'switch';
  }
  if (p?.kind === 'choice') {
    return count > 4 ? 'select' : 'segments';
  }
  return 'pips';
}
