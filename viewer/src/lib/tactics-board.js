// The Tactics board's words: the short role and duty tags the markers carry, the duty colour
// token, and which of the team instructions sit in the Limits card and which in the Weights
// card. Pure: read from the tactics file the hello carries.

import { instructionNames, words } from './tactics-panel.js';

/// Instructions that bound where players may go: the Limits card.
export const LIMITS = Object.freeze(['line_height', 'width']);

/// Instructions that tilt how often an option is chosen: the Weights card. An instruction
/// the file adds later joins this card.
export const WEIGHTS = Object.freeze(['tempo', 'passing_directness', 'pressing', 'time_wasting']);

/// One-word roles whose initial alone would be unclear.
const ONE_WORD_TAGS = Object.freeze({ goalkeeper: 'GK', playmaker: 'PM', striker: 'ST', winger: 'W' });

/// `central_defender` → `CD`: the initials of a role's words, as the board's markers tag them.
export function roleTag(name) {
  const key = String(name ?? '');
  if (ONE_WORD_TAGS[key]) {
    return ONE_WORD_TAGS[key];
  }
  const parts = key.split('_').filter(Boolean);
  if (parts.length === 1) {
    return parts[0].slice(0, 2).toUpperCase();
  }
  return parts.map((p) => p[0].toUpperCase()).join('');
}

/// `support` → `Su`.
export function dutyTag(name) {
  const key = String(name ?? '');
  return key.charAt(0).toUpperCase() + key.charAt(1);
}

/// The skin token a duty's tag is drawn in; the tag's words carry the duty as well.
export function dutyToken(name) {
  if (name === 'attack' || name === 'support' || name === 'defend') {
    return `--duty-${name}`;
  }
  return '--ink';
}

/// `role · duty` as a marker writes it, for slot `slot` of `tactics`.
export function markerTag(schema, tactics, slot) {
  const r = tactics?.roles?.[slot];
  if (!r) {
    return { tag: '', duty: '', token: '--ink' };
  }
  const duty = schema.duties?.[r.duty]?.name ?? '';
  return {
    tag: `${roleTag(schema.roles?.[r.role]?.name)}·${dutyTag(duty)}`,
    duty,
    token: dutyToken(duty),
  };
}

/// The instruction rows of one card: `{ index, name, label, value, levels }`, where `index`
/// is the instruction's place in the engine's order and `levels` the select's options.
export function instructionRows(schema, instructions, card) {
  const names = instructionNames(schema);
  const inCard = (name) =>
    card === 'limits' ? LIMITS.includes(name) : !LIMITS.includes(name);
  return names
    .map((name, index) => ({ name, index }))
    .filter(({ name }) => inCard(name))
    .map(({ name, index }) => ({
      index,
      name,
      label: words(name),
      value: instructions?.[index] ?? 0,
      levels: (schema.instructions[name]?.levels ?? []).map((level, i) => ({
        value: i,
        label: words(level.name),
      })),
    }));
}

/// `Base shape 4-2-3-1 · Mentality Positive`: the words beside the tactic selects.
export function shapeLine(schema, tactics) {
  if (!schema || !tactics) {
    return '';
  }
  const shape = schema.formations?.[tactics.formation]?.name ?? '';
  const mentality = words(schema.mentalities?.[tactics.mentality]?.name);
  return `Base shape ${shape} · Mentality ${mentality}`;
}
