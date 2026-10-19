// @vitest-environment jsdom
// The tactics board's three tabs in jsdom: the Phases board shows every primitive once in
// player words, with no engine id and no card named for a press or a block; a level set before
// kick-off rides the set-lineup, and during play it is one `instructions` change at once; the
// Roles menu disables an instruction a position does not take; and the Plans tab stops at the
// tactics file's limit.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { tick } from 'svelte';
import { test } from 'vitest';

import { placements } from '../../src/lib/board-cards.js';
import { REPO_ROOT } from '../helpers.js';
import { HELLO, VIEW, opened, stubFaults, useFakes } from './harness.js';

useFakes();

const STYLES = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/styles.json'), 'utf8'));
const BOARD_HELLO = {
  ...HELLO,
  'protocol.version': 9,
  styles: STYLES.styles.map((s) => ({
    name: s.name,
    primitives: s.primitives,
    formations: s.formations,
    rules: s.rules ?? [],
    player_instructions: [],
  })),
};
/// An engine id: a lower-case word joined to another by an underscore.
const ENGINE_ID = /\b[a-z]+_[a-z_]+\b/;

const tactics = () => document.querySelector('[data-screen="tactics"]').closest('.app');
const subTab = (name) =>
  [...tactics().querySelectorAll('nav.subnav button')].find((b) => b.textContent.trim() === name);

async function show(name) {
  subTab(name).click();
  await tick();
  return tactics().querySelector(`[data-board]`);
}

/// The visible words of `root`: its text and its controls' names.
function words(root) {
  const names = [...root.querySelectorAll('[aria-label]')].map((e) => e.getAttribute('aria-label'));
  const options = [...root.querySelectorAll('option')].map((o) => o.textContent);
  return [root.textContent, ...names, ...options].join(' \n ');
}

async function live() {
  const { s, socket } = await opened(BOARD_HELLO);
  s.act();
  s.act();
  socket.deliver({ type: 'ack', command: 'set-lineup' });
  await tick();
  s.dugout.update(10, VIEW);
  s.show('tactics');
  await tick();
  return { s, socket };
}

test('the Phases board places every primitive once, in player words', async () => {
  await opened(BOARD_HELLO);
  const board = await show('Phases');
  assert.ok(board, 'the board shows');
  assert.equal(board.querySelectorAll('[data-card]').length, 8);
  for (const p of placements()) {
    assert.equal(board.querySelectorAll(`[data-primitive="${p.primitive}"]`).length, 1, p.primitive);
  }
  const text = words(board);
  const id = text.match(ENGINE_ID);
  assert.equal(id, null, `an engine id shows: ${id?.[0]}`);
  for (const banned of ['High press', 'Mid block', 'Low block']) {
    assert.ok(!text.includes(banned), banned);
  }
  assert.deepEqual(stubFaults(tactics()), []);
});

test('before kick-off a level on the board rides the set-lineup, by name', async () => {
  const { s, socket } = await opened(BOARD_HELLO);
  const board = await show('Phases');
  const row = board.querySelector('[data-primitive="line_of_engagement"]');
  const pips = row.querySelectorAll('button[data-level]');
  const now = [...pips].findIndex((b) => b.getAttribute('aria-pressed') === 'true');
  const next = now === pips.length - 1 ? now - 1 : now + 1;
  pips[next].click();
  await tick();
  assert.equal(socket.sent.length, 0, 'nothing is sent before kick-off');
  s.act();
  s.act();
  const lineup = socket.sent.find((m) => m.type === 'set-lineup');
  assert.equal(lineup.levels.length, 1);
  assert.equal(lineup.levels[0].primitive, 'line_of_engagement');
  assert.equal(typeof lineup.levels[0].level, 'string');
});

test('during play one pip sends one instructions change for that primitive alone', async () => {
  const { socket } = await live();
  const board = await show('Phases');
  const row = board.querySelector('[data-primitive="line_of_engagement"]');
  const pips = row.querySelectorAll('button[data-level]');
  const now = [...pips].findIndex((b) => b.getAttribute('aria-pressed') === 'true');
  pips[now === 0 ? 1 : 0].click();
  await tick();
  const sent = socket.sent.at(-1);
  assert.equal(sent.type, 'queue-change');
  assert.equal(sent['change.kind'], 'instructions');
  assert.equal(sent.detail.levels.length, 1);
  assert.equal(sent.detail.levels[0].primitive, 'line_of_engagement');
});

test('the Roles menu disables forward runs for a centre-back', async () => {
  await opened(BOARD_HELLO);
  const board = await show('Roles');
  // Slot 3 is a centre-back in the first formation.
  board.querySelector('button[aria-label="The player before"]');
  for (let i = 0; i < 7; i += 1) {
    board.querySelector('button[aria-label="The player before"]').click();
  }
  await tick();
  assert.match(board.querySelector('.name').textContent, /CB/);
  [...board.querySelectorAll('button')].find((b) => b.textContent.trim() === 'Add instruction').click();
  await tick();
  const runs = board.querySelector('button[data-add="forward_runs"]');
  assert.ok(runs, 'forward runs is on the menu');
  assert.equal(runs.disabled, true);
  assert.match(runs.textContent, /wingers and strikers only/);
  assert.equal(words(board).match(ENGINE_ID), null);
});

test('the Plans tab adds a plan and stops at the limit', async () => {
  const { s } = await opened(BOARD_HELLO);
  const board = await show('Plans & opposition');
  const max = s.dugout.schema.game_state?.max_rules ?? 6;
  const button = (name) => [...board.querySelectorAll('button')].find((b) => b.textContent.trim() === name);
  for (let n = 0; n < max; n += 1) {
    button('New plan').click();
    await tick();
    button('Save plan').click();
    await tick();
  }
  assert.equal(s.dugout.gameState.rules.length, max);
  assert.equal(button('New plan').disabled, true);
  assert.match(board.querySelector('[data-plans-full]').textContent, /remove one to add another/);
  assert.equal(words(board).match(ENGINE_ID), null);
});
