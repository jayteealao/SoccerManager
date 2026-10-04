// The match session: the match part of the former page (`web/main.mjs`) as one class whose
// `$state` fields the match screen reads. It connects to the engine the status names, keeps
// the whole match in memory, draws the pitch at the rendered tick once per animation frame,
// and brings every panel to that tick. It recovers the way the former page did: a dropped
// connection reconnects by itself, a stopped engine shows its panel, and a missing engine
// shows first run.
//
// Every panel reads the match at the rendered tick, never at the newest tick received, so no
// score, feed row or statistic shows before the pitch reaches it. A field changes only when
// its value changes, so a component redraws at most once per rendered tick.
//
// The dugout (`dugout.svelte.js`) holds the lineup editor, the tactics and the substitutions:
// the kick-off sends the lineup the manager picked, and every `ack`, `reject`, `change-state`,
// `advice` and change event goes to it.
//
// Six views share the session: Tactics, the read-only Pre-match line-ups, the match, the
// Touchline, the report and the replay. Before kick-off the path is Tactics (CONTINUE), then
// Pre-match (KICK OFF); Pre-match sends nothing until KICK OFF.
//
// A report opens when the pitch reaches a break, never while scrubbing, and never behind a
// rewind that jumps past one. The half-time report pauses playback until CONTINUE. The
// full-time report is loading until the engine closes the socket after full time, when every
// frame is stored and a saved file is complete; a stored match is ready at once. Every frame
// is kept as it arrived, so Save replay writes the stream byte for byte. A replay file and
// Replay the whole match play in the replay view; a live stream stays on the match screen.
//
// Skip to result pauses playback and opens the Skip decision. Keep watching goes back to the
// paused match at the same tick. Confirm sends `skip`: the engine plays the rest of the same
// match at full speed, and the session stores every frame as usual but sends no pause, start
// or seen and draws none of it, so no report opens at a break. The report shows "the engine
// plays the rest" until the full-time whistle, then stores, and is ready at the engine's clean
// close, marked with the skip point; the match behind it moves to full time. The skip mark
// lives in the session only: a saved replay keeps the stream's bytes.
//
// The other grounds of the matchday (`matchday.js`) arrive as `matchday`, `ground-event` and
// `ground-progress` messages. The list follows the rendered tick like every other panel, and
// is worked out again only when the rendered tick crosses a simulated second or a ground
// message arrives, so the frame loop does no new work per frame. They are not part of the
// match: no saved replay keeps them, and a replay shows none.

import { COMPONENT_COUNT, decodeInto, newFrame } from './decode.js';
import { Dugout } from './dugout.svelte.js';
import { FeedBatcher, feedRow, minuteStamp } from './feed.js';
import { BANNER_TOTAL_MS, bannerText } from './goal-moment.js';
import { History } from './history.js';
import { between } from './interpolate.js';
import * as launcher from './launcher.js';
import { LeadControl, SeenReport } from './lead.js';
import { lineupModel } from './lineups.js';
import { KIND, MatchState } from './match-state.js';
import { addEvent, addProgress, groundsAt, newMatchday } from './matchday.js';
import { DEFAULT_GROUND, Pitch, groundOf, readTokens } from './pitch.js';
import { Playback } from './playback.js';
import { formationName, kickOffSheet, rosterSheet, rulePackRows, squadSheet } from './prematch.js';
import { backoff, clockAt, loadingSteps, panelModel, stepOptions } from './recovery.js';
import { FrameStore, frameText, readReplay, writeReplay } from './replay-file.js';
import { ReportClock, reportModel } from './report.js';
import { Scheduler, TICKS_PER_SECOND } from './schedule.js';
import { fixtureTitle, scorerLines } from './scoreboard.js';
import { signal } from './signal.js';
import { TICKS_PER_MINUTE, totalMinutes } from './skip.js';
import { MatchSocket, socketAddress } from './socket.js';
import { Stoppages, stopsPlay } from './stoppages.js';

/// The screens the match screen can show.
export const SCREENS = Object.freeze([
  'loading',
  'kickoff',
  'live',
  'paused',
  'error',
  'first-run',
  'reconnecting',
  'full-time',
]);

/// A notice's kind, as the Notice component colours it, and the word it always carries.
export const NOTICES = Object.freeze({
  reconnect: { kind: 'warn', word: 'RECONNECTING' },
  connected: { kind: 'good', word: 'CONNECTED' },
  lag: { kind: 'mid', word: 'LAG' },
  end: { kind: 'neutral', word: 'FULL TIME' },
  error: { kind: 'bad', word: 'STREAM ENDED' },
  skip: { kind: 'warn', word: 'NO SKIP' },
});

/// The one next action per screen, as the cyan action block names it.
export const ACTIONS = Object.freeze({
  loading: 'Please wait',
  kickoff: 'Kick off',
  live: 'Pause',
  paused: 'Resume',
  'first-run': 'Open replay',
  reconnecting: 'Reconnecting',
  'full-time': 'Report',
});

/// The state word the date block shows above the clock, per screen.
const DATE_WORDS = Object.freeze({
  loading: 'GETTING READY',
  kickoff: 'KICK-OFF',
  live: 'LIVE',
  paused: 'PAUSED',
  error: 'STOPPED',
  'first-run': 'NOT CONNECTED',
  reconnecting: 'RECONNECTING',
  'full-time': 'FULL TIME',
});

/// The messages of the other grounds; none is part of the match.
const GROUND_MESSAGES = new Set(['matchday', 'ground-event', 'ground-progress']);

const MS = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

export class MatchSession {
  // What the screen reads. Everything else on the class is the session's own machinery.
  screen = $state('loading');
  steps = $state.raw(loadingSteps(0));
  teams = $state.raw(null);
  score = $state.raw([0, 0]);
  scorers = $state.raw(['', '']);
  tick = $state(0);
  clockText = $state('00:00');
  period = $state('1ST HALF');
  feedRows = $state.raw([]);
  spoken = $state('');
  stats = $state.raw(null);
  notice = $state.raw(null);
  panel = $state.raw(null);
  speed = $state(1);
  effectiveSpeed = $state(1);
  playing = $state(true);
  scrubMax = $state(1);
  banner = $state.raw(null);
  engineWord = $state('');
  engineVersion = $state(null);
  stored = $state(false);
  busy = $state(false);
  /// The view the tabs show: `tactics`, `prematch`, `match`, `touchline`, `report` or
  /// `replay`.
  view = $state('match');
  /// The report on show or last shown: `{ kind, tick, state, model }`, where `kind` is
  /// `half-time` or `full-time` and `state` is `loading` or `ready`. Null before the first.
  report = $state.raw(null);
  /// The replay saved last: `{ name, size, hash }`, or null.
  saved = $state.raw(null);
  /// `true` while the frames of the match are being written to a file.
  saving = $state(false);
  /// The hello of the match shown, as the Pre-match line-ups and the Touchline read it.
  hello = $state.raw(null);
  /// The ground the match shown is played on, in metres, from its hello: the pitch draws it.
  ground = DEFAULT_GROUND;
  /// The launcher's `resume` block while the Resume a saved match screen shows why a save
  /// cannot continue; null otherwise.
  resumeInfo = $state.raw(null);
  /// The Touchline strip's play facts at the rendered tick: whether play is stopped, the
  /// whole seconds to the next known stoppage (or null), and each team's substitutions and
  /// windows used.
  play = $state.raw({ stopped: false, nextIn: null, subsUsed: [0, 0], windowsUsed: [0, 0] });
  /// The skip to the result: `{ state, from, newest }` while it is decided or plays, where
  /// `state` is `deciding`, `playing`, `storing` or `ready`, `from` the tick the player
  /// skipped at, and `newest` the newest tick received while the engine plays the rest
  /// (moved once a minute). Null with no skip.
  skip = $state.raw(null);
  /// The other grounds: the round, every ground event that arrived and how far each ground
  /// has played (`matchday.js`), or null before the `matchday` message and for a replay.
  matchday = $state.raw(null);
  /// What the other-grounds list shows at the rendered tick (`groundsAt`).
  grounds = $state.raw(groundsAt(null, 0));
  /// `false` when the player switched the commentary off in Settings; the match screen then
  /// hides the commentary column.
  commentary = $state(true);
  /// The front door's Menu: called by the Menu button and Esc on the match views; null when
  /// the page has no start screen to return to, so the shell draws no Menu button.
  onMenu = $state.raw(null);
  /// `true` while the in-match menu is open over this session's views.
  menuOpen = $state(false);
  /// The front door's next steps after full time, called with `new` or `return`; null when
  /// the page has no start screen, so the report offers no Next list.
  onNextStep = $state.raw(null);

  /// `fetcher`, `timers`, `raf` and `now` are the browser's unless a test passes its own.
  /// `settings` (`{ speed, commentary }`) are the player's from the start screen: the speed a
  /// match starts at and whether the commentary shows; with none, 1x and on. `onLeave` is the
  /// start screen's way back, called when a replay opened from it closes or cannot be read.
  constructor({
    fetcher,
    timers = globalThis,
    raf = globalThis.requestAnimationFrame?.bind(globalThis),
    now = () => globalThis.performance.now(),
    doc = globalThis.document,
    download = null,
    settings = null,
    onLeave = null,
  } = {}) {
    this.fetcher = fetcher;
    this.timers = timers;
    this.raf = raf;
    this.now = now;
    this.doc = doc;
    this.startSpeed = settings?.speed ?? 1;
    /// `false` when the player switched the commentary off in Settings.
    this.commentary = settings?.commentary ?? true;
    this.onLeave = onLeave;
    /// `true` once the session is disposed: its frame loop stops and its socket is closed.
    this.disposed = false;
    /// Whether the match played when the in-match menu paused it.
    this.playingBeforeMenu = null;
    /// Hands a saved file to the person: the browser's download unless a test passes its own.
    this.download = download ?? ((bytes, name) => this.browserDownload(bytes, name));

    this.scheduler = new Scheduler();
    this.stoppages = new Stoppages();
    this.match = new MatchState();
    this.lead = new LeadControl();
    this.seen = new SeenReport();
    this.batcher = new FeedBatcher();
    this.rendered = new Int16Array(COMPONENT_COUNT);
    this.earlier = new Int16Array(COMPONENT_COUNT);
    this.later = new Int16Array(COMPONENT_COUNT);
    this.previous = newFrame();
    this.incoming = newFrame();

    this.history = null;
    this.pitch = null;
    this.canvas = null;
    /// The pitch canvas of each view that draws one: the match screen's, and the replay's.
    this.canvases = {};
    /// Every frame of the match as it arrived, for the replay file.
    this.frames = new FrameStore();
    /// The record of a loaded version-4 replay, kept so saving it writes the same kind of file.
    this.loadedRecord = null;
    this.helloVersion = null;
    this.reportClock = new ReportClock();
    /// The view a report returns to, and the view the replay returns to.
    this.reportFrom = 'match';
    this.replayFrom = 'match';
    /// `true` once the engine closed the socket after full time: every frame is stored.
    this.streamEnded = false;
    this.lastSaved = null;
    this.socket = null;
    this.playback = null;
    this.status = null;
    this.matchId = null;
    this.renderedTick = 0;
    this.kickedOff = false;
    this.resuming = false;
    this.reconnectAttempt = 0;
    this.scrubbing = false;
    this.resumeAfterScrub = true;
    this.lastRewind = null;
    this.goalsShown = 0;
    this.lastFlushTick = -1;
    /// Every seek the page made, as the ticks it jumped across: a ground's goal inside one
    /// shows with no outline, so a rewind never replays one and a jump never shows one.
    this.groundSeeks = [];
    /// The simulated second the other-grounds list was last worked out for.
    this.groundSecond = -1;
    this.goalShownAtTick = null;
    this.bannerTimer = null;
    this.stepShown = 0;
    /// What the loading steps add for a saved match: the engine version when another
    /// release's engine plays it, and the tick it continues from.
    this.stepOpts = { version: null, resumeTick: null };
    this.teamNames = new Map();
    this.halfTimes = 0;
    /// The type of every command sent, in order, for the browser drives.
    this.sent = [];
    /// The wall time the skip was sent, and whether a reconnect owes the engine the skip.
    this.skipSentAt = 0;
    this.skipOwed = false;
    /// The report on show before the skip, put back when the engine refuses the skip.
    this.reportBeforeSkip = null;
    this.dugout = new Dugout({
      send: (command) => this.command(command),
      onStart: () => this.started(),
    });
  }

  // ---- What the header and the strip show -------------------------------------------------

  /// The fixture, with the score once the match is under way.
  get title() {
    if (this.screen === 'first-run') {
      return 'Touchline';
    }
    const names = this.teams ? this.teams.map((t) => t['team.name']) : null;
    const scored = this.screen === 'live' || this.screen === 'paused' || this.screen === 'full-time';
    return fixtureTitle(names, scored ? this.score : null);
  }

  /// The engine word and version, or what the screen is waiting for.
  get subtitle() {
    switch (this.screen) {
      case 'loading':
        return 'Getting the match ready';
      case 'first-run':
        return 'First run · no engine found';
      default: {
        const words = this.engineVersion ? `${this.engineWord} · v${this.engineVersion}` : this.engineWord;
        return this.screen === 'full-time' && this.skip?.state === 'ready'
          ? `${words} · skipped from ${clockAt(this.skip.from)}`
          : words;
      }
    }
  }

  /// The date block: the clock state word, then the clock.
  get dateWord() {
    return DATE_WORDS[this.screen];
  }

  get dateClock() {
    if (this.screen === 'first-run') {
      return '—';
    }
    if (this.screen === 'error' && this.status?.['snapshot.tick'] != null) {
      return clockAt(this.status['snapshot.tick']);
    }
    // Full time shows the match's final clock, whatever minute the stopped pitch shows.
    if (this.screen === 'full-time' && this.match.fullTimeTick !== null) {
      return clockAt(this.match.fullTimeTick);
    }
    return this.clockText;
  }

  /// The one next action.
  get action() {
    if (this.screen === 'error') {
      if (this.panel?.actions.includes('new-match')) {
        return 'New match';
      }
      if (this.panel?.actions.includes('restart')) {
        return 'Restart';
      }
      return this.panel?.actions.includes('open-replay') ? 'Open replay' : 'Abandon';
    }
    if (this.screen === 'kickoff' && this.kickingOff) {
      return 'Kicking off';
    }
    if (this.screen === 'kickoff' && this.dugout.preMatch && this.view === 'tactics') {
      return 'Continue';
    }
    return ACTIONS[this.screen];
  }

  /// `true` while the action block has nothing to do: the wait before the engine answers, a
  /// reconnect, or a restart or abandon on its way.
  get actionBusy() {
    return (
      this.busy ||
      this.screen === 'loading' ||
      this.screen === 'reconnecting' ||
      this.kickingOff ||
      (this.screen === 'kickoff' && this.dugout.preMatch && !this.lineupReady)
    );
  }

  /// `true` when the lineup the manager picked may kick off.
  get lineupReady() {
    this.dugout.version;
    return this.dugout.editor ? this.dugout.editor.ready : true;
  }

  /// The tag under the score: its words and the state colour it sits on.
  get tag() {
    switch (this.screen) {
      case 'loading':
        return { text: 'GETTING READY', tone: 'cyan', live: false };
      case 'kickoff':
        return { text: `KICK-OFF · ${this.clockText}`, tone: 'cyan', live: false };
      case 'live':
        return { text: `${this.clockText} · ${this.period}`, tone: 'cyan', live: true };
      case 'paused':
        return { text: `PAUSED · ${this.clockText}`, tone: 'cyan', live: false };
      case 'error':
        return { text: 'ENGINE STOPPED', tone: 'bad', live: false };
      case 'first-run':
        return { text: 'SETUP', tone: 'mid', live: false };
      case 'full-time':
        return { text: `FULL TIME · ${this.dateClock}`, tone: 'final', live: false };
      default:
        return { text: `RECONNECTING · ${this.clockText}`, tone: 'warn', live: false };
    }
  }

  // ---- Start-up -----------------------------------------------------------------------------

  /// Waits for the engine and connects, or shows first run or the error panel.
  async start() {
    this.loop();
    this.setStep(0);
    const engine = await launcher.poll((s) => !s || s['engine.state'] !== 'starting', {
      timeoutMs: 60_000,
      fetcher: this.fetcher,
    });
    this.status = engine;
    this.useStepOptions(engine);
    if (engine?.['engine.state'] === 'running' && engine['socket.port']) {
      this.setStep(1);
      this.connect(engine);
      return;
    }
    this.showPanel(panelModel(engine) ?? panelModel(null));
  }

  /// Names the engine version and the resume point in the loading steps, as `status` says.
  useStepOptions(status) {
    this.stepOpts = stepOptions(status);
    this.steps = loadingSteps(this.stepShown, this.stepOpts);
  }

  /// A view's pitch canvas, once its screen has drawn it: `match` (the match screen) or
  /// `replay`. The pitch draws on the replay's canvas while the replay view is open, and on
  /// the match screen's otherwise.
  attachCanvas(canvas, where = 'match') {
    this.canvases[where] = canvas;
    this.selectCanvas();
  }

  /// The screen took its canvas away (the replay view closed).
  detachCanvas(canvas, where = 'match') {
    if (this.canvases[where] === canvas) {
      delete this.canvases[where];
      this.selectCanvas();
    }
  }

  selectCanvas() {
    // The replay and the Skip decision draw on their own canvas while they are open.
    const own = this.view === 'replay' || this.view === 'skip' ? this.canvases[this.view] : null;
    const canvas = own ?? this.canvases.match ?? null;
    if (canvas === this.canvas) {
      return;
    }
    this.canvas = canvas;
    this.pitch = null;
    this.makePitch();
  }

  makePitch() {
    if (!this.canvas || !this.teams || this.pitch) {
      return;
    }
    // A canvas with no 2D context (a test's document) draws nothing and fails nothing.
    if (!this.canvas.getContext?.('2d')) {
      return;
    }
    const kits = this.teams.map((t) => ({
      primary: t['team.kit.primary'],
      secondary: t['team.kit.secondary'],
    }));
    this.pitch = new Pitch(this.canvas, kits, readTokens(this.doc), {
      width: Number(this.canvas.dataset?.width) || undefined,
      height: Number(this.canvas.dataset?.height) || undefined,
      ground: this.ground,
    });
    if (this.history && this.history.count > 0 && this.history.tickAt(this.renderedTick, this.earlier)) {
      this.pitch.draw(this.earlier);
    } else {
      this.pitch.clear();
    }
  }

  loop() {
    if (!this.raf || this.looping) {
      return;
    }
    this.looping = true;
    const step = (timestamp) => {
      if (this.disposed) {
        return;
      }
      this.raf(step);
      this.frame(timestamp);
    };
    this.raf(step);
  }

  /// Ends the session: the frame loop stops, the banner timer is cleared, and the socket is
  /// closed without a recovery. The front door makes a new session for the next match.
  dispose() {
    this.disposed = true;
    this.clearBanner();
    if (this.socket) {
      this.socket.onClose = null;
      this.socket.close();
      this.socket = null;
    }
  }

  /// The in-match menu opened: the match pauses, and the state it had is kept.
  pauseForMenu() {
    if (this.playingBeforeMenu === null) {
      this.playingBeforeMenu = this.playing;
    }
    this.setPlaying(false);
  }

  /// The in-match menu closed with Resume match: the match plays on if it played before.
  resumeFromMenu() {
    const before = this.playingBeforeMenu;
    this.playingBeforeMenu = null;
    if (before) {
      this.setPlaying(true);
    }
  }

  setStep(current) {
    if (current === this.stepShown && this.steps) {
      return;
    }
    this.stepShown = current;
    this.steps = loadingSteps(current, this.stepOpts);
  }

  connect(status) {
    // A session ended while it waited for the launcher opens no socket: the engine takes one
    // connection, and a stray one would take the next match's.
    if (this.disposed) {
      return;
    }
    this.socket = new MatchSocket(socketAddress(status['socket.port'], status['protocol.version']), {
      onHello: (hello) => this.onHello(hello),
      onTick: (buffer) => this.onTick(buffer),
      onMessage: (message) => this.onMessage(message),
      onRaw: (data, message) => this.onRaw(data, message),
    });
    this.socket.onClose = (close) => this.onClose(close);
  }

  // ---- The stream ---------------------------------------------------------------------------

  onHello(hello) {
    this.reconnectAttempt = 0;
    if (this.history && hello['match.id'] === this.matchId) {
      // The same match, after a reconnect or a restart. The stores are kept, and the first
      // tick frame, a keyframe one tick past the stoppage it resumes from, says where to cut.
      this.resuming = true;
      // A skip the engine was playing when the connection dropped is sent again once the
      // resumed match's first tick arrives: each connection starts on a new gate.
      this.skipOwed = this.skip?.state === 'playing';
      this.engineWord = 'Engine connected';
      this.panel = null;
      this.busy = false;
      if (this.screen !== 'full-time') {
        this.screen = this.playing ? 'live' : 'paused';
      }
      this.setNotice('connected', 'Connected again. Play resumes at the last stoppage.');
      return;
    }
    this.matchId = hello['match.id'];
    this.setStep(2);
    this.begin(hello);
  }

  begin(hello, { stored = false } = {}) {
    this.history = new History(hello.ticks_expected);
    this.helloVersion = hello['protocol.version'] ?? null;
    this.reportClock.reset();
    this.report = null;
    this.skip = null;
    this.skipOwed = false;
    this.streamEnded = false;
    this.matchday = null;
    this.groundSeeks = [];
    this.teams = hello.teams;
    this.hello = hello;
    this.ground = groundOf(hello);
    this.teamNames = new Map(hello.teams.map((t) => [t['team.id'], t['team.name']]));
    this.engineVersion = hello['engine.version'] ?? null;
    this.engineWord = stored ? 'Replay' : 'Engine connected';
    this.scrubMax = Math.max(1, hello.ticks_expected);
    this.stored = stored;
    this.playback = new Playback({
      scheduler: this.scheduler,
      onSpeed: (requested, effective) => {
        this.speed = requested;
        this.effectiveSpeed = effective;
      },
      onNotice: (text) => this.setNotice(text ? 'lag' : null, text),
    });
    this.playback.select(this.startSpeed);
    this.pitch = null;
    this.makePitch();
    this.dugout.begin(hello, { stored });
    this.flush(0, { seek: true });
    this.screen = stored ? 'live' : 'kickoff';
    // Before kick-off the manager starts on the Tactics screen when there is a lineup to
    // pick; a stored match opens on the match.
    this.view = this.dugout.preMatch ? 'tactics' : 'match';
    // A saved match the launcher resumed is already under way: the engine streams at once,
    // with no lineup to pick and no kick-off, so the page opens on the live match.
    if (!stored && this.status?.['match.resumed_from'] != null) {
      this.dugout.resumeLive();
      this.kickedOff = true;
      this.setPlaying(true);
      this.screen = 'live';
      this.view = 'match';
      this.loadEarlierEvents();
    }
  }

  /// The events a resumed match played before its save. The engine streams only what follows
  /// the save, so the score, the half, the scorers and the feed start from these, read from
  /// the match's own events file through the launcher.
  async loadEarlierEvents() {
    const matchId = this.matchId;
    const rows = await launcher.fetchEarlierEvents(this.fetcher);
    if (this.matchId !== matchId || this.stored) {
      return;
    }
    for (const row of rows) {
      this.match.add({ ...row, type: 'event' });
    }
    signal('viewer.earlier_events', { 'match.id': matchId, rows: rows.length });
  }

  /// The kick-off. With a lineup to pick, the dugout sends it and the match starts on the
  /// engine's acknowledgement; an engine that takes no lineup starts at once with the
  /// computer manager's.
  kickOff() {
    if (this.screen !== 'kickoff' || !this.socket) {
      return;
    }
    if (this.dugout.preMatch) {
      this.dugout.kickOff();
      return;
    }
    if (this.dugout.phase === 'kicking-off') {
      return;
    }
    this.started();
    signal('viewer.kick_off', { lineup: null, bench: null, patch: null });
  }

  /// The engine accepted the lineup, or needs none: the pitch is reported at tick 0 before
  /// the start, so the engine is held near the pitch from its first tick.
  started() {
    this.command({ type: 'seen', tick: 0 });
    this.command({ type: 'start' });
    this.kickedOff = true;
    this.setPlaying(true);
    this.screen = 'live';
    // The match opens on the pitch, as the former page's editor gave the pitch back.
    this.view = 'match';
  }

  /// `true` while the kick-off waits for the engine's answer to the lineup.
  get kickingOff() {
    return this.dugout.phase === 'kicking-off';
  }

  /// Opens the Match, the Tactics or the Touchline view. The Pre-match line-ups open only by
  /// CONTINUE.
  show(view) {
    this.view = view === 'tactics' || view === 'touchline' ? view : 'match';
  }

  /// CONTINUE on Tactics: with a legal lineup before kick-off, opens the Pre-match line-ups.
  /// It sends nothing.
  continue() {
    if (this.screen !== 'kickoff' || !this.dugout.preMatch || !this.lineupReady) {
      return false;
    }
    this.view = 'prematch';
    return true;
  }

  /// "Change on Tactics": back from the Pre-match line-ups, with the lineup unchanged.
  back() {
    if (this.view === 'prematch' && this.dugout.preMatch) {
      this.view = 'tactics';
    }
  }

  /// The Pre-match line-ups: both sheets, both formations, the kick-off places and the
  /// rule-pack checks. The home sheet is the lineup the manager set on Tactics; the other
  /// comes from the hello. Null until a hello with a lineup to pick.
  sheet() {
    this.dugout.version;
    const editor = this.dugout.editor;
    const hello = this.hello;
    if (!hello || !editor) {
      return null;
    }
    const schema = hello.tactics;
    const message = editor.message();
    const home = squadSheet(this.dugout.squad, message.lineup, message.bench);
    const away = rosterSheet(hello.teams[1]);
    const shapes = [
      schema?.formations?.[editor.formation]?.name ?? '',
      formationName(hello.teams[1], schema),
    ];
    return {
      teams: hello.teams,
      home,
      away,
      shapes,
      dots: kickOffSheet(schema, [
        { slots: schema?.formations?.[editor.formation]?.slots, eleven: home.eleven },
        { formation: shapes[1], eleven: away.eleven },
      ]),
      rules: rulePackRows(hello),
    };
  }

  /// Every frame, exactly as the socket handed it over, before it is decoded. The first hello
  /// of a match opens a new store; a reconnect's first tick cuts every store back first.
  onRaw(data, message = null) {
    if (typeof data === 'string') {
      if (message?.type === 'hello' && message['match.id'] !== this.matchId) {
        this.frames = new FrameStore();
        this.loadedRecord = null;
      }
      if (!GROUND_MESSAGES.has(message?.type)) {
        this.frames.addText(data, message?.type);
      }
      return;
    }
    if (this.resuming) {
      const first = new DataView(data).getUint32(1, true);
      this.resumeAt(first - 1);
    }
    this.frames.addBinary(data);
  }

  /// One tick frame. `live` is false for a frame read from a replay file, which has no socket
  /// to pace and no arrival rate to measure.
  onTick(buffer, live = true) {
    const result = decodeInto(buffer, this.previous, this.incoming);
    if (!result) {
      return;
    }
    const history = this.history;
    history.append(this.incoming.tick, this.incoming.components);
    if (live) {
      this.socket?.noteTick(this.incoming.tick);
      this.playback.noteArrival(this.now(), this.incoming.tick, this.incoming.tick - this.renderedTick);
      this.pace();
      if (this.skip?.state === 'playing') {
        this.skipProgress(this.incoming.tick);
      }
    }
    if (result.kind !== 'delta' && live) {
      history.report();
      history.measurePage();
    }
    if (result.kind === 'restart') {
      this.stoppages.add(this.incoming.tick);
    }
    // A match can run past the announced ticks; the timeline's end follows the newest tick.
    if (history.newestTick > history.ticksExpected) {
      this.scrubMax = history.scrubLimit;
    }
    [this.previous, this.incoming] = [this.incoming, this.previous];
    this.setStep(3);
    // An engine that plays on its own (a recording served as a replay) needs no kick-off.
    if (this.screen === 'kickoff') {
      this.screen = 'live';
    }
    if (live) {
      this.drawStoredWhistle();
    }
  }

  onMessage(message) {
    if (GROUND_MESSAGES.has(message.type)) {
      this.onGround(message);
      return;
    }
    if ((message.type === 'ack' || message.type === 'reject') && message.command === 'skip') {
      this.onSkipAnswer(message);
      return;
    }
    if (message.type === 'ack' || message.type === 'reject') {
      if (!this.stored) {
        this.dugout.answer(message);
      }
      return;
    }
    if (message.type === 'change-state') {
      if (!this.stored) {
        this.dugout.onChangeState(message);
      }
      return;
    }
    if (message.type === 'advice') {
      if (!this.stored) {
        this.dugout.onAdvice(message);
      }
      return;
    }
    if (message.type === 'event' && message['event.type'] === KIND.tacticsChange) {
      this.dugout.onChangeEvent(message);
    }
    this.match.add(message);
    if (stopsPlay(message)) {
      this.stoppages.add(message.tick);
    }
    // Full time marks the real end, so the timeline stops at the last tick that arrived.
    if (message.type === 'event' && message['event.type'] === KIND.fullTime && this.history) {
      this.scrubMax = Math.max(1, this.history.newestTick);
      // The engine reached full time while it played the rest: the report is written and
      // the whole match is being stored.
      if (this.skip?.state === 'playing') {
        this.skip = { ...this.skip, state: 'storing', newest: message.tick };
        if (this.report?.skippedFrom !== undefined) {
          const model = reportModel(this.match.events, message.tick, this.teams ?? []);
          this.report = { ...this.report, state: 'storing', tick: message.tick, model };
        }
        this.drawStoredWhistle();
      }
    }
  }

  /// While a skipped match is stored, the match behind the report moves to the whistle once
  /// its tick is in, so the score strip's figures and scorers are the final ones.
  drawStoredWhistle() {
    if (this.skip?.state === 'storing' && !this.skip.drawn && this.history?.newestTick >= this.skip.newest) {
      this.skip = { ...this.skip, drawn: true };
      this.rewind(this.skip.newest);
    }
  }

  /// One message of the other grounds. A `matchday` on a reconnect replaces the round, and
  /// the engine sends every event up to its tick again; an exact repeat is dropped.
  onGround(message) {
    if (this.stored) {
      return;
    }
    if (message.type === 'matchday') {
      this.matchday = newMatchday(message);
      signal('viewer.matchday', { fixtures: this.matchday.fixtures.length, round: this.matchday.round });
    } else if (!this.matchday) {
      return;
    } else if (message.type === 'ground-event') {
      const before = this.matchday;
      this.matchday = addEvent(before, message, this.renderedTick, this.groundSeeks.length);
      if (message.late && this.matchday !== before) {
        signal('viewer.ground_late', {
          fixture: message.fixture,
          tick: message.tick,
          rendered_tick: this.renderedTick,
        });
      }
    } else {
      this.matchday = addProgress(this.matchday, message);
    }
    this.updateGrounds(true);
  }

  /// Works the other-grounds list out again when the rendered tick has crossed a simulated
  /// second since the last time, or at once when `force` is set.
  updateGrounds(force = false) {
    const second = Math.floor(this.renderedTick / TICKS_PER_SECOND);
    if (!force && second === this.groundSecond) {
      return;
    }
    this.groundSecond = second;
    // At full time the stopped match screen reads every ground final, as the report does:
    // every event that arrived, whatever its tick, until the pitch is moved back before the
    // whistle.
    const final =
      this.screen === 'full-time' &&
      this.match.fullTimeTick !== null &&
      this.renderedTick >= this.match.fullTimeTick;
    this.grounds = groundsAt(this.matchday, final ? Number.MAX_SAFE_INTEGER : this.renderedTick, {
      skip: this.skip,
      seeks: this.groundSeeks,
      total: totalMinutes(this.hello?.ticks_expected),
      stored: this.stored,
      final,
    });
  }

  /// Cuts every store back to `tick`, where the resumed match continues. The engine plays the
  /// later ticks again; keeping both copies would draw and count them twice.
  resumeAt(tick) {
    this.resuming = false;
    const from = this.history.newestTick;
    const gap = this.history.truncate(tick);
    this.stoppages.truncate(tick);
    this.match.truncate(tick);
    this.frames.truncate(tick);
    this.previous.tick = tick;
    if (this.renderedTick > tick) {
      this.rewind(tick);
    } else {
      this.flush(this.renderedTick, { seek: true });
    }
    this.setNotice(null);
    signal('viewer.resumed', { 'match.id': this.matchId, from_tick: from, to_tick: tick, gap });
    if (gap > 0) {
      signal('viewer.resume_gap', { ticks: gap });
    }
    if (this.kickedOff && this.socket && !this.skipRunning) {
      this.command({ type: 'seen', tick: Math.min(this.renderedTick, tick) });
    }
  }

  // ---- One animation frame --------------------------------------------------------------------

  frame(timestamp) {
    const history = this.history;
    if (!history || history.count === 0) {
      return;
    }
    const step = this.scheduler.advance(timestamp, history.newestTick, history.firstTick);
    if (!history.tickAt(step.from, this.earlier)) {
      return;
    }
    if (!history.tickAt(step.to, this.later)) {
      this.later.set(this.earlier);
    }
    between(this.earlier, this.later, step.fraction, this.rendered);
    this.pitch?.draw(this.rendered);
    this.renderedTick = step.from;
    this.flush(this.renderedTick, { seek: false });
    // A report opens when the pitch reaches its break, never while scrubbing.
    if (!this.scrubbing) {
      const due = this.reportClock.due(this.match.events, this.renderedTick);
      if (due) {
        this.openReport(due.kind, due.tick);
      }
    }
    this.pace();
    this.reportSeen(timestamp);
  }

  /// Brings every panel to `tick`. A seek never replays a goal moment: the banner belongs to
  /// the frame in which play crosses the goal, not to a rewind that lands after it.
  flush(tick, { seek }) {
    const previous = this.lastFlushTick;
    const state = this.match.at(tick);
    this.lastFlushTick = tick;
    if (seek && previous >= 0 && previous !== tick) {
      this.groundSeeks.push({
        lo: Math.min(previous, tick),
        hi: Math.max(previous, tick),
        n: this.groundSeeks.length + 1,
      });
    }
    this.updateGrounds(seek);
    this.tick = tick;
    this.clockText = clockAt(tick);
    if (state.home !== this.score[0] || state.away !== this.score[1]) {
      this.score = [state.home, state.away];
    }
    const { reset, batch } = this.batcher.take(state.entries);
    if (reset || batch.length > 0) {
      const rows = batch.map((event) => feedRow(event, this.teamNames));
      this.feedRows = reset ? rows : [...this.feedRows, ...rows];
      // Goals and cards are read out as they are released, never again on a rewind.
      const said = reset ? null : rows.findLast((row) => row.announce);
      if (said) {
        this.spoken = `${said.minute} ${said.text}`;
      }
      this.scorers = scorerLines(state.goals, this.teams);
      this.halfTimes = state.entries.filter((e) => e['event.type'] === KIND.halfTime).length;
      this.period = state.fullTime ? 'FULL TIME' : this.halfTimes > 0 ? '2ND HALF' : '1ST HALF';
    }
    if (state.stats !== this.stats) {
      this.stats = state.stats;
    }
    this.dugout.update(tick, state);
    this.updatePlay(tick, state);

    const goals = state.goals.length;
    if (seek || tick < previous) {
      if (goals < this.goalsShown) {
        this.clearBanner();
      }
      this.goalsShown = goals;
      return;
    }
    if (goals > this.goalsShown) {
      const goal = state.goals[goals - 1];
      this.goalsShown = goals;
      this.showBanner(goal);
      this.goalShownAtTick = tick;
      signal('viewer.goal_moment', {
        goal_tick: goal.tick,
        rendered_tick: tick,
        prev_rendered_tick: previous,
        frame_delta: previous < goal.tick ? 0 : 1,
        'home.score': state.home,
        'away.score': state.away,
      });
    }
  }

  /// The Touchline's play facts. Play reads stopped for three seconds after a stoppage mark;
  /// the next stoppage is the next mark the engine has already sent, which is at most a few
  /// seconds ahead of the pitch.
  updatePlay(tick, state) {
    const TICKS_PER_SECOND = 50;
    const prev = this.stoppages.prev(tick + 1);
    const next = this.stoppages.next(tick);
    const stopped = prev !== null && tick - prev < 3 * TICKS_PER_SECOND;
    const nextIn = next === null ? null : Math.ceil((next - tick) / TICKS_PER_SECOND);
    const subsUsed = state.subsUsed ?? [0, 0];
    const windowsUsed = state.windowsUsed ?? [0, 0];
    const p = this.play;
    if (
      p.stopped !== stopped ||
      p.nextIn !== nextIn ||
      p.subsUsed[0] !== subsUsed[0] ||
      p.subsUsed[1] !== subsUsed[1] ||
      p.windowsUsed[0] !== windowsUsed[0] ||
      p.windowsUsed[1] !== windowsUsed[1]
    ) {
      this.play = { stopped, nextIn, subsUsed: [...subsUsed], windowsUsed: [...windowsUsed] };
    }
  }

  /// The goal banner over the pitch. It runs on wall time: the 1.5-second rule is about how
  /// long a person can read it, and a match-time timer would flash it at eight times speed.
  showBanner(goal) {
    this.clearBanner();
    this.banner = { id: goal.tick, text: bannerText(goal, this.teamNames, minuteStamp(goal)) };
    this.bannerTimer = this.timers.setTimeout(() => {
      this.banner = null;
      this.bannerTimer = null;
    }, BANNER_TOTAL_MS);
  }

  clearBanner() {
    if (this.bannerTimer !== null) {
      this.timers.clearTimeout(this.bannerTimer);
      this.bannerTimer = null;
    }
    this.banner = null;
  }

  /// Keeps a kicked-off engine a few seconds ahead of playback.
  pace() {
    if (!this.kickedOff || !this.socket || !this.history || this.skipRunning) {
      return;
    }
    const command = this.lead.next(
      this.history.newestTick - this.renderedTick,
      this.scheduler.speed,
      this.match.fullTimeTick !== null
    );
    if (command) {
      this.command({ type: command });
    }
  }

  /// Tells a kicked-off engine which tick the pitch shows, so it stays within its buffer.
  reportSeen(now) {
    if (!this.kickedOff || !this.socket || this.skipRunning) {
      return;
    }
    const seen = this.seen.next(this.renderedTick, now);
    if (seen !== null) {
      this.command({ type: 'seen', tick: seen });
    }
  }

  // ---- The controls -------------------------------------------------------------------------

  /// The action block: the screen's one next action. Opening a replay needs a file, which
  /// the screen asks the person for; `pickReplay` is how it asks.
  act(pickReplay = () => {}) {
    switch (this.screen) {
      case 'kickoff':
        if (this.dugout.preMatch && this.view === 'tactics') {
          return this.continue();
        }
        return this.kickOff();
      case 'live':
        return this.setPlaying(false);
      case 'paused':
        return this.setPlaying(true);
      case 'full-time':
        return this.reopenReport();
      case 'first-run':
        return pickReplay();
      case 'error':
        if (this.panel?.actions.includes('new-match')) {
          return this.newMatchEngine();
        }
        if (this.panel?.actions.includes('restart')) {
          return this.restartEngine();
        }
        return this.panel?.actions.includes('open-replay') ? pickReplay() : this.abandonEngine();
      default:
        return undefined;
    }
  }

  /// Plays or pauses the page's own playback. The engine is paced by `seen`, not by this.
  /// It changes only the live and paused screens, so full time never turns LIVE again.
  setPlaying(playing) {
    this.scheduler.setPlaying(playing);
    this.playing = playing;
    if (this.screen === 'live' || this.screen === 'paused') {
      this.screen = playing ? 'live' : 'paused';
    }
  }

  selectSpeed(speed) {
    this.playback?.select(speed);
  }

  /// Back to the newest tick received, playing.
  toNewest() {
    if (this.history && this.history.count > 0) {
      this.rewind(this.history.newestTick);
      this.setPlaying(true);
    }
  }

  /// Rewinds to the next or the previous stoppage.
  nextStop() {
    const next = this.stoppages.next(this.renderedTick);
    if (next !== null) {
      this.rewind(next);
    }
  }

  previousStop() {
    const prev = this.stoppages.prev(this.renderedTick);
    if (prev !== null) {
      this.rewind(prev);
    }
  }

  /// A scrub holds the frame it lands on; playback resumes when the drag ends.
  scrubTo(tick) {
    if (!this.scrubbing) {
      this.scrubbing = true;
      this.resumeAfterScrub = this.scheduler.playing;
      this.scheduler.setPlaying(false);
    }
    this.rewind(tick);
  }

  scrubEnd() {
    this.scrubbing = false;
    this.scheduler.setPlaying(this.resumeAfterScrub);
  }

  /// A rewind draws the stored tick itself, not an interpolation towards it. `exact` is
  /// measured: the drawn frame is compared with the stored tick component by component.
  rewind(tick) {
    if (!this.history) {
      return;
    }
    const from = this.renderedTick;
    // A rewind that jumps past a break opens no report behind it.
    if (tick > from) {
      this.reportClock.pass(this.match.events, tick);
    }
    this.scheduler.seek(tick);
    this.pitch?.clearTrail();
    const stored = this.history.tickAt(tick, this.earlier);
    let exact = false;
    if (stored) {
      between(this.earlier, this.earlier, 0, this.rendered);
      this.pitch?.draw(this.rendered);
      this.renderedTick = tick;
      this.flush(tick, { seek: true });
      exact = this.rendered.every((value, i) => value === this.earlier[i]);
      this.lastRewind = {
        tick,
        drawn: Array.from(this.rendered),
        stored: Array.from(this.earlier),
        exact,
      };
    }
    signal('viewer.rewind', { from_tick: from, to_tick: tick, stored, exact });
  }

  // ---- Notices and recovery -----------------------------------------------------------------

  /// One notice at a time. `kind` is a key of NOTICES, or null to take the notice down; its
  /// word always names it, so colour alone never carries the difference.
  setNotice(kind, message = null) {
    this.notice = kind ? { ...NOTICES[kind], message } : null;
  }

  showPanel(model) {
    // A skip the engine could not finish ends with it; the panel shows on the match screen.
    if (this.skip) {
      this.clearSkip();
    }
    this.panel = model;
    this.busy = false;
    this.screen = model.kind === 'first-run' ? 'first-run' : 'error';
    // A save no shipped engine can finish has its own screen.
    this.resumeInfo = model.kind === 'resume' ? (this.status?.resume ?? null) : null;
    if (model.kind === 'resume') {
      this.view = 'resume';
    }
    if (model.kind !== 'first-run') {
      this.setPlaying(false);
    }
    signal('viewer.recovery_panel', {
      kind: model.kind,
      reason: this.status?.['engine.reason'] ?? null,
    });
  }

  onClose({ clean }) {
    if (this.stored) {
      return;
    }
    // The engine closes the socket after full time on purpose, and that close is not a
    // fault: every tick is stored and the match plays back. Any other close is.
    if (this.match.fullTimeTick !== null) {
      this.engineWord = 'Engine finished';
      this.setNotice('end', 'Full time. The whole match is stored and plays back.');
      this.streamEnded = true;
      if (this.skipRunning) {
        this.finishSkip();
        return;
      }
      if (this.report?.kind === KIND.fullTime && this.report.state === 'loading') {
        this.report = { ...this.report, state: 'ready' };
      }
      return;
    }
    this.recover(!clean);
  }

  /// After a close before full time: reconnect while the engine is still running, or show
  /// what happened and what can still be done. A dropped connection never asks the manager.
  async recover(dropped) {
    this.screen = 'reconnecting';
    this.engineWord = 'Engine reconnecting';
    this.setNotice('reconnect', 'The connection to the engine dropped. Reconnecting.');
    for (;;) {
      const status = await launcher.fetchStatus(this.fetcher);
      if (this.disposed) {
        return;
      }
      this.status = status;
      const state = status?.['engine.state'];
      if (state === 'running' && status['socket.port']) {
        const delay = backoff(this.reconnectAttempt);
        this.reconnectAttempt += 1;
        signal('viewer.reconnecting', { attempt: this.reconnectAttempt, delay_ms: delay, dropped });
        await MS(delay);
        this.connect(status);
        return;
      }
      if (state === 'starting') {
        await MS(250);
        continue;
      }
      this.engineWord = 'Engine stopped';
      const model = panelModel(status);
      if (model) {
        this.setNotice(null);
        this.showPanel(model);
      } else {
        if (this.skip) {
          this.clearSkip();
        }
        this.screen = this.playing ? 'live' : 'paused';
        this.setNotice('error', 'The match is no longer live. Start the engine again to watch another.');
      }
      return;
    }
  }

  async restartEngine() {
    this.busy = true;
    await launcher.restart(this.fetcher);
    const status = await launcher.poll((s) => !s || s['engine.state'] !== 'starting', {
      timeoutMs: 60_000,
      fetcher: this.fetcher,
    });
    this.status = status;
    if (status?.['engine.state'] === 'running') {
      this.panel = null;
      this.busy = false;
      this.screen = 'reconnecting';
      this.setPlaying(true);
      this.engineWord = 'Engine reconnecting';
      this.setNotice('reconnect', 'Restarting from the last stoppage.');
      this.connect(status);
      return;
    }
    this.showPanel(panelModel(status) ?? panelModel(null));
  }

  /// After a save that could not resume: a fresh match with the launch's seed and teams.
  async newMatchEngine() {
    this.busy = true;
    await launcher.newMatch(this.fetcher);
    const status = await launcher.poll((s) => !s || s['engine.state'] !== 'starting', {
      timeoutMs: 60_000,
      fetcher: this.fetcher,
    });
    this.status = status;
    if (status?.['engine.state'] === 'running' && status['socket.port']) {
      this.panel = null;
      this.resumeInfo = null;
      this.busy = false;
      this.screen = 'loading';
      this.view = 'match';
      this.stepShown = -1;
      this.useStepOptions(status);
      this.setStep(1);
      this.connect(status);
      return;
    }
    this.showPanel(panelModel(status) ?? panelModel(null));
  }

  async abandonEngine() {
    this.busy = true;
    const after = await launcher.abandon(this.fetcher);
    this.status = after ?? { 'engine.state': 'abandoned' };
    if (this.socket) {
      this.socket.onClose = null;
      this.socket.close();
    }
    // The abandoned match can still be saved as far as it was received, or another opened.
    const model = panelModel({ 'engine.state': 'abandoned' });
    const actions = this.frames.count > 0 ? model.actions : model.actions.filter((a) => a !== 'save-replay');
    this.showPanel({ ...model, actions });
  }

  /// Plays a replay file with no engine: every stored frame goes through the same path live
  /// frames take, so playback and rewind are the same code as a live match.
  /// With `leave`, the replay was opened from the start screen: closing it, or a file that
  /// cannot be read, goes back there.
  async openReplay(bytes, name = 'replay', { leave = false } = {}) {
    let read;
    try {
      read = await readReplay(bytes);
    } catch (error) {
      const reason = error.reason ?? error.message;
      signal('viewer.replay_refused', { reason });
      if (leave && this.onLeave) {
        this.onLeave({ refused: reason });
        return;
      }
      this.showPanel({
        kind: 'replay-refused',
        word: 'Error',
        title: `The replay could not be read: ${reason}`,
        body: 'Choose another replay file.',
        actions: ['open-replay'],
      });
      // The refusal panel is on the match screen; a file opened from Tactics, the report or
      // the replay would otherwise be refused out of sight.
      this.view = 'match';
      return;
    }
    if (this.socket) {
      this.socket.onClose = null;
      this.socket.close();
      this.socket = null;
    }
    this.kickedOff = false;
    this.skip = null;
    this.skipOwed = false;
    this.match.clear();
    this.stoppages.truncate(-1);
    this.batcher = new FeedBatcher();
    this.goalsShown = 0;
    this.matchId = read.hello['match.id'];
    this.previous = newFrame();
    this.incoming = newFrame();
    this.renderedTick = 0;
    this.begin(read.hello, { stored: true });
    const frames = read.store;
    this.frames = frames;
    this.loadedRecord = read.record;
    for (let i = 1; i < frames.count; i += 1) {
      const { text, payload } = frames.frame(i);
      if (text) {
        const message = JSON.parse(frameText(payload));
        if (message.type !== 'hello') {
          this.onMessage(message);
        }
      } else {
        this.onTick(payload.slice().buffer, false);
      }
    }
    if (this.match.fullTimeTick !== null) {
      this.scrubMax = Math.max(1, this.history.newestTick);
    }
    this.panel = null;
    this.setNotice(null);
    this.setStep(3);
    this.rewind(this.history.firstTick);
    // The rewind to kick-off passed nothing; each break opens its report as play reaches it.
    this.reportClock.reset();
    this.setPlaying(true);
    this.screen = 'live';
    this.replayFrom = leave ? 'start' : 'match';
    this.view = 'replay';
    this.selectCanvas();
    this.spoken = 'Replay loaded. Playing from kick-off.';
    signal('viewer.replay_loaded', { frames: read.frames, ticks: read.ticks, name });
  }

  // ---- Skip to the result -------------------------------------------------------------------

  /// Sends one command to the engine and records its type. `false` when no socket is open.
  command(command) {
    if (!this.socket) {
      return false;
    }
    this.sent.push(command.type);
    return this.socket.send(command);
  }

  /// `true` while the engine plays the rest of a skipped match and the session waits for it.
  get skipRunning() {
    return this.skip?.state === 'playing' || this.skip?.state === 'storing';
  }

  /// Skip to result is offered on a kicked-off live engine match before its full time: a
  /// replay file ignores commands, and a match that has finished has nothing left to play.
  get canSkip() {
    // Read for the screens: the full-time whistle arrives with the ticks, never on its own.
    void this.tick;
    return (
      (this.screen === 'live' || this.screen === 'paused') &&
      this.kickedOff &&
      this.socket !== null &&
      !this.stored &&
      this.skip === null &&
      this.match.fullTimeTick === null
    );
  }

  /// Skip to result: pauses playback, as the pause control does, and opens the Skip decision
  /// at the tick the pitch shows. Nothing is sent yet.
  openSkip() {
    if (!this.canSkip) {
      return false;
    }
    this.setPlaying(false);
    this.skip = { state: 'deciding', from: this.renderedTick, newest: this.renderedTick };
    this.view = 'skip';
    this.selectCanvas();
    signal('viewer.skip_opened', { from_tick: this.renderedTick });
    return true;
  }

  /// Keep watching: back to the match at the same tick, still paused; with `play` (RESUME in
  /// the header), playing.
  keepWatching(play = false) {
    if (this.skip?.state !== 'deciding') {
      return;
    }
    this.skip = null;
    this.view = 'match';
    this.selectCanvas();
    if (play) {
      this.setPlaying(true);
    }
  }

  /// Confirm: skip to result. The engine plays the rest at full speed; the report shows it
  /// playing until full time.
  confirmSkip() {
    if (this.skip?.state !== 'deciding' || !this.socket) {
      return false;
    }
    const from = this.skip.from;
    this.command({ type: 'skip' });
    this.skipSentAt = this.now();
    this.skip = { state: 'playing', from, newest: Math.max(from, this.history?.newestTick ?? from) };
    this.reportBeforeSkip = this.report;
    this.reportFrom = 'match';
    this.report = {
      kind: KIND.fullTime,
      tick: null,
      state: 'playing-rest',
      model: reportModel(this.match.events, from, this.teams ?? []),
      skippedFrom: from,
    };
    this.view = 'report';
    this.selectCanvas();
    signal('viewer.skip_confirmed', { from_tick: from });
    this.spoken = 'Skipping to the result. The engine plays the rest of the match.';
    return true;
  }

  /// The engine's answer to `skip`. A refusal (an engine that cannot skip, or an older engine
  /// program that does not know the command) returns to the paused match with a notice.
  onSkipAnswer(message) {
    if (message.type === 'ack' || !this.skip) {
      return;
    }
    const from = this.skip.from;
    this.clearSkip();
    this.view = 'match';
    this.selectCanvas();
    if (this.history && this.renderedTick !== from) {
      this.rewind(from);
    }
    this.setNotice('skip', `The engine cannot skip to the result: ${message.reason}`);
    signal('viewer.skip_refused', { from_tick: from, reason: message.reason });
  }

  /// Moves the playing step's minute once a minute of the rest has arrived, and sends a skip
  /// a reconnect owes the engine.
  skipProgress(tick) {
    if (this.skipOwed) {
      this.skipOwed = false;
      this.command({ type: 'skip' });
    }
    if (Math.floor(tick / TICKS_PER_MINUTE) !== Math.floor(this.skip.newest / TICKS_PER_MINUTE)) {
      this.skip = { ...this.skip, newest: tick };
    }
  }

  /// The engine closed the socket after full time: every frame is stored. The report is
  /// ready with the skip point marked, and the match behind it moves to full time, with no
  /// goal banner and no report opening at the breaks it passes.
  finishSkip() {
    const from = this.skip.from;
    const tick = this.match.fullTimeTick ?? this.history.newestTick;
    this.skip = { state: 'ready', from, newest: tick };
    this.reportBeforeSkip = null;
    this.rewind(tick);
    this.enterFullTime();
    this.report = {
      kind: KIND.fullTime,
      tick,
      state: 'ready',
      model: reportModel(this.match.events, tick, this.teams ?? []),
      skippedFrom: from,
    };
    if (this.view !== 'report') {
      this.reportFrom = 'match';
      this.view = 'report';
      this.selectCanvas();
    }
    signal('viewer.skip_done', {
      from_tick: from,
      full_time_tick: tick,
      wall_ms: Math.round(this.now() - this.skipSentAt),
    });
    this.spoken = 'Full time. The report is open.';
  }

  /// Ends a skip that did not finish: the report on show before it comes back.
  clearSkip() {
    const playing = this.skipRunning;
    this.skip = null;
    this.skipOwed = false;
    if (playing) {
      this.report = this.reportBeforeSkip;
      if (this.view === 'report') {
        this.view = 'match';
        this.selectCanvas();
      }
    } else if (this.view === 'skip') {
      this.view = 'match';
      this.selectCanvas();
    }
    this.reportBeforeSkip = null;
  }

  // ---- The reports, saving and the replay view --------------------------------------------------

  /// Opens the half-time or full-time report at `tick`. The half-time report pauses playback
  /// until CONTINUE; the engine stays within its bounded lead meanwhile.
  openReport(kind, tick) {
    const model = reportModel(this.match.events, tick, this.teams ?? []);
    if (kind === KIND.halfTime) {
      this.setPlaying(false);
    }
    const ready = kind !== KIND.fullTime || this.stored || this.streamEnded;
    if (kind === KIND.fullTime) {
      this.enterFullTime();
    }
    if (this.view !== 'report') {
      this.reportFrom = ['touchline', 'tactics', 'replay'].includes(this.view) ? this.view : 'match';
    }
    this.report = { kind, tick, state: ready ? 'ready' : 'loading', model };
    this.view = 'report';
    this.selectCanvas();
    signal('viewer.report_shown', { kind, tick });
    this.spoken = kind === KIND.halfTime ? 'Half-time. The report is open.' : 'Full time. The report is open.';
  }

  /// The match is over: playback stops, and the screen holds FULL TIME. `setPlaying` changes
  /// only the live and paused screens, so no control, menu or reconnect can show LIVE again.
  enterFullTime() {
    this.setPlaying(false);
    this.screen = 'full-time';
    this.updateGrounds(true);
  }

  /// CONTINUE, Close or Back to the match on the report: the half-time report goes back to
  /// the view it opened over and resumes playback; the full-time report goes back to the
  /// match at full time.
  closeReport() {
    if (this.view !== 'report') {
      return;
    }
    if (this.report?.kind === KIND.halfTime) {
      this.setPlaying(true);
    }
    this.view = this.report?.kind === KIND.fullTime ? 'match' : this.reportFrom;
    this.selectCanvas();
  }

  /// REPORT at full time: the full-time report again, over the match.
  reopenReport() {
    if (this.report?.kind !== KIND.fullTime) {
      return false;
    }
    this.reportFrom = 'match';
    this.view = 'report';
    this.selectCanvas();
    signal('viewer.report_reopened', { tick: this.report.tick });
    return true;
  }

  /// `true` once the next steps may run: the full-time report is ready, which is after the
  /// engine's clean close, and no skip is still storing the match.
  get nextReady() {
    return this.report?.kind === KIND.fullTime && this.report.state === 'ready' && !this.skipRunning;
  }

  /// `true` when the page has a start screen to lead on to.
  get nextOffered() {
    return this.onNextStep !== null;
  }

  /// New match (`new`) or Return to start (`return`) from the full-time report. Nothing runs
  /// before the match is stored.
  nextStep(id) {
    if (!this.nextReady || !this.onNextStep) {
      return false;
    }
    signal('viewer.next_step', { choice: id });
    this.onNextStep(id);
    return true;
  }

  /// Play the replay from here at full time: the replay view, playing from the minute the
  /// stopped pitch shows, or from kick-off when it shows the end.
  playFromHere() {
    if (!this.history || this.history.count === 0) {
      return;
    }
    const end = this.match.fullTimeTick ?? this.history.newestTick;
    const from = this.renderedTick >= end ? this.history.firstTick : this.renderedTick;
    this.showReplay();
    this.rewind(from);
    this.setPlaying(true);
  }

  /// Replay the whole match: back to kick-off, playing, in the replay view.
  replayWhole() {
    if (!this.history || this.history.count === 0) {
      return;
    }
    this.showReplay();
    this.rewind(this.history.firstTick);
    this.setPlaying(true);
    signal('viewer.replay_whole', { from: this.replayFrom });
  }

  /// The Replay tab of the report: the replay view, where playback stands.
  showReplay() {
    if (!this.history || this.history.count === 0) {
      return;
    }
    if (this.view !== 'replay') {
      this.replayFrom = this.view === 'report' ? 'report' : 'match';
    }
    this.view = 'replay';
    this.selectCanvas();
  }

  /// CONTINUE on the replay: back to the report it came from, or to the match screen.
  closeReplay() {
    if (this.view !== 'replay') {
      return;
    }
    if (this.replayFrom === 'start' && this.onLeave) {
      this.onLeave({});
      return;
    }
    this.view = this.replayFrom === 'report' && this.report ? 'report' : 'match';
    this.selectCanvas();
  }

  /// Moves the rendered tick by `seconds` of match time through the rewind a scrub uses:
  /// Back 10 seconds and Forward on the replay.
  step(seconds) {
    if (!this.history || this.history.count === 0) {
      return;
    }
    const target = this.renderedTick + seconds * 50;
    this.rewind(Math.max(this.history.firstTick, Math.min(this.history.newestTick, target)));
  }

  /// Why Save replay cannot run yet, or null when it can.
  get saveBlocked() {
    if (this.frames.count === 0 || !this.matchId) {
      return 'Nothing is stored yet.';
    }
    if (this.report?.kind === KIND.fullTime && this.report.state !== 'ready') {
      return 'The whole match is still being stored.';
    }
    return null;
  }

  /// Writes every stored frame as a replay file and hands it to the person as a download.
  async saveReplay() {
    if (this.frames.count === 0 || !this.matchId || this.saving) {
      return null;
    }
    this.saving = true;
    try {
      const { bytes, hash } = await writeReplay(this.frames, {
        matchId: this.matchId,
        version: this.helloVersion ?? undefined,
        record: this.loadedRecord,
      });
      const name = `touchline-${this.matchId}.smfx`;
      this.download(bytes, name);
      this.lastSaved = { name, bytes, hash, frames: this.frames.count, ticks: this.frames.tickFrames };
      this.saved = { name, size: bytes.length, hash };
      signal('viewer.replay_saved', {
        bytes: bytes.length,
        frames: this.frames.count,
        ticks: this.frames.tickFrames,
        hash,
      });
      this.spoken = `Replay saved as ${name}.`;
      return this.lastSaved;
    } finally {
      this.saving = false;
    }
  }

  /// The browser's download: a temporary link to an object URL.
  browserDownload(bytes, name) {
    const url = URL.createObjectURL(new Blob([bytes], { type: 'application/octet-stream' }));
    const link = this.doc.createElement('a');
    link.href = url;
    link.download = name;
    link.click();
    this.timers.setTimeout(() => URL.revokeObjectURL(url), 10_000);
  }

  // ---- The read-only test hook --------------------------------------------------------------

  /// What a test or a drive may read. It exposes no setter: a hook that can change the page
  /// is a hook that can hide a fault.
  snapshot() {
    return {
      screen: this.screen,
      renderedTick: this.renderedTick,
      score: [...this.score],
      clock: this.clockText,
      feedCount: this.feedRows.length,
      lastFeed: this.feedRows.at(-1) ?? null,
      goalShownAtTick: this.goalShownAtTick,
      bannerVisible: this.banner !== null,
      bannerText: this.banner?.text ?? null,
      stats: this.stats,
      notice: this.notice ? { ...this.notice } : null,
      speed: this.speed,
      playing: this.playing,
      ...this.lineupSnapshot(),
    };
  }

  /// The commentary's empty text is on show, the tick of the newest condition message at the
  /// rendered tick, and each of the 22 players with the condition word the page gives them.
  lineupSnapshot() {
    const state = this.lastFlushTick >= 0 ? this.match.at(this.lastFlushTick) : null;
    const rows =
      state && this.dugout.rosters.every((r) => r.length > 0)
        ? lineupModel(this.dugout.rosters, this.dugout.teamIds, state).flat()
        : [];
    return {
      emptyStateShown:
        this.feedRows.length === 0 && ['kickoff', 'live', 'paused', 'reconnecting', 'full-time'].includes(this.screen),
      energyTick: state?.energyTick ?? null,
      lineupLabels: rows.map((r) => ({ name: r.name, shirt: r.shirt, condition: r.condition, card: r.cardWord })),
    };
  }
}
