// The start-screen steps the browser specs share: open the page in a skin, leave the splash,
// go from the start screen through match setup to a live match, wait for every animation to
// settle, and take a fast-forwarded match to its full-time report. No step waits for
// real-time play.
import { expect } from '@playwright/test';

const hook = (page, fn, arg) => page.evaluate(fn, arg);
const until = (page, fn, arg, timeout = 120_000) => page.waitForFunction(fn, arg, { timeout, polling: 100 });

/// Opens the page and waits until its skin and fonts are in.
export async function open(page, url, skin = 'broadcast-blue') {
  await page.goto(url);
  await expect(page.locator('html')).toHaveAttribute('data-ready', skin, { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
}

/// Leaves the splash once the engine answered: Enter opens the start screen at once.
export async function pastSplash(page) {
  await until(page, () => window.__touchline.frontDoor().answered);
  await page.keyboard.press('Enter');
  await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 10_000);
}

/// From the start screen through match setup to a live match.
export async function kickOffFromStart(page) {
  await page.locator('[data-choice="new"]').click();
  await until(page, () => window.__touchline.frontDoor().round !== null, undefined, 10_000);
  await page.locator('[data-kickoff]').click();
  await until(page, () => window.__touchline.screen() === 'kickoff', undefined, 30_000);
  const action = page.locator('header button.cont:visible');
  if ((await hook(page, () => window.__touchline.view())) === 'tactics') {
    await expect(action).toHaveText('Continue');
    await action.click();
    await until(page, () => window.__touchline.view() === 'prematch', undefined, 5_000);
  }
  await expect(action).toHaveText('Kick off');
  await expect(action).toBeEnabled();
  await action.click();
  await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
}

/// Waits until every finite animation on the page has finished, so an overlay is captured
/// settled rather than part way through its fade.
export async function settled(page) {
  await page.evaluate(() =>
    Promise.all(
      document
        .getAnimations()
        .filter((a) => a.effect?.getComputedTiming().endTime !== Infinity)
        .map((a) => a.finished.catch(() => null))
    )
  );
}

/// Moves the visible timeline to `tick` the way a drag does, and waits until the pitch draws it.
export async function rewindTo(page, tick) {
  await page.getByRole('slider', { name: 'Rewind to a tick' }).filter({ visible: true }).evaluate((scrub, value) => {
    scrub.value = String(value);
    scrub.dispatchEvent(new Event('input', { bubbles: true }));
    scrub.dispatchEvent(new Event('change', { bubbles: true }));
  }, tick);
  await until(page, (t) => window.__touchline.lastRenderedTick() === t, tick, 10_000);
}

/// A fast-forwarded match, live on the match view, to its full-time report: once the engine
/// has sent the full-time whistle, the pitch jumps to a minute before it and plays at 8x until
/// the report opens. Returns the full-time event.
export async function toFullTime(page) {
  await until(page, () => window.__touchline.events().some((e) => e['event.type'] === 'full-time'), undefined, 180_000);
  const whistle = await hook(page, () => window.__touchline.events().find((e) => e['event.type'] === 'full-time'));
  await rewindTo(page, Math.max(0, whistle.tick - 3_000));
  const playback = page.getByRole('group', { name: 'Playback' }).filter({ visible: true });
  await playback.getByRole('button', { name: '8x', exact: true }).click();
  if (await playback.getByRole('button', { name: 'Play', exact: true }).isVisible()) {
    await playback.getByRole('button', { name: 'Play', exact: true }).click();
  }
  await until(
    page,
    () => {
      const report = window.__touchline.report();
      return report.open && report.kind === 'full-time';
    },
    undefined,
    60_000
  );
  return whistle;
}
