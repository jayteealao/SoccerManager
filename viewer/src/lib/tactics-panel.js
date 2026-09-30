// The tactics panel's pure half: mentality, the team instructions, and a role and duty per
// slot, all read from the tactics file the hello carries, so a new instruction or role needs
// no change here. Before kick-off an edit changes the draft that kick-off sends; during play
// each edit is queued as a change and gets a chip. The components in `src/components/` draw
// the controls.

/// `line_height` → `Line height`.
export function words(name) {
  const spaced = String(name ?? '').replaceAll('_', ' ');
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

/// The instruction names in the order the engine stores their levels. A JSON object carries
/// no key order, so the hello names the order; a tactics file read on its own falls back to
/// its key order.
export function instructionNames(schema) {
  return schema?.instruction_order ?? Object.keys(schema?.instructions ?? {});
}

/// The roles that suit `position`, as indices into `schema.roles`.
export function suitableRoles(schema, position) {
  const roles = schema?.roles ?? [];
  const out = [];
  roles.forEach((role, i) => {
    if (role.positions.includes(position)) {
      out.push(i);
    }
  });
  return out;
}

/// The role a slot keeps when the formation changes: its own when it suits the new
/// position, otherwise the first role that does (the engine's rule).
export function roleForSlot(schema, position, current) {
  const suits = suitableRoles(schema, position);
  if (suits.includes(current)) {
    return current;
  }
  return suits.length > 0 ? suits[0] : 0;
}

/// The queue-change detail for one edit. `edit` is `{ mentality }`, `{ instruction, level }`,
/// or `{ role: { squad, role, duty } }`; every value is an index into the tactics file and a
/// role names its player by squad index.
export function detailFor(edit) {
  const patch = {};
  if (edit.mentality !== undefined) {
    patch.mentality = edit.mentality;
  }
  if (edit.instruction !== undefined) {
    const levels = Array(6).fill(null);
    levels[edit.instruction] = edit.level;
    patch.instructions = levels;
  }
  if (edit.role !== undefined) {
    patch.roles = [{ squad: edit.role.squad, role: edit.role.role, duty: edit.role.duty }];
  }
  return { patch };
}

/// The words a chip shows for one edit.
export function labelFor(edit, schema, playerName = (squad) => `Player ${squad}`) {
  if (edit.mentality !== undefined) {
    return `Mentality: ${words(schema.mentalities[edit.mentality]?.name)}`;
  }
  if (edit.instruction !== undefined) {
    const name = instructionNames(schema)[edit.instruction];
    const level = schema.instructions[name]?.levels[edit.level]?.name;
    return `${words(name)}: ${words(level)}`;
  }
  const { squad, role, duty } = edit.role;
  return `${playerName(squad)}: ${words(schema.roles[role]?.name)}, ${words(schema.duties[duty]?.name)}`;
}

/// `tactics` with `patch` applied, for a team whose slots hold `lineup` (squad indices in
/// slot order): the engine's order, formation first.
export function applyPatch(tactics, patch, lineup, schema) {
  const out = {
    formation: tactics.formation,
    mentality: tactics.mentality,
    instructions: [...tactics.instructions],
    roles: tactics.roles.map((r) => ({ ...r })),
  };
  if (patch.formation !== undefined && patch.formation !== null) {
    out.formation = patch.formation;
    const slots = schema.formations[patch.formation].slots;
    out.roles = out.roles.map((r, slot) => ({
      role: roleForSlot(schema, slots[slot].position, r.role),
      duty: r.duty,
    }));
  }
  if (patch.mentality !== undefined && patch.mentality !== null) {
    out.mentality = patch.mentality;
  }
  (patch.instructions ?? []).forEach((level, i) => {
    if (level !== null && level !== undefined) {
      out.instructions[i] = level;
    }
  });
  for (const r of patch.roles ?? []) {
    const slot = lineup.indexOf(r.squad);
    if (slot >= 0) {
      out.roles[slot] = { role: r.role, duty: r.duty };
    }
  }
  return out;
}

/// The pre-match patch that turns `base` into `draft` for a team whose slots hold `lineup`,
/// or `null` when nothing changed.
export function prematchPatch(base, draft, lineup) {
  const patch = {};
  if (draft.formation !== base.formation) {
    patch.formation = draft.formation;
  }
  if (draft.mentality !== base.mentality) {
    patch.mentality = draft.mentality;
  }
  const levels = draft.instructions.map((level, i) =>
    level === base.instructions[i] ? null : level
  );
  if (levels.some((l) => l !== null)) {
    patch.instructions = levels;
  }
  // After a formation change every slot is sent, because the engine resets unsuitable roles
  // when it applies the formation.
  const roles = [];
  draft.roles.forEach((r, slot) => {
    const was = base.roles[slot];
    const changed = patch.formation !== undefined || r.role !== was.role || r.duty !== was.duty;
    if (changed && lineup[slot] !== null && lineup[slot] !== undefined) {
      roles.push({ squad: lineup[slot], role: r.role, duty: r.duty });
    }
  });
  if (roles.length > 0) {
    patch.roles = roles;
  }
  return Object.keys(patch).length > 0 ? patch : null;
}

/// The tactics a hello `setup` names.
export function tacticsOf(setup) {
  return {
    formation: setup.formation,
    mentality: setup.mentality,
    instructions: [...setup.instructions],
    roles: setup.roles.map((r) => ({ role: r.role, duty: r.duty })),
  };
}

/// A copy of `tactics` that shares no array with it.
export function copyTactics(t) {
  return {
    formation: t.formation,
    mentality: t.mentality,
    instructions: [...t.instructions],
    roles: t.roles.map((r) => ({ ...r })),
  };
}
