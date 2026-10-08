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

async function openSquad(hello = HELLO6) {
  const opening = await opened(hello);
  const tab = [...document.querySelectorAll('nav.subnav button.tab')].find((b) => b.textContent.trim() === 'Squad');
  assert.ok(tab, 'the Tactics screen has a live Squad tab');
  tab.click();
  await settle();
  assert.equal(opening.s.view, 'squad');
  return opening;
}

/// A cell's drawn text, without the screen reader's text kept out of sight beside it.
function drawn(cell) {
  const copy = cell.cloneNode(true);
  for (const sr of copy.querySelectorAll('.sr')) sr.remove();
  return copy.textContent.trim();
}

test('the Squad tab opens the table with the default view and every player', async () => {
  await openSquad();
  const root = squadRoot();
  const headers = [...root.querySelectorAll('[data-squad-table] thead th[data-col]')].map((th) => th.dataset.col);
  assert.deepEqual(headers.slice(0, 2 + DEFAULT_COLUMNS.length), ['no', 'player', ...DEFAULT_COLUMNS]);
  assert.equal(root.querySelectorAll('[data-squad-table] tbody tr[data-row]').length, 22);
  const pace = root.querySelector('tr[data-row="0"] td[data-cell="attr:pace"]');
  assert.equal(drawn(pace), '15');
  // A screen reader hears the number with its band word as real text, not a label on a span.
  assert.equal(pace.querySelector('.sr').textContent, '15, good');
  assert.equal(pace.querySelector('[aria-label]'), null);
  // No number under a hidden column, and no attribute cell with a decimal.
  for (const cell of root.querySelectorAll('td[data-cell^="attr:"]')) {
    assert.ok(!/\./.test(cell.textContent), cell.textContent);
  }
  assert.equal(drawn(root.querySelector('tr[data-row="0"] td[data-cell="consistency"]')), 'Not yet known');
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
  // The view's state is announced in a status region, not inside the View select's name.
  const status = root.querySelector('[data-view-status]');
  assert.equal(status.getAttribute('role'), 'status');
  assert.match(status.textContent, /^View .+, .+\. Showing 22 of 22 players\.$/);
  assert.equal(root.querySelector('[data-view-state]').getAttribute('aria-hidden'), 'true');
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
  assert.equal(document.activeElement?.closest('li')?.dataset.shown, 'build', 'the added row takes focus');
  assert.equal(root.querySelector('[data-column-menu] button[data-add="morale"]'), null);
  // A removed row hands focus to the row after it, so focus never falls out of the menu.
  const shownIds = () => [...root.querySelectorAll('[data-column-menu] li[data-shown]')].map((li) => li.dataset.shown);
  const after = shownIds()[1];
  root.querySelector(`[data-column-menu] li[data-shown="${shownIds()[0]}"] [data-control="remove"]`).click();
  await settle();
  assert.equal(document.activeElement?.closest('li')?.dataset.shown, after, 'the next row takes focus');
  assert.equal(document.activeElement?.dataset.control, 'remove');
  assert.deepEqual(stubFaults(root.querySelector('[data-column-menu]')), []);
  root.querySelector('[data-column-menu]').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  await settle();
  assert.equal(root.querySelector('[data-column-menu]'), null);
  assert.equal(document.activeElement, root.querySelector('[data-columns-chip]'));
  const headers = [...root.querySelectorAll('thead th[data-col]')].map((th) => th.dataset.col);
  assert.equal(headers[2], 'age');
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

test('a match with no squad list says so under the empty table', async () => {
  const bare = { ...HELLO6, teams: [{ ...HELLO6.teams[0], squad: [] }, HELLO6.teams[1]] };
  await openSquad(bare);
  const root = squadRoot();
  assert.equal(root.querySelectorAll('[data-squad-table] tbody tr[data-row]').length, 0);
  assert.match(root.querySelector('[data-squad-empty]').textContent, /no squad list/);
});

test('a protocol 5 player with no attributes still shows his hidden values in words', async () => {
  const hello5 = {
    ...HELLO,
    'protocol.version': 5,
    teams: [
      {
        ...HELLO.teams[0],
        squad: HELLO.teams[0].squad.map((p, i) => ({
          ...p,
          ...(i === 1 ? { 'player.consistency': { word: 'rarely_off', confidence: 'firm' } } : {}),
        })),
      },
      HELLO.teams[1],
    ],
  };
  await openSquad(hello5);
  squadRoot().querySelector('tr[data-row="1"] button.who').click();
  await settle();
  const panel = document.querySelector('[data-screen="player"]');
  assert.equal(panel.querySelectorAll('[data-attr]').length, 0, 'a protocol 5 player has no attributes');
  assert.match(panel.querySelector('[data-hidden="consistency"]').textContent, /Rarely has an off day/);
  assert.ok(panel.querySelector('[data-hidden="injury_proneness"]'));
  assert.match(panel.textContent, /Hidden values/);
});
