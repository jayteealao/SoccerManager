// @vitest-environment jsdom
// The Pre-match line-ups, in jsdom: CONTINUE on Tactics opens them; both sheets, the kick-off
// pitch and the rule-pack rows render from the hello and the lineup set on Tactics; the page
// sends nothing; "Change on Tactics" goes back; every stub is inert and hidden, and a planted
// focusable stub fails the check.

import assert from 'node:assert/strict';
import { tick } from 'svelte';
import { test } from 'vitest';

import { HELLO, SCHEMA, opened, stubFaults, useFakes } from './harness.js';

useFakes();

const page = () => document.querySelector('[data-screen="prematch"]')?.closest('.app') ?? null;
const action = () => page().querySelector('header button.cont');

/// The hello a page session sends: the other club names its formation.
const NAMED = {
  ...HELLO,
  teams: [HELLO.teams[0], { ...HELLO.teams[1], formation: SCHEMA.formations[0].name }],
};

async function prematch(hello = NAMED) {
  const run = await opened(hello);
  run.s.act();
  await tick();
  return run;
}

test('CONTINUE opens the Pre-match line-ups with both sheets, the pitch and the rule pack, and sends nothing', async () => {
  const { s, socket } = await prematch();
  assert.equal(s.view, 'prematch');
  const root = page();
  assert.ok(root, 'the Pre-match page is shown');
  assert.equal(action().textContent.trim(), 'Kick off');
  const sheets = [...root.querySelectorAll('section.sheet')];
  assert.deepEqual(
    sheets.map((el) => el.getAttribute('aria-label')),
    ['Ashford Rovers line-up', 'Port Varrow line-up']
  );
  assert.equal(sheets[0].querySelectorAll('tbody tr').length, 11);
  assert.equal(sheets[1].querySelectorAll('tbody tr').length, 11);
  assert.match(sheets[0].querySelector('tbody tr td.risk').textContent, /^(Low|Raised|High)$/);
  assert.equal(sheets[1].querySelector('td.risk'), null, 'the other club carries no figures');
  assert.equal(root.querySelectorAll('.pitchbox .dotp').length, 22);
  assert.equal(root.querySelectorAll('.pitchbox .dotp.away').length, 11);
  const rules = root.querySelector('section[aria-label="Rule pack"]').textContent;
  assert.match(rules, /5 substitutes in 3 windows/);
  assert.match(rules, /Level after 90/);
  assert.deepEqual(socket.sent.map((m) => m.type), [], 'nothing is sent from Pre-match');
});

test("without the other club's formation its dots are left off, never guessed", async () => {
  await prematch(HELLO);
  assert.equal(page().querySelectorAll('.pitchbox .dotp').length, 11);
  assert.equal(page().querySelectorAll('.pitchbox .dotp.away').length, 0);
});

test('"Change on Tactics" goes back with the lineup unchanged', async () => {
  const { s, socket } = await prematch();
  const before = s.dugout.lineupView();
  const back = [...page().querySelectorAll('button')].find((b) => b.textContent.trim() === 'Change on Tactics');
  back.click();
  await tick();
  assert.equal(s.view, 'tactics');
  assert.deepEqual(s.dugout.lineupView(), before);
  assert.deepEqual(socket.sent, []);
});

test('every Pre-match stub is inert and hidden, and every live control is named', async () => {
  await prematch();
  const root = page();
  for (const note of [
    'action note: Team talk',
    'strip fact: Weather',
    'condition and sharpness',
    'workload note',
    'later rules',
    'panel: Conditions',
    'panel: how much to watch',
    'tab: team-talk',
  ]) {
    assert.ok(root.querySelector(`[data-stub="${note}"]`), note);
  }
  assert.deepEqual(stubFaults(root), []);
  for (const control of root.querySelectorAll('button, select')) {
    if (control.closest('[data-stub]')) {
      continue;
    }
    const name = control.getAttribute('aria-label') || control.textContent.trim();
    assert.ok(name, control.outerHTML.slice(0, 120));
  }
});

test('the stub check fails when a Pre-match stub holds something that takes focus', async () => {
  await prematch();
  const root = page();
  root.querySelector('[data-stub="panel: Conditions"]').append(document.createElement('button'));
  assert.deepEqual(stubFaults(root), ['panel: Conditions: button takes focus']);
});
