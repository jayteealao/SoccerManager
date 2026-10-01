// Match day around the pitch: the goal moment, the empty feed before play, the condition
// words, and the goal moment with reduced motion.
import path from 'node:path';
import { expect, test } from '@playwright/test';
import { WEB, runEngine, startEngine, tempDir } from '../support/engine.mjs';
import { kickOff, openMatch, playUntil, scoreBug, setSpeed, showMatch, until } from '../support/page.mjs';

let fixture;

// Seed 14 scores early in the second half of a ten-minute match (tick 21,930).
test.beforeAll(() => {
  const dir = tempDir('fixture');
  fixture = path.join(dir, 'match.smfx');
  const run = runEngine(['record', '--seed', '14', '--minutes', '10', '--out', fixture], { dataDir: dir });
  expect(run.code, run.stderr).toBe(0);
});

// The half-time report pauses play until CONTINUE; playUntil presses it.
const firstGoalMoment = (page) =>
  playUntil(page, () => window.__touchline.signals().find((s) => s.signal === 'viewer.goal_moment') ?? null, {
    timeout: 120_000,
  });

test('the goal moment shows in the frame that draws the goal', async ({ page }) => {
  const engine = await startEngine({ command: 'replay', args: ['--fixture', fixture, '--speed', '8', '--web', WEB] });
  try {
    await openMatch(page, engine.url);
    await setSpeed(page, 8);
    const moment = await firstGoalMoment(page);
    const day = await page.evaluate(() => window.__touchline.matchDay());
    expect(moment.frame_delta).toBe(0);
    expect(moment.prev_rendered_tick).toBeLessThan(moment.goal_tick);
    expect(moment.rendered_tick).toBeGreaterThanOrEqual(moment.goal_tick);
    expect(day.goalShownAtTick).toBe(moment.rendered_tick);
    await expect(scoreBug(page)).toHaveAttribute(
      'aria-label',
      `Score ${moment['home.score']} – ${moment['away.score']}`
    );
    expect(day.bannerText.length).toBeGreaterThan(0);
    expect(await page.locator('section[aria-label="Commentary"] li[data-kind="goal"]').count()).toBeGreaterThanOrEqual(1);
  } finally {
    engine.cleanUp();
  }
});

test('before play the feed shows its empty text and the score is 0–0', async ({ page }) => {
  const engine = await startEngine({ command: 'serve', args: ['--seed', '3', '--minutes', '10', '--web', WEB] });
  try {
    await openMatch(page, engine.url);
    await until(page, () => window.__touchline.lineup().phase === 'pre-match', { timeout: 30_000 });
    // The viewer opens on Tactics before kick-off; the feed is on the match screen.
    await showMatch(page);
    await expect(page.getByText('No events yet. The feed fills as the match plays.')).toBeVisible();
    await expect(scoreBug(page)).toHaveAttribute('aria-label', 'Score 0 – 0');
    const day = await page.evaluate(() => window.__touchline.matchDay());
    expect(day.emptyStateShown).toBe(true);
    expect(day.feedCount).toBe(0);
  } finally {
    engine.cleanUp();
  }
});

test('every player on the pitch carries a condition word', async ({ page }) => {
  const engine = await startEngine({ command: 'serve', args: ['--seed', '3', '--minutes', '10', '--web', WEB] });
  try {
    await openMatch(page, engine.url);
    await kickOff(page);
    await setSpeed(page, 8);
    // Condition messages arrive once play has run for a while.
    await until(page, () => window.__touchline.matchDay().energyTick > 0, { timeout: 120_000 });
    const labels = await page.evaluate(() => window.__touchline.matchDay().lineupLabels);
    expect(labels).toHaveLength(22);
    const words = ['Fresh', 'Tiring', 'Exhausted', 'Injured', 'Sent off'];
    for (const label of labels) {
      expect(words).toContain(label.condition);
    }
    // The viewer lists no line-ups under the pitch; the word each player carries is the
    // hook's, from the same condition model the Touchline's player state reads.
    const shown = labels.map((l) => l.condition);
    expect(shown.filter((w) => words.includes(w.trim()))).toHaveLength(22);
  } finally {
    engine.cleanUp();
  }
});

test('with reduced motion the goal banner does not animate and the score does not pulse', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const engine = await startEngine({ command: 'replay', args: ['--fixture', fixture, '--speed', '8', '--web', WEB] });
  try {
    await openMatch(page, engine.url);
    await expect(page.locator('html')).toHaveAttribute('data-motion', 'reduce');
    await setSpeed(page, 8);
    await firstGoalMoment(page);
    const state = await page.evaluate(() => {
      const banner = document.querySelector('.pitchbox .banner');
      const bug = [...document.querySelectorAll('.strip .score')].find((b) => b.checkVisibility());
      return {
        banner: banner ? 'true' : 'false',
        animations: banner ? banner.getAnimations({ subtree: true }).length : -1,
        pulse: getComputedStyle(bug).animationName === 'none' ? 'off' : 'on',
        bugAnimations: bug.getAnimations().length,
      };
    });
    expect(state.banner).toBe('true');
    expect(state.animations).toBe(0);
    expect(state.pulse).toBe('off');
    expect(state.bugAnimations).toBe(0);
  } finally {
    engine.cleanUp();
  }
});
