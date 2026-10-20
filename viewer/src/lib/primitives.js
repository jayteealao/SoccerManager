// The tactical primitives of the tactics file the hello carries, read as the engine reads
// them. Pure: no DOM.
//
// A primitive's level lives in one of three places: the mentality, one of the six team
// instructions (both ride the existing tactics patch before kick-off), or the tactics' own
// settings (every other primitive). The board holds one level per primitive, in the tactics
// file's order; `split` and `merge` move between that list and the tactics the Shapes tab
// edits, so the two views never disagree.

import { instructionNames } from './tactics-panel.js';

/// The primitives of `schema`, in the tactics file's order.
export function primitivesOf(schema) {
  return Array.isArray(schema?.primitives) ? schema.primitives : [];
}

/// The index of primitive `name`, or -1.
export function primitiveIndex(schema, name) {
  return primitivesOf(schema).findIndex((p) => p.name === name);
}

/// The names of primitive `p`'s levels, choices or states, in order (the engine's
/// `Primitive::level_names`).
export function levelNames(schema, p) {
  if (!p) {
    return [];
  }
  if (p.levels === 'own') {
    return (p.steps ?? []).map((s) => s.name);
  }
  if (p.kind === 'choice') {
    return (p.choices ?? []).map((c) => c.name);
  }
  if (p.kind === 'on_off' && !p.levels) {
    return [...(p.states ?? [])];
  }
  if (p.levels === 'mentalities') {
    return (schema.mentalities ?? []).map((m) => m.name);
  }
  if (typeof p.levels === 'string' && p.levels.startsWith('instructions.')) {
    const name = p.levels.slice('instructions.'.length);
    return (schema.instructions?.[name]?.levels ?? []).map((l) => l.name);
  }
  return [...(p.states ?? [])];
}

/// The index of primitive `p`'s default level.
export function defaultLevel(schema, p) {
  const at = levelNames(schema, p).indexOf(p?.default);
  return at < 0 ? 0 : at;
}

/// Where primitive `p` lives: `{ mentality: true }`, `{ instruction: i }`, or `{ setting: true }`.
export function home(schema, p) {
  if (p?.levels === 'mentalities') {
    return { mentality: true };
  }
  if (typeof p?.levels === 'string' && p.levels.startsWith('instructions.')) {
    const at = instructionNames(schema).indexOf(p.levels.slice('instructions.'.length));
    if (at >= 0) {
      return { instruction: at };
    }
  }
  return { setting: true };
}

/// The default level of every primitive, in order.
export function defaultLevels(schema) {
  return primitivesOf(schema).map((p) => defaultLevel(schema, p));
}

/// The board's list of levels: `levels` (one per primitive, or empty for an older engine,
/// which reads as the defaults) with the mentality and the team instructions taken from
/// `tactics`.
export function merge(schema, tactics, levels) {
  const base = Array.isArray(levels) && levels.length === primitivesOf(schema).length ? [...levels] : defaultLevels(schema);
  primitivesOf(schema).forEach((p, i) => {
    const at = home(schema, p);
    if (at.mentality && tactics) {
      base[i] = tactics.mentality;
    } else if (at.instruction !== undefined && tactics) {
      base[i] = tactics.instructions[at.instruction];
    }
  });
  return base;
}

/// `tactics` and `levels` with primitive `index` at `level`: the mentality and the team
/// instructions move in the tactics, every other primitive in the levels.
export function withLevel(schema, tactics, levels, index, level) {
  const p = primitivesOf(schema)[index];
  const at = home(schema, p);
  const t = tactics ? { ...tactics, instructions: [...tactics.instructions] } : tactics;
  const out = [...levels];
  out[index] = level;
  if (at.mentality && t) {
    t.mentality = level;
  } else if (at.instruction !== undefined && t) {
    t.instructions[at.instruction] = level;
  }
  return { tactics: t, levels: out };
}

/// The `{ primitive, level }` names of primitive `index` at `level`, as the wire writes them.
export function levelWire(schema, index, level) {
  const p = primitivesOf(schema)[index];
  return { primitive: p?.name, level: levelNames(schema, p)[level] };
}

/// The level index a wire `{ primitive, level }` names: `[index, level]`, or null.
export function fromWire(schema, wire) {
  const index = primitiveIndex(schema, wire?.primitive);
  if (index < 0) {
    return null;
  }
  const level = levelNames(schema, primitivesOf(schema)[index]).indexOf(wire.level);
  return level < 0 ? null : [index, level];
}

/// The pre-match levels the set-lineup carries: every primitive that lives in the settings and
/// differs from `base`, by name. The mentality and the team instructions ride the patch.
export function prematchLevels(schema, base, draft) {
  const out = [];
  primitivesOf(schema).forEach((p, i) => {
    if (home(schema, p).setting && draft[i] !== base[i]) {
      out.push(levelWire(schema, i, draft[i]));
    }
  });
  return out;
}
