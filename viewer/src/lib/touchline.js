// The Touchline page: the pure half. Player state, the other team's bench, the assistant's
// picks and the strip facts, each brought to the rendered tick from the match state and the
// hello. `TouchlineScreen.svelte` and its components draw these rows.
//
// The parts the engine has no model for yet (sprint reserve, mind, taken up, the analysts'
// reads, the gain and cost of a pick, shouts, game-state plans, what each change did) are
// LATER stubs on the page and have no function here.

import { KIND } from './match-state.js';
import { surname } from './prematch.js';
import { instructionNames, words } from './tactics-panel.js';

/// The assistant's reason, in words, per `ai.decision` code.
export const PICK_REASONS = Object.freeze({
  'sub-injury': 'injured player',
  'sub-keeper': 'outfield player in goal',
  'sub-fatigue': 'tired player',
  'mentality-up-trailing': 'trailing',
  'mentality-down-leading': 'leading',
});

/// The words shown when the assistant has no pick open.
export const NO_PICK = 'No pick open. The assistant checks every 30 seconds of play.';

/// The minute a card event names, as the feed writes it: 44' or 45+2'.
function minuteText(event) {
  const added = event['minute.added'];
  return added ? `${event.minute}+${added}'` : `${event.minute}'`;
}

/// The player-state rows of the home eleven at the rendered tick. `rows` are the home rows of
/// `lineupModel`; `entries` the released events. The slow reserve is the engine's one energy
/// value, as a whole percentage with its band word; the card is the last card shown to the
/// player, with its minute and its word.
export function playerStateRows(rows, entries) {
  const cards = new Map();
  for (const e of entries ?? []) {
    if (e['event.type'] === KIND.card && e['player.id']) {
      cards.set(e['player.id'], { kind: e['card.kind'], minute: minuteText(e) });
    }
  }
  return rows.map((row) => {
    const card = cards.get(row.id) ?? null;
    return {
      wire: row.wire,
      shirt: row.shirt,
      name: row.name,
      surname: surname(row.name),
      reserve: Math.round((row.energy ?? 1) * 100),
      band: row.band,
      token: row.token,
      condition: row.condition,
      card: card
        ? {
            kind: card.kind === 'yellow' ? 'yellow' : 'red',
            minute: card.minute,
            word: card.kind === 'yellow' ? 'Yellow card' : card.kind === 'second-yellow' ? 'Second yellow, sent off' : 'Red card, sent off',
          }
        : null,
      sentOff: row.sentOff,
      injured: row.injured,
    };
  });
}

/// The other team's bench at the rendered tick: who is left to come on, and what the other
/// manager has used. `bench` is the away list of `benchModel`; `awayId` its `team.id`.
export function benchRows(bench, state, awayId, limit) {
  const used = (state?.substitutions ?? []).filter((s) => s.team === awayId).length;
  const windows = state?.windowsUsed ? state.windowsUsed[1] : null;
  return {
    players: bench.map((p) => ({
      id: p['player.id'],
      shirt: p['player.shirt'],
      name: p['player.name'],
      position: p['player.position'],
    })),
    used,
    limit,
    text: `${used} of ${limit}${windows === null ? '' : ` · ${windows} ${windows === 1 ? 'window' : 'windows'}`}`,
  };
}

/// A key that names one pick, so an accepted pick is known again in the next advice.
export function pickKey(pick) {
  return JSON.stringify([pick.code, pick.kind, pick.off ?? null, pick.on ?? null, pick.patch ?? null]);
}

/// The `queue-change` detail that queues a pick, as the picker and the tactics controls send
/// theirs.
export function pickDetail(pick) {
  return pick.kind === 'substitution' ? { off: pick.off, on: pick.on } : { patch: pick.patch };
}

/// The words of a tactics pick's patch: the mentality it moves to, then each instruction level.
function patchWords(patch, schema) {
  const parts = [];
  if (patch?.mentality !== undefined && patch.mentality !== null) {
    parts.push(`Mentality ${words(schema?.mentalities?.[patch.mentality]?.name ?? '')}`);
  }
  const names = instructionNames(schema);
  (patch?.instructions ?? []).forEach((level, i) => {
    if (level !== null && level !== undefined) {
      const name = names[i];
      const levelName = schema?.instructions?.[name]?.levels?.[level]?.name ?? level;
      parts.push(`${words(name)} ${String(words(levelName)).toLowerCase()}`);
    }
  });
  return parts.join(', ') || 'Tactics change';
}

/// The assistant's picks as the page lists them: each with its heading, its reason word and
/// the label its queued change carries. `squad` is the hello's home squad; `accepted` the keys
/// of the picks the manager accepted.
export function proposalRows(picks, squad, schema, accepted = new Set()) {
  return (picks ?? []).map((pick, i) => {
    const reason = PICK_REASONS[pick.code] ?? pick.code;
    let what;
    let label;
    if (pick.kind === 'substitution') {
      const off = squad?.[pick.off]?.['player.name'] ?? `Player ${pick.off}`;
      const on = squad?.[pick.on]?.['player.name'] ?? `Player ${pick.on}`;
      what = `${surname(off)} → ${surname(on)}`;
      label = `Substitution: ${off} off, ${on} on`;
    } else {
      what = patchWords(pick.patch, schema);
      label = what;
    }
    const key = pickKey(pick);
    return {
      key,
      n: i + 1,
      pick,
      what,
      reason,
      text: `${what} · ${reason}`,
      label,
      accepted: accepted.has(key),
    };
  });
}

/// The Touchline strip. `play` is `{ stopped, nextIn }`: whether a stoppage mark is at the
/// rendered tick or just before it, and the seconds to the next known stoppage (or null).
/// The concussion cell is a LATER stub on the page and has no fact here.
export function touchlineFacts({ score, clock, play, subsUsed, windowsUsed, limit, windows, mentality }) {
  const next =
    play.nextIn === null
      ? 'Queued changes apply at the next stoppage'
      : `Next stoppage in ${clockWords(play.nextIn)} · queued changes apply there`;
  return [
    { value: play.stopped ? 'Play stopped' : 'Ball in play', label: next },
    { value: `${score[0]} – ${score[1]} · ${clock}`, label: 'Score and clock', num: true },
    { value: `${subsUsed} of ${limit}`, label: 'Substitutes used', num: true },
    { value: `${windowsUsed} of ${windows}`, label: 'Windows used · half-time is free', num: true },
    { value: mentality, label: 'Mentality' },
  ];
}

function clockWords(seconds) {
  const s = Math.max(0, Math.round(seconds));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}
