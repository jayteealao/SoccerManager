// The tactics panel: mentality, the team instructions, and a role and duty per slot, all
// rendered from the tactics file the hello carries, so a new instruction or role needs no
// change here. Before kick-off an edit changes the draft that kick-off sends; during play
// each edit is queued as a change and gets a chip. Ordinary DOM with native controls, so
// every control is reachable and operable from the keyboard.

import { PLAYERS_PER_TEAM } from './lineups.mjs';

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

function option(doc, value, text) {
  const o = doc.createElement('option');
  o.value = String(value);
  o.textContent = text;
  return o;
}

function field(doc, text, select) {
  const label = doc.createElement('label');
  label.className = 'field';
  const span = doc.createElement('span');
  span.className = 'field__label';
  span.textContent = text;
  label.append(span, select);
  return label;
}

/// The DOM side. `onEdit(edit, label)` is called for each edit during play.
export class TacticsPanel {
  constructor({ root, schema, tactics, onEdit, onDraft = () => {} }) {
    this.root = root;
    this.schema = schema;
    this.onEdit = onEdit;
    this.onDraft = onDraft;
    this.tactics = tactics;
    this.mode = 'pre-match';
    /// Each slot's player: `{ squad, name }`.
    this.slots = Array.from({ length: PLAYERS_PER_TEAM }, () => ({ squad: null, name: '' }));
    this.build();
  }

  build() {
    const doc = this.root.ownerDocument;
    const schema = this.schema;
    this.root.replaceChildren();

    this.mentality = doc.createElement('select');
    this.mentality.dataset.testid = 'mentality';
    schema.mentalities.forEach((m, i) => this.mentality.append(option(doc, i, words(m.name))));
    this.mentality.addEventListener('change', () => {
      this.edit({ mentality: Number(this.mentality.value) });
    });
    const top = doc.createElement('div');
    top.className = 'tactics__row';
    top.append(field(doc, 'Mentality', this.mentality));

    const grid = doc.createElement('div');
    grid.className = 'tactics__grid';
    this.instructions = instructionNames(schema).map((name, index) => {
      const select = doc.createElement('select');
      select.dataset.testid = `instruction-${name}`;
      schema.instructions[name].levels.forEach((level, i) =>
        select.append(option(doc, i, words(level.name)))
      );
      select.addEventListener('change', () => {
        this.edit({ instruction: index, level: Number(select.value) });
      });
      grid.append(field(doc, words(name), select));
      return select;
    });

    const details = doc.createElement('details');
    details.className = 'tactics__roles';
    const summary = doc.createElement('summary');
    summary.textContent = 'Roles and duties';
    const list = doc.createElement('ol');
    list.className = 'tactics__role-list';
    this.roleRows = Array.from({ length: PLAYERS_PER_TEAM }, (_, slot) => {
      const li = doc.createElement('li');
      li.className = 'tactics__role-row';
      const who = doc.createElement('span');
      who.className = 'tactics__who';
      const role = doc.createElement('select');
      role.dataset.testid = `role-${slot}`;
      const duty = doc.createElement('select');
      duty.dataset.testid = `duty-${slot}`;
      schema.duties.forEach((d, i) => duty.append(option(doc, i, words(d.name))));
      const changed = () => {
        const squad = this.slots[slot].squad;
        this.edit({ role: { squad, role: Number(role.value), duty: Number(duty.value) } }, slot);
      };
      role.addEventListener('change', changed);
      duty.addEventListener('change', changed);
      li.append(who, role, duty);
      list.append(li);
      return { who, role, duty };
    });
    details.append(summary, list);
    this.root.append(top, grid, details);
    this.render();
  }

  /// Before kick-off, during play, or read-only (a stored match that takes no changes).
  setMode(mode) {
    this.mode = mode;
    const disabled = mode === 'read-only';
    for (const select of this.root.querySelectorAll('select')) {
      select.disabled = disabled;
    }
  }

  /// The formation the lineup editor picked, before kick-off. A role that no longer suits
  /// its slot takes the first role that does.
  setFormation(formation) {
    const slots = this.schema.formations[formation].slots;
    this.tactics = {
      ...this.tactics,
      formation,
      roles: this.tactics.roles.map((r, slot) => ({
        role: roleForSlot(this.schema, slots[slot].position, r.role),
        duty: r.duty,
      })),
    };
    this.render();
  }

  /// Each slot's player, `{ squad, name }` in slot order.
  setSlots(players) {
    this.slots = players.map((p) => ({ squad: p?.squad ?? null, name: p?.name ?? '' }));
    this.renderSlots();
  }

  /// Shows `tactics` in every control.
  setTactics(tactics) {
    this.tactics = tactics;
    this.render();
  }

  edit(edit, slot = null) {
    if (edit.mentality !== undefined) {
      this.tactics.mentality = edit.mentality;
    }
    if (edit.instruction !== undefined) {
      this.tactics.instructions[edit.instruction] = edit.level;
    }
    if (edit.role !== undefined && slot !== null) {
      this.tactics.roles[slot] = { role: edit.role.role, duty: edit.role.duty };
      if (edit.role.squad === null && this.mode === 'live') {
        return;
      }
    }
    if (this.mode === 'live') {
      const names = (squad) => this.slots.find((s) => s.squad === squad)?.name || `Player ${squad}`;
      this.onEdit(edit, labelFor(edit, this.schema, names));
    } else {
      this.onDraft();
    }
  }

  render() {
    const t = this.tactics;
    this.mentality.value = String(t.mentality);
    this.instructions.forEach((select, i) => {
      select.value = String(t.instructions[i]);
    });
    this.renderSlots();
  }

  renderSlots() {
    const doc = this.root.ownerDocument;
    const formation = this.schema.formations[this.tactics.formation];
    this.roleRows.forEach((row, slot) => {
      const position = formation.slots[slot].position;
      const player = this.slots[slot];
      row.who.textContent = `${position} ${player.name || 'Empty'}`;
      const current = this.tactics.roles[slot];
      const suits = suitableRoles(this.schema, position);
      const choices = suits.includes(current.role) ? suits : [current.role, ...suits];
      const key = choices.join(',');
      if (row.role.dataset.choices !== key) {
        row.role.replaceChildren(
          ...choices.map((i) => option(doc, i, words(this.schema.roles[i].name)))
        );
        row.role.dataset.choices = key;
      }
      row.role.value = String(current.role);
      row.duty.value = String(current.duty);
      row.role.setAttribute('aria-label', `Role, ${position} ${player.name || 'empty slot'}`);
      row.duty.setAttribute('aria-label', `Duty, ${position} ${player.name || 'empty slot'}`);
    });
  }
}
