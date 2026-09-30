// The match session: the screen it shows in each state, the commands it sends, and the
// panels it keeps at the rendered tick. A fake socket and a fake status endpoint stand in for
// the engine; nothing here opens a port.

import assert from 'node:assert/strict';
import { afterEach, beforeEach, test } from 'vitest';

import { COMPONENT_COUNT } from '../src/lib/decode.js';
import { ACTIONS, MatchSession, NOTICES, SCREENS } from '../src/lib/match-session.svelte.js';
import { clearSignals } from '../src/lib/signal.js';
import { encodeKeyframe, eventMessage, roster, statsMessage } from './helpers.js';

/// A WebSocket that records what the page sends and lets the test deliver frames.
class FakeSocket {
  static OPEN = 1;
  static made = [];

  constructor(address) {
    this.address = address;
    this.readyState = FakeSocket.OPEN;
    this.sent = [];
    this.listeners = {};
    this.binaryType = 'blob';
    FakeSocket.made.push(this);
  }

  addEventListener(type, fn) {
    (this.listeners[type] ??= []).push(fn);
  }

  send(text) {
    this.sent.push(JSON.parse(text));
  }

  close() {
    this.readyState = 3;
  }

  deliver(data) {
    for (const fn of this.listeners.message ?? []) {
      fn({ data: typeof data === 'string' ? data : data.buffer ?? data });
    }
  }

  drop() {
    this.readyState = 3;
    for (const fn of this.listeners.close ?? []) {
      fn({ code: 1006, wasClean: false, reason: '' });
    }
  }
}

/// A status endpoint that answers from a list of bodies, the last one repeating.
function statusFetch(...bodies) {
  const calls = [];
  const fetcher = async (path, init = {}) => {
    calls.push({ path, method: init.method ?? 'GET' });
    const body = bodies.length > 1 ? bodies.shift() : bodies[0];
    return { ok: body !== null, json: async () => body };
  };
  fetcher.calls = calls;
  return fetcher;
}

const RUNNING = { 'engine.state': 'running', 'socket.port': 7001, 'protocol.version': 3, launcher: true };

const TEAMS = [
  {
    'team.id': 'club-a',
    'team.name': 'Ashford Rovers',
    'team.kit.primary': '#c8102e',
    'team.kit.secondary': '#000000',
    roster: roster(0),
  },
  {
    'team.id': 'club-b',
    'team.name': 'Port Varrow',
    'team.kit.primary': '#6a0dad',
    'team.kit.secondary': '#ff6a13',
    roster: roster(1),
  },
];

const hello = (id = 'match-1') =>
  JSON.stringify({
    type: 'hello',
    'match.id': id,
    'protocol.version': 3,
    'engine.version': '0.3.0',
    ticks_expected: 270000,
    teams: TEAMS,
  });

const frameAt = (tick) => encodeKeyframe(tick, new Array(COMPONENT_COUNT).fill(tick % 100));

/// A session wired to the fakes, with no animation-frame loop: the test draws frames.
async function started(...bodies) {
  const fetcher = statusFetch(...bodies);
  const timers = { setTimeout: (fn, ms) => ({ fn, ms }), clearTimeout: () => {} };
  const session = new MatchSession({ fetcher, timers, raf: null, now: () => 0, doc: null });
  await session.start();
  return { session, fetcher, socket: FakeSocket.made.at(-1) };
}

/// Plays frames `from` to `to` into the session and draws `to`.
function playTo(session, socket, from, to) {
  for (let t = from; t <= to; t += 1) {
    socket.deliver(frameAt(t));
  }
  session.rewind(to);
}

let realSocket;
let realInfo;
beforeEach(() => {
  realSocket = globalThis.WebSocket;
  realInfo = console.info;
  console.info = () => {};
  globalThis.WebSocket = FakeSocket;
  FakeSocket.made = [];
  clearSignals();
});
afterEach(() => {
  globalThis.WebSocket = realSocket;
  console.info = realInfo;
});

test('loading, then kick-off on the hello, then live once the kick-off is sent', async () => {
  const { session, socket } = await started(RUNNING);
  assert.equal(socket.address, 'ws://127.0.0.1:7001/?v=3');
  assert.equal(session.screen, 'loading');
  assert.equal(session.action, 'Please wait');
  assert.equal(session.actionBusy, true);
  assert.equal(session.steps[1].state, 'active', 'connecting to the match');

  socket.deliver(hello());
  assert.equal(session.screen, 'kickoff');
  assert.equal(session.action, 'Kick off');
  assert.equal(session.title, 'Ashford Rovers v Port Varrow');
  assert.equal(session.subtitle, 'Engine connected · v0.3.0');
  assert.equal(session.dateWord, 'KICK-OFF');
  assert.equal(session.dateClock, '00:00');
  assert.equal(session.tag.text, 'KICK-OFF · 00:00');
  assert.equal(session.steps[2].state, 'active', 'waiting for kick-off');

  session.act();
  assert.deepEqual(socket.sent, [{ type: 'seen', tick: 0 }, { type: 'start' }]);
  assert.equal(session.screen, 'live');
  assert.equal(session.action, 'Pause');
  assert.equal(session.tag.live, true);
});

test('with a lineup to pick, kick-off sends it and the match starts on the engine ack', async () => {
  const { session, socket } = await started(RUNNING);
  const squad = roster(0).map((p) => ({ ...p, 'player.natural_fitness': 60, role_fit: [] }));
  const setup = {
    lineup: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    bench: [11, 12, 13, 14, 15, 16, 17],
    formation: 0,
    mentality: 2,
    instructions: [1, 1, 1, 1, 1, 0],
    roles: Array.from({ length: 11 }, () => ({ role: 0, duty: 1 })),
  };
  const tactics = {
    formations: [{ slots: squad.slice(0, 11).map((p, i) => ({ position: p['player.position'], x: i * 9, y: 0 })) }],
    roles: [],
    ai: { bench_size: 7 },
  };
  const message = JSON.parse(hello());
  message.tactics = tactics;
  message.teams[0] = { ...message.teams[0], squad, setup };
  socket.deliver(JSON.stringify(message));
  assert.equal(session.view, 'tactics', 'the Tactics screen opens before kick-off');
  session.act();
  assert.deepEqual(socket.sent, [{ type: 'set-lineup', lineup: setup.lineup, bench: setup.bench }]);
  assert.equal(session.screen, 'kickoff', 'nothing starts before the engine answers');
  assert.equal(session.action, 'Kicking off');
  socket.deliver(JSON.stringify({ type: 'ack', command: 'set-lineup' }));
  assert.deepEqual(socket.sent.slice(1), [{ type: 'seen', tick: 0 }, { type: 'start' }]);
  assert.equal(session.screen, 'live');
  assert.equal(session.view, 'match');
});

test('pause and resume follow the action block, and the date block names the state', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  session.act();
  playTo(session, socket, 1, 3000);
  assert.equal(session.dateWord, 'LIVE');
  assert.equal(session.dateClock, '01:00');
  assert.equal(session.title, 'Ashford Rovers 0–0 Port Varrow');
  session.act();
  assert.equal(session.screen, 'paused');
  assert.equal(session.playing, false);
  assert.equal(session.scheduler.playing, false);
  assert.equal(session.action, 'Resume');
  assert.equal(session.dateWord, 'PAUSED');
  session.act();
  assert.equal(session.screen, 'live');
  assert.equal(session.scheduler.playing, true);
});

test('a recording that plays on its own goes live with no kick-off', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  socket.deliver(frameAt(1));
  assert.equal(session.screen, 'live');
  assert.equal(socket.sent.length, 0, 'nothing is sent to a recording');
});

test('a crashed engine gives the error panel with Restart and Abandon, stopped at its stoppage', async () => {
  const crashed = {
    'engine.state': 'crashed',
    'engine.code': 3,
    'snapshot.tick': 156500,
    launcher: true,
  };
  const { session, socket } = await started(RUNNING, crashed);
  socket.deliver(hello());
  session.act();
  playTo(session, socket, 1, 50);
  socket.drop();
  // The recovery reads the status once more; let its promise settle.
  await new Promise((resolve) => setTimeout(resolve, 0));
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(session.screen, 'error');
  assert.equal(session.panel.title, 'The engine stopped (exit code 3)');
  assert.deepEqual(session.panel.actions, ['restart', 'abandon']);
  assert.equal(session.panel.restartLabel, 'Restart from 52:10');
  assert.equal(session.action, 'Restart');
  assert.equal(session.dateWord, 'STOPPED');
  assert.equal(session.dateClock, '52:10');
  assert.equal(session.tag.text, 'ENGINE STOPPED');
  assert.equal(session.tag.tone, 'bad');
});

test('a missing engine gives first run with the path and Open replay', async () => {
  const { session } = await started({
    'engine.state': 'not-found',
    'engine.path': 'C:\\Games\\Touchline\\engine-cli.exe',
    launcher: true,
  });
  assert.equal(session.screen, 'first-run');
  assert.equal(session.panel.path, 'C:\\Games\\Touchline\\engine-cli.exe');
  assert.equal(session.action, 'Open replay');
  assert.equal(session.title, 'Touchline');
  assert.equal(session.subtitle, 'First run · no engine found');
  let asked = 0;
  session.act(() => {
    asked += 1;
  });
  assert.equal(asked, 1, 'the action block asks for a replay file');
});

test('a drop reconnects by itself, then resumes and cuts the history at the first tick back', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  session.act();
  playTo(session, socket, 1, 400);
  socket.drop();
  assert.equal(session.screen, 'reconnecting');
  assert.equal(session.action, 'Reconnecting');
  assert.equal(session.actionBusy, true);
  assert.equal(session.notice.word, NOTICES.reconnect.word);
  assert.equal(session.tag.text, `RECONNECTING · ${session.clockText}`);
  // The status says the engine still runs; the session waits its backoff, then connects.
  await new Promise((resolve) => setTimeout(resolve, 400));
  const again = FakeSocket.made.at(-1);
  assert.notEqual(again, socket, 'a new connection');
  again.deliver(hello());
  assert.equal(session.screen, 'live');
  assert.equal(session.notice.word, 'CONNECTED');
  // The engine resumes one tick past its stoppage at 300.
  again.deliver(frameAt(301));
  assert.equal(session.history.newestTick, 301);
  assert.equal(session.renderedTick, 300, 'the pitch went back to the stoppage');
  assert.equal(session.notice, null);
});

test('a goal changes the score only when the rendered tick reaches it', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  session.act();
  for (let t = 1; t <= 200; t += 1) {
    socket.deliver(frameAt(t));
  }
  socket.deliver(
    JSON.stringify(
      eventMessage(150, 'goal', {
        'team.id': 'club-a',
        'player.id': 'p-0-9',
        'home.score': 1,
        'away.score': 0,
        minute: 0,
      })
    )
  );
  socket.deliver(JSON.stringify(statsMessage(120)));
  session.rewind(100);
  assert.deepEqual(session.score, [0, 0], 'the goal has not been drawn yet');
  assert.equal(session.stats, null, 'nor has the statistics line');
  assert.equal(session.banner, null);
  // Playing on across the goal's tick, frame by frame, releases it with its banner.
  session.setPlaying(true);
  session.scheduler.setSpeed(8);
  let ts = 0;
  session.frame(ts);
  while (session.renderedTick < 160) {
    ts += 16;
    session.frame(ts);
  }
  assert.deepEqual(session.score, [1, 0]);
  assert.ok(session.stats, 'the statistics line is released');
  assert.equal(session.banner.text, "GOAL — Ashford Rovers 1–0 0'");
  assert.deepEqual(session.scorers, ["0-9 0'", '']);
  assert.equal(session.feedRows.at(-1).kind, 'goal');
  // A rewind before the goal takes the score, the scorer and the banner back.
  session.rewind(100);
  assert.deepEqual(session.score, [0, 0]);
  assert.equal(session.banner, null);
  assert.deepEqual(session.scorers, ['', '']);
});

test('every field the screen reads is defined in every state', async () => {
  const reads = (session) => [
    session.screen,
    session.title,
    session.subtitle,
    session.dateWord,
    session.dateClock,
    session.action,
    session.tag.text,
    session.steps.length,
    session.score.length,
    session.scorers.length,
    session.clockText,
    session.feedRows.length,
  ];
  const seen = new Set();
  const check = (session) => {
    seen.add(session.screen);
    for (const value of reads(session)) {
      assert.notEqual(value, undefined, `${session.screen}`);
    }
  };
  const first = await started(RUNNING, null);
  check(first.session);
  first.socket.deliver(hello());
  check(first.session);
  first.session.act();
  check(first.session);
  first.session.act();
  check(first.session);
  first.session.act();
  first.socket.drop();
  check(first.session);
  await new Promise((resolve) => setTimeout(resolve, 0));
  await new Promise((resolve) => setTimeout(resolve, 0));
  check(first.session);
  const second = await started({ 'engine.state': 'not-found', 'engine.path': 'x' });
  check(second.session);
  assert.deepEqual([...seen].sort(), [...SCREENS].sort());
  assert.deepEqual(Object.keys(ACTIONS).sort(), SCREENS.filter((s) => s !== 'error').sort());
});
