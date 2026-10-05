// @vitest-environment jsdom
// The other-grounds list in each of the board's seven states, in jsdom: every state carries a
// word or a mark (never colour alone), each row sits on the grid of 1fr, 44 px, 1fr and
// 30 px with 12 px crests, the new goal's outline has no animation under reduced motion, and
// the list holds nothing that takes focus.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { render } from '@testing-library/svelte';
import { afterEach, test } from 'vitest';

import OtherGrounds from '../../src/components/OtherGrounds.svelte';
import { addEvent, addProgress, groundsAt, newMatchday } from '../../src/lib/matchday.js';
import { TICKS_PER_MINUTE } from '../../src/lib/skip.js';
import { REPO_ROOT } from '../helpers.js';

const M = TICKS_PER_MINUTE;
const club = (id, name) => ({ 'team.id': id, 'team.name': name, 'team.kit.primary': '#0f5c63', 'team.kit.secondary': '#ffffff', roster: [] });
const MESSAGE = {
  round: 1,
  fixtures: [
    { fixture: 0, home: club('cu', 'Castlemere United'), away: club('gw', 'Greywater') },
    { fixture: 1, home: club('ka', 'Kelder Athletic'), away: club('mt', 'Millbridge Town') },
    { fixture: 2, home: club('df', 'Dunmore FC'), away: club('oc', 'Oldfield City') },
  ],
};
const event = (fixture, tick, kind, minute, score, extra = {}) => ({ fixture, tick, kind, minute, score, ...extra });

function day(events, reached = [99 * M, 99 * M, 99 * M], arrivedAt = 0) {
  let d = newMatchday(MESSAGE);
  for (const e of events) {
    d = addEvent(d, e, arrivedAt);
  }
  return addProgress(d, { reached });
}

function draw(grounds, props = {}) {
  const target = document.createElement('div');
  document.body.append(target);
  render(OtherGrounds, { target, props: { grounds, ...props } });
  return target;
}

const rows = (root) => [...root.querySelectorAll('li')];
const text = (el) => el.textContent.replace(/\s+/g, ' ').trim();

afterEach(() => {
  document.body.innerHTML = '';
});

test('no other matches: the label stays, with the words for an empty matchday', () => {
  const root = draw(groundsAt(newMatchday({ round: 1, fixtures: [] }), 1000));
  assert.match(text(root), /Other grounds\s*Matchday 1/);
  assert.match(text(root), /No other matches this matchday\./);
  assert.equal(rows(root).length, 0);
});

test('not started: every fixture at 0 – 0 with KO', () => {
  const root = draw(groundsAt(day([]), 0));
  assert.match(text(root), /Matchday 1 · on your clock/);
  for (const row of rows(root)) {
    assert.match(text(row), /0 – 0 .*KO$/);
  }
});

test('a new goal: the outlined block, the GOAL word and who scored', () => {
  const d = day([event(0, 67 * M + 100, 'goal', 67, [2, 1], { side: 'home', scorer: 'Tomas Okafor' })]);
  const root = draw(groundsAt(d, 67 * M + 200));
  const first = rows(root)[0];
  assert.ok(first.classList.contains('new'));
  assert.equal(first.querySelector('.tag').textContent, 'GOAL');
  assert.match(text(first), /Okafor 67'\. Castlemere lead\./);
  assert.ok(!rows(root)[1].classList.contains('new'));
});

test('an event shown late: GOAL · LATE and the minute it was shown', () => {
  const d = day(
    [event(1, 55 * M + 100, 'goal', 55, [1, 0], { side: 'home', scorer: 'Ade Harrow', late: true })],
    [61 * M, 58 * M, 61 * M],
    61 * M
  );
  const root = draw(groundsAt(d, 61 * M + 50));
  const row = rows(root)[1];
  assert.ok(row.classList.contains('late'));
  assert.equal(row.querySelector('.tag').textContent, 'GOAL · LATE');
  assert.match(text(row), /Harrow 55', shown at your 61'\./);
});

test('result unavailable: the ! mark, a dash for the score and the words', () => {
  const d = day([event(2, 34 * M + 10, 'unavailable', 34, [0, 0])]);
  const root = draw(groundsAt(d, 52 * M));
  const row = rows(root)[2];
  assert.equal(row.querySelector('.minute').textContent, '!');
  assert.equal(row.querySelector('.score').textContent.trim(), '–');
  assert.match(text(row), /Result unavailable\. A fault stopped this match at 34'\. A bug report is saved\./);
  assert.match(text(rows(root)[0]), /0 – 0/, 'the others play on');
});

test('full time elsewhere: FT on a ground with less added time, the minute on the others', () => {
  const d = day([
    event(0, 45 * M + 3_000, 'half-time', 45, [0, 0], { added: 1 }),
    event(0, 46 * M + 3_000, 'second-half', 45, [0, 0]),
    event(0, 92 * M, 'full-time', 90, [2, 1], { added: 1 }),
    event(1, 45 * M + 3_000, 'half-time', 45, [0, 0], { added: 1 }),
    event(1, 46 * M + 3_000, 'second-half', 45, [0, 0]),
  ]);
  const root = draw(groundsAt(d, 94 * M));
  assert.equal(rows(root)[0].querySelector('.minute').textContent, 'FT');
  assert.match(rows(root)[1].querySelector('.minute').textContent, /^90\+\d'$/);
});

test('after a rewind: later goals are hidden and no row is outlined', () => {
  const d = day([event(0, 30 * M, 'goal', 30, [1, 0], { side: 'home', scorer: 'Tomas Okafor' }), event(1, 55 * M, 'goal', 55, [1, 0], { side: 'home', scorer: 'Ade Harrow' })]);
  const root = draw(groundsAt(d, 40 * M, { seeks: [{ lo: 40 * M, hi: 67 * M, n: 1 }] }));
  assert.match(text(rows(root)[0]), /1 – 0 .*40'$/);
  assert.match(text(rows(root)[1]), /0 – 0 .*40'$/);
  assert.equal(root.querySelectorAll('li.new, li.late').length, 0);
});

test('rows sit on the board grid with 12 px crests, and nothing in the list takes focus', () => {
  const root = draw(groundsAt(day([]), 10 * M));
  const source = fs.readFileSync(path.join(REPO_ROOT, 'viewer/src/components/OtherGrounds.svelte'), 'utf8');
  assert.match(source, /grid-template-columns: 1fr 44px 1fr 30px;/);
  assert.match(source, /height: 20px;/);
  for (const svg of root.querySelectorAll('svg.crest')) {
    assert.equal(svg.getAttribute('width'), '12');
    assert.equal(svg.getAttribute('height'), '13');
    assert.equal(svg.getAttribute('aria-hidden'), 'true');
  }
  assert.equal(root.querySelectorAll('svg.crest').length, 6);
  assert.equal(root.querySelector('a[href], button, input, select, textarea, [tabindex]'), null);
});

test('the outline appears in 200 ms and has no animation under reduced motion', () => {
  const source = fs.readFileSync(path.join(REPO_ROOT, 'viewer/src/components/OtherGrounds.svelte'), 'utf8');
  assert.match(source, /animation: arrive 200ms var\(--ease\) both;/);
  assert.match(source, /:global\(:root\[data-motion='reduce'\]\) \.block \{\s*animation: none;/);
});

test('the skeleton draws three rows and the label while loading', () => {
  const root = draw(groundsAt(null, 0), { skeleton: true });
  assert.equal(root.querySelectorAll('.skel').length, 3);
  assert.match(text(root), /Other grounds/);
});
