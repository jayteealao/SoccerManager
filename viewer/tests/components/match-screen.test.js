// @vitest-environment jsdom
// The match screen in each of its states, in jsdom: the action block names the state's one
// next action, each state draws its panel, every stub is inert and takes no focus, and the
// other grounds of the matchday show in the right column at the rendered tick. Skip to result
// is a real button, live only once the match has kicked off.

import assert from 'node:assert/strict';
import { render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, test } from 'vitest';

import { COMPONENT_COUNT } from '../../src/lib/decode.js';
import { MatchSession } from '../../src/lib/match-session.svelte.js';
import { clearSignals } from '../../src/lib/signal.js';
import MatchScreen from '../../src/screens/MatchScreen.svelte';
import { encodeKeyframe, roster } from '../helpers.js';

class FakeSocket {
  static OPEN = 1;
  static made = [];
  constructor(address) {
    this.address = address;
    this.readyState = 1;
    this.sent = [];
    this.listeners = {};
    FakeSocket.made.push(this);
  }
  addEventListener(type, fn) {
    (this.listeners[type] ??= []).push(fn);
  }
  send(text) {
    this.sent.push(JSON.parse(text));
  }
  close() {}
  deliver(data) {
    for (const fn of this.listeners.message ?? []) {
      fn({ data: typeof data === 'string' ? data : data.buffer });
    }
  }
  drop() {
    for (const fn of this.listeners.close ?? []) {
      fn({ code: 1006, wasClean: false, reason: '' });
    }
  }
}

const RUNNING = { 'engine.state': 'running', 'socket.port': 7001, 'protocol.version': 3, launcher: true };
const CRASHED = { 'engine.state': 'crashed', 'engine.code': 3, 'snapshot.tick': 156500, launcher: true };
const NOT_FOUND = {
  'engine.state': 'not-found',
  'engine.path': 'C:\\Games\\Touchline\\engine-cli.exe',
  launcher: true,
};
const HELLO = JSON.stringify({
  type: 'hello',
  'match.id': 'm',
  'protocol.version': 3,
  'engine.version': '0.3.0',
  ticks_expected: 270000,
  teams: [
    { 'team.id': 'a', 'team.name': 'Ashford Rovers', 'team.kit.primary': '#c8102e', 'team.kit.secondary': '#000000', roster: roster(0) },
    { 'team.id': 'b', 'team.name': 'Port Varrow', 'team.kit.primary': '#6a0dad', 'team.kit.secondary': '#ff6a13', roster: roster(1) },
  ],
});

function statusFetch(...bodies) {
  return async () => {
    const body = bodies.length > 1 ? bodies.shift() : bodies[0];
    return { ok: body !== null, json: async () => body };
  };
}

async function session(...bodies) {
  const s = new MatchSession({
    fetcher: statusFetch(...bodies),
    timers: { setTimeout: () => 0, clearTimeout: () => {} },
    raf: null,
    now: () => 0,
  });
  render(MatchScreen, { session: s });
  await s.start();
  await tick();
  return { s, socket: FakeSocket.made.at(-1) };
}

const settle = async () => {
  for (let i = 0; i < 4; i += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  await tick();
};

const action = () => document.querySelector('header button.cont');

/// Every stub is inert and hidden, and holds nothing that takes focus. Returns what breaks it.
function stubFaults(root) {
  const faults = [];
  for (const stub of root.querySelectorAll('[data-stub]')) {
    if (!stub.hasAttribute('inert')) {
      faults.push(`${stub.dataset.stub}: not inert`);
    }
    if (stub.getAttribute('aria-hidden') !== 'true') {
      faults.push(`${stub.dataset.stub}: not hidden`);
    }
    const focusable = stub.querySelector('a[href], button, input, select, textarea, [tabindex]');
    if (focusable) {
      faults.push(`${stub.dataset.stub}: ${focusable.tagName.toLowerCase()} takes focus`);
    }
  }
  return faults;
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
  document.body.innerHTML = '';
});

test('loading shows the steps and skeletons, with a busy action block', async () => {
  await session(RUNNING);
  assert.equal(action().textContent.trim(), 'Please wait');
  assert.equal(action().disabled, true);
  assert.ok(screen.getByRole('heading', { name: 'Getting the match ready' }));
  assert.ok(screen.getAllByText(/Connecting to the match/).length > 0);
  const announced = [...document.querySelectorAll('[role="status"]')].map((el) => el.textContent);
  assert.ok(
    announced.some((text) => /Step 2 of 3: Connecting to the match/.test(text)),
    'the step in progress is announced',
  );
  assert.ok(document.querySelector('.skels'));
  assert.equal(document.querySelector('.spinner'), null, 'no spinner');
  assert.equal(document.querySelector('[data-screen]').dataset.screen, 'loading');
});

test('kick-off, live and paused name their one next action', async () => {
  const { s, socket } = await session(RUNNING);
  socket.deliver(HELLO);
  await tick();
  assert.equal(action().textContent.trim(), 'Kick off');
  assert.ok(screen.getByText('Ashford Rovers v Port Varrow'));
  assert.ok(screen.getByText('KICK-OFF · 00:00'));
  assert.ok(screen.getByText('No events yet. The feed fills as the match plays.'));
  action().click();
  await tick();
  assert.deepEqual(socket.sent.map((m) => m.type), ['seen', 'start']);
  assert.equal(action().textContent.trim(), 'Pause');
  socket.deliver(encodeKeyframe(1, new Array(COMPONENT_COUNT).fill(0)));
  s.rewind(1);
  await tick();
  assert.ok(screen.getByRole('group', { name: 'Playback' }));
  assert.ok(screen.getByRole('slider', { name: 'Rewind to a tick' }));
  assert.ok(screen.getByRole('region', { name: 'Statistics' }));
  action().click();
  await tick();
  assert.equal(action().textContent.trim(), 'Resume');
  assert.equal(s.screen, 'paused');
});

test('a stopped engine shows the error panel with Restart from its stoppage', async () => {
  const { socket } = await session(RUNNING, CRASHED);
  socket.deliver(HELLO);
  await tick();
  socket.drop();
  await settle();
  assert.equal(action().textContent.trim(), 'Restart');
  const alert = screen.getByRole('alert');
  assert.ok(alert.textContent.includes('Error'), 'the panel names its kind in a word');
  assert.ok(alert.textContent.includes('The engine stopped (exit code 3)'));
  assert.ok(screen.getByRole('button', { name: 'Restart from 52:10' }));
  assert.ok(screen.getByRole('button', { name: 'Abandon' }));
  assert.ok(screen.getByText('ENGINE STOPPED'));
});

test('first run shows the path the launcher looked in, and Open replay', async () => {
  await session(NOT_FOUND);
  assert.equal(action().textContent.trim(), 'Open replay');
  const alert = screen.getByRole('status');
  assert.ok(alert.textContent.includes('Setup'));
  assert.equal(document.querySelector('.cover code').textContent, 'C:\\Games\\Touchline\\engine-cli.exe');
  assert.ok(screen.getByRole('button', { name: 'Open a replay' }));
  assert.ok(screen.getByText('The feed fills when a match plays.'));
});

test('a dropped connection shows the reconnecting notice in words and holds the pitch', async () => {
  const { socket } = await session(RUNNING, { 'engine.state': 'starting' });
  socket.deliver(HELLO);
  await tick();
  socket.drop();
  await tick();
  assert.equal(action().textContent.trim(), 'Reconnecting');
  assert.equal(action().disabled, true);
  const notice = screen.getByRole('status', { name: '' });
  assert.ok(notice.textContent.includes('RECONNECTING'));
  assert.ok(screen.getByText('The pitch holds its last frame. Play resumes at the last stoppage.'));
});

test('every stub is inert and hidden, and none of its parts takes focus', async () => {
  const { s, socket } = await session(RUNNING);
  socket.deliver(HELLO);
  s.act();
  await tick();
  const stubs = document.querySelectorAll('[data-stub]');
  assert.ok(stubs.length >= 30, `${stubs.length} stubs`);
  for (const note of [
    'pitch overlays, zoom and follow',
    'highlight modes and pause rules',
    'momentum chart',
    'win probability',
    'tab: squad',
    'tab: other-grounds',
  ]) {
    assert.ok(document.querySelector(`[data-stub="${note}"]`), note);
  }
  assert.deepEqual(stubFaults(document.body), []);
});

test('Skip to result is a real button: disabled before kick-off, live on a live match, and it opens the decision', async () => {
  const { s, socket } = await session(RUNNING);
  socket.deliver(HELLO);
  await tick();
  const skip = () => screen.getByRole('button', { name: 'Skip to result' });
  assert.equal(skip().disabled, true, 'nothing to skip before kick-off');
  assert.equal(document.querySelector('[data-stub="skip to result"]'), null, 'no longer a stub');
  s.act();
  socket.deliver(encodeKeyframe(1, new Array(COMPONENT_COUNT).fill(0)));
  s.rewind(1);
  await tick();
  assert.equal(s.canSkip, true);
  assert.equal(skip().disabled, false);
  let opened = 0;
  s.openSkip = () => (opened += 1);
  skip().click();
  assert.equal(opened, 1, 'the click calls openSkip');
});

test('the stub check fails when a stub holds something that takes focus', async () => {
  const { s, socket } = await session(RUNNING);
  socket.deliver(HELLO);
  s.act();
  await tick();
  const stub = document.querySelector('[data-stub="win probability"]');
  const planted = document.createElement('span');
  planted.tabIndex = 0;
  stub.append(planted);
  assert.deepEqual(stubFaults(document.body), ['win probability: span takes focus']);
});

const crest = (id, name) => ({ 'team.id': id, 'team.name': name, 'team.kit.primary': '#0f5c63', 'team.kit.secondary': '#ffffff', roster: [] });
const MATCHDAY = JSON.stringify({
  type: 'matchday',
  round: 1,
  fixtures: [
    { fixture: 0, home: crest('cu', 'Castlemere United'), away: crest('gw', 'Greywater') },
    { fixture: 1, home: crest('ka', 'Kelder Athletic'), away: crest('mt', 'Millbridge Town') },
  ],
});

test('the right column lists the other grounds at the rendered tick; the menu tab is still a stub', async () => {
  const { s, socket } = await session(RUNNING);
  socket.deliver(HELLO);
  socket.deliver(MATCHDAY);
  await tick();
  const list = () => document.querySelector('section[aria-label="Other grounds"]');
  assert.match(list().textContent, /Matchday 1 · on your clock/);
  assert.equal(list().querySelectorAll('li').length, 2);
  assert.match(list().querySelector('li').textContent, /Castlemere United.*0 – 0.*Greywater\s*KO/);
  assert.equal(list().querySelector('[data-stub]'), null, 'no longer a stub');
  assert.ok(document.querySelector('[data-stub="tab: other-grounds"]'), 'the tab stays a stub');
  s.act();
  for (let t = 1; t <= 200; t += 1) {
    socket.deliver(encodeKeyframe(t, new Array(COMPONENT_COUNT).fill(0)));
  }
  socket.deliver(
    JSON.stringify({ type: 'ground-event', fixture: 0, tick: 150, kind: 'goal', minute: 0, side: 'home', scorer: 'Tomas Okafor', score: [1, 0] })
  );
  s.rewind(100);
  await tick();
  assert.match(list().querySelector('li').textContent, /0 – 0/, 'not before the pitch reaches it');
  s.rewind(200);
  await tick();
  assert.match(list().querySelector('li').textContent, /1 – 0/);
  assert.equal(list().querySelector('[tabindex], button, a[href]'), null, 'the list takes no focus');
});

test('the other grounds show skeleton rows while loading and in an error', async () => {
  await session(RUNNING);
  await tick();
  const list = document.querySelector('section[aria-label="Other grounds"]');
  assert.equal(list.querySelectorAll('.skel').length, 3);
  assert.equal(list.querySelectorAll('li').length, 0);
});
