// Drives the match viewer. Controls are found by their role and accessible name; state is read
// through the page's read-only hook, `window.__touchline`. The viewer mounts one screen per
// view (Tactics, the Pre-match line-ups, the match, the Touchline, the report, the replay) and
// keeps the match screen mounted but hidden behind the others, so a control is always looked
// for on the screen on show.
import { expect } from '@playwright/test';

/// The screenshot-or-layout call and the pointer helpers for every window size.
export { centre, clearPointer, clickClear, snap } from './snap.mjs';

/// Opens the page and records the `hello` the page received, read off its own socket.
export async function openMatch(page, url) {
  const seen = { hello: null };
  page.on('websocket', (socket) => {
    socket.on('framereceived', ({ payload }) => {
      if (typeof payload !== 'string' || seen.hello) {
        return;
      }
      try {
        const message = JSON.parse(payload);
        if (message.type === 'hello') {
          seen.hello = message;
        }
      } catch {
        // Not JSON; not the hello.
      }
    });
  });
  await page.goto(url);
  await page.waitForFunction(() => Boolean(window.__touchline));
  return seen;
}

/// Evaluates `fn` against the hook.
export function hook(page, fn, arg) {
  return page.evaluate(fn, arg);
}

/// Waits until `fn` returns a truthy value in the page, and returns it.
export async function until(page, fn, { timeout = 60_000, arg } = {}) {
  const handle = await page.waitForFunction(fn, arg, { timeout, polling: 100 });
  return handle.jsonValue();
}

/// The cyan action block of the screen on show. Before kick-off, on Tactics, it reads
/// CONTINUE and the lineup gates it: it is the control that leads to kick-off.
export const kickOffButton = (page) => page.locator('header button.cont:visible');

/// The sub-navigation tab `name` of the screen on show.
export const tab = (page, name) => page.locator('nav.subnav:visible').getByRole('button', { name, exact: true });

/// Kicks off from Tactics: CONTINUE opens the Pre-match line-ups, whose KICK OFF starts the
/// match.
export async function kickOffFromPage(page, timeout = 30_000) {
  const next = page.getByRole('button', { name: 'Continue', exact: true });
  const kick = page.getByRole('button', { name: 'Kick off', exact: true });
  await expect(next.or(kick).first()).toBeVisible({ timeout });
  if (await next.isVisible()) {
    await next.click();
  }
  await expect(kick).toBeEnabled({ timeout });
  await kick.click();
}

/// Waits for the lineup editor, then kicks off with the lineup it shows.
export async function kickOff(page) {
  await until(page, () => window.__touchline.lineup().phase === 'pre-match', { timeout: 30_000 });
  await expect(kickOffButton(page)).toBeEnabled();
  await kickOffFromPage(page);
  await until(page, () => window.__touchline.lineup().phase === 'live', { timeout: 15_000 });
}

/// Brings the match screen to the front when another view is open over it.
export async function showMatch(page) {
  const view = await page.evaluate(() => window.__touchline.view());
  if (view !== 'match' && view !== 'replay') {
    await tab(page, 'Match').click();
    await until(page, () => window.__touchline.view() === 'match', { timeout: 5_000 });
  }
}

/// Opens the Tactics view over the match.
export async function showTactics(page) {
  const view = await page.evaluate(() => window.__touchline.view());
  if (view !== 'tactics') {
    await tab(page, 'Tactics').click();
    await until(page, () => window.__touchline.view() === 'tactics', { timeout: 5_000 });
  }
}

/// The playback row of the screen on show.
export const playback = (page) => page.getByRole('group', { name: 'Playback' });

/// Presses one of the playback speed buttons: 1, 2, 4, or 8.
export async function setSpeed(page, speed) {
  await showMatch(page);
  await playback(page).getByRole('button', { name: `${speed}x`, exact: true }).click();
}

/// The speed button in effect, whose name is the speed asked for (`4x`).
export const speedPressed = (page) => playback(page).getByRole('button', { pressed: true });

/// Pauses or plays with the playback row's own button.
export async function pause(page) {
  await showMatch(page);
  await playback(page).getByRole('button', { name: 'Pause', exact: true }).click();
}

export async function play(page) {
  await showMatch(page);
  await playback(page).getByRole('button', { name: 'Play', exact: true }).click();
}

/// The match clock the date block shows, on the screen on show.
export const matchClock = (page) => page.locator('header .date span:visible');

/// The score the score strip shows, named "Score h – a".
export const scoreBug = (page) => page.locator('.strip .score:visible');

export const renderedTick = (page) => page.evaluate(() => window.__touchline.lastRenderedTick());

/// Plays on until `fn` holds, pressing Continue on the half-time report when it opens.
export async function playUntil(page, fn, { timeout = 15 * 60_000, arg } = {}) {
  const end = Date.now() + timeout;
  for (;;) {
    const done = await page.evaluate(fn, arg);
    if (done) {
      return done;
    }
    const report = await page.evaluate(() => window.__touchline.report());
    if (report.open && report.kind === 'half-time') {
      await page.getByRole('button', { name: 'Continue', exact: true }).click();
    }
    if (Date.now() > end) {
      throw new Error(`playUntil timed out after ${timeout} ms`);
    }
    await page.waitForTimeout(250);
  }
}

/// The queued changes on the screen on show, as the manager reads them.
export function chips(page) {
  return page.evaluate(() =>
    [...document.querySelectorAll('[data-queue-id]')]
      .filter((c) => c.checkVisibility())
      .map((c) => ({
        state: c.dataset.state,
        word: c.querySelector('.tagc')?.textContent ?? '',
        label: c.querySelector('.what b')?.textContent ?? '',
        reason: c.querySelector('.what .g')?.textContent ?? null,
      }))
  );
}

/// The queued-change entries on the screen on show.
export const chipList = (page) => page.locator('[data-queue-id]:visible');

/// The text of every row in the commentary, oldest first. The commentary lists the newest
/// first, as the sketch does; it stays mounted behind the other views.
export function feedRows(page) {
  return page.evaluate(() =>
    [...document.querySelectorAll('section[aria-label="Commentary"] li')].reverse().map((li) => ({
      kind: li.dataset.kind,
      tick: Number(li.dataset.tick),
      text: li.innerText.replace(/\s+/g, ' ').trim(),
    }))
  );
}

/// Waits for a pending change with `queueId` to reach `states`, and returns it.
export function changeReaches(page, queueId, states, timeout = 180_000) {
  return playUntil(
    page,
    ({ id, states }) => {
      const chip = window.__touchline.pending().find((c) => c.queue_id === id);
      return chip && states.includes(chip.state) ? chip : null;
    },
    { arg: { id: queueId, states }, timeout }
  );
}

/// Queues a change through `act` and returns the new pending entry.
export async function queued(page, act) {
  const before = await page.evaluate(() => window.__touchline.pending().length);
  await act();
  return until(page, (n) => window.__touchline.pending()[n] ?? null, { arg: before, timeout: 10_000 });
}

/// Chooses option `index` of the select `locator` with the keyboard, as a manager would.
export async function chooseIndex(locator, index) {
  await locator.focus();
  for (let guard = 0; guard < 40; guard += 1) {
    const current = await locator.evaluate((s) => s.selectedIndex);
    if (current === index) {
      return;
    }
    await locator.press(current < index ? 'ArrowDown' : 'ArrowUp');
  }
  throw new Error(`could not choose option ${index}`);
}

/// The substitution picker's selects on Tactics.
export const comingOff = (page) => page.getByLabel('Coming off');
export const comingOn = (page) => page.getByLabel('Coming on');

/// Queues a substitution of the `off`th player on the pitch for the `on`th substitute, from
/// the Tactics view.
export async function queueSubstitution(page, off, on) {
  await showTactics(page);
  await chooseIndex(comingOff(page), off);
  await chooseIndex(comingOn(page), on);
  return queued(page, () => page.getByRole('button', { name: 'Queue substitution' }).click());
}

/// Moves the scrubber to `tick` the way a drag does: input events, then one change.
export async function scrubTo(page, tick) {
  await showMatch(page);
  await page.getByRole('slider', { name: 'Rewind to a tick' }).evaluate((scrub, value) => {
    scrub.value = String(value);
    scrub.dispatchEvent(new Event('input', { bubbles: true }));
    scrub.dispatchEvent(new Event('change', { bubbles: true }));
  }, tick);
}
