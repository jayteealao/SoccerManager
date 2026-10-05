// @vitest-environment jsdom
// The handshake page, in jsdom: it prints the checks, the socket address, the hello and a
// sample of the tick frames; a message holding markup renders as text; the fact strip says
// each state in words; RUN AGAIN closes the socket, clears the log and connects again; every
// stub is inert and hidden, and a planted focusable stub fails the check.

import assert from 'node:assert/strict';
import { render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, test } from 'vitest';

import { HandshakeRun } from '../../src/lib/handshake-run.svelte.js';
import HandshakeScreen from '../../src/screens/HandshakeScreen.svelte';
import { stubFaults } from './harness.js';

class FakeSocket {
  static made = [];
  constructor(address) {
    this.address = address;
    this.readyState = 1;
    this.closed = false;
    this.listeners = {};
    FakeSocket.made.push(this);
  }
  addEventListener(type, fn) {
    (this.listeners[type] ??= []).push(fn);
  }
  emit(type, event) {
    for (const fn of this.listeners[type] ?? []) {
      fn(event);
    }
  }
  close() {
    this.closed = true;
    this.readyState = 3;
  }
}

afterEach(() => {
  FakeSocket.made = [];
  document.body.innerHTML = '';
});

const STATUS = { 'socket.port': 9001, 'protocol.version': 7 };

/// Timers the test fires by hand: `fire()` runs every pending one.
class FakeTimers {
  pending = new Map();
  next = 1;
  set = (fn, ms) => {
    const id = this.next++;
    this.pending.set(id, { fn, ms });
    return id;
  };
  clear = (id) => {
    this.pending.delete(id);
  };
  fire() {
    const due = [...this.pending.values()];
    this.pending.clear();
    for (const { fn } of due) {
      fn();
    }
  }
}

async function page({ status = STATUS, Socket = FakeSocket } = {}) {
  const timers = new FakeTimers();
  const run = new HandshakeRun({
    scope: { crossOriginIsolated: true, location: { origin: 'http://127.0.0.1:8080' }, performance: {} },
    fetcher: async () => ({ json: async () => status }),
    Socket,
    setTimer: timers.set,
    clearTimer: timers.clear,
  });
  render(HandshakeScreen, { run });
  await run.run();
  await tick();
  return { run, timers, socket: FakeSocket.made.at(-1) };
}

const lastLine = () => {
  const line = log().lastElementChild;
  return [line.className.split(' ')[0], line.textContent.trim()];
};
const socketFact = () => document.querySelector('.strip').textContent;

const log = () => document.querySelector('[role="log"]');

test('the page prints its checks, the address, the hello and sampled tick frames', async () => {
  const { socket } = await page();
  assert.equal(socket.address, 'ws://127.0.0.1:9001/?v=7');
  socket.emit('message', { data: JSON.stringify({ type: 'hello', protocol: 7 }) });
  for (let i = 0; i < 5; i += 1) {
    socket.emit('message', { data: new Uint8Array([1, 0, 0]).buffer });
  }
  socket.emit('close', { code: 1006, wasClean: false });
  await tick();
  const lines = [...log().children].map((l) => [l.className.split(' ')[0], l.textContent.trim()]);
  assert.deepEqual(lines.slice(0, 5), [
    ['plain', 'connecting…'],
    ['plain', 'origin http://127.0.0.1:8080'],
    ['ok', 'crossOriginIsolated true'],
    ['bad', 'memory gauge false'],
    ['plain', 'opening ws://127.0.0.1:9001/?v=7'],
  ]);
  assert.equal(lines[5][0], 'ok');
  assert.match(lines[5][1], /^hello: /);
  assert.equal(lines.filter(([, t]) => t.startsWith('tick frame')).length, 3, 'frames 1 to 3 only');
  assert.deepEqual(lines.at(-1), ['bad', 'closed code=1006 clean=false after 5 tick frames']);
  const strip = document.querySelector('.strip').textContent;
  assert.match(strip, /Yes\s*crossOriginIsolated/);
  assert.match(strip, /Missing\s*Memory gauge/);
  assert.match(strip, /5\s*Tick frames/);
  assert.match(strip, /closed · 1006\s*Socket/);
});

test('a message holding markup renders as text', async () => {
  const { socket } = await page();
  socket.emit('message', { data: '{"type":"note","text":"<b>bold</b><img src=x>"}' });
  await tick();
  assert.equal(log().querySelector('b, img'), null);
  assert.match(log().textContent, /<b>bold<\/b><img src=x>/);
});

test('RUN AGAIN closes the socket, clears the log and connects again', async () => {
  const { run, socket } = await page();
  socket.emit('message', { data: JSON.stringify({ type: 'hello' }) });
  await tick();
  assert.match(log().textContent, /hello/);
  document.querySelector('.cont').click();
  await tick();
  await tick();
  assert.equal(socket.closed, true);
  assert.equal(FakeSocket.made.length, 2);
  assert.doesNotMatch(log().textContent, /hello/);
  // A late event from the first socket prints nothing.
  socket.emit('message', { data: JSON.stringify({ type: 'hello' }) });
  await tick();
  assert.doesNotMatch(log().textContent, /hello/);
  assert.equal(run.ticks, 0);
});

test('with no socket port the page opens no socket and says that no match is running', async () => {
  const { run, timers } = await page({ status: { 'socket.port': null, 'protocol.version': 3 } });
  assert.equal(FakeSocket.made.length, 0);
  assert.deepEqual(lastLine(), [
    'bad',
    'no match is running, so the engine has no socket open. Start a match, then press Run again.',
  ]);
  assert.match(socketFact(), /No match\s*Socket/);
  const bad = [...document.querySelectorAll('.strip .bad')].map((b) => b.textContent.trim());
  assert.ok(bad.includes('No match'), `the socket fact is in the bad style: ${bad}`);
  assert.equal(run.running, false);
  assert.match(document.querySelector('.date').textContent, /Done/);
  assert.equal(timers.pending.size, 0);
});

test('a socket that never opens is closed after the wait, and its late events print nothing', async () => {
  const { run, timers, socket } = await page();
  assert.equal(timers.pending.size, 1);
  assert.equal([...timers.pending.values()][0].ms, 10_000);
  timers.fire();
  await tick();
  assert.equal(socket.closed, true);
  assert.deepEqual(lastLine(), [
    'bad',
    'no answer from the socket after 10 s. The engine serves one page at a time: close the match page, then press Run again.',
  ]);
  assert.match(socketFact(), /No answer\s*Socket/);
  assert.equal(run.running, false);
  const count = log().children.length;
  socket.emit('open', {});
  socket.emit('message', { data: JSON.stringify({ type: 'hello' }) });
  socket.emit('close', { code: 1006, wasClean: false });
  await tick();
  assert.equal(log().children.length, count);
  assert.match(socketFact(), /No answer\s*Socket/);
});

test('a socket that opens stops the wait', async () => {
  const { timers, socket } = await page();
  socket.emit('open', {});
  await tick();
  assert.equal(timers.pending.size, 0);
  assert.match(socketFact(), /Open\s*Socket/);
});

test('a socket the browser refuses to make prints the reason and ends the run', async () => {
  class Refused {
    constructor() {
      throw new SyntaxError("The URL 'ws://127.0.0.1:9001/?v=7' is invalid.");
    }
  }
  const { run, timers } = await page({ Socket: Refused });
  assert.deepEqual(lastLine(), [
    'bad',
    "cannot open ws://127.0.0.1:9001/?v=7: The URL 'ws://127.0.0.1:9001/?v=7' is invalid.",
  ]);
  assert.match(socketFact(), /Error\s*Socket/);
  assert.equal(run.running, false);
  assert.equal(timers.pending.size, 0);
});

test('every handshake stub is inert and hidden, and a planted focusable stub fails the check', async () => {
  await page();
  const root = document.querySelector('.app');
  assert.ok(root.querySelector('[data-stub="world view and help"]'));
  assert.equal(log().getAttribute('tabindex'), '0', 'the log scrolls from the keyboard');
  assert.deepEqual(stubFaults(root), []);
  root.querySelector('[data-stub="world view and help"]').append(document.createElement('button'));
  assert.deepEqual(stubFaults(root), ['world view and help: button takes focus']);
});
