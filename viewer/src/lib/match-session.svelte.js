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
// The lineup editor, the tactics and substitutions, the half-time and full-time reports and
// saving a replay are later parts of the port; until they arrive the kick-off sends no
// lineup (the computer manager's lineup stands) and playback runs through the breaks.

import { COMPONENT_COUNT, decodeInto, newFrame } from './decode.js';
import { FeedBatcher, feedRow, minuteStamp } from './feed.js';
import { BANNER_TOTAL_MS, bannerText } from './goal-moment.js';
import { History } from './history.js';
import { between } from './interpolate.js';
import * as launcher from './launcher.js';
import { LeadControl, SeenReport } from './lead.js';
import { KIND, MatchState } from './match-state.js';
import { Pitch, readTokens } from './pitch.js';
import { Playback } from './playback.js';
import { backoff, clockAt, loadingSteps, panelModel } from './recovery.js';
import { frameText, readReplay } from './replay-file.js';
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

  /// `fetcher`, `timers`, `raf` and `now` are the browser's unless a test passes its own.
  constructor({
    fetcher,
    timers = globalThis,
    raf = globalThis.requestAnimationFrame?.bind(globalThis),
    now = () => globalThis.performance.now(),
    doc = globalThis.document,
  } = {}) {
    this.fetcher = fetcher;
    this.timers = timers;
    this.raf = raf;
    this.now = now;
    this.doc = doc;

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
    this.teamNames = new Map();
    this.halfTimes = 0;
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
      if (this.panel?.actions.includes('restart')) {
        return 'Restart';
      }
      return this.panel?.actions.includes('open-replay') ? 'Open replay' : 'Abandon';
    }
    return ACTIONS[this.screen];
  }

  /// `true` while the action block has nothing to do: the wait before the engine answers, a
  /// reconnect, or a restart or abandon on its way.
  get actionBusy() {
    return this.busy || this.screen === 'loading' || this.screen === 'reconnecting';
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
    if (engine?.['engine.state'] === 'running' && engine['socket.port']) {
      this.setStep(1);
      this.connect(engine);
      return;
    }
    this.showPanel(panelModel(engine) ?? panelModel(null));
  }

  /// The pitch canvas, once the screen has drawn it.
  attachCanvas(canvas) {
    this.canvas = canvas;
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
    this.pitch = new Pitch(this.canvas, kits, readTokens(this.doc));
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
    this.steps = loadingSteps(current);
  }

  connect(status) {
    this.socket = new MatchSocket(socketAddress(status['socket.port'], status['protocol.version']), {
      onHello: (hello) => this.onHello(hello),
      onTick: (buffer) => this.onTick(buffer),
      onMessage: (message) => this.onMessage(message),
      onRaw: (data) => this.onRaw(data),
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
    this.teams = hello.teams;
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
    this.flush(0, { seek: true });
    this.screen = stored ? 'live' : 'kickoff';
  }

  /// The kick-off: the engine is told the pitch shows tick 0 before it starts, so it is held
  /// near the pitch from its first tick. No lineup is sent: the computer manager's stands.
  kickOff() {
    if (this.screen !== 'kickoff' || !this.socket) {
      return;
    }
    this.socket.send({ type: 'seen', tick: 0 });
    this.socket.send({ type: 'start' });
    this.kickedOff = true;
    this.setPlaying(true);
    this.screen = 'live';
    signal('viewer.kick_off', { lineup: null, bench: null, patch: null });
  }

  onRaw(data) {
    if (typeof data === 'string' || !this.resuming) {
      return;
    }
    const first = new DataView(data).getUint32(1, true);
    this.resumeAt(first - 1);
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
      return;
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
    this.pace();
    this.report(timestamp);
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
  report(now) {
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
        return this.kickOff();
      case 'live':
        return this.setPlaying(false);
      case 'paused':
        return this.setPlaying(true);
      case 'first-run':
        return pickReplay();
      case 'error':
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

  async abandonEngine() {
    this.busy = true;
    const after = await launcher.abandon(this.fetcher);
    this.status = after ?? { 'engine.state': 'abandoned' };
    if (this.socket) {
      this.socket.onClose = null;
      this.socket.close();
    }
    // Saving a replay arrives with the reports; the abandoned panel offers opening one.
    const model = panelModel({ 'engine.state': 'abandoned' });
    this.showPanel({ ...model, actions: model.actions.filter((a) => a !== 'save-replay') });
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
    this.setPlaying(true);
    this.screen = 'live';
    signal('viewer.replay_loaded', { frames: read.frames, ticks: read.ticks, name });
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
    };
  }
}
