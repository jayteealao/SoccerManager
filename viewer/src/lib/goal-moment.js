// The goal moment: the score-bug pulse, the banner over the pitch, and the feed highlight,
// all in the frame that releases the goal.
//
// The banner runs on wall time, not match time. The 1.5-second rule is about how long a
// person can read it; at eight times speed a match-time timer would flash it for 190 ms.
// Only one banner shows at a time, and a later goal replaces an earlier one.

import { motionWord } from './settings.js';

/// The motion, in milliseconds. `viewer/tests/goal-moment.test.js` holds these to the
/// `--goal-*` and `--pulse` tokens in every skin's `tokens.css`, which the styles animate with.
export const GOAL_IN_MS = 250;
export const GOAL_HOLD_MS = 1050;
export const GOAL_OUT_MS = 200;
export const PULSE_MS = 600;

/// The banner is visible from the frame the goal is released until this many milliseconds
/// later, when it has finished leaving.
export const BANNER_TOTAL_MS = GOAL_IN_MS + GOAL_HOLD_MS + GOAL_OUT_MS;

/// `GOAL — Oakmere Rangers 1–0 23'`. `minute` is the stamp the feed shows.
export function bannerText(goal, teamNames, minute) {
  const team = teamNames.get(goal['team.id']) ?? 'Goal';
  return `GOAL — ${team} ${goal['home.score']}–${goal['away.score']} ${minute}`;
}

/// `true` when motion should be dropped: the operating-system preference, or the
/// `data-motion="reduce"` attribute a drive sets with `?motion=reduce`. Both paths end in the
/// same attribute, so the stylesheet serves them from one rule block.
export function reducedMotion(doc = globalThis.document) {
  return doc?.documentElement?.dataset.motion === 'reduce';
}

/// Mirrors the reduced-motion preference onto `<html data-motion>`, now and whenever it
/// changes. `?motion=reduce` forces it, for a browser tool that cannot emulate the query.
/// `setting` is the player's motion setting: `follow` (the system's preference), `reduce` or
/// `full`. The returned object's `set(motion)` applies a new setting at once.
export function watchMotion(doc = globalThis.document, win = globalThis, setting = 'follow') {
  const root = doc.documentElement;
  const forced = new URLSearchParams(win.location?.search ?? '').get('motion') === 'reduce';
  const query = win.matchMedia ? win.matchMedia('(prefers-reduced-motion: reduce)') : null;
  let motion = setting;
  const apply = () => {
    root.dataset.motion = forced ? 'reduce' : motionWord(motion, Boolean(query && query.matches));
  };
  apply();
  query?.addEventListener?.('change', apply);
  watching = {
    set(next) {
      motion = next;
      apply();
      return root.dataset.motion;
    },
  };
  return root.dataset.motion;
}

/// The watcher `watchMotion` started last, so a settings change applies to the page at once.
let watching = null;

/// Applies the player's motion setting to the watched page; a no-op before `watchMotion`.
export function setMotion(motion) {
  return watching?.set(motion) ?? null;
}

export class GoalMoment {
  /// `nodes`: `banner`, `bannerText`, `bug` (the score bug).
  constructor(nodes, { doc = globalThis.document, timers = globalThis } = {}) {
    this.nodes = nodes;
    this.doc = doc;
    this.timers = timers;
    this.hideTimer = null;
    this.pulseTimer = null;
    this.shownAtTick = null;
    this.visible = false;
  }

  /// Runs the moment for `goal` in this frame.
  play(goal, text) {
    const { banner, bannerText: label, bug } = this.nodes;
    this.clear();
    label.textContent = text;
    banner.dataset.shown = 'true';
    banner.setAttribute('aria-hidden', 'false');
    this.visible = true;
    this.shownAtTick = goal.tick;
    this.hideTimer = this.timers.setTimeout(() => {
      banner.dataset.shown = 'false';
      this.hideTimer = this.timers.setTimeout(() => {
        banner.setAttribute('aria-hidden', 'true');
        this.visible = false;
        this.hideTimer = null;
      }, GOAL_OUT_MS);
    }, GOAL_IN_MS + GOAL_HOLD_MS);
    if (!reducedMotion(this.doc)) {
      bug.dataset.pulse = 'on';
      this.pulseTimer = this.timers.setTimeout(() => {
        bug.dataset.pulse = 'off';
        this.pulseTimer = null;
      }, PULSE_MS);
    }
  }

  /// Takes the banner and the pulse down at once, for a rewind or a replacing goal.
  clear() {
    const { banner, bug } = this.nodes;
    if (this.hideTimer !== null) {
      this.timers.clearTimeout(this.hideTimer);
      this.hideTimer = null;
    }
    if (this.pulseTimer !== null) {
      this.timers.clearTimeout(this.pulseTimer);
      this.pulseTimer = null;
    }
    banner.dataset.shown = 'false';
    banner.setAttribute('aria-hidden', 'true');
    bug.dataset.pulse = 'off';
    this.visible = false;
  }
}
