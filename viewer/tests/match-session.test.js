// The match session: the screen it shows in each state, the commands it sends, and the
// panels it keeps at the rendered tick. A fake socket and a fake status endpoint stand in for
// the engine; nothing here opens a port.

import assert from 'node:assert/strict';
import { afterEach, beforeEach, test } from 'vitest';

import { COMPONENT_COUNT } from '../src/lib/decode.js';
import { ACTIONS, MatchSession, NOTICES, SCREENS } from '../src/lib/match-session.svelte.js';
import { readReplay } from '../src/lib/replay-file.js';
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

  /// The engine's own close after full time.
  finish() {
    this.readyState = 3;
    for (const fn of this.listeners.close ?? []) {
      fn({ code: 1000, wasClean: true, reason: '' });
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
  const downloads = [];
  const session = new MatchSession({
    fetcher,
    timers,
    raf: null,
    now: () => 0,
    doc: null,
    download: (bytes, name) => downloads.push({ bytes, name }),
  });
  await session.start();
  return { session, fetcher, downloads, socket: FakeSocket.made.at(-1) };
}

/// Plays on from the rendered tick, frame by frame at 8x, until `done` holds or `limit` frames.
function playFrames(session, done, limit = 5000) {
  session.scheduler.setSpeed(8);
  let ts = 0;
  session.frame(ts);
  for (let i = 0; i < limit && !done(); i += 1) {
    ts += 16;
    session.frame(ts);
  }
}

/// A short match: kick-off, a goal, half time at 400, a card, full time at 800.
function shortMatch(socket) {
  const score = { 'home.score': 1, 'away.score': 0 };
  for (let t = 1; t <= 800; t += 1) {
    socket.deliver(frameAt(t));
    if (t === 150) {
      socket.deliver(JSON.stringify(eventMessage(150, 'goal', { 'team.id': 'club-a', ...score })));
    }
    if (t === 400) {
      socket.deliver(JSON.stringify(eventMessage(400, 'half-time', score)));
    }
    if (t === 600) {
      socket.deliver(JSON.stringify(eventMessage(600, 'card', { 'team.id': 'club-b', 'card.kind': 'yellow', ...score })));
    }
    if (t === 800) {
      socket.deliver(JSON.stringify(eventMessage(800, 'full-time', score)));
    }
  }
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

test('with a lineup to pick, CONTINUE opens the Pre-match line-ups, which send nothing until KICK OFF', async () => {
  const { session, socket } = await started(RUNNING);
  const squad = roster(0).map((p) => ({ ...p, 'player.natural_fitness': 120, role_fit: [] }));
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
  assert.equal(session.action, 'Continue');
  session.act();
  assert.equal(session.view, 'prematch', 'CONTINUE opens the Pre-match line-ups');
  assert.equal(session.action, 'Kick off');
  assert.deepEqual(socket.sent, [], 'the Pre-match line-ups send nothing');
  const sheet = session.sheet();
  assert.deepEqual(sheet.home.eleven.map((r) => r.squad), setup.lineup);
  assert.equal(sheet.dots[0].length, 11, 'our eleven stand at their kick-off places');
  session.back();
  assert.equal(session.view, 'tactics', 'Change on Tactics goes back');
  assert.deepEqual(session.sheet().home.eleven.map((r) => r.squad), setup.lineup, 'the lineup is unchanged');
  session.act();
  assert.equal(session.view, 'prematch');
  assert.deepEqual(socket.sent, [], 'still nothing before KICK OFF');
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
  const third = await started(RUNNING);
  third.socket.deliver(hello());
  third.session.act();
  shortMatch(third.socket);
  third.session.rewind(700);
  playFrames(third.session, () => third.session.view === 'report');
  check(third.session);
  assert.deepEqual([...seen].sort(), [...SCREENS].sort());
  assert.deepEqual(Object.keys(ACTIONS).sort(), SCREENS.filter((s) => s !== 'error').sort());
});

test('the half-time report opens once when the pitch reaches the break, pauses, and CONTINUE resumes', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  session.act();
  shortMatch(socket);
  session.rewind(300);
  assert.equal(session.report, null, 'no report before the break');
  playFrames(session, () => session.view === 'report');
  assert.equal(session.view, 'report');
  assert.equal(session.report.kind, 'half-time');
  assert.equal(session.report.state, 'ready');
  assert.equal(session.report.tick, 400);
  assert.deepEqual(session.report.model.score, [1, 0]);
  assert.deepEqual(session.report.model.rows.find((r) => r.id === 'goals').counts, [1, 0]);
  assert.equal(session.playing, false, 'half time pauses playback');
  session.closeReport();
  assert.equal(session.view, 'match');
  assert.equal(session.playing, true, 'CONTINUE resumes');
  // Play on to just before full time: the half-time report does not open again.
  playFrames(session, () => session.renderedTick >= 700);
  assert.equal(session.view, 'match');
});

test('a scrub past a break opens no report behind it', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  session.act();
  shortMatch(socket);
  session.rewind(100);
  session.scrubTo(500);
  session.scrubEnd();
  playFrames(session, () => session.renderedTick >= 700);
  assert.equal(session.view, 'match', 'half time was passed by the scrub');
  assert.equal(session.report, null);
});

test('the full-time report is loading until the engine closes after full time, then ready', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  session.act();
  shortMatch(socket);
  session.rewind(700);
  playFrames(session, () => session.view === 'report');
  assert.equal(session.report.kind, 'full-time');
  assert.equal(session.report.state, 'loading');
  assert.equal(session.saveBlocked, 'The replay is still getting ready.');
  socket.finish();
  assert.equal(session.report.state, 'ready', 'the close after full time completes the store');
  assert.equal(session.saveBlocked, null);
});

test('Save replay writes every frame as it arrived, and the file reads back the same', async () => {
  const { session, socket, downloads } = await started(RUNNING);
  socket.deliver(hello('match-7'));
  session.act();
  shortMatch(socket);
  socket.finish();
  const saved = await session.saveReplay();
  assert.equal(downloads.length, 1);
  assert.equal(downloads[0].name, 'touchline-match-7.smfx');
  assert.equal(saved.ticks, 800, 'every tick frame is stored');
  assert.equal(session.saved.size, downloads[0].bytes.length);
  const read = await readReplay(downloads[0].bytes);
  assert.equal(read.ticks, 800);
  assert.equal(read.frames, saved.frames, 'the file holds the same frames as the store');
  assert.equal(read.hello['match.id'], 'match-7');
});

test('a replay file opens in the replay view, and Replay the whole match rewinds to kick-off', async () => {
  const { session, socket, downloads } = await started(RUNNING);
  socket.deliver(hello('match-7'));
  session.act();
  shortMatch(socket);
  socket.finish();
  await session.saveReplay();
  await session.openReplay(downloads[0].bytes, 'match.smfx');
  assert.equal(session.view, 'replay');
  assert.equal(session.stored, true);
  assert.equal(session.renderedTick, session.history.firstTick);
  // A stored match's full-time report is ready at once.
  session.rewind(700);
  playFrames(session, () => session.view === 'report');
  assert.equal(session.report.kind, 'full-time');
  assert.equal(session.report.state, 'ready');
  session.replayWhole();
  assert.equal(session.view, 'replay');
  assert.equal(session.renderedTick, session.history.firstTick);
  assert.equal(session.playing, true);
  session.step(10);
  assert.equal(session.renderedTick, session.history.firstTick + 500, 'Forward moves ten seconds');
  session.step(-10);
  assert.equal(session.renderedTick, session.history.firstTick, 'Back 10 seconds stops at kick-off');
  session.closeReplay();
  assert.equal(session.view, 'report', 'CONTINUE on the replay goes back to the report');
  session.closeReport();
  assert.equal(session.view, 'match', 'and the full-time report goes back to the match at full time');
  assert.equal(session.screen, 'full-time');
});

test('a replay file that cannot be read shows its refusal on the match view, from any view', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello('match-7'));
  session.act();
  shortMatch(socket);
  socket.finish();
  playFrames(session, () => session.view === 'report');
  assert.equal(session.view, 'report');
  await session.openReplay(new TextEncoder().encode('this is not a replay file\n'), 'junk.smfx');
  assert.equal(session.panel.kind, 'replay-refused');
  assert.equal(session.view, 'match', 'the refusal panel lives on the match view, so the view opens');
});

test('a saved match of the previous release opens live, naming its engine, with no kick-off', async () => {
  const resumed = {
    ...RUNNING,
    'engine.version': '0.2.0-beta.1',
    'launcher.version': '0.3.0',
    'match.resumed_from': 156_500,
  };
  const { session, socket } = await started(resumed);
  assert.deepEqual(
    session.steps.map((s) => s.label),
    ['Starting the engine (0.2.0-beta.1)', 'Connecting to the match', 'Resuming at 52:10']
  );
  // The previous release's hello: no squad, no setup, none of the later fields.
  socket.deliver(
    JSON.stringify({
      type: 'hello',
      'match.id': 'match-old',
      'protocol.version': 3,
      'engine.version': '0.2.0-beta.1',
      ticks_expected: 270000,
      teams: TEAMS,
    })
  );
  assert.equal(session.screen, 'live');
  assert.equal(session.view, 'match');
  assert.equal(session.subtitle, 'Engine connected · v0.2.0-beta.1');
  for (let t = 156_501; t <= 156_560; t += 1) {
    socket.deliver(frameAt(t));
  }
  assert.equal(session.screen, 'live');
  assert.equal(session.steps.every((s) => s.state === 'done'), true);
  assert.equal(socket.sent.some((m) => m.type === 'start'), false, 'a resumed match is not kicked off');
});

test('a save no engine can finish shows the resume screen, and New match starts a fresh one', async () => {
  const refused = {
    'engine.state': 'refused',
    'engine.reason': 'this match was saved by Touchline 0.1.0, two or more versions back',
    'engine.pid': null,
    launcher: true,
    resume: { kind: 'older', 'saved.version': '0.1.0', 'saved.tick': 156_500, reason: 'r' },
  };
  const { session, fetcher } = await started(refused, refused, { ...RUNNING, 'match.resumed_from': null });
  assert.equal(session.view, 'resume');
  assert.equal(session.screen, 'error');
  assert.equal(session.resumeInfo['saved.version'], '0.1.0');
  assert.equal(session.action, 'New match');
  assert.equal(FakeSocket.made.length, 0, 'no engine to connect to');

  await session.act();
  assert.ok(fetcher.calls.some((c) => c.path === 'engine/new-match' && c.method === 'POST'));
  assert.equal(session.view, 'match');
  assert.equal(session.resumeInfo, null);
  assert.equal(session.screen, 'loading');
  const socket = FakeSocket.made.at(-1);
  socket.deliver(hello('match-new'));
  assert.equal(session.screen, 'kickoff');
});

test('the session plays on the ground its hello names, and on 105 by 68 when it names none', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(
    JSON.stringify({ ...JSON.parse(hello('match-ground')), 'ground.length': 100, 'ground.width': 64 })
  );
  assert.deepEqual({ ...session.ground }, { length: 100, width: 64 });
  const other = await started(RUNNING);
  other.socket.deliver(hello('match-default'));
  assert.deepEqual({ ...other.session.ground }, { length: 105, width: 68 });
});

// ---- Skip to the result ---------------------------------------------------------------------

/// A kicked-off session drawn at tick `at`, with frames up to `received` stored.
async function skippable(at = 200, received = 300) {
  const { session, socket, downloads } = await started(RUNNING);
  socket.deliver(hello('match-7'));
  session.act();
  for (let t = 1; t <= received; t += 1) {
    socket.deliver(frameAt(t));
  }
  session.rewind(at);
  socket.sent.length = 0;
  return { session, socket, downloads };
}

test('Skip to result is offered only on a kicked-off live engine match before full time', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  assert.equal(session.canSkip, false, 'not before kick-off');
  session.act();
  playTo(session, socket, 1, 100);
  assert.equal(session.canSkip, true, 'a live match can skip');
  session.act();
  assert.equal(session.canSkip, true, 'a paused match can skip');
  shortMatch(socket);
  assert.equal(session.canSkip, false, 'after the full-time whistle there is nothing left to play');

  const stored = await skippable();
  stored.socket.deliver(JSON.stringify(eventMessage(300, 'full-time')));
  stored.socket.finish();
  await stored.session.saveReplay();
  await stored.session.openReplay(stored.downloads[0].bytes);
  assert.equal(stored.session.canSkip, false, 'a replay file ignores commands');
  assert.equal(stored.session.openSkip(), false);
});

test('Skip to result pauses at the drawn tick and opens the decision; Keep watching sends nothing', async () => {
  const { session, socket } = await skippable(200, 300);
  assert.equal(session.openSkip(), true);
  assert.equal(session.view, 'skip');
  assert.equal(session.playing, false, 'the match is paused as the pause control pauses it');
  assert.equal(session.screen, 'paused');
  assert.deepEqual(session.skip, { state: 'deciding', from: 200, newest: 200 });
  assert.ok(!socket.sent.some((c) => c.type === 'skip'), 'deciding sends no skip');
  session.frame(0);
  session.frame(500);
  assert.equal(session.renderedTick, 200, 'the paused match does not move');

  session.keepWatching();
  assert.equal(session.view, 'match');
  assert.equal(session.skip, null);
  assert.equal(session.renderedTick, 200, 'back at the same tick');
  assert.equal(session.playing, false, 'still paused');
  assert.ok(!socket.sent.some((c) => c.type === 'skip'));

  session.openSkip();
  session.keepWatching(true);
  assert.equal(session.playing, true, 'RESUME in the header plays on');
  assert.equal(session.screen, 'live');
});

test('Confirm sends one skip; while the engine plays the rest nothing paces it and no break report opens', async () => {
  const { session, socket } = await skippable(200, 300);
  session.openSkip();
  assert.equal(session.confirmSkip(), true);
  assert.deepEqual(
    socket.sent.filter((c) => c.type === 'skip'),
    [{ type: 'skip' }]
  );
  assert.equal(session.view, 'report');
  assert.equal(session.report.state, 'playing-rest');
  assert.equal(session.report.skippedFrom, 200);
  assert.equal(session.saveBlocked, 'The replay is still getting ready.');
  socket.deliver(JSON.stringify({ type: 'ack', command: 'skip', 'change.queued_tick': 300 }));
  assert.equal(session.skip.state, 'playing', 'the ack keeps the skip playing');

  const sentBefore = socket.sent.length;
  const score = { 'home.score': 1, 'away.score': 0 };
  for (let t = 301; t <= 800; t += 1) {
    socket.deliver(frameAt(t));
    if (t === 400) {
      socket.deliver(JSON.stringify(eventMessage(400, 'half-time')));
    }
    if (t === 500) {
      socket.deliver(JSON.stringify(eventMessage(500, 'goal', { 'team.id': 'club-a', ...score })));
    }
  }
  for (let ts = 0; ts < 2000; ts += 16) {
    session.frame(ts);
  }
  assert.equal(session.renderedTick, 200, 'none of the rest is drawn');
  assert.equal(session.view, 'report', 'no half-time report opens');
  assert.equal(session.report.state, 'playing-rest');
  assert.deepEqual(socket.sent.slice(sentBefore), [], 'no pause, start or seen while it plays');

  socket.deliver(JSON.stringify(eventMessage(800, 'full-time', score)));
  assert.equal(session.skip.state, 'storing');
  assert.equal(session.report.state, 'storing');
  assert.equal(session.report.tick, 800);

  socket.finish();
  assert.equal(session.skip.state, 'ready');
  assert.equal(session.report.state, 'ready');
  assert.equal(session.report.skippedFrom, 200);
  assert.equal(session.report.tick, 800);
  assert.deepEqual(session.report.model.score, [1, 0], 'the report counts every event');
  assert.equal(session.report.model.rows.find((r) => r.id === 'goals').counts[0], 1);
  assert.equal(session.renderedTick, 800, 'the match behind the report is at full time');
  assert.equal(session.banner, null, 'with no goal banner');
  assert.equal(session.saveBlocked, null);
  session.closeReport();
  assert.equal(session.view, 'match');
  for (let ts = 3000; ts < 3200; ts += 16) {
    session.frame(ts);
  }
  assert.equal(session.view, 'match', 'no report opens behind the skip');
  assert.equal(session.canSkip, false);
  const done = (await import('../src/lib/signal.js')).signals().find((s) => s.signal === 'viewer.skip_done');
  assert.equal(done.from_tick, 200);
  assert.equal(done.full_time_tick, 800);
});

test('a refused skip returns to the paused match with a notice naming the reason', async () => {
  const { session, socket } = await skippable(200, 300);
  session.openSkip();
  session.confirmSkip();
  socket.deliver(
    JSON.stringify({ type: 'reject', command: 'skip', reason: 'cannot read the command: unknown variant `skip`' })
  );
  assert.equal(session.skip, null);
  assert.equal(session.view, 'match');
  assert.equal(session.screen, 'paused');
  assert.equal(session.renderedTick, 200);
  assert.equal(session.report, null, 'the playing report goes');
  assert.equal(session.notice.word, NOTICES.skip.word);
  assert.match(session.notice.message, /^The engine cannot skip to the result: cannot read the command/);
  assert.equal(session.canSkip, true, 'the player may try again');
});

test('a reconnect while the engine plays the rest sends the skip again after the first tick', async () => {
  const { session, socket } = await skippable(200, 300);
  session.openSkip();
  session.confirmSkip();
  for (let t = 301; t <= 400; t += 1) {
    socket.deliver(frameAt(t));
  }
  socket.drop();
  await new Promise((resolve) => setTimeout(resolve, 400));
  const again = FakeSocket.made.at(-1);
  assert.notEqual(again, socket);
  again.deliver(hello('match-7'));
  assert.deepEqual(again.sent, [], 'nothing before the resumed match streams');
  again.deliver(frameAt(301));
  assert.deepEqual(
    again.sent.map((c) => c.type),
    ['skip'],
    'the skip goes again, and no seen'
  );
  assert.equal(session.skip.state, 'playing');
});

test('a crash while the engine plays the rest clears the skip and shows the error panel', async () => {
  const crashed = { 'engine.state': 'crashed', 'engine.code': 3, 'snapshot.tick': 300, launcher: true };
  const { session, socket } = await started(RUNNING, crashed);
  socket.deliver(hello());
  session.act();
  playTo(session, socket, 1, 300);
  session.rewind(200);
  session.openSkip();
  session.confirmSkip();
  socket.drop();
  await new Promise((resolve) => setTimeout(resolve, 0));
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(session.screen, 'error');
  assert.equal(session.skip, null);
  assert.equal(session.view, 'match');
});

test('a replay saved after a skip has the bytes of one saved after the match played through', async () => {
  const play = async (skip) => {
    const { session, socket, downloads } = await skippable(200, 300);
    if (skip) {
      session.openSkip();
      session.confirmSkip();
      socket.deliver(JSON.stringify({ type: 'ack', command: 'skip', 'change.queued_tick': 300 }));
    }
    for (let t = 301; t <= 800; t += 1) {
      socket.deliver(frameAt(t));
    }
    socket.deliver(JSON.stringify(eventMessage(800, 'full-time')));
    socket.finish();
    await session.saveReplay();
    return downloads[0].bytes;
  };
  const skipped = await play(true);
  const watched = await play(false);
  assert.deepEqual(Array.from(skipped), Array.from(watched), 'the skip mark is not written');
});

// ---- The other grounds ------------------------------------------------------------------------

const ground = (team) => ({ 'team.id': team, 'team.name': `${team} Town`, 'team.kit.primary': '#0f5c63', 'team.kit.secondary': '#ffffff', roster: [] });
const matchdayMessage = () =>
  JSON.stringify({
    type: 'matchday',
    round: 1,
    fixtures: [
      { fixture: 0, home: ground('Castlemere'), away: ground('Greywater') },
      { fixture: 1, home: ground('Kelder'), away: ground('Millbridge') },
    ],
  });
const groundGoal = (fixture, tick, score, extra = {}) =>
  JSON.stringify({ type: 'ground-event', fixture, tick, kind: 'goal', minute: 0, side: 'home', scorer: 'Tomas Okafor', score, ...extra });

test('the session keeps the round, shows a ground goal only at its tick, and drops a catch-up repeat', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  socket.deliver(matchdayMessage());
  assert.equal(session.matchday.fixtures.length, 2);
  assert.equal(session.grounds.state, 'rows');
  assert.equal(session.grounds.rows[0].minute, 'KO');
  session.act();
  playTo(session, socket, 1, 100);
  socket.deliver(groundGoal(0, 150, [1, 0]));
  socket.deliver(JSON.stringify({ type: 'ground-progress', tick: 100, reached: [200, 200] }));
  assert.deepEqual(session.grounds.rows[0].score, [0, 0], 'not before the rendered tick reaches 150');
  assert.deepEqual(session.matchday.reached, [200, 200]);
  playTo(session, socket, 101, 200);
  assert.deepEqual(session.grounds.rows[0].score, [1, 0]);
  socket.deliver(groundGoal(0, 150, [1, 0]));
  assert.equal(session.matchday.events[0].length, 1, 'the repeat is dropped');
  const kept = (await import('../src/lib/signal.js')).signals().find((s) => s.signal === 'viewer.matchday');
  assert.equal(kept.fixtures, 2);
});

test('the list is worked out again only on a new simulated second or a ground message', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  socket.deliver(matchdayMessage());
  session.act();
  playTo(session, socket, 1, 120);
  const first = session.grounds;
  session.renderedTick = 121;
  session.flush(121, { seek: false });
  assert.equal(session.grounds, first, 'the same second: the same object');
  session.renderedTick = 150;
  session.flush(150, { seek: false });
  assert.notEqual(session.grounds, first, 'a new second');
  const second = session.grounds;
  socket.deliver(JSON.stringify({ type: 'ground-progress', tick: 150, reached: [150, 150] }));
  assert.notEqual(session.grounds, second, 'a message');
});

test('a late ground goal is signalled and shown at once; a reconnect keeps a new copy of the round', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  socket.deliver(matchdayMessage());
  session.act();
  playTo(session, socket, 1, 400);
  socket.deliver(groundGoal(1, 300, [1, 0], { late: true }));
  assert.equal(session.grounds.rows[1].flag, 'late');
  const late = (await import('../src/lib/signal.js')).signals().find((s) => s.signal === 'viewer.ground_late');
  assert.deepEqual([late.fixture, late.tick, late.rendered_tick], [1, 300, 400]);
  socket.drop();
  await new Promise((resolve) => setTimeout(resolve, 400));
  const again = FakeSocket.made.at(-1);
  again.deliver(hello());
  again.deliver(matchdayMessage());
  assert.equal(session.matchday.events[1].length, 0, 'the round starts again; the engine sends its events again');
  again.deliver(groundGoal(1, 300, [1, 0]));
  assert.equal(session.matchday.events[1].length, 1);
});

test('a ground message is never stored with the frames, so a saved replay has none', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  socket.deliver(matchdayMessage());
  session.act();
  playTo(session, socket, 1, 100);
  socket.deliver(groundGoal(0, 50, [1, 0]));
  socket.deliver(JSON.stringify({ type: 'ground-progress', tick: 100, reached: [100, 100] }));
  for (let i = 0; i < session.frames.count; i += 1) {
    const { text, payload } = session.frames.frame(i);
    if (text) {
      assert.doesNotMatch(new TextDecoder().decode(payload), /matchday|ground-/);
    }
  }
});

test('a match starts at the speed the settings name, and at 1x with none', async () => {
  const fetcher = statusFetch(RUNNING);
  const timers = { setTimeout: (fn, ms) => ({ fn, ms }), clearTimeout: () => {} };
  const session = new MatchSession({ fetcher, timers, raf: null, now: () => 0, doc: null, settings: { speed: 4, commentary: false } });
  await session.start();
  FakeSocket.made.at(-1).deliver(hello());
  assert.equal(session.speed, 4);
  assert.equal(session.commentary, false);

  const plain = await started(RUNNING);
  plain.socket.deliver(hello());
  assert.equal(plain.session.speed, 1);
  assert.equal(plain.session.commentary, true);
});

test('dispose closes the socket with no recovery and stops the frame loop', async () => {
  const fetcher = statusFetch(RUNNING);
  const frames = [];
  const timers = { setTimeout: (fn, ms) => ({ fn, ms }), clearTimeout: () => {} };
  const session = new MatchSession({ fetcher, timers, raf: (fn) => frames.push(fn), now: () => 0, doc: null });
  await session.start();
  const socket = FakeSocket.made.at(-1);
  socket.deliver(hello());
  session.dispose();
  assert.equal(session.disposed, true);
  assert.equal(socket.readyState, 3, 'the socket is closed');
  assert.notEqual(session.screen, 'reconnecting', 'a disposed session does not reconnect');
  for (const fn of frames.splice(0)) {
    fn(16);
  }
  assert.equal(frames.length, 0, 'the loop asks for no frame after dispose');
});

test('a session disposed while it waits for the launcher opens no socket', async () => {
  let answer;
  const pending = new Promise((resolve) => {
    answer = resolve;
  });
  const fetcher = async () => {
    await pending;
    return { ok: true, json: async () => RUNNING };
  };
  const timers = { setTimeout: (fn, ms) => ({ fn, ms }), clearTimeout: () => {} };
  const session = new MatchSession({ fetcher, timers, raf: null, now: () => 0, doc: null });
  const before = FakeSocket.made.length;
  const starting = session.start();
  session.dispose();
  answer();
  await starting;
  assert.equal(FakeSocket.made.length, before, 'no socket after dispose');
  assert.equal(session.socket, null);
});

test('the menu pauses the match and Resume match plays on only if it played before', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  session.act();
  assert.equal(session.playing, true);
  session.pauseForMenu();
  assert.equal(session.playing, false);
  session.resumeFromMenu();
  assert.equal(session.playing, true);

  session.act();
  assert.equal(session.playing, false);
  session.pauseForMenu();
  session.resumeFromMenu();
  assert.equal(session.playing, false, 'a paused match stays paused');
});

// ---- Full time and the next steps ----------------------------------------------------------------

/// A match played through to its full-time report, with the engine's clean close when `close`.
async function atFullTime({ close = true } = {}) {
  const { session, socket, fetcher } = await started(RUNNING);
  socket.deliver(hello('match-7'));
  session.act();
  shortMatch(socket);
  session.rewind(700);
  playFrames(session, () => session.view === 'report');
  if (close) {
    socket.finish();
  }
  return { session, socket, fetcher };
}

test('a match played through holds FULL TIME once its report opens, and the report goes back to it', async () => {
  const { session } = await atFullTime();
  assert.equal(session.screen, 'full-time');
  assert.equal(session.playing, false, 'playback stops at full time');
  session.closeReport();
  assert.equal(session.view, 'match');
  assert.equal(session.dateWord, 'FULL TIME');
  assert.equal(session.dateClock, '00:16', 'the final clock: tick 800');
  assert.equal(session.action, 'Report');
  assert.equal(session.title, 'Ashford Rovers 1–0 Port Varrow');
  assert.deepEqual(session.tag, { text: 'FULL TIME · 00:16', tone: 'final', live: false });
  assert.equal(session.canSkip, false);
});

test('no control, menu or seek brings LIVE or PAUSED back after full time', async () => {
  const { session } = await atFullTime();
  session.closeReport();
  const words = new Set();
  const look = () => {
    words.add(session.dateWord);
    assert.equal(session.screen, 'full-time');
  };
  session.setPlaying(true);
  look();
  session.setPlaying(false);
  look();
  session.toNewest();
  look();
  session.rewind(100);
  look();
  assert.equal(session.dateClock, '00:16', 'the final clock, whatever minute the pitch shows');
  session.scrubTo(300);
  session.scrubEnd();
  look();
  session.pauseForMenu();
  look();
  session.resumeFromMenu();
  look();
  session.replayWhole();
  look();
  session.closeReplay();
  look();
  assert.deepEqual([...words], ['FULL TIME']);
});

test('at full time every ground reads FT, even one whose whistle comes after the match’s', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  socket.deliver(matchdayMessage());
  session.act();
  const groundEnd = (fixture, tick) =>
    JSON.stringify({ type: 'ground-event', fixture, tick, kind: 'full-time', minute: 90, added: 3, score: [0, 0] });
  socket.deliver(groundEnd(0, 790));
  socket.deliver(groundEnd(1, 810));
  shortMatch(socket);
  session.rewind(700);
  playFrames(session, () => session.view === 'report');
  socket.finish();
  session.closeReport();
  assert.equal(session.screen, 'full-time');
  assert.deepEqual(
    session.grounds.rows.map((r) => [r.minute, r.ended]),
    [
      ['FT', true],
      ['FT', true],
    ],
    'the ground that ends after the match’s whistle reads FT too'
  );
  session.rewind(700);
  assert.deepEqual(
    session.grounds.rows.map((r) => r.ended),
    [false, false],
    'back before the whistle, the list is on the player’s clock again'
  );
});

test('at full time a ground with no end yet shows its own minute, not a huge one', async () => {
  const { session, socket } = await started(RUNNING);
  socket.deliver(hello());
  socket.deliver(matchdayMessage());
  session.act();
  socket.deliver(JSON.stringify({ type: 'ground-progress', tick: 0, reached: [600, 900] }));
  shortMatch(socket);
  session.rewind(700);
  playFrames(session, () => session.view === 'report');
  session.closeReport();
  assert.equal(session.screen, 'full-time');
  assert.deepEqual(
    session.grounds.rows.map((r) => [r.minute, r.ended]),
    [
      ["0'", false],
      ["0'", false],
    ],
    'the minute each ground reached, at most the whistle'
  );
});

test('the action block at full time opens the report again', async () => {
  const { session } = await atFullTime();
  session.closeReport();
  session.show('tactics');
  assert.equal(session.act(), true);
  assert.equal(session.view, 'report');
  assert.equal(session.report.kind, 'full-time');
  session.closeReport();
  assert.equal(session.view, 'match', 'back to the match, not to Tactics');
  const reopened = (await import('../src/lib/signal.js')).signals().filter((s) => s.signal === 'viewer.report_reopened');
  assert.equal(reopened.length, 1);
});

test('a skip enters full time when it finishes, and the storing report carries the final figures', async () => {
  const { session, socket } = await skippable(200, 300);
  session.openSkip();
  session.confirmSkip();
  const score = { 'home.score': 1, 'away.score': 0 };
  for (let t = 301; t < 800; t += 1) {
    socket.deliver(frameAt(t));
    if (t === 500) {
      socket.deliver(JSON.stringify(eventMessage(500, 'goal', { 'team.id': 'club-a', ...score })));
    }
  }
  const scorersAtSkip = session.scorers;
  socket.deliver(JSON.stringify(eventMessage(800, 'full-time', score)));
  assert.equal(session.report.state, 'storing');
  assert.deepEqual(session.report.model.score, [1, 0], 'the storing report shows the final score');
  assert.notEqual(session.renderedTick, 800, 'the whistle tick is not in yet');
  socket.deliver(frameAt(800));
  assert.equal(session.renderedTick, 800, 'the match behind moves to the whistle once its tick is in');
  assert.notDeepEqual(session.scorers, scorersAtSkip, 'the strip names the scorer of the rest');
  socket.deliver(frameAt(801));
  assert.equal(session.renderedTick, 800, 'and stays there while the match is stored');
  assert.equal(session.nextReady, false, 'nothing leads on while the match is stored');
  assert.equal(session.screen, 'paused', 'the match behind waits for the store');
  socket.finish();
  assert.equal(session.screen, 'full-time');
  assert.equal(session.nextReady, true);
  assert.equal(session.subtitle, 'Engine finished · v0.3.0 · skipped from 00:04');
  session.closeReport();
  assert.equal(session.dateWord, 'FULL TIME');
  assert.equal(session.action, 'Report');
});

test('a resumed match enters full time when its report opens', async () => {
  const resumed = { ...RUNNING, 'match.resumed_from': 100 };
  const { session, socket } = await started(resumed);
  socket.deliver(hello('match-r'));
  assert.equal(session.screen, 'live');
  shortMatch(socket);
  session.rewind(700);
  playFrames(session, () => session.view === 'report');
  assert.equal(session.screen, 'full-time');
  session.closeReport();
  assert.equal(session.dateWord, 'FULL TIME');
});

test('the next steps wait for the stored match and run only through the front door', async () => {
  const { session, socket } = await atFullTime({ close: false });
  const asked = [];
  assert.equal(session.nextOffered, false, 'a page with no start screen offers no next step');
  assert.equal(session.nextStep('new'), false);
  session.onNextStep = (id) => asked.push(id);
  assert.equal(session.nextOffered, true);
  assert.equal(session.report.state, 'loading');
  assert.equal(session.nextReady, false);
  assert.equal(session.nextStep('new'), false, 'nothing runs while the match is stored');
  assert.deepEqual(asked, []);
  socket.finish();
  assert.equal(session.nextReady, true);
  assert.equal(session.nextStep('return'), true);
  assert.deepEqual(asked, ['return']);
  const sent = (await import('../src/lib/signal.js')).signals().filter((s) => s.signal === 'viewer.next_step');
  assert.deepEqual(sent.map((s) => s.choice), ['return']);
});

test('Play the replay from here opens the replay at the pitch minute, or at kick-off from the end', async () => {
  const { session } = await atFullTime();
  session.closeReport();
  session.playFromHere();
  assert.equal(session.view, 'replay');
  assert.equal(session.renderedTick, session.history.firstTick, 'from the end, the replay starts at kick-off');
  assert.equal(session.playing, true);
  assert.equal(session.screen, 'full-time');
  session.closeReplay();
  assert.equal(session.view, 'match');
  assert.equal(session.playing, false, 'the replay stops under FULL TIME');
  session.rewind(300);
  session.playFromHere();
  assert.equal(session.view, 'replay');
  assert.equal(session.renderedTick, 300);
});

test('at full time the report reopened over a playing pitch stops it', async () => {
  const { session } = await atFullTime();
  session.closeReport();
  session.toNewest();
  assert.equal(session.playing, true);
  assert.equal(session.reopenReport(), true);
  assert.equal(session.playing, false, 'the stopped match lies under the report');
});

test('a stopped pitch is not drawn again every frame, and draws again once it moves', async () => {
  const { session } = await atFullTime();
  session.closeReport();
  let draws = 0;
  session.pitch = {
    shown: null,
    holds(components) {
      return this.shown !== null && this.shown.every((v, i) => v === components[i]);
    },
    draw(components) {
      draws += 1;
      this.shown = Array.from(components);
    },
    clearTrail() {},
  };
  for (let ts = 0; ts < 1600; ts += 16) {
    session.frame(ts);
  }
  assert.equal(draws, 1, 'the full-time pitch is drawn once, then held');
  session.rewind(300);
  session.frame(1600);
  assert.equal(draws, 2, 'a seek draws once, and the frame after holds it');
  session.setPlaying(true);
  for (let ts = 1616; ts < 1700; ts += 16) {
    session.frame(ts);
  }
  assert.ok(draws > 4, `a moving pitch draws every frame: ${draws}`);
});

test('a canvas that takes a new size redraws its pitch there, and a later pitch starts at that size', async () => {
  const fetcher = statusFetch(RUNNING);
  const timers = { setTimeout: (fn, ms) => ({ fn, ms }), clearTimeout: () => {} };
  const session = new MatchSession({ fetcher, timers, raf: () => {}, now: () => 0, doc: null });
  const calls = [];
  const canvas = {};
  session.canvas = canvas;
  session.pitch = { resize: (box) => calls.push(['resize', box]) };
  const box = { width: 1484, height: 624, deviceWidth: 1484, deviceHeight: 624 };
  session.resizeCanvas(canvas, box);
  assert.deepEqual(calls, [['resize', box]], 'the pitch on show takes the size');

  calls.length = 0;
  const other = {};
  const replayBox = { width: 600, height: 231, deviceWidth: 1200, deviceHeight: 462 };
  session.resizeCanvas(other, replayBox);
  assert.deepEqual(calls, [], 'a canvas with no pitch only keeps its box');
  assert.equal(session.boxes.get(other), replayBox);
});
