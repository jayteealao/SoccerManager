// @vitest-environment jsdom
// The Squad screen, its column menu and the player panel, in jsdom, on a protocol 6 hello: the
// table opens from the Tactics tab with the default view; a header sorts and a second select
// reverses; the menu's ↑ keeps focus on the moved row and Escape returns focus to the Columns
// chip; a name opens the panel with whole numbers 1 to 20 and the hidden values in words; every
// stub is inert and hidden.

import assert from 'node:assert/strict';
import { tick } from 'svelte';
import { test } from 'vitest';

import { DEFAULT_COLUMNS } from '../../src/lib/columns.js';
import { HELLO, opened, stubFaults, useFakes } from './harness.js';

useFakes();

const ATTRIBUTES = { pace: 152, acceleration: 147, passing: 121, technique: 133, decisions: 98, stamina: 160, handling: 40 };

/// The harness hello as protocol 6: every home player carries his attributes, body and level.
const HELLO6 = {
  ...HELLO,
  'protocol.version': 6,
  teams: [
    {
      ...HELLO.teams[0],
      squad: HELLO.teams[0].squad.map((p, i) => ({
        ...p,
        'player.attributes': { ...ATTRIBUTES, pace: ATTRIBUTES.pace - i * 3 },
        'player.height': 170 + i,
        'player.age': 19 + i,
        'player.nationality': i % 2 ? 'ESP' : 'ENG',
        'player.build': 'athletic',
        'player.condition': 100,
        'player.matches_at_club': i === 1 ? 31 : 0,
        'player.level': 140,
        'player.plays_between': [126, 140],
        ...(i === 1 ? { 'player.consistency': { word: 'rarely_off', confidence: 'firm' } } : {}),
      })),
    },
    HELLO.teams[1],
  ],
};

const squadRoot = () => document.querySelector('[data-screen="squad"]')?.closest('.app');
const settle = async () => {
  for (let i = 0; i < 4; i += 1) {
    await tick();
    await Promise.resolve();
  }
};

async function openSquad() {
  const opening = await opened(HELLO6);
  const tab = [...document.querySelectorAll('nav.subnav button.tab')].find((b) => b.textContent.trim() === 'Squad');
  assert.ok(tab, 'the Tactics screen has a live Squad tab');
  tab.click();
  await settle();
  assert.equal(opening.s.view, 'squad');
  return opening;
}

test('the Squad tab opens the table with the default view and every player', async () => {
  await openSquad();
  const root = squadRoot();
  const headers = [...root.querySelectorAll('[data-squad-table] thead th[data-col]')].map((th) => th.dataset.col);
  assert.deepEqual(headers.slice(0, 2 + DEFAULT_COLUMNS.length), ['no', 'player', ...DEFAULT_COLUMNS]);
  assert.equal(root.querySelectorAll('[data-squad-table] tbody tr[data-row]').length, 22);
  const pace = root.querySelector('tr[data-row="0"] td[data-cell="attr:pace"]');
  assert.equal(pace.textContent.trim(), '15');
  // No number under a hidden column, and no attribute cell with a decimal.
  for (const cell of root.querySelectorAll('td[data-cell^="attr:"]')) {
    assert.ok(!/\./.test(cell.textContent), cell.textContent);
  }
  assert.equal(root.querySelector('tr[data-row="0"] td[data-cell="consistency"]').textContent.trim(), 'Not yet known');
  assert.deepEqual(stubFaults(root), []);
});

test('a header sorts, a second select reverses, and the view says it changed', async () => {
  await openSquad();
  const root = squadRoot();
  const table = () => root.querySelector('[data-squad-table]');
  table().querySelector('th[data-col="height"] button').click();
  await settle();
  assert.equal(table().dataset.sort, 'height:down');
  assert.equal(table().querySelector('tbody tr[data-row]').dataset.row, '21');
  table().querySelector('th[data-col="height"] button').click();
  await settle();
  assert.equal(table().dataset.sort, 'height:up');
  assert.equal(table().querySelector('tbody tr[data-row]').dataset.row, '0');
});

test('the column menu moves a row by keyboard, keeps its focus, and Escape returns to the chip', async () => {
  await openSquad();
  const root = squadRoot();
  const chip = root.querySelector('[data-columns-chip]');
  chip.focus();
  chip.click();
  await settle();
  const menu = root.querySelector('[data-column-menu]');
  assert.ok(menu, 'the menu opens');
  assert.equal(chip.getAttribute('aria-expanded'), 'true');
  const up = menu.querySelector('li[data-shown="nat"] [data-control="up"]');
  up.click();
  await settle();
  const order = [...root.querySelectorAll('[data-column-menu] li[data-shown]')].map((li) => li.dataset.shown);
  assert.deepEqual(order.slice(0, 2), ['nat', 'age']);
  assert.equal(document.activeElement?.closest('li')?.dataset.shown, 'nat', 'the moved row keeps focus');
  // A column is added from its group, and a LATER column is not offered.
  root.querySelector('[data-column-menu] button[data-add="build"]').click();
  await settle();
  assert.ok(root.querySelector('[data-column-menu] li[data-shown="build"]'));
  assert.equal(root.querySelector('[data-column-menu] button[data-add="morale"]'), null);
  assert.deepEqual(stubFaults(root.querySelector('[data-column-menu]')), []);
  root.querySelector('[data-column-menu]').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  await settle();
  assert.equal(root.querySelector('[data-column-menu]'), null);
  assert.equal(document.activeElement, root.querySelector('[data-columns-chip]'));
  const headers = [...root.querySelectorAll('thead th[data-col]')].map((th) => th.dataset.col);
  assert.equal(headers[2], 'nat');
  assert.ok(headers.includes('build'));
});

test('a name opens the panel: whole numbers, words for hidden values, and back to the squad', async () => {
  const { s } = await openSquad();
  squadRoot().querySelector('tr[data-row="1"] button.who').click();
  await settle();
  assert.equal(s.view, 'player');
  const panel = document.querySelector('[data-screen="player"]');
  assert.equal(panel.dataset.player, 'p-0-2');
  const root = panel.closest('.app');
  assert.equal(panel.querySelector('[data-attr="pace"] dd').textContent.trim(), '15');
  for (const dd of panel.querySelectorAll('[data-attr] dd')) {
    const n = Number(dd.textContent.trim());
    assert.ok(Number.isInteger(n) && n >= 1 && n <= 20, dd.textContent);
  }
  assert.equal(panel.querySelector('[data-attr="handling"]'), null, 'an outfield player shows no goalkeeping group');
  assert.match(panel.querySelector('[data-hidden="consistency"]').textContent, /Rarely has an off day.*31 matches here, sure/);
  assert.equal(panel.querySelector('[data-plays]').textContent.replace(/\s+/g, ' ').trim(), 'Plays between 13 and 14');
  assert.equal(panel.querySelector('[data-ratings]').dataset.ratings, 'none');
  assert.deepEqual(stubFaults(root), []);
  const back = [...root.querySelectorAll('nav.subnav button.tab')].find((b) => b.textContent.trim() === 'Squad');
  back.click();
  await settle();
  assert.equal(s.view, 'squad');
});

test('the views are kept for the session: a saved copy is offered again', async () => {
  const { s } = await openSquad();
  squadRoot().querySelector('[data-save-view]').click();
  await settle();
  const names = [...squadRoot().querySelectorAll('select[data-view] option')].map((o) => o.value);
  assert.ok(names.length >= 2, names.join(', '));
  assert.ok(s.squadViews.views.length >= 2);
});
