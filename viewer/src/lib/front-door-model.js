// The front door's words and facts, from the launcher's `/engine.json` answer and the notices
// file. Pure, so the tests can hold each screen's text without a page.

import { clockAt } from './recovery.js';
import { HOME, DOCUMENT, LEAVE, PLUS, SCREEN, SLIDERS, PLAY } from '../components/icons.js';

/* global __TOUCHLINE_VERSION__ */
/// The version this viewer was built for, from the build (`vite.config.js`); the splash names
/// it before the engine answers.
export const BUILD_VERSION = typeof __TOUCHLINE_VERSION__ === 'string' ? __TOUCHLINE_VERSION__ : '';

/// A sample team in the shape the crest reads.
export function teamOf(sample) {
  if (!sample) {
    return null;
  }
  return {
    'team.id': sample.id,
    'team.name': sample.name,
    'team.kit.primary': sample.kit?.[0],
    'team.kit.secondary': sample.kit?.[1],
  };
}

/// The match clock a save stopped at, `52:10`, from its tick.
export function savedClock(saved) {
  return clockAt(saved?.tick ?? 0);
}

/// `Ashford Rovers 1–1 Port Varrow`, or the fixture alone when the save names no score.
export function scoreLine(saved) {
  const [home, away] = saved?.teams ?? ['Home', 'Away'];
  const score = saved?.score;
  return score ? `${home} ${score[0]}–${score[1]} ${away}` : `${home} v ${away}`;
}

/// `Ashford Rovers 1–1 Port Varrow · 52:10`.
export function savedLine(saved) {
  return `${scoreLine(saved)} · ${savedClock(saved)}`;
}

/// The engine word for a save: this release, the previous release, or another.
export function savedEngine(saved) {
  const version = saved?.version ?? 'unknown';
  if (saved?.kind === 'current') {
    return `${version} · this release`;
  }
  if (saved?.kind === 'previous') {
    return `${version} · previous engine`;
  }
  return `${version} · cannot continue here`;
}

/// The start screen's choices, in the board's order.
export function startItems(saved) {
  return [
    { id: 'new', glyph: { d: PLUS }, label: 'New match', sub: 'Pick two teams and kick off', primary: true },
    saved
      ? { id: 'resume', glyph: { d: PLAY }, label: 'Resume', sub: savedLine(saved) }
      : { id: 'resume', glyph: { d: PLAY }, label: 'Resume', sub: 'No saved match yet', disabled: true },
    { id: 'replays', glyph: { d: SCREEN }, label: 'Replays', sub: 'Open a replay file' },
    { id: 'settings', glyph: { d: SLIDERS }, label: 'Settings', sub: 'Speed, motion, commentary' },
    { id: 'licences', glyph: { d: DOCUMENT }, label: 'Licences and about', sub: 'Versions and open-source notices' },
    { id: 'quit', glyph: { d: LEAVE }, label: 'Quit', sub: 'Save and close Touchline', rule: true },
  ];
}

/// The versions the start, settings and licences screens name.
export function versions(status) {
  return {
    touchline: status?.['launcher.version'] ?? status?.['engine.version'] ?? '—',
    engine: status?.['engine.version'] ?? status?.['launcher.version'] ?? '—',
    previous: status?.['previous.version'] ?? null,
  };
}

/// The start screen's fact strip.
export function startFacts(status) {
  const v = versions(status);
  const saved = status?.saved ?? null;
  return [
    { value: `Touchline ${v.touchline}`, label: 'Football match simulation' },
    { value: 'Ready', label: `Match engine ${v.engine}`, tone: 'good' },
    saved
      ? { value: '1 saved match', label: savedLine(saved) }
      : { value: 'No saved match', label: 'A match saves when you leave it' },
    { value: 'Replays', label: 'Open a replay file to watch it' },
    v.previous
      ? { value: v.previous, label: 'Previous engine · finishes older saves' }
      : { value: 'None', label: 'No previous engine in this folder' },
  ];
}

/// Why Kick off cannot run for this pair, or null.
export function pickProblem(home, away) {
  if (!home || !away) {
    return 'Pick the home team and the away team.';
  }
  if (home === away) {
    return 'A team cannot play itself. Pick a different away team.';
  }
  return null;
}

/// `105 × 68 m`.
export function groundSize(team) {
  const [length, width] = team?.ground ?? [105, 68];
  return `${length} × ${width} m`;
}

/// The home club's ground: the team files name no ground, so it is the club's ground.
export function groundName(team) {
  return team ? `${team.name} ground` : '—';
}

/// The setup screen's fact strip.
export function setupFacts(home, away, round) {
  const same = home && away && home.id === away.id;
  const others = round?.fixtures?.length ?? 0;
  return [
    { value: home?.name ?? 'Pick a team', label: 'Home team' },
    { value: away?.name ?? 'Pick a team', label: same ? 'Away team · same as home' : 'Away team', tone: same ? 'bad' : undefined },
    { value: groundName(home), label: 'Ground of the home team' },
    { value: groundSize(home), label: 'Pitch size', num: true },
    { value: `${others} other ${others === 1 ? 'match' : 'matches'}`, label: 'The round around your match', num: true },
  ];
}

export const SPEED_OPTIONS = Object.freeze([1, 2, 4, 8].map((s) => ({ value: s, label: `${s}×` })));
export const MOTION_OPTIONS = Object.freeze([
  { value: 'follow', label: 'Follow system' },
  { value: 'reduce', label: 'Reduce' },
  { value: 'full', label: 'Full' },
]);
export const COMMENTARY_OPTIONS = Object.freeze([
  { value: true, label: 'On' },
  { value: false, label: 'Off' },
]);

/// What the motion setting means now: `systemReduces` is the system's preference.
export function motionNow(motion, systemReduces) {
  if (motion === 'reduce') {
    return 'Reduce';
  }
  if (motion === 'full') {
    return 'Full';
  }
  return 'Follow system';
}

/// The settings screen's fact strip.
export function settingsFacts(settings, systemReduces, saved) {
  const reduced = settings.motion === 'reduce' || (settings.motion === 'follow' && systemReduces);
  return [
    { value: `${settings.speed}×`, label: 'Default speed', num: true },
    { value: motionNow(settings.motion, systemReduces), label: `Motion · ${reduced ? 'reduced motion now' : 'full motion now'}` },
    { value: settings.commentary ? 'On' : 'Off', label: 'Commentary' },
    saved
      ? { value: 'Saved', label: 'Kept for the next launch', tone: 'good' }
      : { value: 'Kept', label: 'Saved as you change them' },
  ];
}

/// The licences in use across the notices, in first-seen order of their identifiers.
export function licencesInUse(file) {
  const seen = new Set();
  for (const p of file?.packages ?? []) {
    for (const id of String(p.licence ?? '').split(/\s+(?:OR|AND|WITH)\s+|[()]/)) {
      const word = id.trim();
      if (word && ['MIT', 'Apache-2.0', 'OFL-1.1'].includes(word)) {
        seen.add(word);
      }
    }
  }
  return [...seen];
}

/// The licences screen's fact strip.
export function licenceFacts(status, file) {
  const v = versions(status);
  const count = file?.packages?.length ?? 0;
  const inUse = licencesInUse(file);
  return [
    { value: `Touchline ${v.touchline}`, label: 'MIT OR Apache-2.0' },
    { value: v.engine, label: 'Match engine' },
    v.previous ? { value: v.previous, label: 'Previous engine · finishes older saves' } : { value: 'None', label: 'No previous engine' },
    file ? { value: `${count} packages`, label: 'Notices built with this release', num: true } : { value: 'No notices file', label: 'This build carries none' },
    { value: inUse.length ? inUse.join(' · ') : '—', label: 'Licences in use' },
  ];
}

/// The closed page's step list and its first line, from the quit answer's `closed` block.
export function closedModel(closed, status) {
  const v = versions(status);
  const steps = [];
  const saved = closed?.saved ?? null;
  if (closed?.match) {
    steps.push({
      label: 'Match saved',
      word: saved ? `${saved.teams?.[0] ?? 'Home'} v ${saved.teams?.[1] ?? 'Away'} at ${savedClock(saved)}` : 'Saved',
      state: 'done',
    });
  }
  steps.push({ label: 'Match engine stopped', word: `Engine ${v.engine}`, state: 'done' });
  steps.push({ label: 'Launcher stopped', word: 'Nothing is left running', state: 'done' });
  const line =
    closed?.match && saved
      ? `Your match is saved at ${savedClock(saved)}, ${scoreLine(saved)}. Resume it from the start screen next time.`
      : 'No match was playing. Everything is closed.';
  return { steps, line };
}

/// The closed page's glyph, the home glyph for Return to start, re-exported for the menu.
export { HOME };
