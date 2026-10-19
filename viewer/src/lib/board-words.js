// The tactics board's words: every label, level word, choice word and style name the board
// shows, in a football-management player's words, never the tactics file's ids. Pure: no DOM.
//
// Each primitive has a short label (the row), a full name (the control's accessible name and
// the player instruction list), and one word per level, choice or state in the tactics file's
// order. A primitive the table does not know falls back to its name in words, so a new
// primitive in the tactics file still shows, and the unit test names it.

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
    full: 'Creative freedom',
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
    full: 'When we win the ball',
    levels: L('Keep it|Mostly keep|Mixed|Counter|Always counter'),
  },
  on_losing_the_ball: {
    label: 'Reaction',
    full: 'When we lose the ball',
    levels: L('Always drop|Drop back|Mixed|Press|Always press'),
  },
  counter_press_length: { label: 'Press for', full: 'How long to counter-press', levels: L('2 s|4 s|5 s|6 s|8 s') },
  pressing: { label: 'Intensity', full: 'Pressing intensity', levels: L('Very low|Low|Medium|High|Very high') },
  line_height: { label: 'Defensive line', full: 'Defensive line', levels: L('Very deep|Deep|Normal|High|Very high') },
  line_of_engagement: {
    label: 'Press from',
    full: 'Where we start pressing',
    levels: L('Our third|Our half|Halfway|Their half|Their third'),
  },
  defensive_width: {
    label: 'Width',
    full: 'Width without the ball',
    levels: L('Very narrow|Narrow|Normal|Wide|Very wide'),
  },
  force_the_opponent: {
    label: 'Show them',
    full: 'Show the opponent',
    levels: L('Always wide|Wide|Either way|Inside|Always inside'),
  },
  tackling: { label: 'Tackling', full: 'Tackling', levels: L('Stay on feet|Careful|Normal|Committed|Fly in') },
  offside_trap: { label: 'Offside trap', full: 'Offside trap', levels: OFF_ON },
  stop_short_goal_kicks: { label: 'Press goal kicks', full: 'Press short goal kicks', levels: OFF_ON },
  press_on_back_pass: { label: 'They pass back', full: 'Press when they pass back', levels: OFF_ON },
  press_on_poor_touch: { label: 'Bad touch', full: 'Press after a bad touch', levels: OFF_ON },
  press_on_facing_back: { label: 'Back to goal', full: 'Press a player with his back to goal', levels: OFF_ON },
  press_on_wide: { label: 'Ball goes wide', full: 'Press when the ball goes wide', levels: OFF_ON },
  time_wasting: { label: 'Time wasting', full: 'Time wasting', levels: OFF_ON },
});

/// The style presets by name: what the picker calls each, and one line on how it plays. The
/// block is set by two primitives, never by a style's name, so no style is called a press or
/// a block.
export const STYLE_WORDS = Object.freeze({
  possession: { name: 'Possession', note: 'short passes, patient, high line' },
  low_block: { name: 'Deep and compact', note: 'sit deep and compact, then counter' },
  high_press: { name: 'Press from the front', note: 'press from their third, every trigger on' },
  counter_attack: { name: 'Counter-attack', note: 'sit, then break forward fast' },
  direct: { name: 'Direct', note: 'go long, early balls forward' },
  wing_play: { name: 'Wing play', note: 'wide, cross often' },
  gegenpress: { name: 'Gegenpress', note: 'press high, win it back at once' },
  park_the_bus: { name: 'Park the bus', note: 'very deep, narrow, protect the lead' },
});

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
  return STYLE_WORDS[name] ?? { name: words(name), note: '' };
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

/// The engine's reason for a change it did not take, as a sentence a player reads. The
/// reason itself stays as the code for a report.
export function reasonSentence(reason) {
  const text = String(reason ?? '');
  if (/is not a level of|is not a primitive|does not hold/.test(text)) {
    return 'The engine does not know that setting.';
  }
  if (/reaches only|does not reach the position/.test(text)) {
    return 'That instruction does not suit his position.';
  }
  if (/rules; a tactic holds at most/.test(text)) {
    return 'There are too many game plans.';
  }
  if (/condition kind|allowed .* to|needs |takes /.test(text)) {
    return 'A game plan condition is out of range.';
  }
  if (/not connected/.test(text)) {
    return 'The engine is not connected.';
  }
  if (/has not started|not kicked off/.test(text)) {
    return 'The match has not started.';
  }
  return 'The engine did not take it.';
}
