// The pre-match lineup editor's state: the formation's eleven slots, the bench places, and
// the one squad list that shows every player with the place he holds. Pure: no DOM; the
// Tactics screen's components draw it and call its methods.
//
// Clicking is the primary path, as on today's page: pick a player, then a slot (or a slot,
// then a player); two slots picked one after the other swap. Two squad rows picked one after
// the other swap the places the two players hold. Delete on a slot, or "Empty the picked
// slot", empties the picked place. Dragging a row onto another row or onto a slot is the same
// swap by pointer, never the only way.

import { checkLineup, fitWord, fitnessWord, lineupMessage, STARTERS } from './lineup.js';
import { LENGTH, WIDTH } from './pitch.js';
import { positionsFor } from './positions.js';

/// A slot's centre on the diagram, in percent of the pitch tile: `x` is metres from the own
/// goal line and `y` metres across, as the tactics file writes them. The home side attacks to
/// the right, and `y` grows downwards, as on the pitch. The centre is kept a half slot inside
/// the tile.
export function slotPlace(x, y) {
  const left = Math.min(92, Math.max(8, (x / LENGTH) * 100));
  const top = Math.min(90, Math.max(10, ((y + WIDTH / 2) / WIDTH) * 100));
  return { left, top };
}

/// The same centre on a pitch drawn upright, own goal at the bottom (the board's Tactics
/// pitches): across becomes left to right and the length runs upwards.
export function uprightPlace(x, y) {
  const flat = slotPlace(x, y);
  return { left: flat.top, top: 100 - flat.left };
}

const samePlace = (a, b) => a !== null && b !== null && a.kind === b.kind && a.n === b.n;

export class LineupEditor {
  /// `schema` is the hello's tactics file, `squad` the home squad, `setup` the computer
  /// manager's pick to start from, `benchSize` the bench places.
  constructor({ schema, squad, setup, benchSize }) {
    this.schema = schema;
    this.squad = squad;
    this.benchSize = benchSize;
    this.formation = setup.formation;
    this.slots = setup.lineup.slice(0, STARTERS).map((s) => s ?? null);
    while (this.slots.length < STARTERS) {
      this.slots.push(null);
    }
    this.bench = Array.from({ length: benchSize }, (_, i) => setup.bench[i] ?? null);
    /// `{ kind: 'slot' | 'bench', n }`, `{ kind: 'squad', index, place }`, or null.
    this.selection = null;
    this.serverReason = null;
  }

  get(place) {
    return place.kind === 'slot' ? this.slots[place.n] : this.bench[place.n];
  }

  put(place, index) {
    if (place.kind === 'slot') {
      this.slots[place.n] = index;
    } else {
      this.bench[place.n] = index;
    }
    this.serverReason = null;
  }

  /// Every place squad player `index` holds, slots first. More than one is a clash.
  placesOf(index) {
    const out = [];
    this.slots.forEach((s, n) => {
      if (s === index) {
        out.push({ kind: 'slot', n });
      }
    });
    this.bench.forEach((s, n) => {
      if (s === index) {
        out.push({ kind: 'bench', n });
      }
    });
    return out;
  }

  /// A click on a pitch slot or a bench place.
  pickPlace(place) {
    const sel = this.selection;
    if (sel && sel.kind === 'squad') {
      this.put(place, sel.index);
      this.selection = null;
    } else if (sel && samePlace(sel, place)) {
      this.selection = null;
    } else if (sel) {
      const a = this.get(sel);
      const b = this.get(place);
      this.put(sel, b);
      this.put(place, a);
      this.selection = null;
    } else {
      this.selection = place;
    }
  }

  /// A click on a squad row. With a place picked, the player takes that place; with another
  /// row picked, the two players swap places; the same row again drops the pick.
  pickRow(index) {
    const sel = this.selection;
    if (sel && sel.kind !== 'squad') {
      this.put(sel, index);
      this.selection = null;
    } else if (sel && sel.index === index) {
      this.selection = null;
    } else if (sel) {
      this.swap(sel.index, index);
    } else {
      this.selection = { kind: 'squad', index, place: this.placesOf(index)[0] ?? null };
    }
  }

  /// Swaps the places squad players `a` and `b` hold. Two players who hold no place swap
  /// nothing, and the second becomes the picked row.
  swap(a, b) {
    const pa = this.placesOf(a)[0] ?? null;
    const pb = this.placesOf(b)[0] ?? null;
    if (!pa && !pb) {
      this.selection = { kind: 'squad', index: b, place: null };
      return;
    }
    if (pa) {
      this.put(pa, b);
    }
    if (pb) {
      this.put(pb, a);
    }
    this.selection = null;
  }

  /// A row dragged onto a row, or onto a slot or a bench place.
  drop(from, to) {
    if (typeof to === 'number') {
      if (to !== from) {
        this.swap(from, to);
      }
    } else {
      this.put(to, from);
    }
    this.selection = null;
  }

  /// "Empty the picked slot": the picked place, or the place the picked row holds.
  emptyPicked() {
    const sel = this.selection;
    const place = sel?.kind === 'squad' ? sel.place : sel;
    if (place) {
      this.put(place, null);
      this.selection = null;
    }
  }

  /// `true` when "Empty the picked slot" has a place to empty.
  get canEmpty() {
    const sel = this.selection;
    return sel !== null && (sel.kind !== 'squad' || sel.place !== null);
  }

  verdict() {
    return checkLineup({
      slots: this.slots,
      bench: this.bench,
      squad: this.squad,
      benchSize: this.benchSize,
    });
  }

  /// The words the tactic bar shows: the engine's refusal until the lineup changes, else the
  /// page's own check.
  get reason() {
    return this.serverReason ?? this.verdict().reason;
  }

  get ready() {
    return this.verdict().legal && this.serverReason === null;
  }

  /// What the NOT READY words point at, in the check's order: the first empty slot, the
  /// keeper's slot holding an outfield player, or the player placed twice. `{ kind: 'slot', n }`
  /// for an empty slot, `{ kind: 'squad', index }` for a player, or null when no one row is
  /// at fault (a full bench, or the engine's own refusal).
  culprit() {
    if (this.serverReason !== null || this.verdict().legal) {
      return null;
    }
    const empty = this.slots.indexOf(null);
    if (empty !== -1) {
      return { kind: 'slot', n: empty };
    }
    if (this.squad[this.slots[0]]?.['player.position'] !== 'GK') {
      return { kind: 'squad', index: this.slots[0] };
    }
    const seen = new Set();
    for (const index of [...this.slots, ...this.bench]) {
      if (index !== null && seen.has(index)) {
        return { kind: 'squad', index };
      }
      seen.add(index);
    }
    return null;
  }

  /// The engine refused the lineup; its reason shows until the lineup changes.
  refused(reason) {
    this.serverReason = reason;
  }

  /// The `set-lineup` fields.
  message() {
    return lineupMessage(this.slots, this.bench);
  }

  /// Each slot's player as the roles table names it.
  slotPlayers() {
    return this.slots.map((s) =>
      s === null ? { squad: null, name: '' } : { squad: s, name: this.squad[s]['player.name'] }
    );
  }

  /// The eleven slot buttons on the pitch: the formation's place, the player, and the words
  /// each button is named by. `roleOf(n)` is slot `n`'s role index.
  slotRows(roleOf) {
    const formation = this.schema.formations[this.formation];
    return this.slots.map((index, n) => {
      const spot = formation.slots[n];
      const player = index === null ? null : this.squad[index];
      const picked = this.selection?.kind === 'slot' && this.selection.n === n;
      if (!player) {
        return {
          n,
          position: spot.position,
          place: uprightPlace(spot.x, spot.y),
          player: null,
          picked,
          label: `Slot ${n + 1}, ${spot.position}, empty`,
        };
      }
      const fit = player.role_fit[roleOf(n)] ?? 0;
      const fitness = player['player.natural_fitness'];
      return {
        n,
        position: spot.position,
        place: uprightPlace(spot.x, spot.y),
        player: {
          index,
          shirt: player['player.shirt'],
          name: player['player.name'],
          fit,
          fitWord: fitWord(fit),
        },
        picked,
        label:
          `Slot ${n + 1}, ${spot.position}: ${player['player.name']}, ` +
          `${player['player.position']}, role fit ${fit} ${fitWord(fit)}, ` +
          `fitness ${fitness} ${fitnessWord(fitness)}`,
      };
    });
  }

  /// Every squad player once, in three groups: the eleven in slot order, the bench in bench
  /// order, then the players not picked in squad order. Each row carries its slot chip, its
  /// position chips, its fitness, and the words its button is named by.
  squadRows() {
    const formation = this.schema.formations[this.formation];
    const roles = this.schema.roles ?? [];
    const row = (index) => {
      const player = this.squad[index];
      const places = this.placesOf(index);
      const clash = places.length > 1;
      const chips = places.map((p) =>
        p.kind === 'slot' ? formation.slots[p.n].position : `S${p.n + 1}`
      );
      const first = places[0] ?? null;
      let chipKind = 'none';
      if (clash) {
        chipKind = 'clash';
      } else if (first?.kind === 'slot') {
        chipKind = 'eleven';
      } else if (first?.kind === 'bench') {
        chipKind = 'bench';
      }
      const fitness = player['player.natural_fitness'];
      const picked = this.selection?.kind === 'squad' && this.selection.index === index;
      let where = 'not picked';
      if (first?.kind === 'slot') {
        where = `in the eleven at ${chips[0]}`;
      } else if (first?.kind === 'bench') {
        where = `on the bench, substitute ${first.n + 1}`;
      }
      return {
        index,
        id: player['player.id'],
        shirt: player['player.shirt'],
        name: player['player.name'],
        position: player['player.position'],
        positions: positionsFor(player.role_fit, roles, player['player.position']),
        fitness,
        fitnessWord: fitnessWord(fitness),
        group: first?.kind === 'slot' ? 'eleven' : first?.kind === 'bench' ? 'bench' : 'out',
        chip: chips.length > 0 ? chips.join('+') : '—',
        chipKind,
        clash,
        picked,
        label:
          `${player['player.shirt']} ${player['player.name']}, ${player['player.position']}, ` +
          `${where}${clash ? ', named twice' : ''}, fitness ${fitness} ${fitnessWord(fitness)}`,
      };
    };
    const seen = new Set();
    const take = (index) => {
      if (index === null || seen.has(index) || !this.squad[index]) {
        return [];
      }
      seen.add(index);
      return [row(index)];
    };
    const eleven = this.slots.flatMap(take);
    const bench = this.bench.flatMap(take);
    const out = this.squad.map((_, i) => i).flatMap(take);
    return { eleven, bench, out };
  }

  /// Empties one place: Delete on a slot.
  emptyPlace(place) {
    this.put(place, null);
    this.selection = null;
  }

  /// The squad list as the screen draws it: the three groups of `squadRows()`, with a row for
  /// each empty place in the eleven and on the bench, so an emptied place can be filled by a
  /// click or a drop. An empty row is `{ empty: true, place, chip, picked, label }`.
  squadList() {
    const rows = this.squadRows();
    const byIndex = new Map(
      [...rows.eleven, ...rows.bench, ...rows.out].map((r) => [r.index, r])
    );
    const formation = this.schema.formations[this.formation];
    const seen = new Set();
    const group = (places, kind) =>
      places.flatMap((index, n) => {
        const place = { kind, n };
        const chip = kind === 'slot' ? formation.slots[n].position : `S${n + 1}`;
        if (index === null) {
          const where = kind === 'slot' ? `slot ${n + 1}, ${chip}` : `substitute ${n + 1}`;
          return [
            {
              empty: true,
              key: `${kind}-${n}`,
              place,
              chip,
              picked: samePlace(this.selection, place),
              label: `Empty place: ${where}`,
            },
          ];
        }
        if (seen.has(index) || !byIndex.has(index)) {
          return [];
        }
        seen.add(index);
        return [byIndex.get(index)];
      });
    const eleven = group(this.slots, 'slot');
    const bench = group(this.bench, 'bench');
    const out = rows.out.filter((r) => !seen.has(r.index));
    return { eleven, bench, out };
  }

  /// The words the picked-row hint names, or null with nothing picked.
  pickedText() {
    const sel = this.selection;
    if (!sel) {
      return null;
    }
    if (sel.kind === 'squad') {
      const name = this.squad[sel.index]?.['player.name'] ?? `Player ${sel.index}`;
      if (!sel.place) {
        return `${name} picked.`;
      }
      const where =
        sel.place.kind === 'slot'
          ? this.schema.formations[this.formation].slots[sel.place.n].position
          : `S${sel.place.n + 1}`;
      return `${name} · ${where} picked.`;
    }
    const index = this.get(sel);
    const where =
      sel.kind === 'slot'
        ? this.schema.formations[this.formation].slots[sel.n].position
        : `S${sel.n + 1}`;
    const name = index === null ? 'Empty place' : this.squad[index]['player.name'];
    return `${name} · ${where} picked.`;
  }

  /// The read-only view the test hook returns.
  snapshot() {
    const verdict = this.verdict();
    return {
      formation: this.formation,
      slots: [...this.slots],
      bench: [...this.bench],
      legal: verdict.legal,
      reason: this.reason,
      selection: this.selection ? { ...this.selection } : null,
    };
  }
}
