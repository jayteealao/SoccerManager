// The name of the screen on show, for the tab title and the screen-change announcement.
// Pure: no DOM.

const DOOR = Object.freeze({
  splash: 'Loading',
  start: 'Start',
  setup: 'Match setup',
  settings: 'Settings',
  licences: 'Licences and about',
  closed: 'Closed',
});

const MATCH = Object.freeze({
  match: 'Match',
  tactics: 'Tactics',
  prematch: 'Pre-match line-ups',
  touchline: 'Touchline',
  skip: 'Skip to result',
  report: 'Report',
  replay: 'Replay',
  resume: 'Resume a saved match',
});

/// The name of the front door's `view`, or, over a match, of the session's `matchView`.
export function screenName(view, matchView) {
  if (view === 'match') {
    return MATCH[matchView] ?? MATCH.match;
  }
  return DOOR[view] ?? '';
}
