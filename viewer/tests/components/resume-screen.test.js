// @vitest-environment jsdom
// The Resume a saved match screen, in jsdom: a save no engine can finish opens it with no
// socket; the strip and the alert name the save's version; the Replays tab and the rail are
// inert stubs; the live controls are the tab, NEW MATCH, Start a new match and Open a replay;
// Start a new match asks the launcher for a fresh match and leaves for the match view.

import assert from 'node:assert/strict';
import { fireEvent, render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { test } from 'vitest';

import App from '../../src/App.svelte';
import { MatchSession } from '../../src/lib/match-session.svelte.js';
import { FakeSocket, stubFaults, useFakes } from './harness.js';

useFakes();

const REFUSED = {
  'engine.state': 'refused',
  'engine.reason': 'this match was saved by Touchline 0.1.0, two or more versions back',
  'engine.pid': null,
  'engine.version': '0.3.0',
  'launcher.version': '0.3.0',
  launcher: true,
  resume: {
    kind: 'older',
    'saved.version': '0.1.0',
    'saved.build': '3ba8fed',
    'saved.tick': 156_500,
    'saved.teams': ['Ashford Rovers', 'Port Varrow'],
    'saved.score': [1, 0],
    'saved.millis': 1_700_000_000_000,
    engines: ['0.3.0', '0.2.0'],
    reason: 'r',
  },
};
const RUNNING = { 'engine.state': 'running', 'socket.port': 7001, 'protocol.version': 3, launcher: true };

async function refused() {
  const calls = [];
  const bodies = [REFUSED, REFUSED, RUNNING];
  const session = new MatchSession({
    fetcher: async (path, init = {}) => {
      calls.push({ path, method: init.method ?? 'GET' });
      const body = bodies.length > 1 ? bodies.shift() : bodies[0];
      return { ok: true, json: async () => body };
    },
    timers: { setTimeout: () => 0, clearTimeout: () => {} },
    raf: null,
    now: () => 0,
  });
  render(App, { session });
  await session.start();
  await tick();
  return { session, calls };
}

const screen = () => document.querySelector('[data-screen="resume"]');

test('a save no engine can finish opens the resume screen with no engine connected', async () => {
  const { session } = await refused();
  assert.equal(session.view, 'resume');
  assert.ok(screen(), 'the resume screen shows');
  assert.equal(FakeSocket.made.length, 0, 'no socket was opened');
  const text = document.body.textContent;
  for (const words of [
    'Resume a saved match',
    'Touchline 0.3.0 · finishes matches saved by 0.3.0 and 0.2.0',
    'TUE 14 NOVEMBER',
    'Ashford Rovers 1–0 Port Varrow · 52:10',
    'Saved by',
    'Engines in this version',
    'Two or more versions back',
    'This match was saved by Touchline 0.1.0',
    'To finish it, open the save in Touchline 0.1.0.',
    'The save stays on disk. Nothing is deleted.',
  ]) {
    assert.ok(text.includes(words), words);
  }
  const alert = document.querySelector('[role="alert"]');
  assert.ok(alert.textContent.includes('Cannot resume'));
});

test('only the built controls take focus; every stub is inert and hidden', async () => {
  await refused();
  const live = [...document.querySelectorAll('a[href], button:not([disabled]), input:not([tabindex="-1"])')]
    .filter((el) => !el.closest('[inert], [hidden]'))
    .map((el) => el.textContent.trim() || el.getAttribute('aria-label'));
  assert.deepEqual(live, ['Home', 'New match', 'Saved matches', 'Start a new match', 'Open a replay']);
  assert.deepEqual(stubFaults(document.body), []);
  const replays = [...document.querySelectorAll('[data-stub]')].find((s) => s.dataset.stub === 'tab: replays');
  assert.ok(replays, 'the Replays tab is a stub');
});

test('Start a new match asks the launcher for a fresh match and opens the match view', async () => {
  const { session, calls } = await refused();
  const start = [...document.querySelectorAll('button')].find((b) => b.textContent.trim() === 'Start a new match');
  await fireEvent.click(start);
  for (let i = 0; i < 5; i += 1) {
    await Promise.resolve();
  }
  await tick();
  assert.ok(calls.some((c) => c.path === 'engine/new-match' && c.method === 'POST'));
  assert.equal(session.view, 'match');
  assert.equal(screen(), null);
  assert.equal(FakeSocket.made.length, 1, 'the fresh match is connected');
});
