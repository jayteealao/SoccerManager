// @vitest-environment jsdom
// The front door's screens and shared parts, in jsdom: the splash, the start screen with and
// without a save, match setup and its same-team refusal, the settings, the licences and
// about, the in-match menu, the confirmations, the closed page and the shell's Menu button.

import assert from 'node:assert/strict';
import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { createRawSnippet, flushSync } from 'svelte';
import { test, vi } from 'vitest';

import AppShell from '../../src/components/AppShell.svelte';
import ConfirmDialog from '../../src/components/ConfirmDialog.svelte';
import MenuPopover from '../../src/components/MenuPopover.svelte';
import { FrontDoor } from '../../src/lib/front-door.svelte.js';
import ClosedScreen from '../../src/screens/ClosedScreen.svelte';
import LicencesScreen from '../../src/screens/LicencesScreen.svelte';
import SettingsScreen from '../../src/screens/SettingsScreen.svelte';
import SetupScreen from '../../src/screens/SetupScreen.svelte';
import SplashScreen from '../../src/screens/SplashScreen.svelte';
import StartScreen from '../../src/screens/StartScreen.svelte';

const TEAMS = [
  { id: 'club-00000001-00', name: 'Oakmere Rangers', short_name: 'OAK', kit: ['#1d4ed8', '#ffffff'], ground: [105, 68], strength: 62 },
  { id: 'club-00000002-00', name: 'Eldstead City', short_name: 'ELD', kit: ['#b91c1c', '#ffffff'], ground: [100, 64], strength: 55 },
];

const SAVED = {
  kind: 'current',
  version: '0.2.0-dev',
  tick: 156500,
  teams: ['Oakmere Rangers', 'Eldstead City'],
  score: [1, 1],
  millis: 0,
  positions: {
    pitch: [105, 68],
    players: Array.from({ length: 22 }, (_, i) => [i < 11 ? 0 : 1, 10 + i * 4, 34]),
  },
};

const IDLE = {
  'engine.state': 'idle',
  'engine.version': '0.2.0-dev',
  'launcher.version': '0.2.0-dev',
  'previous.version': '0.2.0-beta.1',
  'protocol.version': 3,
  'front-door': true,
  teams: TEAMS,
  saved: null,
  settings: { speed: 1, motion: 'follow', commentary: true },
};

const NOTICES = {
  version: '0.2.0-dev',
  packages: [
    { kind: 'font', name: 'Saira', version: null, licence: 'OFL-1.1', description: 'Screen text', text: 'SIL OPEN FONT LICENSE Version 1.1' },
    { kind: 'npm', name: 'svelte', version: '5.57.1', licence: 'MIT', description: 'Cybernetically enhanced web apps', text: 'MIT svelte text' },
    { kind: 'crate', name: 'rhai', version: '1.26.1', licence: 'MIT OR Apache-2.0', description: 'Embedded scripting', text: 'MIT rhai text' },
  ],
};

class FakeSession {
  constructor(options) {
    this.options = options;
    this.view = 'match';
  }

  start() {}

  dispose() {}
}

function frontDoor(status = IDLE, routes = {}) {
  const calls = [];
  const fetcher = async (path, init = {}) => {
    calls.push({ path, method: init.method ?? 'GET', body: init.body ? JSON.parse(init.body) : undefined });
    const key = Object.keys(routes).find((k) => path.startsWith(k));
    const body = key ? routes[key] : null;
    return { ok: body !== null, json: async () => body };
  };
  const door = new FrontDoor({
    fetcher,
    timers: { setTimeout: () => 0, clearTimeout: () => {} },
    now: () => 0,
    makeSession: (options) => new FakeSession(options),
    reduced: () => false,
  });
  if (status) {
    door.answer(status);
  }
  door.calls = calls;
  return door;
}

const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

test('the splash names the game, the studio mark and the engine before the answer', () => {
  const door = frontDoor(null);
  const { container } = render(SplashScreen, { door });
  assert.ok(screen.getByRole('heading', { name: 'Touchline' }));
  assert.ok(screen.getByText('Football match simulation'));
  assert.ok(screen.getByText(/Powered by the Touchline match engine/));
  assert.ok(container.querySelector('[data-stub="studio mark placeholder"]'));
  assert.ok(screen.getByText('The start screen opens when the engine answers.'));
  assert.equal(container.querySelector('[data-screen="splash"]').dataset.phase, 'reveal');
});

test('the ready splash asks for any key or click', () => {
  const door = frontDoor(null);
  door.revealDone = true;
  door.started = -10_000;
  door.answer(IDLE);
  render(SplashScreen, { door });
  assert.equal(door.splash.phase, 'ready');
  assert.ok(screen.getByText('Press any key or click to continue'));
  assert.ok(screen.getByText(/Rules, teams and grounds loaded/));
  assert.ok(screen.getByText('Laws of the Game rule pack · 2 sample teams'));
});

test('the start screen lists six choices; Resume is disabled with its reason when nothing is saved', () => {
  const door = frontDoor();
  door.view = 'start';
  const { container } = render(StartScreen, { door });
  const choices = [...container.querySelectorAll('[data-choice]')].map((b) => b.dataset.choice);
  assert.deepEqual(choices, ['new', 'resume', 'replays', 'settings', 'licences', 'quit']);
  const resume = container.querySelector('[data-choice="resume"]');
  assert.equal(resume.disabled, true);
  assert.match(resume.textContent, /No saved match yet/);
  assert.ok(container.querySelector('[data-empty="saved"]'));
  // The career rows and the recent replays are inert LATER stubs.
  const career = container.querySelector('[data-stub="career: new career, load career, save slots"]');
  assert.ok(career.hasAttribute('inert'));
  assert.ok(container.querySelector('[data-stub="recent replays list: no replay library yet"]').hasAttribute('inert'));
});

test('with a save the start screen shows its pitch, its facts and Resume', () => {
  const door = frontDoor({ ...IDLE, saved: SAVED });
  const { container } = render(StartScreen, { door });
  const resume = container.querySelector('[data-choice="resume"]');
  assert.equal(resume.disabled, false);
  assert.match(resume.textContent, /Oakmere Rangers 1–1 Eldstead City · 52:10/);
  assert.equal(container.querySelector('svg.pitch').getAttribute('data-markers'), '22');
  assert.equal(container.querySelectorAll('svg.pitch circle.marker').length, 22);
  assert.ok(screen.getByRole('button', { name: /Resume at 52:10/ }));
  assert.ok(screen.getByText('0.2.0-dev · this release'));
});

test('a previous release save draws the pitch without markers', () => {
  const door = frontDoor({ ...IDLE, saved: { ...SAVED, kind: 'previous', version: '0.2.0-beta.1', positions: null } });
  const { container } = render(StartScreen, { door });
  assert.equal(container.querySelectorAll('svg.pitch circle.marker').length, 0);
  assert.ok(screen.getByText('0.2.0-beta.1 · previous engine'));
});

test('arrow keys move between the live start choices', async () => {
  const door = frontDoor();
  const { container } = render(StartScreen, { door });
  const first = container.querySelector('[data-choice="new"]');
  first.focus();
  await fireEvent.keyDown(first, { key: 'ArrowDown' });
  assert.equal(document.activeElement.dataset.choice, 'replays', 'the disabled Resume is skipped');
});

test('match setup shows both tables, the ground and the round; the same team twice is refused', async () => {
  const round = { fixtures: [{ home: TEAMS[1], away: TEAMS[0] }] };
  const door = frontDoor(IDLE, { 'engine/round': round });
  door.open('setup');
  await flush();
  const { container } = render(SetupScreen, { door });
  assert.equal(container.querySelectorAll('[data-column="home"] tbody tr').length, 2);
  assert.ok(screen.getAllByText('105 × 68 m').length >= 1);
  assert.equal(container.querySelector('[data-round]').dataset.round, '1');
  assert.equal(container.querySelector('[data-kickoff]').disabled, false);

  door.pick('away', 'club-00000001-00');
  flushSync();
  const alert = screen.getByRole('alert');
  assert.match(alert.textContent, /Same team/);
  assert.match(alert.textContent, /A team cannot play itself\. Pick a different away team\./);
  assert.equal(container.querySelector('[data-kickoff]').disabled, true);
  assert.ok(screen.getByRole('button', { name: 'Pick an away team' }).disabled);
});

test('settings show the three values pressed and save a change at once', async () => {
  const saved = { ...IDLE, settings: { speed: 4, motion: 'follow', commentary: true } };
  const door = frontDoor(IDLE, { 'engine/settings': saved });
  door.open('settings');
  render(SettingsScreen, { door });
  const speed = screen.getByRole('group', { name: 'Default speed' });
  assert.equal(within(speed).getByRole('button', { pressed: true }).textContent.trim(), '1×');
  assert.equal(
    within(screen.getByRole('group', { name: 'Animation' })).getByRole('button', { pressed: true }).textContent.trim(),
    'Follow system'
  );
  assert.equal(
    within(screen.getByRole('group', { name: 'Commentary' })).getByRole('button', { pressed: true }).textContent.trim(),
    'On'
  );
  await fireEvent.click(within(speed).getByRole('button', { name: '4×' }));
  await flush();
  const post = door.calls.find((c) => c.path === 'engine/settings');
  assert.deepEqual(post.body, { speed: 4, motion: 'follow', commentary: true });
  flushSync();
  assert.equal(within(speed).getByRole('button', { pressed: true }).textContent.trim(), '4×');
  assert.ok(screen.getByText('Saved'));
  assert.ok(document.querySelector('[data-stub="look and access settings: skin picker, text size, colour modes"]'));
});

test('licences and about list the notices file and show the chosen licence text', async () => {
  const door = frontDoor(IDLE, { 'notices.json': NOTICES });
  door.open('licences');
  await flush();
  const { container } = render(LicencesScreen, { door });
  assert.equal(container.querySelectorAll('[data-package]').length, 3);
  assert.equal(container.querySelector('pre.text').textContent, 'SIL OPEN FONT LICENSE Version 1.1');
  await fireEvent.click(screen.getByRole('button', { name: 'rhai' }));
  assert.equal(container.querySelector('pre.text').textContent, 'MIT rhai text');
  assert.ok(screen.getByText('3 packages'));
  assert.ok(screen.getAllByText('0.2.0-beta.1').length >= 2);
});

test('a build without the notices file says so', async () => {
  const door = frontDoor(IDLE, {});
  door.open('licences');
  await flush();
  render(LicencesScreen, { door });
  assert.ok(screen.getByText(/This build carries no notices file/));
});

test('the menu lists its five items, focuses Resume match and names the pause', async () => {
  const onchoose = vi.fn();
  const { container } = render(MenuPopover, { clock: '67:12', onchoose });
  flushSync();
  const items = [...container.querySelectorAll('[role="menuitem"]')].map((b) => b.textContent.replace(/\s+/g, ' ').trim());
  assert.deepEqual(items, ['Resume match Esc', 'Return to start screen', 'Settings', 'Licences and about', 'Quit Touchline']);
  assert.match(container.querySelector('.head').textContent, /Match paused · 67:12/);
  assert.equal(document.activeElement.dataset.item, 'resume');
  await fireEvent.keyDown(document.activeElement, { key: 'ArrowUp' });
  assert.equal(document.activeElement.dataset.item, 'quit');
  await fireEvent.click(container.querySelector('[data-item="return"]'));
  assert.equal(onchoose.mock.calls[0][0], 'return');
});

test('a confirmation names its word, title and lines; Keep playing cancels', async () => {
  const onconfirm = vi.fn();
  const oncancel = vi.fn();
  render(ConfirmDialog, {
    word: 'Quit',
    title: 'Quit Touchline?',
    lines: ['The match saves at 67:12.'],
    primary: 'Save and quit',
    onconfirm,
    oncancel,
  });
  flushSync();
  const dialog = screen.getByRole('dialog', { name: 'Quit Touchline?' });
  assert.equal(dialog.getAttribute('aria-modal'), 'true');
  assert.equal(document.activeElement.textContent, 'Save and quit');
  await fireEvent.click(screen.getByRole('button', { name: 'Keep playing' }));
  assert.equal(oncancel.mock.calls.length, 1);
  await fireEvent.click(screen.getByRole('button', { name: 'Save and quit' }));
  assert.equal(onconfirm.mock.calls.length, 1);
});

test('the closed page shows the three steps of a quit during a match', () => {
  const door = frontDoor();
  door.closed = { match: true, saved: SAVED };
  door.view = 'closed';
  render(ClosedScreen, { door });
  assert.ok(screen.getByRole('heading', { name: 'Touchline has closed' }));
  assert.ok(screen.getByText('You can close this tab.'));
  assert.ok(screen.getByText(/Your match is saved at 52:10, Oakmere Rangers 1–1 Eldstead City/));
  assert.ok(screen.getByText(/Match saved/));
  assert.ok(screen.getByText(/Launcher stopped/));
});

test('the shell draws the Menu button only when given a menu', async () => {
  const text = createRawSnippet(() => ({ render: () => '<p>body</p>' }));
  const menu = vi.fn();
  const { unmount } = render(AppShell, { title: 'T', date: 'D', action: 'A', menu, children: text });
  const button = screen.getByRole('button', { name: /Menu/ });
  assert.equal(button.getAttribute('aria-haspopup'), 'menu');
  assert.equal(button.getAttribute('aria-expanded'), 'false');
  await fireEvent.click(button);
  assert.equal(menu.mock.calls.length, 1);
  unmount();
  render(AppShell, { title: 'T', date: 'D', action: 'A', children: text });
  assert.equal(screen.queryByRole('button', { name: /Menu/ }), null);
});
