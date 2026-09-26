// The page: socket in, pitch out, controls between them.

import { COMPONENT_COUNT, decodeInto, newFrame } from './decode.mjs';
import { Feed, minuteStamp } from './feed.mjs';
import { GoalMoment, bannerText, watchMotion } from './goal-moment.mjs';
import { History } from './history.mjs';
import { between } from './interpolate.mjs';
import { LeadControl, SeenReport } from './lead.mjs';
import { LineupEditor } from './lineup-editor.mjs';
import { Lineups, benchModel } from './lineups.mjs';
import { colours as markColours, drawMark, setFavicon } from './mark.mjs';
import { abandon as abandonMatch, fetchStatus, poll, restart as restartMatch } from './launcher.mjs';
import { KIND, MatchState } from './match-state.mjs';
import { createPendingList } from './pending.mjs';
import { Pitch } from './pitch.mjs';
import { Playback, SPEEDS } from './playback.mjs';
import { backoff, clockAt, loadingSteps, panelModel } from './recovery.mjs';
import { FrameStore, frameText, readReplay, writeReplay } from './replay-file.mjs';
import { ReportClock, ReportDialog, reportModel } from './report.mjs';
import { Scheduler } from './schedule.mjs';
import { Scoreboard } from './scoreboard.mjs';
import { signal, signals } from './signal.mjs';
import { MatchSocket, socketAddress } from './socket.mjs';
import { StatsPanel } from './stats.mjs';
import { Stoppages, stopsPlay } from './stoppages.mjs';
import { SubstitutionPicker } from './substitution-picker.mjs';
import {
  TacticsPanel,
  applyPatch,
  detailFor,
  prematchPatch,
  tacticsOf,
} from './tactics-panel.mjs';

const el = (id) => document.getElementById(id);

const scheduler = new Scheduler();
const stoppages = new Stoppages();
const rendered = new Int16Array(COMPONENT_COUNT);
const earlier = new Int16Array(COMPONENT_COUNT);
const later = new Int16Array(COMPONENT_COUNT);

let history = null;
let pitch = null;
let socket = null;
let playback = null;
let renderedTick = 0;
let scrubbing = false;
let resumeAfterScrub = true;
let lastRewind = null;

/// The match-day panels. Every one reads the match at the rendered tick, once per frame.
const match = new MatchState();
let panels = null;

/// Every frame of the match as it arrived, for the replay file.
let frames = new FrameStore();
/// The match being shown, and the protocol version its hello named.
let matchId = null;
let helloVersion = null;
/// A reconnect to the same match: the stores are cut back at the first tick that arrives.
let resuming = false;
/// `true` while a replay file plays: there is no socket and nothing to recover.
let storedReplay = false;
/// The newest engine status read, and the surface on show.
let engineStatus = null;
let surface = null;
let reconnectAttempt = 0;
let reconnecting = false;
let lastSaved = null;
const reportClock = new ReportClock();
let reports = null;

function announce(text) {
  el('live').textContent = text;
}

/// One notice element, two kinds, and the state word always says which. A colour on its
/// own never carries the difference between "the engine is slow" and "the stream ended".
function showNotice(text, kind = 'lag') {
  const notice = el('notice');
  const shown = Boolean(text);
  notice.dataset.shown = shown ? 'true' : 'false';
  notice.dataset.kind = kind;
  notice.setAttribute('aria-hidden', shown ? 'false' : 'true');
  const words = { error: 'Stream ended', end: 'Full time', lag: 'Lag', reconnect: 'Reconnecting' };
  el('notice-word').textContent = shown ? (words[kind] ?? 'Lag') : '';
  el('notice-text').textContent = text ?? '';
}

function setSpeedButtons(requested, effective) {
  for (const speed of SPEEDS) {
    el(`speed-${speed}`).setAttribute('aria-pressed', String(speed === requested));
  }
  el('speed-effective').textContent = `${effective}x`;
  panels?.scoreboard.setSpeed(effective);
  announce(
    requested === effective
      ? `Speed ${requested} times real time.`
      : `Speed ${requested} times asked for. Playing at ${effective} times.`
  );
}

/// The header says in words whether the engine is connected, beside the version its hello
/// named. The word follows the socket: it never claims a connection that has dropped.
let engineVersion = null;
function setEngineWord(word) {
  el('engine-version').textContent = engineVersion ? `${word} · v${engineVersion}` : '';
}

function start(hello, { stored = false } = {}) {
  history = new History(hello.ticks_expected);
  pitch = new Pitch(el('pitch'), [
    { primary: hello.teams[0]['team.kit.primary'], secondary: hello.teams[0]['team.kit.secondary'] },
    { primary: hello.teams[1]['team.kit.primary'], secondary: hello.teams[1]['team.kit.secondary'] },
  ]);
  panels.start(hello);
  dugout.begin(hello, { stored });
  engineVersion = hello['engine.version'];
  setEngineWord(stored ? 'Replay' : 'Engine connected');
  el('scrub').max = String(hello.ticks_expected);
  playback = new Playback({
    scheduler,
    onSpeed: setSpeedButtons,
    onNotice: showNotice,
  });
  playback.select(1);
}

let previous = newFrame();
let incoming = newFrame();

/// One tick frame. `live` is false for a frame read from a replay file, which has no socket
/// to pace and no arrival rate to measure.
function onTick(buffer, live = true) {
  const result = decodeInto(buffer, previous, incoming);
  if (!result) {
    return;
  }
  history.append(incoming.tick, incoming.components);
  if (live) {
    socket.noteTick(incoming.tick);
    playback.noteArrival(performance.now(), incoming.tick, incoming.tick - renderedTick);
    // Paced on arrival as well as on each drawn frame: a hidden or throttled tab draws few
    // frames, and the engine must still stop a few seconds ahead of the drawn tick.
    dugout.pace(history.newestTick - renderedTick);
  }
  if (result.kind !== 'delta') {
    if (live) {
      history.report();
      history.measurePage();
    }
    if (result.kind === 'restart') {
      stoppages.add(incoming.tick);
    }
  }
  // A match can run past the announced ticks; the scrubber's end follows the newest tick.
  if (history.newestTick > history.ticksExpected) {
    el('scrub').max = String(history.scrubLimit);
  }
  if (live && !scrubbing) {
    el('scrub').value = String(incoming.tick);
  }
  [previous, incoming] = [incoming, previous];
  setStep(3);
  revealPitch();
}

/// Every frame, exactly as the socket handed it over, before it is decoded. The first hello
/// of a match opens a new store; a reconnect's first tick cuts every store back first.
function onRaw(data, message) {
  if (typeof data === 'string') {
    if (message?.type === 'hello' && message['match.id'] !== matchId) {
      frames = new FrameStore();
    }
    frames.addText(data, message?.type);
    return;
  }
  if (resuming) {
    const first = new DataView(data).getUint32(1, true);
    resumeAt(first - 1);
  }
  frames.addBinary(data);
}

function onHello(hello) {
  reconnectAttempt = 0;
  reconnecting = false;
  if (history && hello['match.id'] === matchId) {
    // The same match, after a reconnect or a restart. The stores are kept, and the first
    // tick frame, a keyframe one tick past the stoppage it resumes from, says where to cut.
    resuming = true;
    setEngineWord('Engine connected');
    hideSurface();
    showNotice('Connected again. Play resumes at the last stoppage.', 'reconnect');
    return;
  }
  matchId = hello['match.id'];
  helloVersion = hello['protocol.version'];
  setStep(2);
  start(hello);
}

/// Cuts every store back to `tick`, where the resumed match continues. The engine plays the
/// later ticks again; keeping both copies would draw and count them twice.
function resumeAt(tick) {
  resuming = false;
  const from = history.newestTick;
  const gap = history.truncate(tick);
  stoppages.truncate(tick);
  match.truncate(tick);
  frames.truncate(tick);
  previous.tick = tick;
  if (renderedTick > tick) {
    rewind(tick);
  } else {
    panels.flush(renderedTick, { seek: true });
  }
  showNotice(null);
  announce(`Play resumes at ${clockAt(tick)}.`);
  signal('viewer.resumed', { 'match.id': matchId, from_tick: from, to_tick: tick, gap });
  if (gap > 0) {
    signal('viewer.resume_gap', { ticks: gap });
  }
  if (dugout.phase === 'live' && socket) {
    socket.send({ type: 'seen', tick: Math.min(renderedTick, tick) });
  }
}

/// The loading steps. `current` is the step in progress, 0 to 2, or 3 once all are done.
let stepShown = -1;
function setStep(current) {
  if (current === stepShown) {
    return;
  }
  stepShown = current;
  const items = el('steps').children;
  loadingSteps(current).forEach((step, i) => {
    items[i].dataset.state = step.state;
    items[i].querySelector('.steps__word').textContent = step.word;
  });
}

/// Cross-fades the skeleton away on the first tick. `hidden` alone would snap, because
/// `display` is not animatable.
let revealed = false;
function revealPitch() {
  if (revealed) {
    return;
  }
  revealed = true;
  const skeleton = el('skeleton');
  skeleton.dataset.leaving = 'true';
  setTimeout(() => {
    skeleton.hidden = true;
  }, 250);
}

function onMessage(message) {
  if (message.type === 'ack' || message.type === 'reject') {
    if (storedReplay) {
      return;
    }
    dugout.answer(message);
    return;
  }
  if (message.type === 'event' && message['event.type'] === KIND.tacticsChange) {
    dugout.pending.onChangeEvent(message);
  }
  match.add(message);
  if (stopsPlay(message)) {
    stoppages.add(message.tick);
  }
  // `ticks_expected` is the most ticks the match can last. Full time marks the real end,
  // so the scrubber stops at the last tick that arrived.
  if (message.type === 'event' && message['event.type'] === 'full-time' && history) {
    el('scrub').max = String(history.newestTick);
  }
}

function frame(timestamp) {
  requestAnimationFrame(frame);
  if (!history || history.count === 0) {
    return;
  }
  const step = scheduler.advance(timestamp, history.newestTick, history.firstTick);
  if (!history.tickAt(step.from, earlier)) {
    return;
  }
  if (!history.tickAt(step.to, later)) {
    later.set(earlier);
  }
  between(earlier, later, step.fraction, rendered);
  pitch.draw(rendered);
  renderedTick = step.from;
  panels.flush(renderedTick, { seek: false });
  // A report opens when the pitch reaches the break, never when its event arrives.
  const due = scrubbing ? null : reportClock.due(match.events, renderedTick);
  if (due) {
    showReport(due.kind, due.tick);
  }
  showGauge();
  dugout.pace(history.newestTick - renderedTick);
  dugout.report(renderedTick, timestamp);
}

/// Every panel around the pitch. One `flush` per frame writes only what changed, and never
/// shows a message the pitch has not reached: the match is read at the rendered tick.
class Panels {
  constructor() {
    this.scoreboard = new Scoreboard({
      names: [el('team-home'), el('team-away')],
      crests: [el('crest-home'), el('crest-away')],
      bug: el('score-bug'),
      home: el('score-home'),
      away: el('score-away'),
      clock: el('clock'),
      speed: el('header-speed'),
    });
    this.feed = new Feed({ list: el('feed'), empty: el('feed-empty'), live: el('feed-live') });
    this.lineups = new Lineups(el('lineups'));
    this.stats = new StatsPanel(el('stats'));
    this.goal = new GoalMoment({
      banner: el('goal-banner'),
      bannerText: el('goal-banner-text'),
      bug: el('score-bug'),
    });
    this.teamNames = new Map();
    this.goalsShown = 0;
    this.lastTick = -1;
    this.goalShownAtTick = null;
    this.lineupView = null;
    this.lineupKey = '';
    this.state = match.at(0);
  }

  start(hello) {
    const teams = hello.teams;
    this.teamNames = new Map(teams.map((t) => [t['team.id'], t['team.name']]));
    this.scoreboard.setTeams(teams, markColours(document).band);
    this.feed.setTeams(this.teamNames);
    this.lineups.setTeams(teams);
    this.lineupView = null;
    this.stats.setTeams(teams.map((t) => t['team.name']));
    this.teams = teams;
    this.flush(0, { seek: true });
  }

  /// The home lineup the manager kicked off with. The hello's roster was sent before the
  /// choice, so the lineup panel reads its rows from the accepted lineup instead.
  setHomeRoster(roster) {
    this.teams = this.teams.map((team, i) => (i === 0 ? { ...team, roster } : team));
    this.lineups.setTeams(this.teams);
    this.lineupView = null;
    this.lineupKey = '';
  }

  /// Brings every panel to `tick`. A seek never replays a goal moment: the banner belongs to
  /// the frame in which play crosses the goal, not to a rewind that lands after it.
  flush(tick, { seek }) {
    const previous = this.lastTick;
    const state = match.at(tick);
    this.state = state;
    this.lastTick = tick;
    this.scoreboard.setScore(state.home, state.away);
    this.scoreboard.setClock(clockAt(tick));
    this.feed.flush(state.entries);
    // The lineups change only when a released event or condition message changes them, so
    // they are rebuilt on that, never per tick.
    const lineupKey = `${state.entries.length}|${state.energyTick}`;
    if (state !== this.lineupView || lineupKey !== this.lineupKey) {
      this.lineupView = state;
      this.lineupKey = lineupKey;
      this.lineups.update(state);
    }
    this.stats.update(state.stats);
    dugout.update(
      tick,
      state,
      this.lineups.model[0] ?? [],
      benchModel(this.lineups.rosters, state)[0] ?? []
    );
    const goals = state.goals.length;
    if (seek || tick < previous) {
      if (goals < this.goalsShown) {
        this.goal.clear();
      }
      this.goalsShown = goals;
      return;
    }
    if (goals > this.goalsShown) {
      const goal = state.goals[goals - 1];
      this.goalsShown = goals;
      this.goal.play(goal, bannerText(goal, this.teamNames, minuteStamp(goal)));
      this.feed.highlightGoal(goal.tick);
      this.goalShownAtTick = tick;
      signal('viewer.goal_moment', {
        goal_tick: goal.tick,
        rendered_tick: tick,
        prev_rendered_tick: previous,
        // 0 when this frame is the first whose rendered tick reached the goal, which is the
        // normal case: the score, banner, and feed line are applied in that same frame. 1
        // means the goal event arrived after the pitch had already passed its tick.
        frame_delta: previous < goal.tick ? 0 : 1,
        'home.score': state.home,
        'away.score': state.away,
      });
    }
  }

  /// The read-only view the test hook returns.
  snapshot() {
    const state = this.state;
    return {
      renderedTick,
      score: [state.home, state.away],
      feedCount: this.feed.count,
      lastFeed: this.feed.last,
      goalShownAtTick: this.goalShownAtTick,
      bannerVisible: this.goal.visible,
      bannerText: el('goal-banner-text').textContent,
      stats: state.stats,
      energyTick: state.energyTick,
      lineupLabels: this.lineups.labels(),
      emptyStateShown: !el('feed-empty').hidden,
      motion: document.documentElement.dataset.motion,
    };
  }
}

/// A hello roster entry for squad player `index`, as the lineup panel reads it.
function rosterEntry(player, index) {
  return {
    'player.id': player['player.id'],
    'player.name': player['player.name'],
    'player.shirt': player['player.shirt'],
    'player.position': player['player.position'],
    'player.squad_index': index,
  };
}

const copyTactics = (t) => ({
  formation: t.formation,
  mentality: t.mentality,
  instructions: [...t.instructions],
  roles: t.roles.map((r) => ({ ...r })),
});

/// Everything the manager does from the page: the lineup before kick-off, the tactics and
/// the substitutions during play, and the chips that say where each change stands. The
/// engine decides every outcome; the page asks and shows the answer.
class Dugout {
  constructor() {
    /// `waiting`, `pre-match`, `kicking-off`, `live`, or `stored` (a recording).
    this.phase = 'waiting';
    this.pending = createPendingList();
    this.requests = [];
    this.lead = new LeadControl();
    this.seen = new SeenReport();
    this.editor = null;
    this.panel = null;
    this.picker = null;
    this.chipKey = '';
    this.slotKey = '';
    this.seenRejections = new Set();
    this.over = false;
  }

  begin(hello, { stored = false } = {}) {
    const home = hello.teams[0];
    this.homeId = home['team.id'];
    this.pending = createPendingList({ homeTeamId: this.homeId });
    this.schema = hello.tactics;
    this.squad = home.squad ?? [];
    this.setup = home.setup ?? null;
    this.squadById = new Map(this.squad.map((p, i) => [p['player.id'], i]));
    this.picker = new SubstitutionPicker({
      root: el('picker'),
      limit: hello.substitutions?.limit ?? 0,
      onQueue: (detail, label) => this.queue('substitution', detail, label),
    });
    if (stored || !this.setup || !this.schema || !Array.isArray(this.schema.formations)) {
      // A recording, or an engine that takes no lineup: nothing reaches a live match.
      if (this.editor) {
        this.showPreMatch(false);
      }
      this.editor = null;
      this.panel = null;
      this.over = true;
      this.phase = 'stored';
      el('tactics').textContent =
        'Changes need a live match. This one is played back from a recording.';
      this.picker.setEnabled(false, 'Changes need a live match.');
      return;
    }
    this.phase = 'pre-match';
    this.base = tacticsOf(this.setup);
    this.panel = new TacticsPanel({
      root: el('tactics'),
      schema: this.schema,
      tactics: tacticsOf(this.setup),
      onEdit: (edit, label) => this.queue('tactics', detailFor(edit), label),
      onDraft: () => this.editor?.render(),
    });
    this.editor = new LineupEditor({
      board: el('board'),
      side: el('lineup-editor'),
      squadList: el('squad'),
      schema: this.schema,
      squad: this.squad,
      setup: this.setup,
      benchSize: this.schema.ai?.bench_size ?? 7,
      roleOf: (slot) => this.panel.tactics.roles[slot].role,
      onFormation: (formation) => this.panel.setFormation(formation),
      onKickOff: () => this.kickOff(),
      onChange: () => this.editor && this.panel.setSlots(this.editor.slotPlayers()),
    });
    this.panel.setSlots(this.editor.slotPlayers());
    this.showPreMatch(true);
    announce('Pick the lineup, then kick off.');
  }

  /// Before kick-off the editor borrows the pitch tile, the left column, and the statistics
  /// region; at kick-off each goes back to its match-day panel.
  showPreMatch(on) {
    el('board').hidden = !on;
    el('lineup-editor').hidden = !on;
    el('squad').hidden = !on;
    el('lineups').hidden = on;
    el('stats').hidden = on;
    el('skeleton').hidden = on;
    el('left-label').textContent = on ? 'Lineup' : 'Lineups';
    el('stats-label').textContent = on ? 'Squad' : 'Statistics';
  }

  /// Sends one command and remembers who waits for its answer. Answers come back in the
  /// order the commands were read, one per command.
  request(command, onAck, onReject) {
    if (!socket || !socket.send(command)) {
      return false;
    }
    this.requests.push({ command: command.type, onAck, onReject });
    return true;
  }

  answer(message) {
    const i = this.requests.findIndex((r) => r.command === message.command);
    if (i < 0) {
      return;
    }
    const [waiting] = this.requests.splice(i, 1);
    if (message.type === 'ack') {
      waiting.onAck(message);
    } else {
      waiting.onReject(message);
    }
  }

  kickOff() {
    const message = this.editor.message();
    const draft = { ...copyTactics(this.panel.tactics), formation: this.editor.formation };
    const patch = prematchPatch(this.base, draft, message.lineup);
    this.editor.setBusy(true);
    this.phase = 'kicking-off';
    const command = { type: 'set-lineup', ...message };
    if (patch) {
      command.patch = patch;
    }
    const sent = this.request(
      command,
      () => this.startMatch(message, patch),
      (reject) => {
        this.phase = 'pre-match';
        this.editor.refused(reject.reason);
      }
    );
    if (!sent) {
      this.phase = 'pre-match';
      this.editor.refused('The engine is not connected.');
    }
  }

  startMatch(message, patch) {
    // Reported before the start, so the engine is held near the pitch from its first tick.
    socket.send({ type: 'seen', tick: 0 });
    socket.send({ type: 'start' });
    this.phase = 'live';
    this.lineup = message.lineup;
    this.confirmed = patch
      ? applyPatch(this.base, patch, message.lineup, this.schema)
      : copyTactics(this.base);
    const roster = [...message.lineup, ...message.bench].map((i) =>
      rosterEntry(this.squad[i], i)
    );
    panels.setHomeRoster(roster);
    this.showPreMatch(false);
    this.panel.setMode('live');
    this.panel.setTactics(copyTactics(this.confirmed));
    this.picker.setEnabled(true);
    signal('viewer.kick_off', {
      lineup: message.lineup,
      bench: message.bench,
      patch: patch ?? null,
    });
    announce('Kick-off.');
  }

  /// Queues one change. The chip appears on the engine's acknowledgement.
  queue(kind, detail, label) {
    if (this.phase !== 'live' || this.over) {
      return;
    }
    const sent = this.request(
      { type: 'queue-change', 'change.kind': kind, detail },
      (ack) => this.pending.queued(ack, label, kind, detail.patch ?? detail),
      (reject) => this.pending.rejectedAtQueue(reject, label, kind)
    );
    if (!sent) {
      this.pending.rejectedAtQueue({ reason: 'The engine is not connected.' }, label, kind);
    }
  }

  /// The home slots in slot order, by squad index, at the rendered tick.
  slotsFrom(rows) {
    return rows.map((row) => ({
      squad: this.squadById.get(row.id) ?? null,
      name: row.name,
      shirt: row.shirt,
      sentOff: row.sentOff,
    }));
  }

  /// The tactics the controls show: what the engine applied, then every change still
  /// waiting. A refused change drops out, so its control returns to the value in force.
  tacticsView(slots) {
    const lineup = slots.map((s) => s.squad);
    let tactics = copyTactics(this.confirmed);
    for (const chip of this.pending.all(Number.MAX_SAFE_INTEGER)) {
      if (chip.kind === 'tactics' && chip.state !== 'rejected' && chip.detail) {
        tactics = applyPatch(tactics, chip.detail, lineup, this.schema);
      }
    }
    return tactics;
  }

  /// Once per frame, at the rendered tick.
  update(tick, state, homeRows, benchRows) {
    this.renderChips(tick);
    if (this.phase !== 'live') {
      return;
    }
    const slots = this.slotsFrom(homeRows);
    const key = `${slots.map((s) => `${s.squad}${s.sentOff ? 'x' : ''}`).join(',')}|` +
      benchRows.map((p) => p['player.squad_index']).join(',');
    if (key !== this.slotKey && slots.length > 0) {
      this.slotKey = key;
      this.panel.setSlots(slots);
      this.picker.setPlayers(
        slots.filter((s) => s.squad !== null && !s.sentOff),
        benchRows.map((p) => ({
          squad: p['player.squad_index'],
          shirt: p['player.shirt'],
          name: p['player.name'],
        }))
      );
    }
    this.picker.setUsed(state.substitutions.filter((s) => s.team === this.homeId).length);
    for (const chip of this.pending.chips(tick)) {
      if (chip.state === 'rejected' && !this.seenRejections.has(chip.queue_id)) {
        this.seenRejections.add(chip.queue_id);
        if (chip.kind === 'tactics') {
          this.panel.setTactics(this.tacticsView(slots));
        }
      }
    }
    if (state.fullTime && !this.over) {
      this.over = true;
      this.panel.setMode('read-only');
      this.picker.setEnabled(false, 'The match is over.');
    }
  }

  renderChips(tick) {
    const chips = this.pending.chips(tick);
    const key = chips.map((c) => `${c.queue_id}:${c.state}`).join('|');
    if (key === this.chipKey) {
      return;
    }
    this.chipKey = key;
    const list = el('chips');
    const doc = list.ownerDocument;
    list.replaceChildren(
      ...chips.map((chip) => {
        const li = doc.createElement('li');
        li.className = 'chip';
        li.dataset.state = chip.state;
        li.dataset.testid = `chip-${chip.queue_id}`;
        const word = doc.createElement('span');
        word.className = 'chip__word';
        word.textContent = chip.word;
        const text = doc.createElement('span');
        text.className = 'chip__text';
        const label = doc.createElement('span');
        label.className = 'chip__label';
        label.textContent = chip.label;
        label.title = chip.label;
        text.append(label);
        if (chip.reason) {
          const reason = doc.createElement('span');
          reason.className = 'chip__reason';
          reason.textContent = chip.reason;
          text.append(reason);
        }
        li.append(word, text);
        if (chip.state === 'rejected') {
          const dismiss = doc.createElement('button');
          dismiss.type = 'button';
          dismiss.className = 'match-control match-control--sm match-control--quiet chip__dismiss';
          dismiss.textContent = 'Dismiss';
          dismiss.setAttribute('aria-label', `Dismiss: ${chip.label}`);
          dismiss.addEventListener('click', () => {
            this.pending.dismiss(chip.queue_id);
            this.chipKey = '';
            this.renderChips(renderedTick);
          });
          li.append(dismiss);
        }
        return li;
      })
    );
  }

  /// Keeps the engine a few seconds ahead of playback during a live match.
  pace(lead) {
    if (this.phase !== 'live' || !socket) {
      return;
    }
    const command = this.lead.next(lead, scheduler.speed, match.fullTimeTick !== null);
    if (command) {
      socket.send({ type: command });
    }
  }

  /// Tells a live engine which tick the pitch shows, so it stays within its buffer bound.
  report(tick, now) {
    if (this.phase !== 'live' || !socket) {
      return;
    }
    const seen = this.seen.next(tick, now);
    if (seen !== null) {
      socket.send({ type: 'seen', tick: seen });
    }
  }

  /// The read-only views the test hook returns.
  lineupView() {
    const editor = this.editor ? this.editor.snapshot() : null;
    return {
      phase: this.phase,
      ...(editor ?? {}),
      kicked_off: this.lineup ?? null,
    };
  }
}

const dugout = new Dugout();

/// Frames between two refreshes of the gauge: its figures are percentiles of a window of
/// hundreds of frames, and a person reads it a few times a second at most.
const GAUGE_EVERY_FRAMES = 15;
let gaugeFrames = 0;
let gaugeShown = '';

/// Refreshes the gauge every `GAUGE_EVERY_FRAMES` frames, and writes it only on a change.
function showGauge() {
  gaugeFrames = (gaugeFrames + 1) % GAUGE_EVERY_FRAMES;
  if (gaugeFrames !== 1 && gaugeShown !== '') {
    return;
  }
  const text = gaugeText();
  if (text !== gaugeShown) {
    gaugeShown = text;
    el('gauge').textContent = text;
  }
}

function gaugeText() {
  const budget = scheduler.budget();
  const megabytes = Math.round(history.bytes() / 1_000_000);
  // `refresh_hz` rides beside the frame rate everywhere the frame rate appears. A panel
  // running at 144 or at 32 makes a bare frame count meaningless on its own.
  return `${budget.fps_median} fps · ${budget.refresh_hz} Hz · ${megabytes} MB`;
}

/// A rewind draws the stored tick itself, not an interpolation towards it.
///
/// `exact` is measured, not assumed: the drawn frame is compared against the stored tick
/// component by component. A rewind that quietly drew an interpolation would satisfy an
/// `exact` flag that only asked whether the tick was in the history.
function rewind(tick) {
  const from = renderedTick;
  scheduler.seek(tick);
  pitch.clearTrail();
  const stored = history.tickAt(tick, earlier);
  let exact = false;
  if (stored) {
    between(earlier, earlier, 0, rendered);
    pitch.draw(rendered);
    renderedTick = tick;
    panels.flush(tick, { seek: true });
    exact = rendered.every((value, i) => value === earlier[i]);
    lastRewind = { tick, drawn: Array.from(rendered), stored: Array.from(earlier), exact };
  }
  signal('viewer.rewind', { from_tick: from, to_tick: tick, stored, exact });
  announce(`Rewound to ${clockAt(tick)}.`);
}

/// Plays or pauses the page's own playback, and keeps the button's word in step.
function setPlaying(playing) {
  scheduler.setPlaying(playing);
  el('play').dataset.playing = String(playing);
  el('play').textContent = playing ? 'Pause' : 'Play';
}

function wire() {
  el('play').addEventListener('click', () => {
    const playing = el('play').dataset.playing !== 'true';
    setPlaying(playing);
    announce(playing ? 'Playing.' : 'Paused.');
  });

  reports = new ReportDialog({
    dialog: el('report'),
    title: el('report-title'),
    score: el('report-score'),
    table: el('report-table'),
    moments: el('report-moments'),
    actions: el('report-actions'),
  });
  // Closing the half-time report, by Continue or by Escape, resumes playback.
  el('report').addEventListener('close', () => {
    if (reports.kind === 'half-time') {
      setPlaying(true);
      announce('Second half.');
    }
  });
  el('report-continue').addEventListener('click', () => reports.close());
  el('report-close').addEventListener('click', () => reports.close());
  el('report-save').addEventListener('click', () => saveReplay());
  el('report-open').addEventListener('click', () => el('replay-input').click());

  el('surface-restart').addEventListener('click', () => restartEngine());
  el('surface-abandon').addEventListener('click', () => abandonEngine());
  el('surface-save').addEventListener('click', () => saveReplay());
  el('surface-open').addEventListener('click', () => el('replay-input').click());
  el('replay-input').addEventListener('change', async () => {
    const input = el('replay-input');
    const file = input.files && input.files[0];
    if (!file) {
      return;
    }
    const bytes = new Uint8Array(await file.arrayBuffer());
    input.value = '';
    await openReplay(bytes, file.name);
  });

  for (const speed of SPEEDS) {
    el(`speed-${speed}`).addEventListener('click', () => playback.select(speed));
  }

  el('skip').addEventListener('click', () => {
    const next = stoppages.next(renderedTick);
    if (next === null) {
      announce('No later stoppage.');
      return;
    }
    rewind(next);
  });

  // A scrub holds the frame it lands on. Playback resumes when the drag ends, so the
  // pitch shows the stored tick for as long as the manager is looking at it.
  const scrub = el('scrub');
  scrub.addEventListener('input', () => {
    if (!scrubbing) {
      scrubbing = true;
      resumeAfterScrub = scheduler.playing;
      scheduler.setPlaying(false);
    }
    rewind(Number(scrub.value));
  });
  scrub.addEventListener('change', () => {
    scrubbing = false;
    scheduler.setPlaying(resumeAfterScrub);
  });
}

/// Opens the half-time or full-time report at `tick`. The half-time report pauses the page's
/// playback until Continue; the engine stays within its bounded lead meanwhile.
function showReport(kind, tick) {
  const model = reportModel(match.events, tick, panels.teams ?? []);
  if (kind === KIND.halfTime) {
    setPlaying(false);
  }
  reports.show(kind, model);
  signal('viewer.report_shown', { kind, tick });
  announce(kind === KIND.halfTime ? 'Half-time. The report is open.' : 'Full time. The report is open.');
}

/// Shows the first-run or error panel for `model`, or hides it for null.
function showSurface(model) {
  surface = model;
  const root = el('surface');
  if (!model) {
    root.hidden = true;
    root.dataset.kind = '';
    return;
  }
  root.dataset.kind = model.kind;
  el('surface-word').textContent = model.word;
  el('surface-title').textContent = model.title;
  el('surface-body').textContent = model.body ?? '';
  const path = el('surface-path');
  path.hidden = model.path === undefined;
  path.textContent = model.path ?? '';
  const hint = el('surface-hint');
  const hintText = model.instruction ?? model.hint ?? null;
  hint.hidden = !hintText;
  hint.textContent = hintText ?? '';
  const actions = {
    restart: el('surface-restart'),
    abandon: el('surface-abandon'),
    'save-replay': el('surface-save'),
    'open-replay': el('surface-open'),
  };
  for (const [action, button] of Object.entries(actions)) {
    button.hidden = !model.actions.includes(action);
    button.disabled = false;
  }
  if (model.restartLabel) {
    actions.restart.textContent = model.restartLabel;
  }
  el('skeleton').hidden = true;
  root.hidden = false;
  signal('viewer.recovery_panel', {
    kind: model.kind,
    reason: engineStatus?.['engine.reason'] ?? null,
  });
  announce(`${model.word}. ${model.title}`);
}

function hideSurface() {
  if (surface) {
    showSurface(null);
  }
}

/// Connects to the engine the status names.
function connect(status) {
  socket = new MatchSocket(socketAddress(status['socket.port'], status['protocol.version']), {
    onHello,
    onTick,
    onMessage,
    onRaw,
  });
  socket.onClose = onClose;
}

function onClose({ clean }) {
  if (storedReplay) {
    return;
  }
  // The engine closes the socket after full time on purpose, and that close is not a
  // fault: every tick is stored and the match plays back. Any other close is.
  if (match.fullTimeTick !== null) {
    setEngineWord('Engine finished');
    showNotice('Full time. The whole match is stored and plays back.', 'end');
    announce('Full time. The whole match is stored and plays back.');
    return;
  }
  recover(!clean);
}

/// After a close before full time: reconnect while the engine is still running, or show
/// what happened and what can still be done. A dropped connection never asks the manager.
async function recover(dropped) {
  reconnecting = true;
  setEngineWord('Engine reconnecting');
  showNotice('The connection to the engine dropped. Reconnecting.', 'reconnect');
  for (;;) {
    const status = await fetchStatus();
    engineStatus = status;
    const state = status?.['engine.state'];
    if (state === 'running' && status['socket.port']) {
      const delay = backoff(reconnectAttempt);
      reconnectAttempt += 1;
      signal('viewer.reconnecting', { attempt: reconnectAttempt, delay_ms: delay, dropped });
      await new Promise((resolve) => setTimeout(resolve, delay));
      connect(status);
      return;
    }
    if (state === 'starting') {
      await new Promise((resolve) => setTimeout(resolve, 250));
      continue;
    }
    reconnecting = false;
    setEngineWord('Engine stopped');
    const model = panelModel(status);
    if (model) {
      showNotice(null);
      showSurface(model);
    } else {
      showNotice('The match is no longer live. Start the engine again to watch another.', 'error');
      announce('The stream ended. The match is no longer live.');
    }
    return;
  }
}

async function restartEngine() {
  el('surface-restart').disabled = true;
  el('surface-abandon').disabled = true;
  announce('Restarting the engine from the last stoppage.');
  await restartMatch();
  const status = await poll((s) => !s || s['engine.state'] !== 'starting', {
    timeoutMs: 60_000,
  });
  engineStatus = status;
  if (status?.['engine.state'] === 'running') {
    showNotice('Restarting from the last stoppage.', 'reconnect');
    hideSurface();
    connect(status);
    return;
  }
  showSurface(panelModel(status) ?? panelModel(null));
}

async function abandonEngine() {
  el('surface-restart').disabled = true;
  el('surface-abandon').disabled = true;
  const after = await abandonMatch();
  engineStatus = after ?? { 'engine.state': 'abandoned' };
  if (socket) {
    socket.onClose = null;
    socket.close();
  }
  showSurface(panelModel({ 'engine.state': 'abandoned' }));
}

/// Writes every stored frame as a replay file and hands it to the browser as a download.
async function saveReplay() {
  if (frames.count === 0 || !matchId) {
    return;
  }
  const { bytes, hash } = await writeReplay(frames, {
    matchId,
    version: helloVersion ?? undefined,
  });
  const name = `touchline-${matchId}.smfx`;
  const url = URL.createObjectURL(new Blob([bytes], { type: 'application/octet-stream' }));
  const link = document.createElement('a');
  link.href = url;
  link.download = name;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
  lastSaved = { name, bytes, hash, frames: frames.count, ticks: frames.tickFrames };
  signal('viewer.replay_saved', {
    bytes: bytes.length,
    frames: frames.count,
    ticks: frames.tickFrames,
    hash,
  });
  announce(`Replay saved as ${name}.`);
}

/// Plays a replay file with no engine: every stored frame goes through the same path live
/// frames take, so playback and rewind are the same code as a live match.
async function openReplay(bytes, name = 'replay') {
  let read;
  try {
    read = await readReplay(bytes);
  } catch (error) {
    const reason = error.reason ?? error.message;
    signal('viewer.replay_refused', { reason });
    showSurface({
      kind: 'replay-refused',
      word: 'Error',
      title: `The replay could not be read: ${reason}`,
      body: 'Choose another replay file.',
      actions: ['open-replay'],
    });
    return;
  }
  storedReplay = true;
  if (socket) {
    socket.onClose = null;
    socket.close();
    socket = null;
  }
  reports.close();
  reportClock.reset();
  match.clear();
  stoppages.truncate(-1);
  frames = read.store;
  matchId = read.hello['match.id'];
  helloVersion = read.version;
  previous = newFrame();
  incoming = newFrame();
  renderedTick = 0;
  start(read.hello, { stored: true });
  for (let i = 1; i < frames.count; i += 1) {
    const { text, payload } = frames.frame(i);
    if (text) {
      const message = JSON.parse(frameText(payload));
      if (message.type !== 'hello') {
        onMessage(message);
      }
    } else {
      onTick(payload.slice().buffer, false);
    }
  }
  if (match.fullTimeTick !== null) {
    el('scrub').max = String(history.newestTick);
  }
  hideSurface();
  showNotice(null);
  el('skeleton').hidden = true;
  rewind(history.firstTick);
  setPlaying(true);
  signal('viewer.replay_loaded', { frames: read.frames, ticks: read.ticks, name });
  announce('Replay loaded. Playing from kick-off.');
}

/// The read-only test hook. Three acceptance criteria read it, and it exposes no setter:
/// a hook that can change the page is a hook that can hide a fault.
function hook() {
  globalThis.__touchline = {
    lastRendered: () => Array.from(rendered),
    lastRenderedTick: () => renderedTick,
    signals,
    history: () => ({
      ticks_stored: history ? history.count : 0,
      history_bytes: history ? history.bytes() : 0,
      page_bytes: history ? history.pageBytes : null,
      page_bytes_reason: history ? history.pageBytesReason : 'no history yet',
      newest_tick: history ? history.newestTick : 0,
    }),
    frame: () => scheduler.budget(),
    matchDay: () => (panels ? panels.snapshot() : null),
    pending: () => dugout.pending.all(renderedTick),
    lineup: () => dugout.lineupView(),
    dugout: () => ({
      phase: dugout.phase,
      lead_holding: dugout.lead.holding,
      lead_pauses: dugout.lead.pauses,
      tactics: dugout.panel ? copyTactics(dugout.panel.tactics) : null,
      subs_left: dugout.picker ? dugout.picker.left : null,
    }),
    lastRewind: () => lastRewind,
    stoppageAt: (tick) => stoppages.next(tick),
    tickAt: (tick) => {
      const out = new Int16Array(COMPONENT_COUNT);
      return history && history.tickAt(tick, out) ? Array.from(out) : null;
    },
    recovery: () => ({
      kind: surface ? surface.kind : null,
      title: surface ? surface.title : null,
      actions: surface ? [...surface.actions] : [],
      path: surface?.path ?? null,
      engine_state: engineStatus ? engineStatus['engine.state'] : null,
      engine_pid: engineStatus ? engineStatus['engine.pid'] ?? null : null,
      snapshot_tick: engineStatus ? engineStatus['snapshot.tick'] ?? null : null,
      reconnecting,
      step: stepShown,
    }),
    report: () => ({
      kind: reports ? reports.kind : null,
      open: reports ? reports.open : false,
      tick: reports?.model ? reports.model.tick : null,
      score: reports?.model ? [...reports.model.score] : null,
      counts: reports?.model
        ? Object.fromEntries(reports.model.rows.map((r) => [r.id, [...r.counts]]))
        : null,
    }),
    replay: () => ({
      stored: storedReplay,
      frames: frames.count,
      ticks: frames.tickFrames,
      last_saved_name: lastSaved ? lastSaved.name : null,
      last_saved_size: lastSaved ? lastSaved.bytes.length : null,
      last_saved_hash: lastSaved ? lastSaved.hash : null,
    }),
    lastSavedBytes: () => (lastSaved ? Array.from(lastSaved.bytes) : null),
    events: () => match.events.map((e) => ({ ...e })),
  };
}

async function main() {
  // The mark is drawn from the live token values, never loaded from an image file.
  const mark = markColours(document);
  setFavicon(document, 32, mark);
  drawMark(el('mark').getContext('2d'), 0, 0, 28, mark);
  watchMotion(document, globalThis);
  panels = new Panels();

  wire();
  hook();
  requestAnimationFrame(frame);

  // The launcher reports `starting` until the engine has opened its socket.
  setStep(0);
  const engine = await poll((s) => !s || s['engine.state'] !== 'starting', {
    timeoutMs: 60_000,
  });
  engineStatus = engine;
  if (engine?.['engine.state'] === 'running' && engine['socket.port']) {
    setStep(1);
    connect(engine);
    return;
  }
  showSurface(panelModel(engine) ?? panelModel(null));
}

main();
