// Drives the match page. Controls are found by their role and accessible name; state is read
// through the page's read-only hook, `window.__touchline`. A `data-testid` is used only where
// the page gives a control no unique accessible name.
import { expect } from '@playwright/test';

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

export const kickOffButton = (page) => page.getByRole('button', { name: 'Kick off' });

/// Waits for the lineup editor, then kicks off with the lineup it shows.
export async function kickOff(page) {
  await until(page, () => window.__touchline.lineup().phase === 'pre-match', { timeout: 30_000 });
  await expect(kickOffButton(page)).toBeEnabled();
  await kickOffButton(page).click();
  await until(page, () => window.__touchline.lineup().phase === 'live', { timeout: 15_000 });
}

/// Presses one of the playback speed buttons: 1, 2, 4, or 8.
export async function setSpeed(page, speed) {
  await page.getByRole('group', { name: 'Playback' }).getByRole('button', { name: `${speed}x`, exact: true }).click();
}

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
      await page.getByRole('button', { name: 'Continue' }).click();
    }
    if (Date.now() > end) {
      throw new Error(`playUntil timed out after ${timeout} ms`);
    }
    await page.waitForTimeout(250);
  }
}

/// The chips in the tactics panel, as the manager reads them.
export function chips(page) {
  return page.evaluate(() =>
    [...document.querySelectorAll('#chips .chip')].map((c) => ({
      state: c.dataset.state,
      word: c.querySelector('.chip__word')?.textContent ?? '',
      label: c.querySelector('.chip__label')?.textContent ?? '',
      reason: c.querySelector('.chip__reason')?.textContent ?? null,
    }))
  );
}

/// The text of every row in the match feed, oldest first.
export function feedRows(page) {
  return page.evaluate(() =>
    [...document.querySelectorAll('#feed li')].map((li) => ({
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

/// Queues a substitution of the `off`th player on the pitch for the `on`th substitute.
export async function queueSubstitution(page, off, on) {
  await chooseIndex(page.getByLabel('Player coming off'), off);
  await chooseIndex(page.getByLabel('Substitute coming on'), on);
  return queued(page, () => page.getByRole('button', { name: 'Queue substitution' }).click());
}

/// Moves the scrubber to `tick` the way a drag does: input events, then one change.
export async function scrubTo(page, tick) {
  await page.getByRole('slider', { name: 'Rewind to a tick' }).evaluate((scrub, value) => {
    scrub.value = String(value);
    scrub.dispatchEvent(new Event('input', { bubbles: true }));
    scrub.dispatchEvent(new Event('change', { bubbles: true }));
  }, tick);
}
