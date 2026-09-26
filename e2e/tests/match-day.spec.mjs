// Match day around the pitch: the goal moment, the empty feed before play, the condition
// words, and the goal moment with reduced motion.
import path from 'node:path';
import { expect, test } from '@playwright/test';
import { runEngine, startEngine, tempDir } from '../support/engine.mjs';
import { kickOff, openMatch, setSpeed, until } from '../support/page.mjs';

let fixture;

// Seed 3 scores in the first minutes of a ten-minute match.
test.beforeAll(() => {
  const dir = tempDir('fixture');
  fixture = path.join(dir, 'match.smfx');
  const run = runEngine(['record', '--seed', '3', '--minutes', '10', '--out', fixture], { dataDir: dir });
  expect(run.code, run.stderr).toBe(0);
});

const firstGoalMoment = (page) =>
  until(page, () => window.__touchline.signals().find((s) => s.signal === 'viewer.goal_moment') ?? null, {
    timeout: 120_000,
  });

test('the goal moment shows in the frame that draws the goal', async ({ page }) => {
  const engine = await startEngine({ command: 'replay', args: ['--fixture', fixture, '--speed', '8', '--web', 'web'] });
  try {
    await openMatch(page, engine.url);
    await setSpeed(page, 8);
    const moment = await firstGoalMoment(page);
    const day = await page.evaluate(() => window.__touchline.matchDay());
    expect(moment.frame_delta).toBe(0);
    expect(moment.prev_rendered_tick).toBeLessThan(moment.goal_tick);
    expect(moment.rendered_tick).toBeGreaterThanOrEqual(moment.goal_tick);
    expect(day.goalShownAtTick).toBe(moment.rendered_tick);
    await expect(page.locator('#score-bug')).toHaveAttribute(
      'aria-label',
      `Score ${moment['home.score']} – ${moment['away.score']}`
    );
    expect(day.bannerText.length).toBeGreaterThan(0);
    expect(await page.locator('#feed li[data-kind="goal"]').count()).toBeGreaterThanOrEqual(1);
  } finally {
    engine.cleanUp();
  }
});

test('before play the feed shows its empty text and the score is 0–0', async ({ page }) => {
  const engine = await startEngine({ command: 'serve', args: ['--seed', '3', '--minutes', '10', '--web', 'web'] });
  try {
    await openMatch(page, engine.url);
    await until(page, () => window.__touchline.lineup().phase === 'pre-match', { timeout: 30_000 });
    await expect(page.getByText('No events yet. The feed fills as the match plays.')).toBeVisible();
    await expect(page.locator('#score-bug')).toHaveAttribute('aria-label', 'Score 0 – 0');
    const day = await page.evaluate(() => window.__touchline.matchDay());
    expect(day.emptyStateShown).toBe(true);
    expect(day.feedCount).toBe(0);
  } finally {
    engine.cleanUp();
  }
});

test('every player on the pitch carries a condition word', async ({ page }) => {
  const engine = await startEngine({ command: 'serve', args: ['--seed', '3', '--minutes', '10', '--web', 'web'] });
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
    const shown = await page.locator('#lineups .lineup__condition').allTextContents();
    expect(shown.filter((w) => words.includes(w.trim()))).toHaveLength(22);
  } finally {
    engine.cleanUp();
  }
});

test('with reduced motion the goal banner does not animate and the score does not pulse', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const engine = await startEngine({ command: 'replay', args: ['--fixture', fixture, '--speed', '8', '--web', 'web'] });
  try {
    await openMatch(page, engine.url);
    await expect(page.locator('html')).toHaveAttribute('data-motion', 'reduce');
    await setSpeed(page, 8);
    await firstGoalMoment(page);
    const state = await page.evaluate(() => ({
      banner: document.getElementById('goal-banner').dataset.shown,
      animations: document.getElementById('goal-banner').getAnimations({ subtree: true }).length,
      pulse: document.getElementById('score-bug').dataset.pulse,
      bugAnimations: document.getElementById('score-bug').getAnimations().length,
    }));
    expect(state.banner).toBe('true');
    expect(state.animations).toBe(0);
    expect(state.pulse).toBe('off');
    expect(state.bugAnimations).toBe(0);
  } finally {
    engine.cleanUp();
  }
});
