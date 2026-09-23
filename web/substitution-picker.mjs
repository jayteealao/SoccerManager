// The substitution picker: a player on the pitch, a player on the bench, and the count the
// rule pack leaves. The engine applies the change at the next stoppage that admits one and
// refuses it there with its own reason; the picker never predicts that verdict.

/// Substitutions the home club has left: the rule pack's limit less the home substitutions
/// playback has reached.
export function remaining(limit, substitutions, homeTeamId) {
  const used = substitutions.filter((s) => s.team === homeTeamId).length;
  return Math.max(0, limit - used);
}

/// The words beside the count, so the figure is never read alone.
export function remainingText(left, limit) {
  return `${left} of ${limit} substitutions left`;
}

function option(doc, value, text) {
  const o = doc.createElement('option');
  o.value = String(value);
  o.textContent = text;
  return o;
}

/// The DOM side. `onQueue({ off, on }, label)` is called when the manager queues a change.
export class SubstitutionPicker {
  constructor({ root, limit, onQueue }) {
    this.root = root;
    this.limit = limit;
    this.onQueue = onQueue;
    this.left = limit;
    const doc = root.ownerDocument;
    root.replaceChildren();

    const head = doc.createElement('div');
    head.className = 'picker__head';
    const title = doc.createElement('h3');
    title.className = 'picker__title';
    title.textContent = 'Substitution';
    this.count = doc.createElement('span');
    this.count.className = 'picker__count tl-num';
    this.count.dataset.testid = 'subs-left';
    head.append(title, this.count);

    this.off = doc.createElement('select');
    this.off.dataset.testid = 'sub-off';
    this.off.setAttribute('aria-label', 'Player coming off');
    this.on = doc.createElement('select');
    this.on.dataset.testid = 'sub-on';
    this.on.setAttribute('aria-label', 'Substitute coming on');
    this.button = doc.createElement('button');
    this.button.type = 'button';
    this.button.className = 'match-control match-control--sm';
    this.button.dataset.testid = 'sub-queue';
    this.button.textContent = 'Queue substitution';
    this.button.addEventListener('click', () => this.queue());
    this.note = doc.createElement('p');
    this.note.className = 'picker__note';

    const row = doc.createElement('div');
    row.className = 'picker__row';
    row.append(this.off, this.on);
    root.append(head, row, this.button, this.note);
    this.players = { off: [], on: [] };
    this.setEnabled(false, 'Substitutions open at kick-off.');
    this.setUsed(0);
  }

  /// The players on the pitch and on the bench, each `{ squad, shirt, name }`.
  setPlayers(onPitch, bench) {
    const key = `${onPitch.map((p) => p.squad).join(',')}|${bench.map((p) => p.squad).join(',')}`;
    if (key === this.key) {
      return;
    }
    this.key = key;
    const doc = this.root.ownerDocument;
    const keepOff = this.off.value;
    const keepOn = this.on.value;
    this.players = { off: onPitch, on: bench };
    this.off.replaceChildren(
      ...onPitch.map((p) => option(doc, p.squad, `${p.shirt} ${p.name} off`))
    );
    this.on.replaceChildren(...bench.map((p) => option(doc, p.squad, `${p.shirt} ${p.name} on`)));
    if (onPitch.some((p) => String(p.squad) === keepOff)) {
      this.off.value = keepOff;
    }
    if (bench.some((p) => String(p.squad) === keepOn)) {
      this.on.value = keepOn;
    }
    this.refresh();
  }

  /// Home substitutions playback has reached.
  setUsed(used) {
    this.left = Math.max(0, this.limit - used);
    this.count.textContent = remainingText(this.left, this.limit);
  }

  setEnabled(enabled, note = '') {
    this.enabled = enabled;
    this.note.textContent = note;
    this.refresh();
  }

  refresh() {
    const ready = this.enabled && this.players.off.length > 0 && this.players.on.length > 0;
    this.off.disabled = !this.enabled;
    this.on.disabled = !this.enabled;
    this.button.disabled = !ready;
  }

  queue() {
    const off = Number(this.off.value);
    const on = Number(this.on.value);
    const outgoing = this.players.off.find((p) => p.squad === off);
    const incoming = this.players.on.find((p) => p.squad === on);
    if (!outgoing || !incoming) {
      return;
    }
    this.onQueue(
      { off, on },
      `Substitution: ${outgoing.name} off, ${incoming.name} on`
    );
  }
}
