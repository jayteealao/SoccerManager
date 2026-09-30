// The dugout: everything the manager sends from the viewer, as one class whose `$state`
// fields the Tactics screen reads. Before kick-off it holds the lineup and the draft tactics
// and sends them with `set-lineup`; during play it queues each tactics change and
// substitution with `queue-change`, and withdraws one with `cancel-change`. Every chip starts
// at the engine's acknowledgement and moves only on the engine's word. It ports the former
// page's `Dugout` (`web/main.mjs`) field for field, so the engine receives the same messages.
//
// Edit is a withdrawal and a new change: the dugout sends `cancel-change`, and on its
// acknowledgement queues the edited change. The engine sees two commands in order.
//
// The assistant's picks arrive as `advice` messages. Each shows when playback reaches its
// tick; Accept queues the pick like any other change, and nothing is sent until then.

import { LineupEditor } from './lineup-editor.js';
import { benchModel, lineupModel } from './lineups.js';
import { createPendingList } from './pending.js';
import { signal } from './signal.js';
import { pickerBlock, pickerPlayers } from './substitution-picker.js';
import { benchRows, pickDetail, pickKey, playerStateRows, proposalRows } from './touchline.js';
import {
  applyPatch,
  copyTactics,
  detailFor,
  labelFor,
  prematchPatch,
  roleForSlot,
  tacticsOf,
} from './tactics-panel.js';

/// A hello roster entry for squad player `index`, as the lineups read it.
function rosterEntry(player, index) {
  return {
    'player.id': player['player.id'],
    'player.name': player['player.name'],
    'player.shirt': player['player.shirt'],
    'player.position': player['player.position'],
    'player.squad_index': index,
  };
}

export class Dugout {
  /// `waiting`, `pre-match`, `kicking-off`, `live`, or `stored` (a recording, or an engine
  /// that takes no lineup).
  phase = $state('waiting');
  /// Bumped on every change a component must redraw for.
  version = $state(0);
  /// The tactics the controls show: the draft before kick-off; during play what the engine
  /// applied, then every change still waiting.
  tactics = $state.raw(null);
  /// The chips at the rendered tick.
  chips = $state.raw([]);
  /// The picker: who may come off and on, the count left, and why it cannot queue.
  picker = $state.raw({ off: [], on: [], left: 0, limit: 0, block: 'Changes need a live match.' });
  /// The home eleven at the rendered tick, as the roles table lists it.
  homeRows = $state.raw([]);
  /// The change being edited: `{ queue_id, kind, label, detail }`, or null.
  editing = $state.raw(null);
  /// The engine's last refusal of a withdrawal, in its words, or null.
  cancelRefused = $state(null);
  over = $state(false);
  /// The assistant's picks at the rendered tick: the newest `advice` message at or before it,
  /// or null.
  advice = $state.raw(null);
  /// Bumped when a pick is accepted or refused, so the proposals redraw.
  acceptedVersion = $state(0);
  /// The Touchline's player state (the home eleven's energy and cards) and the other bench,
  /// at the rendered tick.
  playerState = $state.raw([]);
  otherBench = $state.raw({ players: [], used: 0, limit: 0, text: '' });

  /// `send(command)` sends one client command and returns `false` when no socket takes it;
  /// `onStart(message, patch)` runs when the engine accepts the lineup.
  constructor({ send = () => false, onStart = () => {} } = {}) {
    this.send = send;
    this.onStart = onStart;
    this.pending = createPendingList();
    this.requests = [];
    this.editor = null;
    this.schema = null;
    this.squad = [];
    this.setup = null;
    this.homeId = null;
    this.teamIds = [null, null];
    this.rosters = [[], []];
    this.limit = 0;
    this.confirmed = null;
    this.lineup = null;
    this.chipKey = '';
    this.rowKey = '';
    this.tacticsKey = '';
    this.renderedTick = 0;
    this.adviceList = [];
    this.accepted = new Set();
    this.stateKey = '';
    this.benchKey = '';
  }

  /// The hello: the squad, the computer manager's setup, and the tactics file. A stored match
  /// or an engine that sends no setup takes no change.
  begin(hello, { stored = false } = {}) {
    const home = hello.teams[0];
    this.homeId = home['team.id'];
    this.teamIds = hello.teams.map((t) => t['team.id']);
    this.rosters = hello.teams.map((t) => t.roster ?? []);
    this.pending = createPendingList({ homeTeamId: this.homeId });
    this.requests = [];
    this.schema = hello.tactics;
    this.squad = home.squad ?? [];
    this.setup = home.setup ?? null;
    this.limit = hello.substitutions?.limit ?? 0;
    this.lineup = null;
    this.editing = null;
    this.cancelRefused = null;
    this.chips = [];
    this.chipKey = '';
    this.rowKey = '';
    this.adviceList = [];
    this.advice = null;
    this.accepted = new Set();
    this.stateKey = '';
    this.benchKey = '';
    this.playerState = [];
    this.otherBench = { players: [], used: 0, limit: this.limit, text: '' };
    if (stored || !this.setup || !this.schema || !Array.isArray(this.schema.formations)) {
      this.editor = null;
      this.over = true;
      this.phase = 'stored';
      this.base = null;
      this.tactics = null;
      this.refreshPicker();
      this.bump();
      return;
    }
    this.over = false;
    this.phase = 'pre-match';
    this.base = tacticsOf(this.setup);
    this.tactics = tacticsOf(this.setup);
    this.editor = new LineupEditor({
      schema: this.schema,
      squad: this.squad,
      setup: this.setup,
      benchSize: this.schema.ai?.bench_size ?? 7,
    });
    this.refreshPicker();
    this.bump();
  }

  bump() {
    this.version += 1;
  }

  /// `true` while the lineup and the draft tactics can change.
  get preMatch() {
    return this.phase === 'pre-match';
  }

  /// `true` while changes can be queued.
  get live() {
    return this.phase === 'live' && !this.over;
  }

  /// Slot `n`'s role index in the tactics shown.
  roleOf(n) {
    return this.tactics?.roles[n]?.role ?? 0;
  }

  // ---- Before kick-off ----------------------------------------------------------------------

  /// Runs one editor action and redraws. Every action is refused once kick-off is asked.
  lineupAction(fn) {
    if (!this.preMatch || !this.editor) {
      return;
    }
    fn(this.editor);
    this.bump();
  }

  /// The formation the tactic bar picked, before kick-off. A role that no longer suits its
  /// slot takes the first role that does.
  setFormation(formation) {
    if (!this.preMatch || !this.editor) {
      return;
    }
    const slots = this.schema.formations[formation].slots;
    this.editor.formation = formation;
    this.tactics = {
      ...this.tactics,
      formation,
      roles: this.tactics.roles.map((r, slot) => ({
        role: roleForSlot(this.schema, slots[slot].position, r.role),
        duty: r.duty,
      })),
    };
    this.bump();
  }

  /// One tactics edit: `{ mentality }`, `{ instruction, level }`, or
  /// `{ role: { squad, role, duty } }` with `slot`. Before kick-off it changes the draft;
  /// during play it is queued (or, while a tactics change is edited, replaces it).
  editTactics(edit, slot = null) {
    if (this.preMatch) {
      const t = copyTactics(this.tactics);
      if (edit.mentality !== undefined) {
        t.mentality = edit.mentality;
      }
      if (edit.instruction !== undefined) {
        t.instructions[edit.instruction] = edit.level;
      }
      if (edit.role !== undefined && slot !== null) {
        t.roles[slot] = { role: edit.role.role, duty: edit.role.duty };
      }
      this.tactics = t;
      this.bump();
      return;
    }
    if (!this.live) {
      return;
    }
    if (edit.role !== undefined && (edit.role.squad === null || edit.role.squad === undefined)) {
      return;
    }
    const names = (squad) =>
      this.homeRows.find((r) => r.squad === squad)?.name || `Player ${squad}`;
    const label = labelFor(edit, this.schema, names);
    const detail = detailFor(edit);
    if (this.editing?.kind === 'tactics') {
      this.edit(this.editing.queue_id, 'tactics', detail, label);
    } else {
      this.queue('tactics', detail, label);
    }
  }

  /// Sends the lineup and, only when the draft differs from the computer manager's pick, the
  /// pre-match tactics; the match starts on the acknowledgement.
  kickOff() {
    if (!this.preMatch || !this.editor || !this.editor.ready) {
      return false;
    }
    const message = this.editor.message();
    const draft = { ...copyTactics(this.tactics), formation: this.editor.formation };
    const patch = prematchPatch(this.base, draft, message.lineup);
    this.phase = 'kicking-off';
    this.bump();
    const command = { type: 'set-lineup', ...message };
    if (patch) {
      command.patch = patch;
    }
    const sent = this.request(
      command,
      () => this.startMatch(message, patch),
      (reject) => {
        this.phase = 'pre-match';
        this.editor.refused(reject.reason);
        this.bump();
      }
    );
    if (!sent) {
      this.phase = 'pre-match';
      this.editor.refused('The engine is not connected.');
      this.bump();
    }
    return sent;
  }

  startMatch(message, patch) {
    this.phase = 'live';
    this.lineup = message.lineup;
    this.confirmed = patch
      ? applyPatch(this.base, patch, message.lineup, this.schema)
      : copyTactics(this.base);
    this.rosters = [
      [...message.lineup, ...message.bench].map((i) => rosterEntry(this.squad[i], i)),
      this.rosters[1],
    ];
    this.tactics = copyTactics(this.confirmed);
    this.rowKey = '';
    this.onStart(message, patch);
    signal('viewer.kick_off', {
      lineup: message.lineup,
      bench: message.bench,
      patch: patch ?? null,
    });
    this.bump();
  }

  // ---- During play --------------------------------------------------------------------------

  /// Sends one command and remembers who waits for its answer. Answers come back in the
  /// order the commands were read, one per command.
  request(command, onAck, onReject) {
    if (!this.send(command)) {
      return false;
    }
    this.requests.push({ command: command.type, onAck, onReject });
    return true;
  }

  /// An `ack` or a `reject` from the engine.
  answer(message) {
    const i = this.requests.findIndex((r) => r.command === message.command);
    if (i < 0) {
      return;
    }
    const [waiting] = this.requests.splice(i, 1);
    if (message.type === 'ack') {
      waiting.onAck(message);
    } else {
      waiting.onReject(message);
    }
    this.refreshChips();
  }

  /// Queues one change. The chip appears on the engine's acknowledgement; a refusal runs
  /// `onRefused`.
  queue(kind, detail, label, onRefused = () => {}) {
    if (!this.live) {
      return false;
    }
    const sent = this.request(
      { type: 'queue-change', 'change.kind': kind, detail },
      (ack) => this.pending.queued(ack, label, kind, detail.patch ?? detail),
      (reject) => {
        this.pending.rejectedAtQueue(reject, label, kind);
        onRefused(reject);
      }
    );
    if (!sent) {
      this.pending.rejectedAtQueue({ reason: 'The engine is not connected.' }, label, kind);
      onRefused(null);
    }
    this.refreshChips();
    return sent;
  }

  /// A substitution from the picker: squad `off` for squad `on`. With `formation`, a new
  /// shape queues with it: the substitution first, then the shape, so the engine, which
  /// applies substitutions before tactics at one stoppage, takes them in that order.
  substitute(off, on, formation = null) {
    const outgoing = this.picker.off.find((p) => p.squad === off);
    const incoming = this.picker.on.find((p) => p.squad === on);
    if (!outgoing || !incoming) {
      return;
    }
    const label = `Substitution: ${outgoing.name} off, ${incoming.name} on`;
    if (this.editing?.kind === 'substitution') {
      this.edit(this.editing.queue_id, 'substitution', { off, on }, label);
    } else {
      this.queue('substitution', { off, on }, label);
    }
    const current = this.tactics?.formation;
    if (formation !== null && formation !== undefined && formation !== current && this.schema) {
      const from = this.schema.formations[current]?.name ?? '';
      const to = this.schema.formations[formation]?.name ?? '';
      this.queue('tactics', { patch: { formation } }, `Shape: ${from} → ${to}`);
    }
  }

  // ---- The assistant's picks ----------------------------------------------------------------

  /// An `advice` message. It shows when playback reaches its tick.
  onAdvice(message) {
    const at = this.adviceList.findIndex((a) => a.tick > message.tick);
    if (at < 0) {
      this.adviceList.push(message);
    } else {
      this.adviceList.splice(at, 0, message);
    }
    this.refreshAdvice();
  }

  refreshAdvice() {
    let current = null;
    for (const a of this.adviceList) {
      if (a.tick > this.renderedTick) {
        break;
      }
      current = a;
    }
    if (current !== this.advice) {
      this.advice = current;
    }
  }

  /// The picks at the rendered tick as the Touchline lists them.
  proposals() {
    this.acceptedVersion;
    return proposalRows(this.advice?.picks ?? [], this.squad, this.schema, this.accepted);
  }

  /// Accepts a pick: its change is queued like any other, with the pick's own words, and the
  /// pick reads Queued. A pick the engine refuses can be accepted again.
  accept(pick) {
    const key = pickKey(pick);
    if (!this.live || this.accepted.has(key)) {
      return false;
    }
    const row = proposalRows([pick], this.squad, this.schema)[0];
    this.accepted.add(key);
    this.acceptedVersion += 1;
    return this.queue(pick.kind, pickDetail(pick), row.label, () => {
      this.accepted.delete(key);
      this.acceptedVersion += 1;
    });
  }

  /// Withdraws a queued change. The chip leaves on the acknowledgement; a refusal keeps it
  /// and shows the engine's reason.
  cancel(queueId, then = () => {}) {
    const chip = this.pending.get(queueId, this.renderedTick);
    if (!chip || !chip.editable || !this.live) {
      return false;
    }
    this.cancelRefused = null;
    return this.request(
      { type: 'cancel-change', 'change.queue_id': queueId },
      () => {
        this.pending.cancelled(queueId);
        if (this.editing?.queue_id === queueId) {
          this.editing = null;
        }
        then();
      },
      (reject) => {
        this.cancelRefused = reject.reason;
        if (this.editing?.queue_id === queueId) {
          this.editing = null;
        }
      }
    );
  }

  /// Replaces a queued change: the withdrawal first, then, on its acknowledgement, the
  /// edited change.
  edit(queueId, kind, detail, label) {
    this.cancel(queueId, () => this.queue(kind, detail, label));
  }

  /// Starts or stops editing a queued change. While a change is edited, the next change of its
  /// kind replaces it.
  startEdit(queueId) {
    const chip = this.pending.get(queueId, this.renderedTick);
    if (!chip || !chip.editable || !this.live) {
      return;
    }
    this.editing = { queue_id: chip.queue_id, kind: chip.kind, label: chip.label, detail: chip.detail };
  }

  stopEdit() {
    this.editing = null;
  }

  dismiss(queueId) {
    this.pending.dismiss(queueId);
    this.refreshChips();
  }

  /// A `tactics-change` event or a `change-state` message.
  onChangeEvent(event) {
    this.pending.onChangeEvent(event);
  }

  onChangeState(note) {
    this.pending.onChangeState(note);
    this.refreshChips();
  }

  // ---- Once per rendered tick ---------------------------------------------------------------

  /// Brings the picker, the roles table and the chips to `tick`, at the view `state`.
  update(tick, state) {
    this.renderedTick = tick;
    this.refreshChips();
    this.refreshAdvice();
    if (this.phase !== 'live') {
      return;
    }
    const both = lineupModel(this.rosters, this.teamIds, state);
    const rows = both[0] ?? [];
    const benches = benchModel(this.rosters, state);
    const bench = benches[0] ?? [];
    this.refreshTouchline(rows, benches[1] ?? [], state);
    const used = state.substitutions.filter((s) => s.team === this.homeId).length;
    const key =
      rows.map((r) => `${r.squad}${r.sentOff ? 'x' : ''}`).join(',') +
      `|${bench.map((p) => p['player.squad_index']).join(',')}|${used}|${state.fullTime}`;
    if (key !== this.rowKey) {
      this.rowKey = key;
      this.homeRows = rows;
      if (state.fullTime && !this.over) {
        this.over = true;
        this.editing = null;
      }
      this.refreshPicker(rows, bench, used);
    }
    const tactics = this.tacticsView();
    const tacticsKey = JSON.stringify(tactics);
    if (tacticsKey !== this.tacticsKey) {
      this.tacticsKey = tacticsKey;
      this.tactics = tactics;
    }
  }

  /// The Touchline's player state and the other bench, redrawn only when a figure it shows
  /// changes.
  refreshTouchline(rows, otherBench, state) {
    const cards = state.entries ? state.entries.filter((e) => e['event.type'] === 'card').length : 0;
    const key = rows.map((r) => `${r.wire}:${Math.round(r.energy * 100)}:${r.condition}`).join(',') + `|${cards}`;
    if (key !== this.stateKey) {
      this.stateKey = key;
      this.playerState = playerStateRows(rows, state.entries);
    }
    const out = benchRows(otherBench, state, this.teamIds[1], this.limit);
    const benchKey = `${out.players.map((p) => p.id).join(',')}|${out.text}`;
    if (benchKey !== this.benchKey) {
      this.benchKey = benchKey;
      this.otherBench = out;
    }
  }

  refreshPicker(rows = [], bench = [], used = 0) {
    const players = pickerPlayers(rows, bench);
    const left = Math.max(0, this.limit - used);
    this.picker = {
      ...players,
      left,
      limit: this.limit,
      block: pickerBlock({ live: this.live, left, ...players }),
    };
  }

  refreshChips() {
    const chips = this.pending.chips(this.renderedTick);
    const key = chips.map((c) => `${c.queue_id}:${c.state}`).join('|');
    if (key !== this.chipKey) {
      this.chipKey = key;
      this.chips = chips;
      if (this.editing && !chips.some((c) => c.queue_id === this.editing.queue_id && c.editable)) {
        this.editing = null;
      }
    }
  }

  /// The tactics the controls show: what the engine applied, then every change still
  /// waiting. A refused change drops out, so its control returns to the value in force.
  tacticsView() {
    const lineup = this.homeRows.map((r) => r.squad);
    let tactics = copyTactics(this.confirmed);
    for (const chip of this.pending.all(Number.MAX_SAFE_INTEGER)) {
      if (chip.kind === 'tactics' && chip.state !== 'rejected' && !chip.cancelled && chip.detail) {
        tactics = applyPatch(tactics, chip.detail, lineup, this.schema);
      }
    }
    return tactics;
  }

  // ---- The read-only views the test hook returns ---------------------------------------------

  lineupView() {
    const editor = this.editor ? this.editor.snapshot() : null;
    return {
      phase: this.phase,
      ...(editor ?? {}),
      kickOffDisabled: !this.editor || !this.editor.ready || this.phase !== 'pre-match',
      kicked_off: this.lineup ?? null,
    };
  }
}
