// The Resume a saved match screen, for a save no engine of this version can finish: what the
// header, the fact strip and the alert say, from the launcher's `resume` block
// (`/engine.json`). Pure: no DOM, so each kind of refusal is tested without a browser.

import { clockAt } from './recovery.js';

/// The strip's word under "Cannot resume", per kind of refusal.
const WHY = Object.freeze({
  older: 'Two or more versions back',
  newer: 'A newer version',
  other: 'Not carried by this version',
  unreleased: 'An unreleased build',
  'previous-missing': 'Its engine is missing',
});

const DAYS = ['SUN', 'MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT'];
const MONTHS = [
  'JANUARY',
  'FEBRUARY',
  'MARCH',
  'APRIL',
  'MAY',
  'JUNE',
  'JULY',
  'AUGUST',
  'SEPTEMBER',
  'OCTOBER',
  'NOVEMBER',
  'DECEMBER',
];

/// The date block's day for a match stamp (milliseconds since 1970, UTC), as the board
/// writes it: `SAT 14 NOVEMBER`. Null without a stamp.
export function savedDay(millis) {
  if (typeof millis !== 'number' || !Number.isFinite(millis) || millis <= 0) {
    return null;
  }
  const day = new Date(millis);
  return `${DAYS[day.getUTCDay()]} ${day.getUTCDate()} ${MONTHS[day.getUTCMonth()]}`;
}

/// The saved match in one line: `Ashford Rovers 1–0 Port Varrow · 52:10`. A save from
/// before the summary was recorded names only its clock.
export function savedMatch(info) {
  const clock = info['saved.tick'] === null || info['saved.tick'] === undefined ? null : clockAt(info['saved.tick']);
  const teams = info['saved.teams'];
  const score = info['saved.score'];
  if (Array.isArray(teams) && teams.length === 2 && Array.isArray(score)) {
    const line = `${teams[0]} ${score[0]}–${score[1]} ${teams[1]}`;
    return clock ? `${line} · ${clock}` : line;
  }
  return clock ? `A match at ${clock}` : 'A saved match';
}

/// Who saved it: `Touchline 0.1.0`, or the build no release shipped.
export function savedBy(info) {
  const version = info['saved.version'];
  return version ? `Touchline ${version}` : `Unreleased build ${info['saved.build'] ?? ''}`.trim();
}

/// Everything the screen shows, or null when there is no refusal to show.
export function resumeModel(info) {
  if (!info) {
    return null;
  }
  const [current, previous] = Array.isArray(info.engines) ? info.engines : [null, null];
  const kind = info.kind ?? 'other';
  const saved = info['saved.version'] ?? null;
  const carried = `This version finishes matches saved by ${current} and ${previous}.`;
  const body = {
    older: `${carried} A match saved two or more versions back cannot continue here, because its engine no longer ships with the game.`,
    newer: `${carried} A match saved by a newer version cannot continue here, because this version does not have its engine.`,
    other: `${carried} The engine that saved this match does not ship with this version.`,
    unreleased: `${carried} This match was saved by a build that no release shipped, so no engine in the game can finish it.`,
    'previous-missing': `${carried} The engine of ${saved} ships beside this version, but it is missing from this copy of the game.`,
  }[kind];
  const hint = {
    older: `To finish it, open the save in Touchline ${saved}.`,
    newer: `To finish it, open the save in Touchline ${saved} or later.`,
    other: `To finish it, open the save in Touchline ${saved}.`,
    unreleased: null,
    'previous-missing': 'Install the game again to bring that engine back.',
  }[kind];
  return {
    kind,
    title: 'Resume a saved match',
    subtitle: `Touchline ${current} · finishes matches saved by ${current} and ${previous}`,
    date: savedDay(info['saved.millis']) ?? 'SAVED MATCH',
    dateSub: 'Saved match',
    facts: [
      { value: 'Saved match', label: savedMatch(info) },
      { value: savedBy(info), label: 'Saved by' },
      { value: `Touchline ${current}`, label: 'This version' },
      { value: `${current} and ${previous}`, label: 'Engines in this version' },
      { value: 'Cannot resume', label: WHY[kind] ?? WHY.other, tone: 'warn' },
    ],
    heading: saved ? `This match was saved by Touchline ${saved}` : 'This match was saved by an unreleased build',
    body,
    hint,
    note: 'The save stays on disk. Nothing is deleted.',
  };
}
