// @vitest-environment jsdom
// The player panel on a protocol 7 hello, in jsdom: every section shows (the positions pitch,
// the feet, the personality summary with the assistant view and the mood, the state rows, the
// season block from the views answer, the traits); no personality value, mood or form shows as
// a digit, a band colour or a bar, in text or in an aria label, and the old "Thirteen
// personality attributes" copy is gone; the attribute hover opens on pointer and on focus with
// the same rows, lists no minor weight, and closes on Escape with the row keeping focus; an
// out-of-position starter reads it on the familiarity row; every stub stays inert.

import assert from 'node:assert/strict';
import { tick } from 'svelte';
import { test } from 'vitest';

import { actionsOf } from '../../src/lib/attribute-actions.js';
import { PERSONALITY_NAMES } from '../../src/lib/hidden-words.js';
import { HELLO, opened, stubFaults, useFakes } from './harness.js';

useFakes();

const ATTRIBUTES = { crossing: 152, technique: 133, passing: 121, decisions: 98, pace: 150, stamina: 160 };
const word = (keys, confidence) =>
  Object.fromEntries(PERSONALITY_NAMES.map((n) => [n, { word: keys[n] ?? 'middling', confidence }]));

/// The harness hello as protocol 7. Player 2 (index 1) is known: 31 matches here, driven and
/// calm, three traits. Player 10 (index 9, in the ST slot of the 4-4-2) is an AM rated 20
/// there and 13 at ST. Player 12 (index 11) is new, with no traits.
const HELLO7 = {
  ...HELLO,
  'protocol.version': 7,
  teams: [
    {
      ...HELLO.teams[0],
      squad: HELLO.teams[0].squad.map((p, i) => ({
        ...p,
        'player.position': i === 9 ? 'AM' : p['player.position'],
        'player.attributes': ATTRIBUTES,
        'player.condition': 100,
        'player.matches_at_club': i === 11 ? 0 : 31,
        'player.level': 140,
        'player.plays_between': [126, 140],
        'player.positions': { GK: 10, CB: 60, LB: 70, RB: 70, DM: 110, CM: 150, AM: 200, LW: 120, RW: 140, ST: 130 },
        'player.strong_foot': 'right',
        'player.weaker_foot': 64,
        ...(i === 11 ? {} : {
          'player.traits': [
            { name: 'tries_tricks', strength: 'moderate' },
            { name: 'tries_killer_balls', strength: 'strong' },
            { name: 'switches_play', strength: 'strong' },
          ],
        }),
        'player.personality':
          i === 11
            ? word({}, 'not_yet_known')
            : word({ professionalism: 'high', ambition: 'high', temperament: 'high' }, 'firm'),
        'player.mood': i === 11 ? 'steady' : 'lifted',
      })),
    },
    HELLO.teams[1],
  ],
};

const SEASON = {
  matches: ['m-1', 'm-2'],
  players: {
    'p-0-2': {
      starts: 2,
      minutes: 180,
      goals: 1,
      assists: 2,
      clean_sheets: 0,
      red_cards: 0,
      ratings: [{ match: 'm-1', rating: 7.1 }, { match: 'm-2', rating: 7.6 }],
      goal: { kind: 'assists', target: 10 },
      form: 7.4,
      progress: 190,
    },
    'p-0-12': {
      starts: 0,
      minutes: 0,
      goals: 0,
      assists: 0,
      clean_sheets: 0,
      red_cards: 0,
      ratings: [],
      goal: { kind: 'starts', target: 10 },
      form: null,
      progress: null,
    },
  },
};

const settle = async () => {
  for (let i = 0; i < 6; i += 1) {
    await tick();
    await Promise.resolve();
  }
};

/// Opens squad player `index`'s panel with a views answer that carries the season.
async function openPanel(index) {
  const opening = await opened(HELLO7);
  opening.s.viewsAdapter = {
    kind: 'fake',
    read: async () => ({ views: [], ratings: {}, season: SEASON }),
    save: async () => true,
  };
  opening.s.openPlayer(index);
  await settle();
  const panel = document.querySelector('[data-screen="player"]');
  assert.ok(panel, 'the panel opens');
  return { ...opening, panel };
}

/// Every text and aria label under `el`, the screen reader's hidden text included.
function said(el) {
  const labels = [el, ...el.querySelectorAll('*')].map((n) => n.getAttribute?.('aria-label') ?? '').join(' ');
  return `${el.textContent} ${labels}`;
}

const BANDS = ['v1', 'v2', 'v3', 'v4', 'rt', 'bar', 'range'];

/// A pointer event of `type` from a `pointerType` device (jsdom may lack PointerEvent).
function pointer(type, pointerType) {
  const event = new Event(type);
  Object.defineProperty(event, 'pointerType', { value: pointerType });
  return event;
}

test('a known player shows every section as the boards draw it', async () => {
  const { panel } = await openPanel(1);
  assert.equal(panel.querySelectorAll('[data-chip]').length, 10);
  assert.equal(panel.querySelector('[data-chip="AM"]').dataset.natural, undefined, 'player 2 is an LB');
  assert.equal(panel.querySelector('[data-chip="LB"]').dataset.natural, 'true');
  assert.match(panel.querySelector('[data-pitch-list]').textContent, /GK not rated/);
  assert.deepEqual(
    [...panel.querySelectorAll('[data-foot] .kv')].map((f) => `${f.children[0].textContent} ${f.children[1].textContent}`),
    ['Right (strong) 100', 'Left · decent 64%']
  );
  assert.equal(panel.querySelector('[data-summary]').textContent.trim(), 'Driven');
  assert.equal(panel.querySelector('[data-sureness]').textContent.trim(), 'professional, ambitious, calm · sure · 31 matches here');
  assert.match(panel.querySelector('[data-assistant]').textContent, /Assistant manager:.*Model pro.*Chasing his assists target/);
  assert.match(panel.querySelector('[data-mood]').textContent.replace(/\s+/g, ' '), /Mood today: ▲ ?Lifted/);
  assert.deepEqual(
    [...panel.querySelectorAll('[data-trait]')].map((t) => `${t.children[0].textContent} ${t.children[1].textContent}`),
    ['Tries killer balls Strong', 'Switches play Strong', 'Tries tricks Moderate']
  );
  assert.equal(panel.querySelector('[data-season]').dataset.season, 'some');
  assert.equal(panel.querySelector('[data-season-line]').dataset.seasonLine, '2');
  assert.match(panel.querySelector('[data-season-row="goal"]').textContent, /Make 10 assists · 2 of 10, ahead/);
  assert.match(panel.querySelector('[data-season-row="form"]').textContent, /In form/);
  assert.match(panel.querySelector('[data-state="familiarity"]').textContent, /Natural position/);
  assert.match(panel.querySelector('[data-state="surroundings"]').textContent, /No effect today/);
  assert.deepEqual(stubFaults(panel.closest('.app')), []);
});

for (const index of [1, 11]) {
  test(`player ${index + 1}: no personality, mood or form as a number, a band colour or a bar; the old copy is gone`, async () => {
    const { panel } = await openPanel(index);
    const guarded = [
      panel.querySelector('[data-summary]'),
      panel.querySelector('[data-assistant]'),
      panel.querySelector('[data-mood]'),
      panel.querySelector('[data-state="mind"]'),
      panel.querySelector('[data-season-row="form"]'),
    ];
    for (const el of guarded) {
      assert.ok(el, 'each guarded part shows');
      assert.ok(!/\d/.test(said(el)), `${index}: ${said(el)}`);
      for (const node of [el, ...el.querySelectorAll('*')]) {
        for (const band of BANDS) {
          assert.ok(!node.classList?.contains(band), `${index}: ${band} on ${node.outerHTML}`);
        }
      }
    }
    // The personality block lists no value: no row per value, no number but the matches here.
    const block = panel.querySelector('[data-section="personality"]');
    const digits = block.textContent.replace(panel.querySelector('[data-sureness]').textContent, '');
    assert.ok(!/\d/.test(digits), digits);
    assert.ok(!/Thirteen personality attributes/.test(panel.closest('.app').textContent));
  });
}

test('a new player reads not yet known, too early to say and an empty season, with no traits', async () => {
  const { panel } = await openPanel(11);
  assert.equal(panel.querySelector('[data-summary]').textContent.trim(), 'Not yet known');
  assert.equal(panel.querySelector('[data-sureness]').textContent.trim(), 'no match for us yet');
  assert.match(panel.querySelector('[data-assistant]').textContent, /Too early to say what he is like\./);
  assert.match(panel.querySelector('[data-mood]').textContent, /Settling in/);
  assert.equal(panel.querySelector('[data-traits]').textContent.trim(), 'No traits of play.');
  assert.equal(panel.querySelector('[data-season]').dataset.season, 'empty');
  assert.match(panel.querySelector('[data-season-empty]').textContent, /No matches this season yet\./);
  assert.match(panel.querySelector('[data-season-row="goal"]').textContent, /Start 10 matches · 0 of 10/);
});

test('an AM in the ST slot reads out of position on the familiarity row', async () => {
  const { panel } = await openPanel(9);
  const row = panel.querySelector('[data-state="familiarity"]');
  assert.equal(row.dataset.out, 'true');
  assert.match(row.textContent, /Out of position at ST — costs him some/);
});

test('the hover lists the same actions on pointer and on focus, no minor weight, and Escape closes it', async () => {
  const { panel } = await openPanel(1);
  const row = panel.querySelector('[data-attr="crossing"]');
  // The attributes take one place in the Tab order: the first row; the arrow keys move it.
  const attrs = [...panel.querySelectorAll('[data-attr]')];
  assert.deepEqual(
    attrs.filter((el) => el.getAttribute('tabindex') === '0'),
    [attrs[0]]
  );
  attrs[0].focus();
  await settle();
  attrs[0].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
  await settle();
  assert.equal(document.activeElement, attrs[1], 'ArrowDown moves to the next row');
  assert.equal(attrs[1].getAttribute('tabindex'), '0');
  assert.equal(attrs[0].getAttribute('tabindex'), '-1');
  attrs[1].blur();
  await settle();
  const expected = actionsOf('crossing').map((a) => `${a.words}${a.role}`);
  const rows = () => [...panel.querySelectorAll('[data-hover="crossing"] li')].map((li) => li.textContent.trim());
  row.dispatchEvent(pointer('pointerenter', 'mouse'));
  await settle();
  const pop = panel.querySelector('[data-hover="crossing"]');
  assert.ok(pop, 'the pointer opens it');
  assert.equal(pop.getAttribute('role'), 'tooltip');
  assert.equal(row.getAttribute('aria-describedby'), pop.id);
  assert.match(pop.textContent, /Crossing drives/);
  const byPointer = rows();
  assert.deepEqual(byPointer, expected);
  assert.ok(!byPointer.some((r) => /Penalty/.test(r)));
  assert.equal(pop.querySelector('button, a, input, [tabindex]'), null, 'it holds no control');
  row.dispatchEvent(pointer('pointerleave', 'mouse'));
  await settle();
  assert.equal(panel.querySelector('[data-hover]'), null, 'leaving closes it');
  row.focus();
  await settle();
  assert.deepEqual(rows(), byPointer, 'focus opens the same list');
  row.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  await settle();
  assert.equal(panel.querySelector('[data-hover]'), null, 'Escape closes it');
  assert.equal(document.activeElement, row, 'the row keeps focus');
  // A touch does not open it.
  row.blur();
  row.dispatchEvent(pointer('pointerenter', 'touch'));
  await settle();
  assert.equal(panel.querySelector('[data-hover]'), null);
});
