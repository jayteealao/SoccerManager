// @vitest-environment jsdom
// The Touchline's player state in jsdom: the Mood column shows each of the five words with its
// mark, only the mark takes a colour, no digit sits in a mood cell or its label; the foot note
// says mood is never a number; the out-of-position sentence shows only when there is one.

import assert from 'node:assert/strict';
import { render } from '@testing-library/svelte';
import { afterEach, test } from 'vitest';

import PlayerState from '../../src/components/PlayerState.svelte';
import { playerStateRows } from '../../src/lib/touchline.js';

afterEach(() => {
  document.body.innerHTML = '';
});

const MOODS = ['confident', 'lifted', 'steady', 'low', 'rattled', null];

function rows() {
  return playerStateRows(
    MOODS.map((mood, i) => ({
      wire: i,
      id: `h-${i}`,
      name: `Home Player${i}`,
      shirt: i + 1,
      energy: 0.9,
      band: 'Fresh',
      token: '--good',
      condition: 'Fresh',
      level: 130,
      base: 130,
      mood,
      sentOff: false,
      injured: false,
    })),
    []
  );
}

test('the mood column draws the five words with their marks, only the marks coloured, never a digit', () => {
  render(PlayerState, { rows: rows(), clock: '12:00' });
  const header = [...document.querySelectorAll('th')].map((th) => th.textContent.trim());
  assert.ok(header.includes('Mood'));
  assert.ok(!header.some((h) => /Mind/.test(h)), 'the Mind stub is gone');
  const cells = [...document.querySelectorAll('td.mood')];
  assert.equal(cells.length, 6);
  const words = cells.map((c) => c.querySelector('.mw').textContent.trim());
  assert.deepEqual(words, ['Confident', 'Lifted', 'Steady', 'Low', 'Rattled', '—']);
  const marks = cells.map((c) => c.querySelector('.mk')?.textContent ?? null);
  assert.deepEqual(marks, ['▲', '▲', '=', '▼', '▼', null]);
  const tones = cells.map((c) => c.querySelector('.mk')?.style.color ?? null);
  assert.deepEqual(tones, ['var(--good)', 'var(--good)', '', 'var(--warn)', 'var(--warn)', null]);
  for (const cell of cells) {
    assert.equal(cell.querySelector('.mw').getAttribute('style'), null, 'the word takes no colour');
    assert.ok(!/\d/.test(cell.textContent), cell.textContent);
    assert.ok(!/\d/.test(cell.getAttribute('aria-label')), cell.getAttribute('aria-label'));
  }
  assert.equal(cells[5].dataset.mood, 'unknown');
  assert.equal(cells[5].getAttribute('aria-label'), 'Mood not known');
  assert.match(document.querySelector('.foot').textContent, /never shown as a number/);
  assert.equal(document.querySelector('[data-out-of-position]'), null);
});

test('the out-of-position sentence shows under the table when a starter plays away from home', () => {
  const note = [{ id: 'h-4', text: 'Player4 plays ST, away from his natural CM: familiarity lowers his level.' }];
  render(PlayerState, { rows: rows(), clock: '12:00', outOfPosition: note });
  const p = document.querySelector('[data-out-of-position]');
  assert.equal(p.dataset.outOfPosition, 'h-4');
  assert.equal(p.textContent.trim(), note[0].text);
});
