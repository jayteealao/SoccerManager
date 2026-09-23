// The page: socket in, pitch out, controls between them.

import { COMPONENT_COUNT, decodeInto, newFrame } from './decode.mjs';
import { Feed, minuteStamp } from './feed.mjs';
import { GoalMoment, bannerText, watchMotion } from './goal-moment.mjs';
import { History } from './history.mjs';
import { between } from './interpolate.mjs';
import { LeadControl } from './lead.mjs';
import { LineupEditor } from './lineup-editor.mjs';
import { Lineups, benchModel } from './lineups.mjs';
import { colours as markColours, drawMark, setFavicon } from './mark.mjs';
import { KIND, MatchState } from './match-state.mjs';
import { createPendingList } from './pending.mjs';
import { Pitch } from './pitch.mjs';
import { Playback, SPEEDS } from './playback.mjs';
import { Scheduler, TICKS_PER_SECOND } from './schedule.mjs';
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

/// The clock, as a manager reads it: minutes and seconds of match time.
function clockText(tick) {
  const seconds = Math.floor(tick / TICKS_PER_SECOND);
  return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
}

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
  const words = { error: 'Stream ended', end: 'Full time', lag: 'Lag' };
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

function start(hello) {
  history = new History(hello.ticks_expected);
  pitch = new Pitch(el('pitch'), [
    { primary: hello.teams[0]['team.kit.primary'], secondary: hello.teams[0]['team.kit.secondary'] },
    { primary: hello.teams[1]['team.kit.primary'], secondary: hello.teams[1]['team.kit.secondary'] },
  ]);
  panels.start(hello);
  dugout.begin(hello);
  el('engine-version').textContent = `engine ${hello['engine.version']}`;
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

function onTick(buffer) {
  const result = decodeInto(buffer, previous, incoming);
  if (!result) {
    return;
  }
  history.append(incoming.tick, incoming.components);
  socket.noteTick(incoming.tick);
  playback.noteArrival(performance.now(), incoming.tick);
  // Paced on arrival as well as on each drawn frame: a hidden or throttled tab draws few
  // frames, and the engine must still stop a few seconds ahead of the drawn tick.
  dugout.pace(history.newestTick - renderedTick);
  if (result.kind !== 'delta') {
    history.report();
    history.measurePage();
    if (result.kind === 'restart') {
      stoppages.add(incoming.tick);
    }
  }
  if (!scrubbing) {
    el('scrub').value = String(incoming.tick);
  }
  [previous, incoming] = [incoming, previous];
  revealPitch();
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
  el('gauge').textContent = gaugeText();
  dugout.pace(history.newestTick - renderedTick);
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
    this.scoreboard.setClock(clockText(tick));
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
    this.editor = null;
    this.panel = null;
    this.picker = null;
    this.chipKey = '';
    this.slotKey = '';
    this.seenRejections = new Set();
    this.over = false;
  }

  begin(hello) {
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
    if (!this.setup || !this.schema || !Array.isArray(this.schema.formations)) {
      // A recording, or an engine that takes no lineup: nothing reaches a live match.
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
  announce(`Rewound to ${clockText(tick)}.`);
}

function wire() {
  el('play').addEventListener('click', () => {
    const playing = el('play').dataset.playing !== 'true';
    scheduler.setPlaying(playing);
    el('play').dataset.playing = String(playing);
    el('play').textContent = playing ? 'Pause' : 'Play';
    announce(playing ? 'Playing.' : 'Paused.');
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

  const engine = await fetch('engine.json').then((r) => r.json());
  socket = new MatchSocket(socketAddress(engine['socket.port'], engine['protocol.version']), {
    onHello: start,
    onTick,
    onMessage,
  });
  socket.onClose = () => {
    // The engine closes the socket after full time on purpose, and that close is not a
    // fault: every tick is stored and the match plays back. Any other close is.
    if (match.fullTimeTick !== null) {
      showNotice('Full time. The whole match is stored and plays back.', 'end');
      announce('Full time. The whole match is stored and plays back.');
      return;
    }
    // The canvas error state, stubbed here and completed in a later version: the panel and
    // the text, with no recovery action yet.
    showNotice('The match is no longer live. Start the engine again to watch another.', 'error');
    announce('The stream ended. The match is no longer live.');
  };
}

main();
