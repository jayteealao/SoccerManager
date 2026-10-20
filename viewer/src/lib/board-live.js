// The tactics board during play: what the board shows, folded from the tactics in force at
// kick-off and every change since; the live strip's chips; and the change the engine did not
// take. Pure: no DOM.
//
// A level or game-state change applies on the tick it is queued, and the players take it up
// over the tactics file's take-up time; a tactics change (the formation, a role, or a level
// from the Shapes tab) waits for a stoppage. The board shows a level or game-state change
// only once the engine applied it, so it never shows a level the engine did not take. A
// tactics change shows from its acknowledgement, as the Shapes tab always has.

import { levelWord, primitiveWords } from './board-words.js';
import { fromWire, levelNames, merge, primitivesOf, withLevel } from './primitives.js';
import { TICKS_PER_SECOND } from './schedule.js';
import { minuteOf } from './skip.js';
import { applyPatch, copyTactics } from './tactics-panel.js';

/// The tactics, the levels and the game state the board shows: `start` (`{ tactics, levels,
/// gameState }` at kick-off) with every chip of `chips` (the pending list's `all`) applied in
/// queue order.
export function fold(schema, start, chips, lineup) {
  let tactics = copyTactics(start.tactics);
  let levels = merge(schema, tactics, start.levels);
  let gameState = start.gameState;
  for (const chip of chips) {
    if (chip.cancelled || !chip.detail) {
      continue;
    }
    if (chip.kind === 'tactics' && chip.state !== 'rejected') {
      tactics = applyPatch(tactics, chip.detail, lineup, schema);
      levels = merge(schema, tactics, levels);
    } else if (chip.kind === 'instructions' && chip.engine_verdict === 'applied') {
      for (const wire of chip.detail.levels ?? []) {
        const at = fromWire(schema, wire);
        if (at) {
          ({ tactics, levels } = withLevel(schema, tactics, levels, at[0], at[1]));
        }
      }
    } else if (chip.kind === 'game-state' && chip.engine_verdict === 'applied') {
      gameState = chip.detail.game_state ?? gameState;
    }
  }
  return { tactics, levels, gameState };
}

/// A level change's chip words: "Pressing intensity → High".
export function levelLabel(schema, index, level) {
  const p = primitivesOf(schema)[index];
  const names = levelNames(schema, p);
  return `${primitiveWords(p?.name, names).full} → ${levelWord(p?.name, level, names)}`;
}

/// The take-up time in ticks.
export function takeUpTicks(schema) {
  return Math.round((schema?.game_state?.take_up_s ?? 30) * TICKS_PER_SECOND);
}

/// The live strip at `renderedTick`: the newest changes first, at most `limit`. A level or
/// game-state change the engine applied is "players adjusting" for the take-up time after it
/// applied, then "in effect since" its minute; a formation change still waiting is "at the
/// next stoppage" and can be cancelled.
export function liveChips(schema, chips, renderedTick, limit = 4) {
  const take = takeUpTicks(schema);
  const out = [];
  for (const chip of [...chips].reverse()) {
    if (chip.cancelled || chip.dismissed) {
      continue;
    }
    if ((chip.kind === 'instructions' || chip.kind === 'game-state') && chip.state === 'applied') {
      const at = chip.applied_tick ?? chip.verdict_tick ?? 0;
      const left = at + take - renderedTick;
      out.push(
        left > 0
          ? {
              queue_id: chip.queue_id,
              tone: 'run',
              label: chip.label,
              note: `players adjusting · about ${Math.max(1, Math.round(left / TICKS_PER_SECOND))} s`,
            }
          : { queue_id: chip.queue_id, tone: 'ok', label: chip.label, note: `in effect since ${minuteOf(at) + 1}’` }
      );
    } else if (
      chip.kind === 'tactics' &&
      chip.detail?.formation !== undefined &&
      chip.detail?.formation !== null &&
      (chip.state === 'queued' || chip.state === 'applies-now')
    ) {
      const name = schema?.formations?.[chip.detail.formation]?.name ?? '';
      out.push({
        queue_id: chip.queue_id,
        tone: 'wa',
        label: `Formation ${name}`,
        note: 'at the next stoppage',
        cancellable: chip.editable,
      });
    }
    if (out.length >= limit) {
      break;
    }
  }
  return out;
}

/// The newest level or game-state change the engine did not take that is still on show, as
/// the Not applied banner names it: the change, the primitive it was about (or null), the
/// kept level and the code. Null when there is none.
export function refusal(schema, chips, levels) {
  const chip = [...chips]
    .reverse()
    .find(
      (c) =>
        !c.dismissed &&
        (c.kind === 'instructions' || c.kind === 'game-state') &&
        (c.state === 'rejected' || c.engine_verdict === 'rejected')
    );
  if (!chip) {
    return null;
  }
  const wire = chip.detail?.levels?.[0];
  const at = wire ? fromWire(schema, wire) : null;
  const index = at ? at[0] : primitivesOf(schema).findIndex((p) => p.name === wire?.primitive);
  const p = index >= 0 ? primitivesOf(schema)[index] : null;
  const kept = p ? levelWord(p.name, levels[index], levelNames(schema, p)) : null;
  return {
    queue_id: chip.queue_id,
    kind: chip.kind,
    label: chip.label,
    primitive: p?.name ?? null,
    kept,
    code: chip.reason ?? '',
    detail: chip.detail,
  };
}
