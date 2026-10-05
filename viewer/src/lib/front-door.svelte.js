// The front door: which screen the page shows before, between and after matches. The splash
// covers the engine's start; the start screen leads to match setup, Resume, Replays,
// Settings, Licences and about, and Quit; the in-match menu leads back to the start screen
// or out of the game.
//
// The launcher keeps the state that must outlive the page (the saved match, the settings),
// and `/engine.json` says whether it opened on the start screen (`front-door: true`). When
// it did not (a served test page, `launch --no-start-screen`, `launch --resume`), the splash
// leaves at the first answer and the match session starts as the page always started it.
//
// Each match gets a new session; the one before it is disposed, so its socket and its frame
// loop end with it. After full time the report leads on to a new match with the same two
// clubs, or back to the start screen.

import * as launcher from './launcher.js';
import { setMotion } from './goal-moment.js';
import { clockAt } from './recovery.js';
import { MatchSession } from './match-session.svelte.js';
import { DEFAULTS, readSettings } from './settings.js';

/// The logo reveal's length, from the splash's first frame (02c-craft.md, Motion).
export const REVEAL_MS = 1800;
/// The splash never leaves before this, so the title and the credits can be read.
export const FLOOR_MS = 1500;
/// Once the reveal and the engine are both ready, the ready state holds this long before the
/// start screen opens by itself; a key or a click ends it at once.
export const READY_HOLD_MS = 1500;

/// The screens of the front door, and `match` for the match views of the session.
export const VIEWS = Object.freeze(['splash', 'start', 'setup', 'settings', 'licences', 'match', 'closed']);

/// The match views the in-match menu opens over.
const MENU_VIEWS = new Set(['match', 'tactics', 'touchline']);

/// How long a next step after full time waits for the finished match's worker to end.
const WORKER_END_MS = 10_000;

const defaultFetch = (...args) => globalThis.fetch(...args);

export class FrontDoor {
  /// The screen on show: one of VIEWS.
  view = $state('splash');
  /// What sits over the paused match: `menu`, `return`, `quit`, or null.
  overlay = $state(null);
  /// The launcher's newest `/engine.json` body, or null before it answers.
  status = $state.raw(null);
  /// The splash: `phase` is `reveal` or `ready`; `finished` jumps the reveal to its last frame;
  /// `answered` is true once the engine answered.
  splash = $state.raw({ phase: 'reveal', finished: false, answered: false });
  /// The player's settings, and whether the last change was saved.
  settings = $state.raw({ ...DEFAULTS });
  saved = $state(false);
  /// Match setup: the picked club ids and the rest of the round for the pair.
  picks = $state.raw({ home: null, away: null });
  round = $state.raw(null);
  /// The open-source notices: `{ state, file }`, where `state` is `loading`, `ready` or
  /// `missing` and `file` the notices file the build wrote.
  notices = $state.raw({ state: 'loading', file: null });
  /// The closed page's facts: `{ match, saved }`, from the quit answer.
  closed = $state.raw(null);
  /// The match session on show, or the idle one before the first match.
  session = $state.raw(null);
  /// A line the start screen shows: why a replay or an action was refused.
  message = $state(null);
  /// `true` while an action waits for the launcher.
  busy = $state(false);
  /// Where Settings and Licences go back to: `start`, or `match` when opened over a paused
  /// match from its menu.
  returnTo = $state('start');
  /// The clock the return and quit dialogs name: the newest stoppage the engine saved.
  saveClock = $state('');

  /// `fetcher`, `timers` and `now` are the browser's unless a test passes its own.
  /// `makeSession(options)` builds a session (a test passes a fake); `onSession(session)` is
  /// told of each new session, so the test hook follows it; `reduced()` says whether motion
  /// is reduced now.
  constructor({
    fetcher = defaultFetch,
    timers = globalThis,
    now = () => Date.now(),
    makeSession = (options) => new MatchSession(options),
    onSession = () => {},
    reduced = () => globalThis.document?.documentElement?.dataset.motion === 'reduce',
  } = {}) {
    this.fetcher = fetcher;
    this.timers = timers;
    this.now = now;
    this.makeSession = makeSession;
    this.onSession = onSession;
    this.reduced = reduced;
    this.started = now();
    this.revealDone = false;
    this.timer = null;
    this.newSession();
    if (reduced()) {
      this.finishReveal();
    } else {
      this.timer = timers.setTimeout(() => {
        this.timer = null;
        this.revealDone = true;
        this.maybeReady();
      }, REVEAL_MS);
    }
  }

  // ---- What the screens read ------------------------------------------------------------------

  /// `true` when the launcher opened on the start screen.
  get frontDoor() {
    return this.status?.['front-door'] === true;
  }

  /// The sample teams match setup offers.
  get teams() {
    return this.status?.teams ?? [];
  }

  /// The unfinished match Resume continues, or null.
  get savedMatch() {
    return this.status?.saved ?? null;
  }

  /// The engine's version, as the splash and the about block name it.
  get engineVersion() {
    return this.status?.['engine.version'] ?? this.status?.['launcher.version'] ?? null;
  }

  /// Why Kick off cannot run, or null when it can.
  get pickProblem() {
    const { home, away } = this.picks;
    if (!home || !away) {
      return 'Pick the home team and the away team.';
    }
    if (home === away) {
      return 'A team cannot play itself. Pick a different away team.';
    }
    return null;
  }

  /// `true` when the match on show is over and stored, so leaving it keeps nothing for Resume.
  get matchOver() {
    return this.session?.isOver === true && this.session.nextReady === true;
  }

  /// The quit confirmation's lines: a finished match is not kept; any other match saves at
  /// the newest stoppage.
  get quitLines() {
    const close = 'The engine and the launcher stop. This tab then shows that Touchline has closed.';
    return this.matchOver
      ? ['The match is over and is not kept for Resume.', close]
      : [`The match saves at ${this.saveClock}. Resume it from the start screen next time.`, close];
  }

  /// The quit confirmation's button: a finished match has nothing to save.
  get quitPrimary() {
    return this.matchOver ? 'Quit' : 'Save and quit';
  }

  /// The read-only test hook's view of the front door.
  snapshot() {
    return {
      view: this.view,
      overlay: this.overlay,
      splash: this.splash.phase,
      revealFinished: this.splash.finished,
      answered: this.splash.answered,
      frontDoor: this.frontDoor,
      picks: { ...this.picks },
      round: this.round ? this.round.fixtures.map((f) => [f.home?.name ?? null, f.away?.name ?? null]) : null,
      settings: { ...this.settings },
      saved: this.savedMatch ? { ...this.savedMatch, positions: undefined } : null,
      savedMarkers: this.savedMatch?.positions?.players?.length ?? 0,
      saveClock: this.saveClock,
      returnTo: this.returnTo,
      closed: this.closed,
      message: this.message,
    };
  }

  // ---- The splash ---------------------------------------------------------------------------

  /// The engine's first answer. Without the front door, the match starts as it always did.
  answer(status) {
    this.status = status;
    this.settings = readSettings(status?.settings);
    setMotion(this.settings.motion);
    this.splash = { ...this.splash, answered: true };
    if (!this.frontDoor) {
      this.clearTimer();
      this.startSession();
      return;
    }
    if (this.reduced() && !this.revealDone) {
      // The Reduce setting (or the system, under Follow system) drops the reveal.
      this.clearTimer();
      this.finishReveal();
      return;
    }
    this.maybeReady();
  }

  /// A key or a click on the splash: before the engine answers it finishes the reveal; after,
  /// it opens the start screen at once.
  press() {
    if (this.view !== 'splash') {
      return;
    }
    if (!this.splash.answered) {
      this.finishReveal();
      return;
    }
    this.enterStart();
  }

  finishReveal() {
    this.revealDone = true;
    this.splash = { ...this.splash, finished: true };
    this.maybeReady();
  }

  /// The ready state once the reveal ended, the engine answered and the floor passed; the start
  /// screen after the ready hold.
  maybeReady() {
    if (this.view !== 'splash' || !this.splash.answered || !this.revealDone || !this.frontDoor) {
      return;
    }
    if (this.splash.phase === 'ready') {
      return;
    }
    const waited = this.now() - this.started;
    if (waited < FLOOR_MS) {
      this.clearTimer();
      this.timer = this.timers.setTimeout(() => {
        this.timer = null;
        this.maybeReady();
      }, FLOOR_MS - waited);
      return;
    }
    this.clearTimer();
    this.splash = { ...this.splash, phase: 'ready' };
    this.timer = this.timers.setTimeout(() => {
      this.timer = null;
      this.enterStart();
    }, READY_HOLD_MS);
  }

  clearTimer() {
    if (this.timer !== null) {
      this.timers.clearTimeout(this.timer);
      this.timer = null;
    }
  }

  /// Leaves the splash for the start screen; a match the launcher already plays (a reloaded
  /// page) or a missing engine goes to the match screen instead.
  enterStart() {
    this.clearTimer();
    const state = this.status?.['engine.state'];
    if (state === 'running' || state === 'starting' || state === 'not-found') {
      this.startSession();
      return;
    }
    this.view = 'start';
  }

  // ---- Sessions -----------------------------------------------------------------------------

  /// A new session in place of the old one, which is disposed.
  newSession() {
    this.session?.dispose?.();
    const session = this.makeSession({
      settings: this.settings,
      onLeave: (why) => this.leaveReplay(why),
    });
    session.onMenu = this.frontDoor ? () => this.openMenu() : null;
    session.onNextStep = this.frontDoor ? (id) => this.afterFullTime(id) : null;
    this.session = session;
    this.onSession(session);
    return session;
  }

  /// The match screen over a new session, started on the launcher's match.
  startSession() {
    const session = this.newSession();
    this.view = 'match';
    this.overlay = null;
    session.start();
  }

  /// A replay opened from the start screen closed or could not be read.
  leaveReplay(why = {}) {
    this.newSession();
    this.view = 'start';
    this.message = why.refused ? `The replay could not be read: ${why.refused}` : null;
  }

  // ---- The start screen ---------------------------------------------------------------------

  /// Asks the launcher again, so the start screen shows the newest saved match.
  async refresh() {
    const status = await launcher.fetchStatus(this.fetcher);
    if (status) {
      this.status = status;
    }
    return status;
  }

  open(view) {
    if (view === 'setup' && !this.picks.home && this.teams.length >= 2) {
      this.pick('home', this.teams[0].id);
      this.pick('away', this.teams[1].id);
    }
    if (view === 'licences') {
      this.loadNotices();
    }
    if (view === 'settings' || view === 'licences') {
      // Between Settings and Licences the way back stays where the first one came from.
      if (this.view !== 'settings' && this.view !== 'licences') {
        this.returnTo = this.view === 'match' ? 'match' : 'start';
      }
      this.overlay = null;
      if (this.session) {
        this.session.menuOpen = false;
      }
    }
    this.saved = false;
    this.message = null;
    this.view = view;
  }

  /// Back from Settings or Licences: to the paused match it was opened over, with its menu
  /// open again, or to the start screen.
  back() {
    if (this.returnTo === 'match') {
      this.view = 'match';
      this.overlay = 'menu';
      if (this.session) {
        this.session.menuOpen = true;
      }
    } else {
      this.view = 'start';
    }
    this.returnTo = 'start';
  }

  async resume() {
    if (this.busy || !this.savedMatch) {
      return;
    }
    this.busy = true;
    let reason = '';
    const answer = await launcher.resume(this.fetcher, (text) => {
      reason = text;
    });
    this.busy = false;
    if (!answer) {
      this.message = failure(
        'The saved match could not be resumed',
        reason,
        'Choose Resume again, or start a new match.',
      );
      await this.refresh();
      return;
    }
    this.status = answer;
    this.startSession();
  }

  /// Plays a replay file from the start screen in the replay view; closing it comes back.
  async openReplay(bytes, name) {
    const session = this.newSession();
    this.view = 'match';
    await session.openReplay(bytes, name, { leave: true });
  }

  // ---- Match setup --------------------------------------------------------------------------

  /// Picks `id` as the `side` (`home` or `away`) team and asks for the rest of the round.
  pick(side, id) {
    this.picks = { ...this.picks, [side]: id };
    this.round = null;
    const { home, away } = this.picks;
    if (home && away && home !== away) {
      const asked = { home, away };
      launcher.fetchRound(this.fetcher, home, away).then((round) => {
        if (this.picks.home === asked.home && this.picks.away === asked.away) {
          this.round = round;
        }
      });
    }
  }

  async kickOff() {
    if (this.busy || this.pickProblem) {
      return;
    }
    this.busy = true;
    let reason = '';
    const answer = await launcher.newMatch(this.fetcher, { ...this.picks }, (text) => {
      reason = text;
    });
    this.busy = false;
    if (!answer) {
      this.message = failure(
        'The engine did not start the match',
        reason,
        'Choose KICK OFF again, or pick other teams.',
      );
      return;
    }
    this.status = answer;
    this.startSession();
  }

  // ---- Settings and licences ----------------------------------------------------------------

  /// Saves one or more settings at once; the page applies motion and commentary at once.
  async saveSettings(change) {
    const next = { ...this.settings, ...change };
    this.settings = next;
    this.saved = false;
    setMotion(next.motion);
    if (this.session) {
      this.session.commentary = next.commentary;
    }
    const answer = await launcher.saveSettings(this.fetcher, next);
    if (answer) {
      this.status = answer;
      this.settings = readSettings(answer.settings);
      this.saved = true;
    }
  }

  /// Reads the notices file the build wrote beside the page, once.
  async loadNotices() {
    if (this.notices.state === 'ready') {
      return;
    }
    try {
      const response = await this.fetcher('notices.json', { cache: 'no-store' });
      const file = response.ok ? await response.json() : null;
      this.notices = file ? { state: 'ready', file } : { state: 'missing', file: null };
    } catch {
      this.notices = { state: 'missing', file: null };
    }
  }

  // ---- The in-match menu --------------------------------------------------------------------

  openMenu() {
    if (this.view !== 'match' || this.overlay || !MENU_VIEWS.has(this.session?.view)) {
      return;
    }
    this.session.pauseForMenu();
    this.session.menuOpen = true;
    this.overlay = 'menu';
  }

  /// Resume match (or Esc): the menu closes and the match plays on if it played before.
  closeMenu() {
    if (!this.overlay) {
      return;
    }
    this.overlay = null;
    if (this.session) {
      this.session.menuOpen = false;
      this.session.resumeFromMenu();
    }
  }

  /// Esc on a match view: opens the menu, or closes what is open.
  escape() {
    if (this.overlay) {
      this.closeMenu();
    } else {
      this.openMenu();
    }
  }

  /// Opens the return (`return`) or quit (`quit`) confirmation over the paused match. It names
  /// the newest stoppage the engine saved: the newest one the page has seen at once, then the
  /// launcher's own word for it.
  async ask(which) {
    // A finished, stored match keeps nothing, so Return to start needs no confirmation.
    if (which === 'return' && this.matchOver) {
      await this.afterFullTime('return');
      return;
    }
    this.overlay = which;
    if (this.session) {
      this.session.menuOpen = false;
    }
    const seen = this.session?.stoppages?.prev?.(Number.MAX_SAFE_INTEGER);
    this.saveClock = clockAt(seen ?? this.session?.renderedTick ?? 0);
    const status = await launcher.fetchStatus(this.fetcher);
    const tick = status?.['snapshot.tick'];
    if (this.overlay === which && typeof tick === 'number') {
      this.saveClock = clockAt(tick);
    }
  }

  /// Save and leave: the launcher stops the match and keeps its save for Resume.
  async returnToStart() {
    if (this.busy) {
      return;
    }
    this.busy = true;
    const answer = await launcher.stop(this.fetcher);
    this.busy = false;
    this.overlay = null;
    if (answer) {
      this.status = answer;
    }
    this.newSession();
    this.view = 'start';
  }

  /// New match (`new`) or Return to start (`return`) after full time. The launcher counts the
  /// finished match's worker as running for a moment after its close and refuses a new match
  /// meanwhile, so the page waits for the worker to end, then stops it as Return to start
  /// does; a finished match leaves no save. New match then opens match setup with the ended
  /// match's two clubs picked.
  async afterFullTime(id) {
    const session = this.session;
    if (this.busy || !session?.nextReady) {
      return;
    }
    this.busy = true;
    const picks = id === 'new' ? this.endedPicks(session) : null;
    await launcher.poll((s) => !s || (s['engine.state'] !== 'running' && s['engine.state'] !== 'starting'), {
      timeoutMs: WORKER_END_MS,
      fetcher: this.fetcher,
    });
    const answer = await launcher.stop(this.fetcher);
    this.busy = false;
    this.overlay = null;
    this.newSession();
    this.message = null;
    if (answer) {
      this.status = answer;
    } else {
      this.message = failure('The launcher did not answer the stop', '', 'Start a new match, or quit.');
    }
    if (picks) {
      this.saved = false;
      this.pick('home', picks.home);
      this.pick('away', picks.away);
      this.view = 'setup';
    } else {
      this.view = 'start';
    }
  }

  /// The ended match's clubs as setup picks: by club id, then by name; otherwise the last
  /// picks, or the first two teams.
  endedPicks(session) {
    const ids = (session.teams ?? []).map((team) => {
      const club = this.teams.find((t) => t.id === team['team.id']) ?? this.teams.find((t) => t.name === team['team.name']);
      return club?.id ?? null;
    });
    if (ids.length === 2 && ids[0] && ids[1] && ids[0] !== ids[1]) {
      return { home: ids[0], away: ids[1] };
    }
    if (this.picks.home && this.picks.away) {
      return { ...this.picks };
    }
    return { home: this.teams[0]?.id ?? null, away: this.teams[1]?.id ?? null };
  }

  /// Save and quit (or Quit on the start screen): the launcher stops the match, keeps its save
  /// and ends; the closed page shows what it answered.
  async quit() {
    if (this.busy) {
      return;
    }
    this.busy = true;
    const answer = await launcher.quit(this.fetcher);
    this.busy = false;
    this.overlay = null;
    this.session?.dispose?.();
    this.closed = answer?.closed ?? { match: false, saved: null };
    if (answer) {
      this.status = answer;
    }
    this.view = 'closed';
  }
}

/// A failure line: what failed, the launcher's reason when it gave one, and what to do next.
export function failure(what, reason, next) {
  const cause = reason ? `: ${reason.replace(/[.\s]+$/, '')}` : '';
  return `${what}${cause}. ${next}`;
}
