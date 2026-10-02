// The other grounds of the matchday, on the player's clock. Pure, with no DOM and no socket:
// the session keeps the round, every ground event that arrived and how far each ground has
// played; these functions say what the list shows at the tick the page draws.
//
// Every other match kicks off with the player's match, so an event's tick is also its moment
// on the player's clock. The engine sends an event only once its own tick of the player's
// match reaches it; the page shows it only once the tick it draws reaches it. So a pause
// holds the list, a speed change moves it with the clock, a rewind hides what came later,
// and a skip holds the list at the skip point until the report.

import { TICKS_PER_MINUTE, minuteOf } from './skip.js';

/// How long a new goal stays outlined: 8 seconds of the player's clock.
export const HIGHLIGHT_TICKS = 400;

/// The matchday's number while the generated round is the only matchday.
export const MATCHDAY_NUMBER = 1;

/// Minutes in one period of extra time.
const EXTRA_TIME_MINUTES = 15;

/// The skip states in which the list stays at the skip point.
const HELD = new Set(['deciding', 'playing', 'storing']);

/// A new matchday from the `matchday` message: its fixtures, no events yet, and every ground
/// at kick-off.
export function newMatchday(message) {
  const fixtures = (message.fixtures ?? []).map((f) => ({ fixture: f.fixture, home: f.home, away: f.away }));
  return {
    round: message.round ?? MATCHDAY_NUMBER,
    fixtures,
    events: fixtures.map(() => []),
    reached: fixtures.map(() => 0),
  };
}

/// `matchday` with `event` added, or the same object when the event is a repeat a catch-up
/// sent again (the same fixture, tick and kind) or names no fixture of the round.
/// `renderedTick` is the tick the page drew when the event arrived: a late event shows from
/// there. `seekCount` is how many seeks the page had made by then: only a later seek can
/// take a late event's outline away.
export function addEvent(matchday, event, renderedTick, seekCount = 0) {
  const list = matchday.events[event.fixture];
  if (!list || list.some((e) => e.tick === event.tick && e.kind === event.kind)) {
    return matchday;
  }
  const entry = {
    ...event,
    shownAt: event.late ? Math.max(event.tick, renderedTick) : event.tick,
    seeksBefore: event.late ? seekCount : 0,
  };
  const next = [...list, entry].sort((a, b) => a.tick - b.tick);
  const events = matchday.events.slice();
  events[event.fixture] = next;
  return { ...matchday, events };
}

/// `matchday` with the ticks each ground has reached, from a `ground-progress` message.
export function addProgress(matchday, message) {
  const reached = matchday.fixtures.map((_, i) => Math.max(matchday.reached[i] ?? 0, message.reached?.[i] ?? 0));
  return { ...matchday, reached };
}

/// `23'`, or `45+2'` in added time, as the event's own clock gave it.
export function eventStamp(event) {
  return event.added ? `${event.minute}+${event.added}'` : `${event.minute}'`;
}

/// The minute a ground shows at `tick` on its own clock: its periods start at the ticks of
/// its second-half and extra-time events, and each lasts its regulation minutes before added
/// time. `total` is the regulation length of the match (90 for a full match).
export function minuteLabel(tick, periods, total = 90) {
  const half = Math.max(1, Math.round(total / 2));
  let start = 0;
  let before = 0;
  let length = half;
  periods.forEach((p, i) => {
    if (p.tick <= tick) {
      start = p.tick;
      before = i === 0 ? half : total + (i - 1) * EXTRA_TIME_MINUTES;
      length = i === 0 ? half : EXTRA_TIME_MINUTES;
    }
  });
  const elapsed = Math.max(0, tick - start);
  const regulation = length * TICKS_PER_MINUTE;
  if (elapsed < regulation) {
    return `${before + minuteOf(elapsed)}'`;
  }
  return `${before + length}+${minuteOf(elapsed - regulation) + 1}'`;
}

/// The first word of a club's name: Castlemere United is Castlemere.
export function shortName(team) {
  return String(team?.['team.name'] ?? '').split(/\s+/)[0] ?? '';
}

/// The scorer's family name, the last word of the name the engine gives.
function surname(name) {
  const words = String(name ?? '').trim().split(/\s+/);
  return words.at(-1) ?? '';
}

/// The words after a goal: who leads, or that it is level.
function standing(score, home, away) {
  if (score[0] === score[1]) {
    return `Level at ${score[0]}–${score[1]}.`;
  }
  return `${shortName(score[0] > score[1] ? home : away)} lead.`;
}

/// What the list shows at `renderedTick`.
///
/// `skip` is the session's skip; while it is decided or plays the list stays at its tick.
/// `seeks` lists every seek the page made, `{ lo, hi, n }`: the ticks it jumped across and
/// its number from 1. A goal whose moment lies in a seek's span was jumped over or shown
/// already, so it shows with no outline when the clock reaches it. `total` is the regulation
/// length of the match. `final` draws the report's list: no outline, every event that
/// arrived up to `renderedTick`. `stored` is a replay or a stored match, which keeps no
/// other grounds.
///
/// Returns `{ state: 'none', words, hint }` with no fixture to show, or `{ state: 'rows',
/// rows }` with one row per fixture: `home`, `away`, `score` (null when the result is
/// unavailable), `minute` (`KO`, a minute, `HT`, `FT` or `!`), `started`, `ended`, `behind`,
/// `unavailable`, and `flag` (`new`, `late` or null) with its `tag` and `words`.
export function groundsAt(matchday, renderedTick, { skip = null, seeks = [], total = 90, final = false, stored = false } = {}) {
  if (stored) {
    return {
      state: 'none',
      words: 'A replay keeps no other grounds.',
      hint: 'Their scores show only while the engine plays the match.',
    };
  }
  if (!matchday || matchday.fixtures.length === 0) {
    return {
      state: 'none',
      words: 'No other matches this matchday.',
      hint: 'When the matchday has other fixtures, their scores show here on your clock.',
    };
  }
  const tick = skip && HELD.has(skip.state) ? skip.from : renderedTick;
  const rows = matchday.fixtures.map((f, i) => {
    const shown = matchday.events[i].filter((e) => e.tick <= tick);
    const last = shown.at(-1) ?? null;
    const score = last ? [...last.score] : [0, 0];
    const failed = shown.find((e) => e.kind === 'unavailable') ?? null;
    const whistle = shown.find((e) => e.kind === 'full-time') ?? null;
    const reached = matchday.reached[i] ?? 0;
    const ended = whistle !== null || failed !== null;
    const started = tick > 0;
    const behind = started && !ended && reached < tick && !final;
    const periods = shown.filter((e) => e.kind === 'second-half' || e.kind === 'extra-time');
    const lastPeriod = shown.findLast((e) => ['half-time', 'second-half', 'extra-time'].includes(e.kind));
    let minute;
    if (failed) {
      minute = '!';
    } else if (whistle) {
      minute = 'FT';
    } else if (!started) {
      minute = 'KO';
    } else if (lastPeriod?.kind === 'half-time') {
      minute = 'HT';
    } else {
      minute = minuteLabel(Math.min(tick, behind ? reached : tick), periods, total);
    }
    let flag = null;
    let tag = null;
    let words = null;
    const goal = shown.findLast((e) => e.kind === 'goal') ?? null;
    if (goal && !final && !failed) {
      const at = goal.shownAt ?? goal.tick;
      // A late goal counts from the tick it arrived at, and only a seek after its arrival
      // can take its outline away.
      const after = goal.seeksBefore ?? 0;
      const spoiled = seeks.some((s) => s.n > after && at > s.lo && at <= s.hi);
      if (!spoiled && tick >= at && tick - at < HIGHLIGHT_TICKS) {
        const who = `${surname(goal.scorer)} ${eventStamp(goal)}`;
        if (goal.late) {
          flag = 'late';
          tag = 'GOAL · LATE';
          words = `${who}, shown at your ${minuteOf(at)}'.`;
        } else {
          flag = 'new';
          tag = 'GOAL';
          words = `${who}. ${standing(goal.score, f.home, f.away)}`;
        }
      }
    }
    return {
      fixture: f.fixture,
      home: f.home,
      away: f.away,
      score: failed ? null : score,
      minute,
      started,
      ended,
      behind,
      unavailable: failed !== null,
      failedAt: failed ? eventStamp(failed) : null,
      flag,
      tag,
      words,
    };
  });
  return { state: 'rows', rows };
}

/// How many grounds have finished, of all of them: `3 of 4 final`. A ground whose result is
/// unavailable never counts as final.
export function finalSummary(matchday) {
  if (!matchday || matchday.fixtures.length === 0) {
    return 'none';
  }
  const done = matchday.events.filter((list) => list.some((e) => e.kind === 'full-time')).length;
  return `${done} of ${matchday.fixtures.length} final`;
}

/// The report strip's figure: the first fixture's result, `Castlemere 3–1`, or a dash.
export function headline(matchday) {
  if (!matchday || matchday.fixtures.length === 0) {
    return '—';
  }
  const first = matchday.fixtures[0];
  const list = matchday.events[0];
  if (list.some((e) => e.kind === 'unavailable')) {
    return `${shortName(first.home)} –`;
  }
  const score = list.at(-1)?.score ?? [0, 0];
  return `${shortName(first.home)} ${score[0]}–${score[1]}`;
}

/// `true` when every ground has ended (full time, or no result).
export function allEnded(matchday) {
  return (
    matchday !== null &&
    matchday.events.every((list) => list.some((e) => e.kind === 'full-time' || e.kind === 'unavailable'))
  );
}
