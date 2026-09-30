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

async function page() {
  const run = new HandshakeRun({
    scope: { crossOriginIsolated: true, location: { origin: 'http://127.0.0.1:8080' }, performance: {} },
    fetcher: async () => ({ json: async () => STATUS }),
    Socket: FakeSocket,
  });
  render(HandshakeScreen, { run });
  await run.run();
  await tick();
  return { run, socket: FakeSocket.made.at(-1) };
}

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

test('every handshake stub is inert and hidden, and a planted focusable stub fails the check', async () => {
  await page();
  const root = document.querySelector('.app');
  assert.ok(root.querySelector('[data-stub="world view and help"]'));
  assert.equal(log().getAttribute('tabindex'), '0', 'the log scrolls from the keyboard');
  assert.deepEqual(stubFaults(root), []);
  root.querySelector('[data-stub="world view and help"]').append(document.createElement('button'));
  assert.deepEqual(stubFaults(root), ['world view and help: button takes focus']);
});
