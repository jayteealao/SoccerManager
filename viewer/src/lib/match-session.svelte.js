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
import { DEFAULT_GROUND, Pitch, groundOf, readTokens } from './pitch.js';
import { Playback } from './playback.js';
import { formationName, kickOffSheet, rosterSheet, rulePackRows, squadSheet } from './prematch.js';
import { backoff, clockAt, loadingSteps, panelModel, stepOptions } from './recovery.js';
import { FrameStore, frameText, readReplay, writeReplay } from './replay-file.js';
import { ReportClock, reportModel } from './report.js';
import { Scheduler } from './schedule.js';
import { fixtureTitle, scorerLines } from './scoreboard.js';
import { signal } from './signal.js';
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
]);

/// A notice's kind, as the Notice component colours it, and the word it always carries.
export const NOTICES = Object.freeze({
  reconnect: { kind: 'warn', word: 'RECONNECTING' },
  connected: { kind: 'good', word: 'CONNECTED' },
  lag: { kind: 'mid', word: 'LAG' },
  end: { kind: 'neutral', word: 'FULL TIME' },
  error: { kind: 'bad', word: 'STREAM ENDED' },
});

/// The one next action per screen, as the cyan action block names it.
export const ACTIONS = Object.freeze({
  loading: 'Please wait',
  kickoff: 'Kick off',
  live: 'Pause',
  paused: 'Resume',
  'first-run': 'Open replay',
  reconnecting: 'Reconnecting',
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
});

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

  /// `fetcher`, `timers`, `raf` and `now` are the browser's unless a test passes its own.
  constructor({
    fetcher,
    timers = globalThis,
    raf = globalThis.requestAnimationFrame?.bind(globalThis),
    now = () => globalThis.performance.now(),
    doc = globalThis.document,
    download = null,
  } = {}) {
    this.fetcher = fetcher;
    this.timers = timers;
    this.raf = raf;
    this.now = now;
    this.doc = doc;
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
    this.goalShownAtTick = null;
    this.bannerTimer = null;
    this.stepShown = 0;
    /// What the loading steps add for a saved match: the engine version when another
    /// release's engine plays it, and the tick it continues from.
    this.stepOpts = { version: null, resumeTick: null };
    this.teamNames = new Map();
    this.halfTimes = 0;
    this.dugout = new Dugout({
      send: (command) => (this.socket ? this.socket.send(command) : false),
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
    const scored = this.screen === 'live' || this.screen === 'paused';
    return fixtureTitle(names, scored ? this.score : null);
  }

  /// The engine word and version, or what the screen is waiting for.
  get subtitle() {
    switch (this.screen) {
      case 'loading':
        return 'Getting the match ready';
      case 'first-run':
        return 'First run · no engine found';
      default:
        return this.engineVersion ? `${this.engineWord} · v${this.engineVersion}` : this.engineWord;
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
    const canvas = (this.view === 'replay' ? this.canvases.replay : null) ?? this.canvases.match ?? null;
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
      this.raf(step);
      this.frame(timestamp);
    };
    this.raf(step);
  }

  setStep(current) {
    if (current === this.stepShown && this.steps) {
      return;
    }
    this.stepShown = current;
    this.steps = loadingSteps(current, this.stepOpts);
  }

  connect(status) {
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
      this.engineWord = 'Engine connected';
      this.panel = null;
      this.busy = false;
      this.screen = this.playing ? 'live' : 'paused';
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
    this.streamEnded = false;
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
    this.playback.select(1);
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
    }
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
    this.socket?.send({ type: 'seen', tick: 0 });
    this.socket?.send({ type: 'start' });
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
      this.frames.addText(data, message?.type);
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
  }

  onMessage(message) {
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
    }
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
    if (this.kickedOff && this.socket) {
      this.socket.send({ type: 'seen', tick: Math.min(this.renderedTick, tick) });
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
    if (!this.kickedOff || !this.socket || !this.history) {
      return;
    }
    const command = this.lead.next(
      this.history.newestTick - this.renderedTick,
      this.scheduler.speed,
      this.match.fullTimeTick !== null
    );
    if (command) {
      this.socket.send({ type: command });
    }
  }

  /// Tells a kicked-off engine which tick the pitch shows, so it stays within its buffer.
  reportSeen(now) {
    if (!this.kickedOff || !this.socket) {
      return;
    }
    const seen = this.seen.next(this.renderedTick, now);
    if (seen !== null) {
      this.socket.send({ type: 'seen', tick: seen });
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
  async openReplay(bytes, name = 'replay') {
    let read;
    try {
      read = await readReplay(bytes);
    } catch (error) {
      const reason = error.reason ?? error.message;
      signal('viewer.replay_refused', { reason });
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
    this.replayFrom = 'match';
    this.view = 'replay';
    this.selectCanvas();
    this.spoken = 'Replay loaded. Playing from kick-off.';
    signal('viewer.replay_loaded', { frames: read.frames, ticks: read.ticks, name });
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
    if (this.view !== 'report') {
      this.reportFrom = ['touchline', 'tactics', 'replay'].includes(this.view) ? this.view : 'match';
    }
    this.report = { kind, tick, state: ready ? 'ready' : 'loading', model };
    this.view = 'report';
    this.selectCanvas();
    signal('viewer.report_shown', { kind, tick });
    this.spoken = kind === KIND.halfTime ? 'Half-time. The report is open.' : 'Full time. The report is open.';
  }

  /// CONTINUE or Close on the report: back to the view it opened over. Closing the half-time
  /// report resumes playback.
  closeReport() {
    if (this.view !== 'report') {
      return;
    }
    if (this.report?.kind === KIND.halfTime) {
      this.setPlaying(true);
    }
    this.view = this.reportFrom;
    this.selectCanvas();
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
    if (this.report?.kind === KIND.fullTime && this.report.state === 'loading') {
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
      emptyStateShown: this.feedRows.length === 0 && ['kickoff', 'live', 'paused', 'reconnecting'].includes(this.screen),
      energyTick: state?.energyTick ?? null,
      lineupLabels: rows.map((r) => ({ name: r.name, shirt: r.shirt, condition: r.condition, card: r.cardWord })),
    };
  }
}
