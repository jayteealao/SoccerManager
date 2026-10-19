// The style presets the hello carries, as the tactics board's picker reads them. A style is
// only a preset of primitive levels and game plans: picking one sets every team instruction
// and the game plans to the preset's, keeps the player instructions, and changes neither the
// formation nor the duties. No code here names a style. Pure: no DOM.

import { defaultLevels, fromWire } from './primitives.js';

/// The presets of `hello`, in the styles file's order: `{ name, primitives, formations,
/// rules, player_instructions }`, each by name. An engine without styles offers none.
export function presetsOf(hello) {
  return Array.isArray(hello?.styles) ? hello.styles : [];
}

/// The level of every primitive under `preset`: the preset's own, and the default for every
/// primitive it does not name.
export function presetLevels(schema, preset) {
  const levels = defaultLevels(schema);
  for (const wire of preset?.primitives ?? []) {
    const at = fromWire(schema, wire);
    if (at) {
      levels[at[0]] = at[1];
    }
  }
  return levels;
}

/// The primitives (indices) whose level in `levels` differs from `preset`'s.
export function changesFrom(schema, levels, preset) {
  if (!preset) {
    return [];
  }
  const want = presetLevels(schema, preset);
  const out = [];
  levels.forEach((level, i) => {
    if (level !== want[i]) {
      out.push(i);
    }
  });
  return out;
}

/// The preset the picker starts on: the one every level matches, or null (Custom).
export function initialPick(schema, levels, presets) {
  return presets.find((p) => changesFrom(schema, levels, p).length === 0)?.name ?? null;
}

/// The game state picking `preset` gives: its game plans, and the player instructions of
/// `current` kept.
export function presetGameState(preset, current) {
  return {
    rules: structuredClone(preset?.rules ?? []),
    instructions: structuredClone(current?.instructions ?? []),
  };
}
