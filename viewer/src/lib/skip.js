// Skip to the result: the words and the geometry of the four skip surfaces. Pure, with no
// DOM and no socket: the session keeps the skip's state, and the screens render what these
// return.
//
// A skip is the same match playing on: the engine stops waiting for the page and plays the
// rest from the exact tick at full speed, so the result is the one the player would have
// watched. The part after the skip point was not watched live; the report hatches it on the
// timeline and tags its goals and cards NOT LIVE, and the replay holds all of it.

import { clockAt } from './recovery.js';
import { TICKS_PER_SECOND } from './schedule.js';

/// Ticks in one minute of play.
export const TICKS_PER_MINUTE = TICKS_PER_SECOND * 60;

/// The match minute a tick falls in, as the clock shows it: tick 90 000 is minute 30.
export function minuteOf(tick) {
  return Math.floor(Math.max(0, tick) / TICKS_PER_MINUTE);
}

/// The regulation minutes the engine plays to: 90, 120 for a knockout match, or the
/// shorter length a test match names.
export function totalMinutes(ticksExpected, knockout = false) {
  if (knockout) {
    return 120;
  }
  return Math.max(1, Math.min(90, Math.round((ticksExpected ?? 0) / TICKS_PER_MINUTE)));
}

/// The four steps of "the engine plays the rest", as the board lists them: freeze the match
/// at the skip point, play to full time with the newest minute received, write the report,
/// and store the whole match. `stage` is `playing` (step 2 current), `storing` (the full-time
/// whistle has arrived; step 4 current) or `ready` (every step done).
export function skipSteps(from, newestTick, total, stage) {
  const at = clockAt(from);
  const minute = Math.min(total, minuteOf(Math.max(from, newestTick)));
  const playing = stage === 'playing';
  const share = total * TICKS_PER_MINUTE > from ? (newestTick - from) / (total * TICKS_PER_MINUTE - from) : 1;
  return [
    {
      label: `Freeze the match at ${at}`,
      state: 'done',
      word: 'The state and the random draws are kept',
    },
    {
      label: `Play ${at} to full time`,
      state: playing ? 'current' : 'done',
      word: `${minute}' of ${total} · at full speed`,
      progress: Math.round(Math.min(1, Math.max(0, share)) * 100),
    },
    {
      label: 'Write the report',
      state: playing ? 'pending' : 'done',
      word: 'Figures, goals and cards',
    },
    {
      label: 'Store the whole match',
      state: stage === 'ready' ? 'done' : stage === 'storing' ? 'current' : 'pending',
      word: "The replay from 0' to full time",
      progress: 90,
    },
  ];
}

/// The Skip decision's strip and its "Where the match stands" rows, from the match at the
/// paused tick: the clock, the score and its scorers, the substitutes used of the rule
/// pack's limit, the queued changes, and the engine version.
export function decisionFacts({ from, score, scorers, subsUsed, limit, queued, engineVersion, period, mentality }) {
  const clock = clockAt(from);
  const minute = `${minuteOf(from)}'`;
  const scored = scorers.filter(Boolean).join(' · ');
  const waiting = queued.length;
  return {
    strip: [
      { value: `Play paused · ${clock}`, label: 'You chose Skip to result' },
      { value: `${score[0]} – ${score[1]} · ${minute}`, label: scored || 'No goals yet', num: true },
      { value: `${subsUsed} of ${limit}`, label: 'Substitutes used', num: true },
      waiting === 0
        ? { value: 'Nothing queued', label: 'No change waits for a stoppage' }
        : { value: `${waiting} queued`, label: `${queued[0]}${waiting > 1 ? ` and ${waiting - 1} more` : ''} at the next stoppage` },
      {
        value: engineVersion ? `Engine ${engineVersion}` : 'Engine',
        label: 'Plays the rest at full speed',
      },
    ],
    rows: [
      { key: 'Clock', value: `${clock} · ${period}` },
      { key: 'Queued change', value: waiting === 0 ? 'None' : queued.join(', ') },
      { key: 'Mentality', value: mentality || '—' },
    ],
  };
}

/// `true` for a moment the player did not watch live: one after the skip point.
export function notLive(moment, from) {
  return from !== null && from !== undefined && moment.tick > from;
}

/// The hatched part of a timeline drawn `width` wide from `inset`, over `end` ticks: from
/// the skip point to the end. Null with no skip.
export function hatchSpan(from, end, width, inset = 0) {
  if (from === null || from === undefined || !(end > 0)) {
    return null;
  }
  const x = inset + (Math.min(Math.max(0, from), end) / end) * width;
  return { x, width: Math.max(0, inset + width - x) };
}

/// The report's sub-line after a skip: `skipped from 67:12`.
export function skippedWord(from) {
  return `skipped from ${clockAt(from)}`;
}
