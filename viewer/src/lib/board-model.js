// What one instruction row of the tactics board shows: its words, its level, its default,
// and whether it differs from the style or failed to apply. Pure: no DOM.

import { controlFor } from './board-cards.js';
import { levelWord, primitiveWords } from './board-words.js';
import { defaultLevel, levelNames, primitiveIndex, primitivesOf } from './primitives.js';

/// The row of primitive `name`, or null when the tactics file has no such primitive.
/// `levels` are the board's levels; `styleLevels` the style's (or null for Custom);
/// `refused` the Not applied banner (or null).
export function rowModel(schema, name, { levels = [], styleLevels = null, refused = null } = {}) {
  const index = primitiveIndex(schema, name);
  if (index < 0) {
    return null;
  }
  const p = primitivesOf(schema)[index];
  const names = levelNames(schema, p);
  const w = primitiveWords(name, names);
  const value = levels[index] ?? defaultLevel(schema, p);
  const words = names.map((_, i) => levelWord(name, i, names));
  const changed = styleLevels !== null && styleLevels[index] !== undefined && styleLevels[index] !== value;
  return {
    index,
    primitive: name,
    label: w.label,
    full: w.full,
    words,
    value,
    def: defaultLevel(schema, p),
    control: controlFor(p, names.length),
    changed,
    styleWord: styleLevels ? words[styleLevels[index]] : null,
    error: refused?.primitive === name,
  };
}
