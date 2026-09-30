// The goal moment: inside 1.5 seconds of wall time, one banner at a time, and no motion
// when motion is reduced.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import {
  BANNER_TOTAL_MS,
  GOAL_HOLD_MS,
  GOAL_IN_MS,
  GOAL_OUT_MS,
  GoalMoment,
  PULSE_MS,
  bannerText,
  reducedMotion,
  watchMotion,
} from '../src/lib/goal-moment.js';
import { REPO_ROOT } from './helpers.js';

function token(css, name) {
  const found = css.match(new RegExp(`${name}:\\s*([0-9.]+)ms`));
  assert.ok(found, `tokens.css defines ${name}`);
  return Number(found[1]);
}

test('the timings equal the motion tokens and fit inside 1.5 seconds', () => {
  for (const skin of ['broadcast-blue', 'interim-light']) {
    const css = fs.readFileSync(
      path.join(REPO_ROOT, `viewer/src/skins/${skin}/tokens.css`),
      'utf8'
    );
    assert.equal(token(css, '--goal-in'), GOAL_IN_MS);
    assert.equal(token(css, '--goal-hold'), GOAL_HOLD_MS);
    assert.equal(token(css, '--goal-out'), GOAL_OUT_MS);
    assert.equal(token(css, '--pulse'), PULSE_MS);
  }
  assert.ok(BANNER_TOTAL_MS <= 1500, `${BANNER_TOTAL_MS} ms`);
});

test('the banner names the club, the score, and the minute', () => {
  const goal = { 'team.id': 'club-a', 'home.score': 1, 'away.score': 0, tick: 10 };
  assert.equal(
    bannerText(goal, new Map([['club-a', 'Oakmere Rangers']]), "23'"),
    "GOAL — Oakmere Rangers 1–0 23'"
  );
});

/// Timers that run only when the test says so, on a clock the test owns.
function fakeTimers() {
  let now = 0;
  let id = 0;
  const pending = new Map();
  return {
    setTimeout(fn, ms) {
      id += 1;
      pending.set(id, { at: now + ms, fn });
      return id;
    },
    clearTimeout(handle) {
      pending.delete(handle);
    },
    advance(ms) {
      const until = now + ms;
      for (;;) {
        const due = [...pending.entries()].filter(([, t]) => t.at <= until).sort((a, b) => a[1].at - b[1].at);
        if (due.length === 0) {
          break;
        }
        const [handle, timer] = due[0];
        pending.delete(handle);
        now = timer.at;
        timer.fn();
      }
      now = until;
    },
  };
}

function fakeNode() {
  const attributes = new Map();
  return {
    dataset: {},
    textContent: '',
    setAttribute: (k, v) => attributes.set(k, v),
    getAttribute: (k) => attributes.get(k),
  };
}

function fakeDoc(motion) {
  return { documentElement: { dataset: { motion } } };
}

test('the banner shows at once, leaves by 1.5 seconds, and the pulse runs', () => {
  const timers = fakeTimers();
  const nodes = { banner: fakeNode(), bannerText: fakeNode(), bug: fakeNode() };
  const moment = new GoalMoment(nodes, { doc: fakeDoc('full'), timers });
  moment.play({ tick: 900 }, 'GOAL');
  assert.equal(nodes.banner.dataset.shown, 'true');
  assert.equal(nodes.bug.dataset.pulse, 'on');
  assert.equal(moment.visible, true);
  timers.advance(GOAL_IN_MS + GOAL_HOLD_MS);
  assert.equal(nodes.banner.dataset.shown, 'false', 'leaving after the hold');
  timers.advance(GOAL_OUT_MS);
  assert.equal(moment.visible, false);
  assert.equal(nodes.bug.dataset.pulse, 'off');
});

test('reduced motion keeps the banner and drops the pulse', () => {
  const timers = fakeTimers();
  const nodes = { banner: fakeNode(), bannerText: fakeNode(), bug: fakeNode() };
  const doc = fakeDoc('reduce');
  assert.equal(reducedMotion(doc), true);
  const moment = new GoalMoment(nodes, { doc, timers });
  moment.play({ tick: 900 }, 'GOAL');
  assert.equal(nodes.banner.dataset.shown, 'true');
  assert.equal(nodes.bug.dataset.pulse, 'off');
});

test('a second goal replaces the first banner and restarts its time', () => {
  const timers = fakeTimers();
  const nodes = { banner: fakeNode(), bannerText: fakeNode(), bug: fakeNode() };
  const moment = new GoalMoment(nodes, { doc: fakeDoc('full'), timers });
  moment.play({ tick: 900 }, 'first');
  timers.advance(1000);
  moment.play({ tick: 950 }, 'second');
  assert.equal(nodes.bannerText.textContent, 'second');
  timers.advance(1000);
  assert.equal(nodes.banner.dataset.shown, 'true', 'the first banner timer was cleared');
  timers.advance(BANNER_TOTAL_MS);
  assert.equal(moment.visible, false);
});

test('the operating-system preference and ?motion=reduce set the same attribute', () => {
  const doc = { documentElement: { dataset: {} } };
  const query = { matches: true, addEventListener() {} };
  watchMotion(doc, { location: { search: '' }, matchMedia: () => query });
  assert.equal(doc.documentElement.dataset.motion, 'reduce');
  query.matches = false;
  watchMotion(doc, { location: { search: '?motion=reduce' }, matchMedia: () => query });
  assert.equal(doc.documentElement.dataset.motion, 'reduce');
  watchMotion(doc, { location: { search: '' }, matchMedia: () => query });
  assert.equal(doc.documentElement.dataset.motion, 'full');
});
