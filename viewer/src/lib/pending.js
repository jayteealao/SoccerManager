// The pending-change list: one chip per change the manager queued, mirroring the engine's
// queue. Pure: no DOM.
//
// The page never predicts an outcome. A chip starts at the engine's acknowledgement, and it
// moves only on the engine's verdict event. The engine runs a few seconds ahead of playback,
// so a verdict usually arrives before the pitch reaches it; the chip shows the verdict only
// once the rendered tick reaches the verdict's tick, the frame in which the manager sees the
// dead ball. Every chip carries its state word; colour never carries a state alone.
//
// A chip the engine has not settled can be withdrawn: the chip leaves on the engine's
// acknowledgement of `cancel-change`, and stays when the engine refuses the withdrawal because
// the change already applied or was refused. "Applies now" comes from the engine's
// `change-state` message, sent when the stoppage that takes the change opens.

import { signal } from './signal.js';

/// The words the engine's `ChangeState::label()` gives each state.
export const STATE_WORDS = Object.freeze({
  queued: 'Queued',
  'applies-now': 'Applies now',
  applied: 'Applied',
  rejected: 'Rejected',
});

/// Rendered ticks an Applied chip stays before it leaves: three seconds of match time.
export const APPLIED_HOLD_TICKS = 150;

export function createPendingList({ homeTeamId = null } = {}) {
  /// Chips in the order the manager queued them.
  const chips = [];
  let local = 0;

  const find = (queueId) => chips.find((c) => c.queue_id === queueId);

  /// The chip as the manager should see it at `renderedTick`.
  const view = (chip, renderedTick) => {
    const reached = chip.verdictTick !== null && renderedTick >= chip.verdictTick;
    const state = reached ? chip.verdict : chip.state;
    return {
      queue_id: chip.queue_id,
      kind: chip.kind,
      label: chip.label,
      state,
      word: STATE_WORDS[state],
      reason: state === 'rejected' ? chip.reason : null,
      queued_tick: chip.queued_tick,
      applied_tick: chip.applied_tick,
      verdict_tick: chip.verdictTick,
      detail: chip.detail,
      // Edit and Cancel act on a change no stoppage has taken yet.
      editable: state === 'queued',
    };
  };

  return {
    /// The engine acknowledged a change. `ack` is the acknowledgement message.
    queued(ack, label, kind, detail = null) {
      const chip = {
        queue_id: ack['change.queue_id'],
        kind,
        label,
        detail,
        state: 'queued',
        reason: null,
        queued_tick: ack['change.queued_tick'] ?? null,
        applied_tick: null,
        verdict: null,
        verdictTick: null,
        dismissed: false,
        cancelled: false,
      };
      chips.push(chip);
      return chip.queue_id;
    },

    /// The socket refused a change before it was queued. The chip is Rejected at once, with
    /// the socket's reason, and stays until the manager dismisses it.
    rejectedAtQueue(reject, label, kind) {
      local += 1;
      const chip = {
        queue_id: `local-${local}`,
        kind,
        label,
        detail: null,
        state: 'rejected',
        reason: reject.reason,
        queued_tick: null,
        applied_tick: null,
        verdict: null,
        verdictTick: null,
        dismissed: false,
        cancelled: false,
      };
      chips.push(chip);
      return chip.queue_id;
    },

    /// A `change-state` message: the stoppage that takes the change has opened. Returns
    /// `true` when it moved one of this list's chips.
    onChangeState(note) {
      if (note.state !== 'applies-now') {
        return false;
      }
      const chip = find(note['change.queue_id']);
      if (!chip || chip.cancelled || chip.verdict !== null) {
        return false;
      }
      chip.state = 'applies-now';
      return true;
    },

    /// The engine acknowledged the withdrawal of `queueId`: the chip leaves for good.
    cancelled(queueId) {
      const chip = find(queueId);
      if (chip) {
        chip.cancelled = true;
        chip.dismissed = true;
      }
    },

    /// The chip `queueId` as the manager sees it at `renderedTick`, or null.
    get(queueId, renderedTick) {
      const chip = find(queueId);
      return chip ? view(chip, renderedTick) : null;
    },

    /// A `tactics-change` event. Returns `true` when it resolved one of this list's chips.
    /// The socket's own "queued" rows, the other club's changes, and identifiers this list
    /// never saw change nothing; an unknown identifier on the home club's verdict is logged.
    onChangeEvent(event) {
      const state = event['change.state'];
      if (state !== 'applied' && state !== 'rejected' && state !== 'applies-now') {
        return false;
      }
      const team = event['team.id'];
      if (homeTeamId !== null && team !== undefined && team !== null && team !== homeTeamId) {
        return false;
      }
      const chip = find(event['change.queue_id']);
      if (!chip) {
        signal('viewer.change_unknown', {
          'change.queue_id': event['change.queue_id'] ?? null,
          'change.state': state,
          tick: event.tick,
        });
        return false;
      }
      if (state === 'applies-now') {
        chip.state = 'applies-now';
        return true;
      }
      chip.verdict = state;
      chip.verdictTick = event.tick;
      chip.reason = event['change.rejected_reason'] ?? null;
      chip.applied_tick = state === 'applied' ? (event['change.applied_tick'] ?? event.tick) : null;
      return true;
    },

    /// The manager dismissed a Rejected chip.
    dismiss(queueId) {
      const chip = find(queueId);
      if (chip) {
        chip.dismissed = true;
      }
    },

    /// The chips to show at `renderedTick`. An Applied chip leaves after its hold; a
    /// Rejected chip stays until dismissed.
    chips(renderedTick) {
      return chips
        .filter((chip) => !chip.dismissed)
        .map((chip) => view(chip, renderedTick))
        .filter(
          (c) => !(c.state === 'applied' && renderedTick >= c.verdict_tick + APPLIED_HOLD_TICKS)
        );
    },

    /// Every chip ever queued, as the test hook reads it, at `renderedTick`.
    all(renderedTick) {
      return chips.map((chip) => ({
        ...view(chip, renderedTick),
        dismissed: chip.dismissed,
        cancelled: chip.cancelled,
      }));
    },

    /// The changes the engine has applied, in the order it applied them.
    applied() {
      return chips
        .filter((c) => c.verdict === 'applied')
        .sort((a, b) => a.verdictTick - b.verdictTick);
    },
  };
}
