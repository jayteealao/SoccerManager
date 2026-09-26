// The pre-match lineup editor: the formation's eleven slots on a small pitch diagram, the
// bench beside it, and the squad under the pitch. Ordinary DOM buttons, so every slot and
// every player is reachable and operable from the keyboard.
//
// Clicking is the primary path: pick a player, then a slot (or a slot, then a player). Two
// slots picked one after the other swap. Delete or Backspace on a slot empties it. Dragging a
// player onto a slot is an enhancement over the same path, never the only way.

import { checkLineup, fitWord, fitnessWord, lineupMessage, STARTERS } from './lineup.mjs';
import { LENGTH, WIDTH } from './pitch.mjs';

/// A slot's centre on the diagram, in percent of the pitch tile: `x` is metres from the own
/// goal line and `y` metres across, as the tactics file writes them. The home side attacks to
/// the right, and `y` grows downwards, as on the pitch. The centre is kept a half slot inside
/// the tile.
export function slotPlace(x, y) {
  const left = Math.min(92, Math.max(8, (x / LENGTH) * 100));
  const top = Math.min(90, Math.max(10, ((y + WIDTH / 2) / WIDTH) * 100));
  return { left, top };
}

function button(doc, className, testid) {
  const b = doc.createElement('button');
  b.type = 'button';
  b.className = className;
  b.dataset.testid = testid;
  return b;
}

export class LineupEditor {
  constructor({ board, side, squadList, schema, squad, setup, benchSize, roleOf, onFormation, onKickOff, onChange = () => {} }) {
    this.onChange = onChange;
    this.board = board;
    this.side = side;
    this.squadList = squadList;
    this.schema = schema;
    this.squad = squad;
    this.benchSize = benchSize;
    this.roleOf = roleOf;
    this.onFormation = onFormation;
    this.onKickOff = onKickOff;
    this.formation = setup.formation;
    this.slots = setup.lineup.slice(0, STARTERS).map((s) => s ?? null);
    this.bench = Array.from({ length: benchSize }, (_, i) => setup.bench[i] ?? null);
    this.selection = null;
    this.busy = false;
    this.serverReason = null;
    this.build();
    this.render();
  }

  build() {
    const doc = this.board.ownerDocument;

    // The diagram: one button per formation slot.
    this.board.replaceChildren();
    this.slotButtons = Array.from({ length: STARTERS }, (_, n) => {
      const b = button(doc, 'board__slot', `slot-${n}`);
      b.innerHTML =
        '<span class="board__line"><span class="board__pos"></span>' +
        '<span class="board__name"></span></span>' +
        '<span class="board__fit tl-num"></span><span class="board__fitness tl-num"></span>';
      this.wireTarget(b, 'slot', n);
      this.board.append(b);
      return b;
    });

    // The side column: formation, bench, the reason, and kick-off.
    this.side.replaceChildren();
    const formationField = doc.createElement('label');
    formationField.className = 'field';
    const formationText = doc.createElement('span');
    formationText.className = 'field__label';
    formationText.textContent = 'Formation';
    this.formationSelect = doc.createElement('select');
    this.formationSelect.dataset.testid = 'formation';
    this.schema.formations.forEach((f, i) => {
      const o = doc.createElement('option');
      o.value = String(i);
      o.textContent = f.name;
      this.formationSelect.append(o);
    });
    this.formationSelect.addEventListener('change', () => {
      this.formation = Number(this.formationSelect.value);
      this.onFormation(this.formation);
      this.render();
    });
    formationField.append(formationText, this.formationSelect);

    const hint = doc.createElement('p');
    hint.className = 'editor__hint';
    hint.textContent =
      'Pick a player from the squad, then a slot. Two slots picked in turn swap. ' +
      'Delete empties a slot.';

    const benchTitle = doc.createElement('h3');
    benchTitle.className = 'editor__title';
    benchTitle.textContent = 'Bench';
    const benchList = doc.createElement('ol');
    benchList.className = 'editor__bench';
    this.benchButtons = this.bench.map((_, n) => {
      const li = doc.createElement('li');
      const b = button(doc, 'editor__bench-slot', `bench-${n}`);
      this.wireTarget(b, 'bench', n);
      li.append(b);
      benchList.append(li);
      return b;
    });

    this.clearButton = button(doc, 'match-control match-control--sm match-control--quiet', 'clear-slot');
    this.clearButton.textContent = 'Empty the picked slot';
    this.clearButton.addEventListener('click', () => {
      if (this.selection && this.selection.kind !== 'squad') {
        this.put(this.selection, null);
        this.selection = null;
        this.render();
      }
    });

    this.reason = doc.createElement('p');
    this.reason.className = 'editor__reason';
    this.reason.id = 'lineup-reason';
    this.reason.dataset.testid = 'lineup-reason';
    this.reason.setAttribute('role', 'status');

    this.kickOff = button(doc, 'match-control match-control--md', 'kick-off');
    this.kickOff.textContent = 'Kick off';
    this.kickOff.setAttribute('aria-describedby', 'lineup-reason');
    this.kickOff.addEventListener('click', () => {
      if (!this.verdict().legal || this.busy) {
        return;
      }
      this.onKickOff();
    });

    this.side.append(
      formationField,
      hint,
      benchTitle,
      benchList,
      this.clearButton,
      this.reason,
      this.kickOff
    );

    // The squad under the pitch.
    this.squadList.replaceChildren();
    const list = doc.createElement('ul');
    list.className = 'squad__list';
    list.setAttribute('aria-label', 'Squad');
    this.squadButtons = this.squad.map((player, i) => {
      const li = doc.createElement('li');
      const b = button(doc, 'squad__player', `squad-${i}`);
      b.draggable = true;
      b.innerHTML =
        '<span class="squad__pos"></span><span class="squad__shirt tl-num"></span>' +
        '<span class="squad__name"></span><span class="squad__place"></span>';
      b.querySelector('.squad__pos').textContent = player['player.position'];
      b.querySelector('.squad__shirt').textContent = String(player['player.shirt']);
      b.querySelector('.squad__name').textContent = player['player.name'];
      b.addEventListener('click', () => this.pickSquad(i));
      b.addEventListener('dragstart', (event) => {
        event.dataTransfer.setData('text/plain', String(i));
        event.dataTransfer.effectAllowed = 'copy';
      });
      li.append(b);
      list.append(li);
      return b;
    });
    this.squadList.append(list);
  }

  /// A slot or a bench place: click, keyboard, and drop target.
  wireTarget(b, kind, n) {
    b.addEventListener('click', () => this.pickPlace({ kind, n }));
    b.addEventListener('keydown', (event) => {
      if (event.key === 'Delete' || event.key === 'Backspace') {
        event.preventDefault();
        this.put({ kind, n }, null);
        this.render();
      }
    });
    b.addEventListener('dragover', (event) => event.preventDefault());
    b.addEventListener('drop', (event) => {
      event.preventDefault();
      const index = Number(event.dataTransfer.getData('text/plain'));
      if (Number.isInteger(index) && this.squad[index]) {
        this.put({ kind, n }, index);
        this.selection = null;
        this.render();
      }
    });
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

  pickSquad(index) {
    const sel = this.selection;
    if (sel && sel.kind !== 'squad') {
      this.put(sel, index);
      this.selection = null;
    } else if (sel && sel.kind === 'squad' && sel.index === index) {
      this.selection = null;
    } else {
      this.selection = { kind: 'squad', index };
    }
    this.render();
  }

  pickPlace(place) {
    const sel = this.selection;
    if (sel && sel.kind === 'squad') {
      this.put(place, sel.index);
      this.selection = null;
    } else if (sel && sel.kind === place.kind && sel.n === place.n) {
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
    this.render();
  }

  verdict() {
    return checkLineup({
      slots: this.slots,
      bench: this.bench,
      squad: this.squad,
      benchSize: this.benchSize,
    });
  }

  /// The engine refused the lineup; its reason shows until the lineup changes.
  refused(reason) {
    this.busy = false;
    this.serverReason = reason;
    this.render();
  }

  setBusy(busy) {
    this.busy = busy;
    this.render();
  }

  /// The `set-lineup` fields.
  message() {
    return lineupMessage(this.slots, this.bench);
  }

  /// Each slot's player as the tactics panel names it.
  slotPlayers() {
    return this.slots.map((s) =>
      s === null ? { squad: null, name: '' } : { squad: s, name: this.squad[s]['player.name'] }
    );
  }

  /// The read-only view the test hook returns.
  snapshot() {
    const verdict = this.verdict();
    return {
      formation: this.formation,
      slots: [...this.slots],
      bench: [...this.bench],
      legal: verdict.legal,
      reason: this.serverReason ?? verdict.reason,
      kickOffDisabled: this.kickOff.disabled,
      selection: this.selection,
    };
  }

  placeWord(index) {
    if (this.slots.includes(index)) {
      return 'In the eleven';
    }
    return this.bench.includes(index) ? 'On the bench' : '';
  }

  render() {
    const formation = this.schema.formations[this.formation];
    this.formationSelect.value = String(this.formation);
    const sel = this.selection;
    this.slotButtons.forEach((b, n) => {
      const spot = formation.slots[n];
      const place = slotPlace(spot.x, spot.y);
      b.style.left = `${place.left}%`;
      b.style.top = `${place.top}%`;
      const index = this.slots[n];
      const player = index === null ? null : this.squad[index];
      b.querySelector('.board__pos').textContent = spot.position;
      b.dataset.state = player ? 'filled' : 'empty';
      b.setAttribute('aria-pressed', String(sel?.kind === 'slot' && sel.n === n));
      if (!player) {
        b.querySelector('.board__name').textContent = 'Empty';
        b.querySelector('.board__fit').textContent = '';
        b.querySelector('.board__fitness').textContent = '';
        b.setAttribute('aria-label', `Slot ${n + 1}, ${spot.position}, empty`);
        return;
      }
      const fit = player.role_fit[this.roleOf(n)] ?? 0;
      const fitness = player['player.natural_fitness'];
      b.querySelector('.board__name').textContent =
        `${player['player.shirt']} ${player['player.name']}`;
      b.querySelector('.board__fit').textContent = `Fit ${fit} ${fitWord(fit)}`;
      b.querySelector('.board__fitness').textContent =
        `Fitness ${fitness} ${fitnessWord(fitness)}`;
      b.setAttribute(
        'aria-label',
        `Slot ${n + 1}, ${spot.position}: ${player['player.name']}, ` +
          `${player['player.position']}, role fit ${fit} ${fitWord(fit)}, ` +
          `fitness ${fitness} ${fitnessWord(fitness)}`
      );
    });
    this.benchButtons.forEach((b, n) => {
      const index = this.bench[n];
      const player = index === null ? null : this.squad[index];
      b.dataset.state = player ? 'filled' : 'empty';
      b.setAttribute('aria-pressed', String(sel?.kind === 'bench' && sel.n === n));
      b.textContent = player
        ? `${player['player.position']} ${player['player.shirt']} ${player['player.name']}`
        : `Substitute ${n + 1}: empty`;
      b.setAttribute(
        'aria-label',
        player
          ? `Bench place ${n + 1}: ${player['player.name']}, ${player['player.position']}`
          : `Bench place ${n + 1}, empty`
      );
    });
    this.squadButtons.forEach((b, i) => {
      const where = this.placeWord(i);
      b.querySelector('.squad__place').textContent = where;
      b.dataset.placed = where ? 'true' : 'false';
      b.setAttribute('aria-pressed', String(sel?.kind === 'squad' && sel.index === i));
      const player = this.squad[i];
      b.setAttribute(
        'aria-label',
        `${player['player.position']} ${player['player.shirt']} ${player['player.name']}` +
          (where ? `, ${where.toLowerCase()}` : '')
      );
    });
    this.clearButton.disabled = !sel || sel.kind === 'squad';
    const verdict = this.verdict();
    const reason = this.serverReason ?? verdict.reason;
    this.reason.textContent = reason ?? 'The lineup is ready.';
    this.reason.dataset.legal = String(verdict.legal && !this.serverReason);
    this.kickOff.disabled = !verdict.legal || this.busy;
    this.kickOff.textContent = this.busy ? 'Kicking off' : 'Kick off';
    this.onChange();
  }
}
