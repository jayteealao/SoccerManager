// The tactics board's words: every label, level word, choice word and style name the board
// shows, in a football-management player's words, never the tactics file's ids. Pure: no DOM.
//
// Each primitive has a short label (the row), a full name (the control's accessible name and
// the player instruction list) that holds the label, so a player who names what he sees
// reaches the control, and one word per level, choice or state in the tactics file's
// order. A primitive the table does not know falls back to its name in words, so a new
// primitive in the tactics file still shows, and the unit test names it.

import { fromWire, levelNames, primitivesOf } from './primitives.js';
import { words } from './tactics-panel.js';

const L = (s) => s.split('|');
const OFF_ON = ['Off', 'On'];

/// The words of every primitive of the shipped tactics file.
export const PRIMITIVE_WORDS = Object.freeze({
  mentality: { label: 'Mentality', full: 'Mentality', levels: L('Defensive|Cautious|Balanced|Positive|Attacking') },
  tempo: { label: 'Tempo', full: 'Tempo', levels: L('Very fast|Fast|Normal|Slow|Very slow') },
  passing_directness: { label: 'Passing', full: 'Passing', levels: L('Very short|Short|Mixed|Direct|Very direct') },
  width: { label: 'Width', full: 'Width with the ball', levels: L('Very narrow|Narrow|Normal|Wide|Very wide') },
  build_up: { label: 'From the back', full: 'Play from the back', levels: L('Always short|Short|Mixed|Long|Always long') },
  attacking_channel: {
    label: 'Attack down',
    full: 'Attack down',
    levels: L('Left|Right|Both wings|Middle|Left of centre|Right of centre|Anywhere'),
  },
  final_third: { label: 'Shooting', full: 'Shooting', levels: L('Work it in|Patient|Mixed|Shoot early|On sight') },
  dribbling: { label: 'Dribbling', full: 'Dribbling', levels: L('Rarely|Less|Normal|More|Often') },
  crossing: { label: 'Crossing', full: 'Crossing', levels: L('Rarely|Less|Normal|More|Often') },
  cross_type: { label: 'Cross type', full: 'Cross type', levels: L('Early|Low|Floated|Mixed') },
  creative_freedom: {
    label: 'Creativity',
    full: 'Creativity: how much freedom the players have',
    levels: L('Disciplined|Structured|Balanced|Expressive|Free rein'),
  },
  forward_runs: {
    label: 'Forward runs',
    full: 'Forward runs',
    levels: L('Come short|Mostly short|Mixed|Mostly behind|In behind'),
  },
  fullback_runs: { label: 'Full-back runs', full: 'Full-back runs', levels: L('Stay|Underlap|Overlap') },
  fullback_run_frequency: {
    label: 'How often',
    full: 'How often the full-backs run',
    levels: L('Rarely|Less|Normal|More|Often'),
  },
  on_winning_the_ball: {
    label: 'Reaction',
    full: 'Reaction when we win the ball',
    levels: L('Keep it|Mostly keep|Mixed|Counter|Always counter'),
  },
  on_losing_the_ball: {
    label: 'Reaction',
    full: 'Reaction when we lose the ball',
    levels: L('Always drop|Drop back|Mixed|Press|Always press'),
  },
  counter_press_length: { label: 'Press for', full: 'Press for how long after losing the ball', levels: L('2 s|4 s|5 s|6 s|8 s') },
  pressing: { label: 'Intensity', full: 'Pressing intensity', levels: L('Very low|Low|Medium|High|Very high') },
  line_height: { label: 'Defensive line', full: 'Defensive line', levels: L('Very deep|Deep|Normal|High|Very high') },
  line_of_engagement: {
    label: 'Press from',
    full: 'Press from where on the pitch',
    levels: L('Our third|Our half|Halfway|Their half|Their third'),
  },
  defensive_width: {
    label: 'Width',
    full: 'Width without the ball',
    levels: L('Very narrow|Narrow|Normal|Wide|Very wide'),
  },
  force_the_opponent: {
    label: 'Show them',
    full: 'Show them inside or wide',
    levels: L('Always wide|Wide|Either way|Inside|Always inside'),
  },
  tackling: { label: 'Tackling', full: 'Tackling', levels: L('Stay on feet|Careful|Normal|Committed|Fly in') },
  offside_trap: { label: 'Offside trap', full: 'Offside trap', levels: OFF_ON },
  stop_short_goal_kicks: { label: 'Press goal kicks', full: 'Press goal kicks taken short', levels: OFF_ON },
  press_on_back_pass: { label: 'They pass back', full: 'Press when they pass back', levels: OFF_ON },
  press_on_poor_touch: { label: 'Bad touch', full: 'Press after a bad touch', levels: OFF_ON },
  press_on_facing_back: { label: 'Back to goal', full: 'Press a player with his back to goal', levels: OFF_ON },
  press_on_wide: { label: 'Ball goes wide', full: 'Press when the ball goes wide', levels: OFF_ON },
  time_wasting: { label: 'Time wasting', full: 'Time wasting', levels: OFF_ON },
});

/// The style presets by name: what the picker calls each. The block is set by two
/// primitives, never by a style's name, so no style is called a press or a block. How a style
/// plays is never written here: [`presetNote`] reads it from the preset's own levels, so the
/// picker cannot drift from the styles file.
export const STYLE_WORDS = Object.freeze({
  possession: { name: 'Possession' },
  low_block: { name: 'Deep and compact' },
  high_press: { name: 'Press from the front' },
  counter_attack: { name: 'Counter-attack' },
  direct: { name: 'Direct' },
  wing_play: { name: 'Wing play' },
  gegenpress: { name: 'Gegenpress' },
  park_the_bus: { name: 'Park the bus' },
});

/// How many of a preset's settings its note names.
const NOTE_SETTINGS = 3;

/// One line on how `preset` plays, from its own settings in the styles file's order: the
/// first three it sets besides the mentality, each as its label and level word, such as
/// "tempo slow, passing short, from the back always short".
export function presetNote(schema, preset) {
  const prims = primitivesOf(schema);
  const parts = [];
  for (const wire of preset?.primitives ?? []) {
    if (parts.length >= NOTE_SETTINGS || wire.primitive === 'mentality') {
      continue;
    }
    const at = fromWire(schema, wire);
    if (!at) {
      continue;
    }
    const names = levelNames(schema, prims[at[0]]);
    const w = primitiveWords(wire.primitive, names);
    parts.push(`${w.label} ${levelWord(wire.primitive, at[1], names)}`.toLowerCase());
  }
  return parts.join(', ');
}
/// The words of primitive `name`: `{ label, full, levels }`. A primitive the table does not
/// know reads its name and its level names in words.
export function primitiveWords(name, levelNames = []) {
  const known = PRIMITIVE_WORDS[name];
  if (known) {
    return known;
  }
  return { label: words(name), full: words(name), levels: levelNames.map(words) };
}

/// The word of level `level` of primitive `name`.
export function levelWord(name, level, levelNames = []) {
  const w = primitiveWords(name, levelNames);
  return w.levels[level] ?? words(levelNames[level] ?? String(level));
}

/// The picker's words for style `name`.
export function styleWords(name) {
  return STYLE_WORDS[name] ?? { name: words(name) };
}

/// Where the team presses from gives the block its word: Low, Mid or High. It is read from
/// the level and has no setting of its own.
export const BLOCK_WORDS = Object.freeze(['Low', 'Low', 'Mid', 'High', 'High']);

export function blockWord(level) {
  return BLOCK_WORDS[level] ?? 'Mid';
}

/// A position code's words for a player instruction that reaches only some positions.
const POSITION_GROUPS = [
  { codes: ['LW', 'RW', 'ST'], words: 'wingers and strikers only' },
  { codes: ['GK'], words: 'goalkeepers only' },
];

export function reachWords(codes) {
  const sorted = [...codes].sort().join(',');
  const group = POSITION_GROUPS.find((g) => [...g.codes].sort().join(',') === sorted);
  return group ? group.words : `${codes.join(', ')} only`;
}

/// Why a change did not go through, in a player's words, and what to do next: `sentence`
/// names the problem and the next step, and `retry` says whether trying the same change again
/// can work (only when the match was not connected or had not started, or for a reason the
/// board does not know). The reason itself stays as the code for a report.
export function reasonAdvice(reason) {
  const text = String(reason ?? '');
  if (/is not a level of|is not a primitive|does not hold/.test(text)) {
    return { sentence: 'That setting is not available in this match. Choose another level from the list.', retry: false };
  }
  if (/reaches only|does not reach the position/.test(text)) {
    return { sentence: 'That instruction does not suit his position. Choose one his position takes.', retry: false };
  }
  if (/rules; a tactic holds at most/.test(text)) {
    return { sentence: 'The side already has the most game plans it can hold. Remove a plan, then add this one.', retry: false };
  }
  if (/condition kind|allowed .* to|needs |takes /.test(text)) {
    return { sentence: 'One of the plan’s conditions is outside what a plan allows. Edit the plan and change it.', retry: false };
  }
  if (/not connected/.test(text)) {
    return { sentence: 'The match is not connected. Try again once it is back.', retry: true };
  }
  if (/has not started|not kicked off/.test(text)) {
    return { sentence: 'The match has not kicked off yet. Try again after kick-off.', retry: true };
  }
  return { sentence: 'The match did not take the change. Try again; if it keeps failing, report it with this code.', retry: true };
}

/// The sentence of [`reasonAdvice`].
export function reasonSentence(reason) {
  return reasonAdvice(reason).sentence;
}
