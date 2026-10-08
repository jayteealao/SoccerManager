// The session harness the screen tests share: a fake socket, a hello with a lineup to pick,
// a session opened on it with the viewer's views rendered, and the stub check.

import fs from 'node:fs';
import path from 'node:path';
import { render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach } from 'vitest';

import App from '../../src/App.svelte';
import { MatchSession } from '../../src/lib/match-session.svelte.js';
import { clearSignals } from '../../src/lib/signal.js';
import { REPO_ROOT, roster } from '../helpers.js';

export const SCHEMA = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/tactics.json'), 'utf8'));

export class FakeSocket {
  static OPEN = 1;
  static made = [];
  constructor() {
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
  deliver(message) {
    for (const fn of this.listeners.message ?? []) {
      fn({ data: JSON.stringify(message) });
    }
  }
}

const POSITIONS = [
  'GK', 'LB', 'CB', 'CB', 'RB', 'LW', 'CM', 'CM', 'RW', 'ST', 'ST',
  'GK', 'CB', 'LB', 'RB', 'DM', 'CM', 'AM', 'LW', 'RW', 'ST', 'ST',
];

const squad = (team) =>
  POSITIONS.map((position, i) => ({
    'player.id': `p-${team}-${i + 1}`,
    'player.name': `Player ${team}-${i + 1}`,
    'player.shirt': i + 1,
    'player.position': position,
    'player.natural_fitness': 140,
    'player.consistency': { confidence: 'not_yet_known' },
    'player.injury_proneness': { word: 'rarely_injured', confidence: 'tentative' },
    role_fit: SCHEMA.roles.map(() => 100),
  }));

export const SETUP = {
  lineup: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
  bench: [11, 12, 13, 14, 15, 16, 17],
  formation: 0,
  mentality: 2,
  instructions: [1, 1, 1, 1, 1, 0],
  roles: [0, 3, 1, 1, 3, 9, 6, 6, 9, 11, 11].map((role) => ({ role, duty: 1 })),
};

export const HELLO = {
  type: 'hello',
  'match.id': 'm',
  'protocol.version': 3,
  'engine.version': '0.3.0',
  ticks_expected: 270000,
  tactics: SCHEMA,
  substitutions: { limit: 5, windows: 3 },
  teams: [
    {
      'team.id': 'a',
      'team.name': 'Ashford Rovers',
      'team.kit.primary': '#c8102e',
      'team.kit.secondary': '#000000',
      roster: roster(0),
      squad: squad(0),
      setup: SETUP,
    },
    {
      'team.id': 'b',
      'team.name': 'Port Varrow',
      'team.kit.primary': '#6a0dad',
      'team.kit.secondary': '#ff6a13',
      roster: roster(1),
    },
  ],
};

export const VIEW = {
  substitutions: [],
  energy: null,
  cards: new Map(),
  sentOff: new Set(),
  injuries: new Set(),
  fullTime: false,
};

const RUNNING = { 'engine.state': 'running', 'socket.port': 7001, 'protocol.version': 3, launcher: true };

export async function opened(hello = HELLO) {
  const s = new MatchSession({
    fetcher: async () => ({ ok: true, json: async () => RUNNING }),
    timers: { setTimeout: () => 0, clearTimeout: () => {} },
    raf: null,
    now: () => 0,
  });
  render(App, { session: s });
  await s.start();
  const socket = FakeSocket.made.at(-1);
  socket.deliver(hello);
  await tick();
  return { s, socket };
}

/// Every stub is inert and hidden, and holds nothing that takes focus. Returns what breaks it.
export function stubFaults(root) {
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

/// Installs the fake socket before each test and clears the page after it.
export function useFakes() {
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
}
