// The front door's order of screens: the splash's timing, the start screen, match setup, the
// settings, the menu over a match, return to start and quit. Fake timers, a fake launcher and
// a fake session stand in for the browser; nothing here opens a port.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { FLOOR_MS, FrontDoor, READY_HOLD_MS, REVEAL_MS } from '../src/lib/front-door.svelte.js';
import {
  savedClock,
  closedModel,
  licenceFacts,
  savedLine,
  settingsFacts,
  setupFacts,
  startFacts,
  startItems,
  teamOf,
} from '../src/lib/front-door-model.js';

/// Timers on a clock the test moves.
function fakeTime() {
  let now = 0;
  let id = 0;
  const due = new Map();
  return {
    now: () => now,
    timers: {
      setTimeout(fn, ms) {
        id += 1;
        due.set(id, { fn, at: now + ms });
        return id;
      },
      clearTimeout(handle) {
        due.delete(handle);
      },
    },
    advance(ms) {
      const end = now + ms;
      for (;;) {
        const next = [...due.entries()].filter(([, t]) => t.at <= end).sort((a, b) => a[1].at - b[1].at)[0];
        if (!next) {
          break;
        }
        due.delete(next[0]);
        now = next[1].at;
        next[1].fn();
      }
      now = end;
    },
  };
}

const TEAMS = [
  { id: 'club-00000001-00', name: 'Oakmere Rangers', short_name: 'OAK', kit: ['#1d4ed8', '#ffffff'], ground: [105, 68], strength: 12.5 },
  { id: 'club-00000002-00', name: 'Eldstead City', short_name: 'ELD', kit: ['#b91c1c', '#ffffff'], ground: [100, 64], strength: 11.0 },
  { id: 'club-000007ea-00', name: 'Belfield Athletic', short_name: 'BEL', kit: ['#065f46', '#fde047'], ground: [105, 68], strength: 9.6 },
];

const SAVED = {
  kind: 'current',
  version: '0.2.0-dev',
  tick: 156500,
  teams: ['Oakmere Rangers', 'Eldstead City'],
  score: [1, 1],
  millis: 3130000,
  positions: { pitch: [105, 68], players: [[0, 50, 30], [1, 60, 40]] },
};

const IDLE = {
  'engine.state': 'idle',
  'engine.version': '0.2.0-dev',
  'launcher.version': '0.2.0-dev',
  'previous.version': '0.2.0-beta.1',
  'protocol.version': 7,
  'front-door': true,
  teams: TEAMS,
  saved: null,
  settings: { schema_version: 1, speed: 1, motion: 'follow', commentary: true },
};

/// A launcher that answers each path from `routes` (a body, or a function of the request).
function fakeLauncher(routes) {
  const calls = [];
  const fetcher = async (path, init = {}) => {
    calls.push({ path, method: init.method ?? 'GET', body: init.body ? JSON.parse(init.body) : undefined });
    const key = Object.keys(routes).find((k) => path.startsWith(k));
    const route = key ? routes[key] : null;
    const body = typeof route === 'function' ? route(path, init) : route;
    return { ok: body !== null && body !== undefined, json: async () => body };
  };
  fetcher.calls = calls;
  return fetcher;
}

class FakeSession {
  static made = [];

  constructor(options) {
    this.options = options;
    this.view = 'match';
    this.started = false;
    this.disposed = false;
    this.playing = true;
    this.commentary = options.settings?.commentary ?? true;
    this.menuOpen = false;
    this.onMenu = null;
    this.renderedTick = 4000;
    this.clockText = '00:40';
    this.stoppages = { prev: () => 3600 };
    this.replays = [];
    FakeSession.made.push(this);
  }

  start() {
    this.started = true;
  }

  get isOver() {
    return this.screen === 'full-time';
  }

  dispose() {
    this.disposed = true;
  }

  pauseForMenu() {
    this.before = this.playing;
    this.playing = false;
  }

  resumeFromMenu() {
    this.playing = this.before;
  }

  async openReplay(bytes, name, opts) {
    this.replays.push({ name, opts });
    this.view = 'replay';
  }
}

function door(routes = {}, { reduced = false } = {}) {
  const time = fakeTime();
  const fetcher = fakeLauncher({ 'engine.json': IDLE, ...routes });
  FakeSession.made = [];
  const sessions = [];
  const d = new FrontDoor({
    fetcher,
    timers: time.timers,
    now: time.now,
    makeSession: (options) => new FakeSession(options),
    onSession: (s) => sessions.push(s),
    reduced: () => reduced,
  });
  return { d, time, fetcher, sessions };
}

const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

test('the splash waits for the reveal and the answer, then holds the ready state 1.5 s', () => {
  const { d, time } = door();
  assert.equal(d.view, 'splash');
  assert.equal(d.splash.phase, 'reveal');
  time.advance(3000);
  assert.equal(d.view, 'splash', 'no answer yet: the splash stays');
  d.answer(IDLE);
  assert.equal(d.splash.phase, 'ready', 'the reveal is over and the engine answered');
  time.advance(READY_HOLD_MS - 1);
  assert.equal(d.view, 'splash');
  time.advance(1);
  assert.equal(d.view, 'start');
});

test('a quick answer still waits for the reveal, and never leaves before 1.5 s', () => {
  const { d, time } = door();
  time.advance(100);
  d.answer(IDLE);
  assert.equal(d.splash.phase, 'reveal');
  time.advance(REVEAL_MS - 100 - 1);
  assert.equal(d.splash.phase, 'reveal');
  time.advance(1);
  assert.equal(d.splash.phase, 'ready');
  assert.ok(REVEAL_MS >= FLOOR_MS);
});

test('a key before the answer finishes the reveal only; a key after it opens the start screen at once', () => {
  const { d, time } = door();
  time.advance(500);
  d.press();
  assert.equal(d.splash.finished, true);
  assert.equal(d.view, 'splash');
  time.advance(700);
  d.answer(IDLE);
  d.press();
  assert.equal(d.view, 'start', 'a key after the answer opens the start screen, inside the floor too');
});

test('with reduced motion the reveal shows its last frame at once', () => {
  const { d } = door({}, { reduced: true });
  assert.equal(d.splash.finished, true);
});

test('without the start screen the session starts at the first answer, as before', () => {
  const { d, sessions } = door();
  d.answer({ 'engine.state': 'running', 'socket.port': 7001 });
  assert.equal(d.view, 'match');
  const session = sessions.at(-1);
  assert.equal(session.started, true);
  assert.equal(session.onMenu, null, 'no start screen to return to: no Menu');
  assert.equal(sessions[0].disposed, true, 'the idle session is disposed');
});

test('no answer at all starts the session, which shows the engine is missing', () => {
  const { d, sessions } = door();
  d.answer(null);
  assert.equal(d.view, 'match');
  assert.equal(sessions.at(-1).started, true);
});

test('a reloaded page whose launcher already plays goes to the match', () => {
  const { d, sessions } = door();
  d.answer({ ...IDLE, 'engine.state': 'running' });
  d.press();
  assert.equal(d.view, 'match');
  assert.equal(sessions.at(-1).started, true);
});

test('the start screen offers six entries, Resume disabled with its reason when nothing is saved', () => {
  const items = startItems(null);
  assert.deepEqual(
    items.map((i) => i.label),
    ['New match', 'Resume', 'Replays', 'Settings', 'Licences and about', 'Quit']
  );
  const resume = items.find((i) => i.id === 'resume');
  assert.equal(resume.disabled, true);
  assert.equal(resume.sub, 'No saved match yet');
  assert.equal(items.find((i) => i.id === 'quit').rule, true);
  const withSave = startItems(SAVED).find((i) => i.id === 'resume');
  assert.equal(withSave.disabled, undefined);
  assert.equal(withSave.sub, 'Oakmere Rangers 1–1 Eldstead City · 52:10');
  assert.equal(savedLine(SAVED), 'Oakmere Rangers 1–1 Eldstead City · 52:10');
  assert.equal(savedClock(SAVED), '52:10', 'tick 156500 at 50 ticks a second');
  assert.equal(startFacts({ ...IDLE, saved: SAVED })[2].value, '1 saved match');
  assert.equal(startFacts(IDLE)[2].value, 'No saved match');
});

test('match setup picks the two default teams, asks for the round and refuses the same team twice', async () => {
  const round = { fixtures: [{ home: TEAMS[2], away: null }] };
  const { d, fetcher } = door({ 'engine/round': round });
  d.answer(IDLE);
  d.press();
  d.open('setup');
  assert.deepEqual(d.picks, { home: 'club-00000001-00', away: 'club-00000002-00' });
  await flush();
  assert.deepEqual(d.round, round);
  assert.ok(fetcher.calls.some((c) => c.path === 'engine/round?home=club-00000001-00&away=club-00000002-00'));
  assert.equal(d.pickProblem, null);

  d.pick('away', 'club-00000001-00');
  assert.equal(d.pickProblem, 'A team cannot play itself. Pick a different away team.');
  assert.equal(d.round, null);
  const facts = setupFacts(TEAMS[0], TEAMS[0], null);
  assert.equal(facts[1].label, 'Away team · same as home');
  assert.equal(facts[1].tone, 'bad');
  await d.kickOff();
  assert.ok(!fetcher.calls.some((c) => c.path === 'engine/new-match'), 'the same team twice posts nothing');
});

test('Kick off posts the pair and starts a new session with the settings', async () => {
  const { d, fetcher, sessions } = door({
    'engine/new-match': { ...IDLE, 'engine.state': 'starting' },
    'engine/round': { fixtures: [] },
  });
  d.answer({ ...IDLE, settings: { speed: 4, motion: 'full', commentary: false } });
  d.press();
  d.open('setup');
  await d.kickOff();
  const post = fetcher.calls.find((c) => c.path === 'engine/new-match');
  assert.deepEqual(post.body, { home: 'club-00000001-00', away: 'club-00000002-00' });
  assert.equal(post.method, 'POST');
  assert.equal(d.view, 'match');
  const session = sessions.at(-1);
  assert.equal(session.started, true);
  assert.deepEqual(session.options.settings, { speed: 4, motion: 'full', commentary: false });
  assert.equal(typeof session.onMenu, 'function');
});

test('a settings change applies at once and is saved', async () => {
  const saved = { ...IDLE, settings: { schema_version: 1, speed: 8, motion: 'reduce', commentary: false } };
  const { d, fetcher, sessions } = door({ 'engine/settings': saved });
  d.answer(IDLE);
  d.press();
  d.open('settings');
  await d.saveSettings({ speed: 8, motion: 'reduce', commentary: false });
  const post = fetcher.calls.find((c) => c.path === 'engine/settings');
  assert.deepEqual(post.body, { speed: 8, motion: 'reduce', commentary: false });
  assert.deepEqual(d.settings, { speed: 8, motion: 'reduce', commentary: false });
  assert.equal(d.saved, true);
  assert.equal(sessions.at(-1).commentary, false);
  assert.equal(settingsFacts(d.settings, false, true)[1].label, 'Motion · reduced motion now');
});

test('Resume posts resume and starts the saved match; with no save it does nothing', async () => {
  const { d, fetcher, sessions } = door({ 'engine/resume': { ...IDLE, 'engine.state': 'starting' } });
  d.answer(IDLE);
  d.press();
  await d.resume();
  assert.ok(!fetcher.calls.some((c) => c.path === 'engine/resume'));
  d.status = { ...IDLE, saved: SAVED };
  await d.resume();
  assert.ok(fetcher.calls.some((c) => c.path === 'engine/resume' && c.method === 'POST'));
  assert.equal(d.view, 'match');
  assert.equal(sessions.at(-1).started, true);
});

test('the menu pauses the match; Esc and Resume match close it and play on', async () => {
  const { d, sessions } = door({ 'engine/new-match': IDLE, 'engine/round': { fixtures: [] } });
  d.answer(IDLE);
  d.press();
  d.open('setup');
  await d.kickOff();
  const session = sessions.at(-1);
  session.onMenu();
  assert.equal(d.overlay, 'menu');
  assert.equal(session.playing, false);
  assert.equal(session.menuOpen, true);
  d.escape();
  assert.equal(d.overlay, null);
  assert.equal(session.playing, true);
  d.escape();
  assert.equal(d.overlay, 'menu', 'Esc opens the menu too');

  session.view = 'report';
  d.closeMenu();
  d.openMenu();
  assert.equal(d.overlay, null, 'the menu opens on the match, Tactics and Touchline views only');
});

test('Settings over the paused match returns to it with the menu open', async () => {
  const { d, sessions } = door({ 'engine/new-match': IDLE, 'engine/round': { fixtures: [] } });
  d.answer(IDLE);
  d.press();
  d.open('setup');
  await d.kickOff();
  sessions.at(-1).onMenu();
  d.open('settings');
  assert.equal(d.view, 'settings');
  assert.equal(d.returnTo, 'match');
  d.open('licences');
  assert.equal(d.returnTo, 'match', 'Licences from Settings keeps the way back');
  d.back();
  assert.equal(d.view, 'match');
  assert.equal(d.overlay, 'menu');
  assert.equal(sessions.at(-1).playing, false, 'still paused');
});

test('Return to start asks first, names the saved clock, stops the match and keeps Resume', async () => {
  const stopped = { ...IDLE, saved: SAVED };
  const { d, fetcher, sessions } = door({
    'engine.json': { ...IDLE, 'snapshot.tick': 120000 },
    'engine/new-match': IDLE,
    'engine/round': { fixtures: [] },
    'engine/stop': stopped,
  });
  d.answer(IDLE);
  d.press();
  d.open('setup');
  await d.kickOff();
  const session = sessions.at(-1);
  session.onMenu();
  await d.ask('return');
  assert.equal(d.overlay, 'return');
  assert.equal(d.saveClock, '40:00', 'tick 120000 at 50 ticks a second');
  await d.returnToStart();
  assert.ok(fetcher.calls.some((c) => c.path === 'engine/stop' && c.method === 'POST'));
  assert.equal(session.disposed, true);
  assert.equal(d.view, 'start');
  assert.deepEqual(d.savedMatch, SAVED);
  assert.equal(startItems(d.savedMatch).find((i) => i.id === 'resume').disabled, undefined);
});

test('Keep playing closes the dialog and plays on', async () => {
  const { d, sessions } = door({ 'engine/new-match': IDLE, 'engine/round': { fixtures: [] } });
  d.answer(IDLE);
  d.press();
  d.open('setup');
  await d.kickOff();
  sessions.at(-1).onMenu();
  await d.ask('quit');
  d.closeMenu();
  assert.equal(d.overlay, null);
  assert.equal(sessions.at(-1).playing, true);
});

test('Save and quit posts quit and shows the closed page from the answer', async () => {
  const answer = { ...IDLE, 'engine.state': 'closed', closed: { match: true, saved: SAVED } };
  const { d, fetcher, sessions } = door({ 'engine/quit': answer, 'engine/new-match': IDLE, 'engine/round': { fixtures: [] } });
  d.answer(IDLE);
  d.press();
  d.open('setup');
  await d.kickOff();
  await d.quit();
  assert.ok(fetcher.calls.some((c) => c.path === 'engine/quit' && c.method === 'POST'));
  assert.equal(d.view, 'closed');
  assert.equal(sessions.at(-1).disposed, true);
  const model = closedModel(d.closed, d.status);
  assert.deepEqual(
    model.steps.map((s) => s.label),
    ['Match saved', 'Match engine stopped', 'Launcher stopped']
  );
  assert.match(model.line, /saved at 52:10, Oakmere Rangers 1–1 Eldstead City/);
});

test('Quit from the start screen has no match step', async () => {
  const answer = { ...IDLE, 'engine.state': 'closed', closed: { match: false, saved: null } };
  const { d } = door({ 'engine/quit': answer });
  d.answer(IDLE);
  d.press();
  await d.quit();
  const model = closedModel(d.closed, d.status);
  assert.deepEqual(
    model.steps.map((s) => s.label),
    ['Match engine stopped', 'Launcher stopped']
  );
});

test('a replay from the start screen returns there when it closes, or says why it could not be read', async () => {
  const { d, sessions } = door();
  d.answer(IDLE);
  d.press();
  await d.openReplay(new Uint8Array([1]), 'a.smfx');
  assert.equal(d.view, 'match');
  const session = sessions.at(-1);
  assert.deepEqual(session.replays[0].opts, { leave: true });
  session.options.onLeave({});
  assert.equal(d.view, 'start');
  assert.equal(d.message, null);
  await d.openReplay(new Uint8Array([1]), 'b.smfx');
  sessions.at(-1).options.onLeave({ refused: 'not a replay file' });
  assert.equal(d.message, 'The replay could not be read: not a replay file');
});

test('the licences screen reads the notices file once, or says the build carries none', async () => {
  const file = { version: '0.2.0-dev', packages: [{ kind: 'crate', name: 'rhai', version: '1.26.1', licence: 'MIT OR Apache-2.0', text: 'MIT' }] };
  const { d, fetcher } = door({ 'notices.json': file });
  d.answer(IDLE);
  d.press();
  d.open('licences');
  await flush();
  assert.equal(d.notices.state, 'ready');
  d.open('licences');
  await flush();
  assert.equal(fetcher.calls.filter((c) => c.path === 'notices.json').length, 1);
  const facts = licenceFacts(IDLE, file);
  assert.equal(facts[3].value, '1 packages');
  assert.equal(facts[4].value, 'MIT · Apache-2.0');

  const none = door({ 'notices.json': null });
  none.d.answer(IDLE);
  none.d.open('licences');
  await flush();
  assert.equal(none.d.notices.state, 'missing');
});

test('a sample team reads as a crest team', () => {
  assert.deepEqual(teamOf(TEAMS[0]), {
    'team.id': 'club-00000001-00',
    'team.name': 'Oakmere Rangers',
    'team.kit.primary': '#1d4ed8',
    'team.kit.secondary': '#ffffff',
  });
  assert.equal(teamOf(null), null);
});

// ---- After full time ---------------------------------------------------------------------------

/// A door whose match ended: the session is at full time with its report `ready` (or not), and
/// it played `teams` (the hello's clubs). `states` are the launcher's engine states in turn.
async function finished({ ready = true, teams = null, states = ['finished'], stop = { ...IDLE } } = {}) {
  const seen = [...states];
  const { d, fetcher, sessions } = door({
    'engine.json': () => ({ ...IDLE, 'engine.state': seen.length > 1 ? seen.shift() : seen[0] }),
    'engine/new-match': IDLE,
    'engine/round': { fixtures: [] },
    'engine/stop': stop,
  });
  d.answer(IDLE);
  d.press();
  d.open('setup');
  d.pick('home', TEAMS[2].id);
  d.pick('away', TEAMS[0].id);
  await d.kickOff();
  const session = sessions.at(-1);
  session.screen = 'full-time';
  session.nextReady = ready;
  session.teams = teams ?? [
    { 'team.id': TEAMS[1].id, 'team.name': TEAMS[1].name },
    { 'team.id': TEAMS[0].id, 'team.name': TEAMS[0].name },
  ];
  return { d, fetcher, sessions, session };
}

const stops = (fetcher) => fetcher.calls.filter((c) => c.path === 'engine/stop' && c.method === 'POST').length;

test('the front door gives each session its next steps, and a page with no start screen none', async () => {
  const { session } = await finished();
  assert.equal(typeof session.onNextStep, 'function');
  const { d, sessions } = door({ 'engine.json': { ...IDLE, 'front-door': false } });
  d.answer({ ...IDLE, 'front-door': false });
  assert.equal(sessions.at(-1).onNextStep, null);
});

test('Return to start after full time waits for the worker to end, stops it and shows the start screen', async () => {
  const { d, fetcher, session } = await finished({ states: ['running', 'finished'] });
  await session.onNextStep('return');
  const order = fetcher.calls.map((c) => c.path).filter((p) => p === 'engine.json' || p === 'engine/stop');
  assert.deepEqual(order.slice(-3), ['engine.json', 'engine.json', 'engine/stop'], 'the stop waits for the worker');
  assert.equal(session.disposed, true);
  assert.equal(d.view, 'start');
  assert.equal(d.savedMatch, null, 'a finished match leaves nothing to resume');
});

test('New match after full time shows setup with the ended match’s clubs and asks for the round', async () => {
  const { d, fetcher, session } = await finished();
  await session.onNextStep('new');
  assert.equal(stops(fetcher), 1);
  assert.equal(session.disposed, true);
  assert.equal(d.view, 'setup');
  assert.deepEqual(d.picks, { home: TEAMS[1].id, away: TEAMS[0].id }, 'the hello’s clubs, not the last picks');
  const round = fetcher.calls.filter((c) => c.path.startsWith('engine/round')).at(-1);
  assert.match(round.path, new RegExp(`home=${TEAMS[1].id}&away=${TEAMS[0].id}`));
});

test('the ended clubs are found by name when the ids differ, and the last picks stand in for a stranger', async () => {
  const byName = await finished({
    teams: [
      { 'team.id': 'other-1', 'team.name': TEAMS[0].name },
      { 'team.id': 'other-2', 'team.name': TEAMS[2].name },
    ],
  });
  await byName.session.onNextStep('new');
  assert.deepEqual(byName.d.picks, { home: TEAMS[0].id, away: TEAMS[2].id });
  const stranger = await finished({
    teams: [
      { 'team.id': 'x', 'team.name': 'Nowhere Town' },
      { 'team.id': 'y', 'team.name': 'Elsewhere' },
    ],
  });
  await stranger.session.onNextStep('new');
  assert.deepEqual(stranger.d.picks, { home: TEAMS[2].id, away: TEAMS[0].id }, 'the picks of the match that ended');
});

test('nothing runs while the match is still stored', async () => {
  const { d, fetcher, session } = await finished({ ready: false });
  await session.onNextStep('new');
  await session.onNextStep('return');
  assert.equal(stops(fetcher), 0);
  assert.equal(session.disposed, false);
  assert.equal(d.view, 'match');
});

test('a stop the launcher refuses still leads on, with a line that says so', async () => {
  const { d, session } = await finished({ stop: null });
  await session.onNextStep('return');
  assert.equal(d.view, 'start');
  assert.match(d.message, /^The launcher did not answer the stop\. Start a new match, or quit\.$/);
});

test('at full time the menu’s Return to start leaves at once, and Quit says the match is not kept', async () => {
  const { d, fetcher, session } = await finished();
  assert.match(d.quitLines[0], /^The match is over and is not kept for Resume\.$/);
  session.onMenu();
  await d.ask('return');
  assert.equal(d.overlay, null, 'no confirmation');
  assert.equal(stops(fetcher), 1);
  assert.equal(d.view, 'start');

  const early = await finished({ ready: false });
  early.session.onMenu();
  await early.d.ask('return');
  assert.equal(early.d.overlay, 'return', 'before the match is stored the dialog still asks');
  assert.match(early.d.quitLines[0], /^The match saves at /);
});

test('at full time the quit button says Quit, since nothing is saved', async () => {
  const { d } = await finished();
  assert.equal(d.quitPrimary, 'Quit');
  const early = await finished({ ready: false });
  assert.equal(early.d.quitPrimary, 'Save and quit');
});
