// @vitest-environment jsdom
// The report, in jsdom: the half-time report has CONTINUE as its one action and no full-time
// actions; the full-time report is loading, with Save replay disabled and its reason, until
// the whole match is stored; then its figures equal the report model, its goals and cards
// are listed with their words, and its actions call the session; every stub is inert and
// hidden, and a planted focusable stub fails the check. After a skip the report first hides
// the scores while the engine plays the rest, then carries the skip marks; a report with no
// skip has none of them.

import assert from 'node:assert/strict';
import { tick } from 'svelte';
import { test } from 'vitest';

import { reportModel } from '../../src/lib/report.js';
import { eventMessage } from '../helpers.js';
import { opened, stubFaults, useFakes } from './harness.js';

useFakes();

const page = () => document.querySelector('[data-screen="report"]')?.closest('.app') ?? null;
const button = (text) =>
  [...page().querySelectorAll('button')].find((b) => b.textContent.trim() === text) ?? null;

/// A session with a goal each way, a card, half time and full time in its event list.
async function played() {
  const run = await opened();
  const score = (h, a) => ({ 'home.score': h, 'away.score': a });
  for (const message of [
    eventMessage(150, 'goal', { 'team.id': 'a', 'player.id': 'p-0-10', ...score(1, 0) }),
    eventMessage(400, 'half-time', score(1, 0)),
    eventMessage(600, 'card', { 'team.id': 'b', 'card.kind': 'yellow', ...score(1, 0) }),
    eventMessage(700, 'goal', { 'team.id': 'b', ...score(1, 1) }),
    eventMessage(800, 'full-time', score(1, 1)),
  ]) {
    run.socket.deliver(message);
  }
  await tick();
  return run;
}

test('the half-time report has CONTINUE as its one action, and CONTINUE goes back', async () => {
  const { s } = await played();
  const from = s.view;
  s.openReport('half-time', 400);
  await tick();
  const root = page();
  assert.equal(root.querySelector('[data-screen="report"]').dataset.kind, 'half-time');
  assert.match(root.querySelector('.hd').textContent, /Half-time/);
  assert.equal(button('Replay the whole match'), null, 'no full-time actions at half time');
  assert.equal(button('Save replay'), null);
  assert.equal(root.querySelector('.cont').textContent.trim(), 'Continue');
  root.querySelector('.cont').click();
  await tick();
  assert.equal(s.view, from, 'back to the view the report opened over');
  assert.equal(page(), null);
});

test('the full-time report is loading until the match is stored, with Save replay waiting', async () => {
  const { s } = await played();
  s.openReport('full-time', 800);
  await tick();
  const root = page();
  assert.equal(root.querySelector('[data-screen="report"]').dataset.state, 'loading');
  const steps = [...root.querySelectorAll('.steps li')].map((li) => li.textContent);
  assert.match(steps[0], /Write the report/);
  assert.match(steps[1], /Store the whole match/);
  assert.equal(button('Save replay').disabled, true);
  assert.match(root.textContent, /Nothing is stored yet\.|The whole match is still being stored\./);
  assert.equal(root.querySelector('.timeline'), null, 'no figures while loading');
});

test("the ready report's figures equal the report model, and its moments are words", async () => {
  const { s } = await played();
  s.streamEnded = true;
  s.openReport('full-time', 800);
  await tick();
  const root = page();
  assert.equal(root.querySelector('[data-screen="report"]').dataset.state, 'ready');
  const model = reportModel(s.match.events, 800, s.teams);
  const bars = [...root.querySelectorAll('.pair .row')].map((row) =>
    [...row.children].map((c) => c.textContent.trim())
  );
  assert.deepEqual(
    bars,
    model.rows.map((r) => [String(r.counts[0]), r.label, String(r.counts[1])])
  );
  const moments = [...root.querySelectorAll('.moments li')].map((li) => li.textContent.replace(/\s+/g, ' ').trim());
  assert.equal(moments.length, 3);
  assert.match(moments[0], /^0' Goal 0-10 · Ashford Rovers$/, 'the scorer from the roster, then the club');
  assert.match(moments[1], /Yellow card Port Varrow$/);
  assert.match(moments[2], /Goal Port Varrow$/);
  assert.match(root.querySelector('.timeline').getAttribute('aria-label'), /Goal at/);
  assert.match(root.querySelector('.strip').textContent, /1 – 1/);
});

test('the full-time actions call the session: Replay the whole match, Close', async () => {
  const { s } = await played();
  s.streamEnded = true;
  const from = s.view;
  s.openReport('full-time', 800);
  await tick();
  assert.ok(button('Open a replay'));
  button('Close').click();
  await tick();
  assert.equal(s.view, from, 'Close goes back to the view the report opened over');
  s.openReport('full-time', 800);
  await tick();
  let called = 0;
  s.replayWhole = () => (called += 1);
  button('Replay the whole match').click();
  assert.equal(called, 1);
});

test('every report stub is inert and hidden, and a planted focusable stub fails the check', async () => {
  const { s } = await played();
  s.streamEnded = true;
  s.openReport('full-time', 800);
  await tick();
  const root = page();
  for (const note of ['highlights', 'what each change did', 'what it means', 'other grounds', 'tab: ratings', 'tab: press']) {
    assert.ok(root.querySelector(`[data-stub="${note}"]`), note);
  }
  assert.equal(root.querySelector('[data-stub="tab: replay"]'), null, 'Replay is live at full time');
  assert.deepEqual(stubFaults(root), []);
  root.querySelector('[data-stub="highlights"]').append(document.createElement('button'));
  assert.deepEqual(stubFaults(root), ['highlights: button takes focus']);
});

/// The played session's report after a skip at tick 500, in `state`.
async function skippedReport(state, tickAt = 800) {
  const run = await played();
  run.s.skip = { state: state === 'ready' ? 'ready' : 'playing', from: 500, newest: 650 };
  run.s.report = {
    kind: 'full-time',
    tick: state === 'ready' ? tickAt : null,
    state,
    model: reportModel(run.s.match.events, state === 'ready' ? tickAt : 500, run.s.teams),
    skippedFrom: 500,
  };
  run.s.view = 'report';
  await tick();
  return run;
}

test('while the engine plays the rest the scores are hidden and four steps show the minute', async () => {
  await skippedReport('playing-rest');
  const root = page();
  assert.equal(root.querySelector('[data-screen="report"]').dataset.state, 'playing-rest');
  assert.match(root.querySelector('.hd').textContent, /Skip to result/);
  assert.match(root.querySelector('.hd').textContent, /Ashford Rovers v Port Varrow · the engine plays the rest/);
  assert.equal(root.querySelector('.cont').disabled, true, 'Please wait is busy');
  const strip = root.querySelector('.strip');
  assert.equal(strip.querySelector('.score').textContent.trim(), '– –');
  assert.equal(strip.querySelector('.score').getAttribute('aria-label'), 'Score hidden until full time');
  assert.equal((strip.textContent.match(/Scores hidden until full time/g) ?? []).length, 2);
  assert.ok(!/1 – 1/.test(strip.textContent), 'no score leaks');
  assert.match(strip.textContent, /PLAYING THE REST · 0'/);
  assert.match(strip.textContent, /00:10\s*Skipped at/);
  assert.match(strip.textContent, /0' of 90\s*Engine at full speed/);
  const steps = [...root.querySelectorAll('.steps li')].map((li) => li.textContent.replace(/\s+/g, ' ').trim());
  assert.equal(steps.length, 4);
  assert.match(steps[0], /Freeze the match at 00:10/);
  assert.match(steps[1], /Play 00:10 to full time.*0' of 90 · at full speed/);
  assert.match(root.textContent, /No cancel: the match ends the same way either way/);
  assert.equal(root.querySelector('.timeline'), null);
  assert.equal(root.querySelector('[data-stub="tab: replay"]') !== null, true, 'Replay waits');
});

test('the ready report after a skip carries the skip marks, and NOT LIVE only after the skip', async () => {
  await skippedReport('ready');
  const root = page();
  assert.equal(root.querySelector('[data-screen="report"]').dataset.skipped, '500');
  assert.match(root.querySelector('.hd').textContent, /Full time · skipped from 00:10/);
  assert.match(root.querySelector('.strip').textContent, /Skipped at 00:10\s*The engine played the rest/);
  assert.ok(root.querySelector('[data-stub="strip fact: Other grounds"]'));
  assert.match(root.textContent, /the rest played by the engine from the exact moment you skipped/);
  const timeline = root.querySelector('.timeline');
  assert.ok(timeline.querySelector('pattern#skip-hatch'));
  assert.ok(timeline.querySelector('.hatched'));
  assert.match(timeline.textContent, /SKIPPED AT 00:10 · NOT WATCHED LIVE →/);
  assert.match(timeline.getAttribute('aria-label'), /you watched 0 to 00:10 live; the engine played 00:10 to full time/);
  assert.equal(timeline.querySelectorAll('circle.hollow').length, 2, 'the card at 600 and the goal at 700');
  const rows = [...root.querySelectorAll('.moments li')].map((li) => li.textContent.replace(/\s+/g, ' ').trim());
  assert.deepEqual(
    rows.map((r) => r.endsWith('NOT LIVE')),
    [false, true, true],
    'the goal at 150 was watched; the card and goal after 500 were not'
  );
  assert.match(root.textContent, /Moments marked NOT LIVE happened after you skipped\. The replay shows them\./);
  assert.deepEqual(stubFaults(root), []);
});

test('a report with no skip has none of the skip marks', async () => {
  const { s } = await played();
  s.streamEnded = true;
  s.openReport('full-time', 800);
  await tick();
  const root = page();
  assert.equal(root.querySelector('[data-screen="report"]').dataset.skipped, undefined);
  assert.equal(root.querySelector('pattern'), null);
  assert.equal(root.querySelector('.hollow'), null);
  assert.ok(!/NOT LIVE|skipped from|Skipped at/.test(root.textContent));
  assert.match(root.querySelector('.strip').textContent, /Possession/);
});
