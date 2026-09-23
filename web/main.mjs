// The page: socket in, pitch out, controls between them.

import { COMPONENT_COUNT, decodeInto, newFrame } from './decode.mjs';
import { Feed, minuteStamp } from './feed.mjs';
import { GoalMoment, bannerText, watchMotion } from './goal-moment.mjs';
import { History } from './history.mjs';
import { between } from './interpolate.mjs';
import { Lineups } from './lineups.mjs';
import { colours as markColours, drawMark, setFavicon } from './mark.mjs';
import { MatchState } from './match-state.mjs';
import { Pitch } from './pitch.mjs';
import { Playback, SPEEDS } from './playback.mjs';
import { Scheduler, TICKS_PER_SECOND } from './schedule.mjs';
import { Scoreboard } from './scoreboard.mjs';
import { signal, signals } from './signal.mjs';
import { MatchSocket, socketAddress } from './socket.mjs';
import { StatsPanel } from './stats.mjs';
import { Stoppages, stopsPlay } from './stoppages.mjs';

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
    this.flush(0, { seek: true });
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
