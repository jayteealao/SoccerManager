// After full time on the built viewer, served by the release program's `launch` (the start
// screen) or `serve` (a test page with no start screen).
//
// Screenshots, each in both skins at 1280 by 800, against their baselines, from matches of
// seed 7 between the two default clubs: the match screen at full time after a match played
// through, and after a skip at 20:00; the full-time report after the skip with its next
// steps; and the same report while the skipped match is still being stored (the engine's
// close never reaches the page). Every other ground is final in each. The light skin is
// chosen by configuration only (a content copy's slots.json).
//
// Drives: full time after a match played through, skipped, resumed and served with no start
// screen shows FULL TIME with the final clock and no live control; the path the person
// reported (skip, back to the match, then the action block) never shows LIVE or PAUSED again,
// which a recorder of every date word on show proves, with its own planted control; Return to
// start leaves the launcher idle with nothing to resume and the old worker gone; New match
// opens match setup with the two clubs that played and kicks off a fresh match; a Tab walk
// reaches every new control and no stub; and both screens meet WCAG AA contrast. No drive
// waits for real-time play: every match is fast-forwarded. Contrast is asserted in the default
// skin; the light skin's report is kept as evidence.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { contrastReport } from '../support/contrast.mjs';
import {
  VIEWER,
  contentWithSkin,
  fastForward,
  frontDoor,
  processAlive,
  startEngine,
  waitForGrounds,
} from '../support/engine.mjs';
import { kickOffFromStart, open, pastSplash, rewindTo, settled, toFullTime } from '../support/front.mjs';
import { clickClear, playUntil, snap } from '../support/page.mjs';

const SKINS = ['broadcast-blue', 'interim-light'];
const SEED = '7';
/// The skip point: 20:00, where the person skipped.
const SKIP_TICK = 60_000;
/// Past the end of any match: a match with this fast-forward plays straight through.
const PAST_THE_END = 1_000_000;
/// The controls of a live match, none of which may show after full time.
const LIVE_CONTROLS = ['Pause', 'Play', 'Resume', 'Back to live', 'Next stop', 'Skip to result', 'Previous stop'];

const light = {};

test.beforeAll(() => {
  light.dir = contentWithSkin('interim-light');
});

const evidence = (info, name) => {
  const out = process.env.MATCH_EVIDENCE_DIR
    ? path.join(process.env.MATCH_EVIDENCE_DIR, name)
    : info.outputPath(name);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  return out;
};

const hook = (page, fn, arg) => page.evaluate(fn, arg);
const until = (page, fn, arg, timeout = 120_000) => page.waitForFunction(fn, arg, { timeout, polling: 100 });
const action = (page) => page.locator('header button.cont:visible');
const button = (page, name) => page.getByRole('button', { name, exact: true }).filter({ visible: true });
const nextRow = (page, id) => page.getByRole('group', { name: 'Next' }).locator(`[data-choice="${id}"]`);
const status = (page) => page.evaluate(() => fetch('engine.json', { cache: 'no-store' }).then((r) => r.json()));

/// The clock of `tick`, as the date block writes it.
const clockOf = (tick) => {
  const seconds = Math.floor(tick / 50);
  return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
};

const launch = (skin, fastForwardTo) =>
  frontDoor({
    args: ['--seed', SEED],
    fastForwardTo,
    env: skin === 'broadcast-blue' ? {} : { SM_CONTENT_DIR: light.dir },
  });

/// The page on the start screen of `engine`, then a live match of the two default clubs.
async function liveFromStart(page, engine, skin = 'broadcast-blue') {
  await open(page, engine.url, skin);
  await pastSplash(page);
  await kickOffFromStart(page);
}

/// Skips at 20:00 and waits until the engine has played the rest and, with `stored`, until
/// the whole match is stored.
async function skipAt20(page, { stored = true } = {}) {
  await until(page, (t) => window.__touchline.history().newest_tick >= t, SKIP_TICK, 120_000);
  if ((await action(page).textContent()).trim() === 'Pause') {
    await action(page).click();
  }
  await expect(action(page)).toHaveText('Resume');
  await rewindTo(page, SKIP_TICK);
  await button(page, 'Skip to result').click();
  await until(page, () => window.__touchline.view() === 'skip', undefined, 5_000);
  await button(page, 'Confirm: skip to result').click();
  if (stored) {
    await until(page, () => window.__touchline.skip()?.state === 'ready', undefined, 5 * 60_000);
    await until(page, () => window.__touchline.report().state === 'ready', undefined, 10_000);
  }
}

/// The match screen at full time: FULL TIME and the final clock, REPORT, no live control.
async function expectFullTime(page, tick) {
  expect(await hook(page, () => window.__touchline.screen())).toBe('full-time');
  expect(await hook(page, () => window.__touchline.view())).toBe('match');
  await expect(page.locator('header .date b:visible')).toHaveText('FULL TIME');
  await expect(page.locator('header .date span:visible')).toHaveText(clockOf(tick));
  await expect(action(page)).toHaveText('Report');
  for (const name of LIVE_CONTROLS) {
    await expect(button(page, name), `no ${name} at full time`).toHaveCount(0);
  }
  // The stage never scrolls sideways, whatever control was pressed on the way here.
  expect(await hook(page, () => document.querySelector('.app').scrollLeft)).toBe(0);
}

/// Records every date word on show from now on: a page-side observer of the whole page that
/// reads each visible date block whenever anything changes. `plant` proves the recorder sees a
/// word: a visible LIVE date block is added and taken away, and must be recorded.
async function recordDateWords(page, { plant = false } = {}) {
  await page.evaluate((planted) => {
    const words = [];
    const read = () => {
      for (const b of document.querySelectorAll('header .date b')) {
        if (b.offsetParent !== null) {
          words.push(b.textContent.trim());
        }
      }
    };
    read();
    const observer = new MutationObserver(read);
    observer.observe(document.body, { subtree: true, childList: true, characterData: true, attributes: true });
    window.__dateWords = { words, observer };
    if (planted) {
      const header = document.createElement('header');
      header.innerHTML = '<div class="date"><b>LIVE</b></div>';
      document.body.append(header);
    }
  }, plant);
  if (plant) {
    await until(page, () => window.__dateWords.words.includes('LIVE'), undefined, 5_000);
    await page.evaluate(() => {
      document.body.lastElementChild.remove();
      window.__dateWords.words.length = 0;
    });
  }
}

const dateWords = (page) =>
  page.evaluate(() => {
    window.__dateWords.observer.takeRecords();
    return [...new Set(window.__dateWords.words)];
  });

async function tabWalk(page, presses) {
  await clickClear(page);
  const walk = [];
  for (let press = 1; press <= presses; press += 1) {
    await page.keyboard.press('Tab');
    walk.push(
      await page.evaluate((n) => {
        const el = document.activeElement;
        return {
          press: n,
          tag: el?.tagName.toLowerCase() ?? null,
          name: (el?.getAttribute('aria-label') ?? el?.textContent ?? '').trim().replace(/\s+/g, ' ').slice(0, 60),
          stub: el?.closest('[data-stub]')?.dataset.stub ?? null,
          inert: Boolean(el?.closest('[inert]')),
          hidden: Boolean(el?.closest('[hidden]')),
        };
      }, press)
    );
  }
  return walk;
}

function expectClean(walk) {
  for (const step of walk) {
    expect(step.stub, `press ${step.press}: ${step.name}`).toBeNull();
    expect(step.inert, `press ${step.press}: ${step.name}`).toBe(false);
    expect(step.hidden, `press ${step.press}: ${step.name}`).toBe(false);
  }
}

for (const skin of SKINS) {
  test.describe(`after full time in ${skin}`, () => {
    test('the match screen at full time after a match played through', { tag: '@sizes' }, async ({ page }) => {
      test.setTimeout(6 * 60_000);
      const engine = await launch(skin, PAST_THE_END);
      try {
        await liveFromStart(page, engine, skin);
        const whistle = await toFullTime(page);
        await until(page, () => window.__touchline.report().state === 'ready', undefined, 60_000);
        await waitForGrounds(page, Number.MAX_SAFE_INTEGER, 180_000);
        await button(page, 'Back to the match at full time').click();
        await until(page, () => window.__touchline.view() === 'match', undefined, 5_000);
        await rewindTo(page, whistle.tick);
        await expectFullTime(page, whistle.tick);
        await expect(page.locator('.pitchbox .ftag')).toHaveText(`FULL TIME · ${clockOf(whistle.tick)}`);
        await settled(page);
        await snap(page, `post-full-time-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('the match screen at full time after a skip, and the report with its next steps', { tag: '@sizes' }, async ({ page }, info) => {
      test.setTimeout(8 * 60_000);
      const engine = await launch(skin, SKIP_TICK);
      try {
        await liveFromStart(page, engine, skin);
        await skipAt20(page);
        await waitForGrounds(page, Number.MAX_SAFE_INTEGER, 180_000);
        const whistle = await hook(page, () => window.__touchline.events().find((e) => e['event.type'] === 'full-time'));
        await button(page, 'Back to the match at full time').click();
        await until(page, () => window.__touchline.view() === 'match', undefined, 5_000);
        await expectFullTime(page, whistle.tick);
        await expect(page.locator('.strip:visible')).toContainText('Skipped at');
        await settled(page);
        await snap(page, `post-full-time-skipped-${skin}.png`);
        const matchContrast = await contrastReport(page);

        await action(page).click();
        await until(page, () => window.__touchline.report().open, undefined, 5_000);
        await expect(action(page)).toHaveText('New match');
        await expect(nextRow(page, 'new')).toBeEnabled();
        await settled(page);
        await snap(page, `post-report-next-${skin}.png`);
        const reportContrast = await contrastReport(page);
        fs.writeFileSync(
          evidence(info, `post-contrast-${skin}.json`),
          `${JSON.stringify({ match: matchContrast, report: reportContrast }, null, 2)}\n`
        );
        for (const report of [matchContrast, reportContrast]) {
          expect(report.checked).toBeGreaterThan(20);
          expect(report.failures, JSON.stringify(report.failures, null, 2)).toEqual([]);
        }
      } finally {
        engine.cleanUp();
      }
    });

    test('the report while the skipped match is still being stored', { tag: '@sizes' }, async ({ page }) => {
      test.setTimeout(8 * 60_000);
      // The engine's close after full time never reaches the page: the store stays open. The
      // match socket's close listener is dropped in the page, so no frame passes through the
      // test (a routed socket relays every fast-forwarded frame and exhausts the worker).
      await page.addInitScript(() => {
        const add = WebSocket.prototype.addEventListener;
        WebSocket.prototype.addEventListener = function (type, ...rest) {
          if (type === 'close' && /\/\?v=\d+$/.test(this.url)) {
            return;
          }
          return add.call(this, type, ...rest);
        };
      });
      const engine = await launch(skin, SKIP_TICK);
      try {
        await liveFromStart(page, engine, skin);
        await skipAt20(page, { stored: false });
        await until(page, () => window.__touchline.report().state === 'storing', undefined, 5 * 60_000);
        // Every frame up to the whistle is in: only the close is missing.
        await until(
          page,
          () => {
            const whistle = window.__touchline.events().find((e) => e['event.type'] === 'full-time');
            return whistle && window.__touchline.history().newest_tick >= whistle.tick;
          },
          undefined,
          5 * 60_000
        );
        await waitForGrounds(page, Number.MAX_SAFE_INTEGER, 180_000);
        // The whole match's expected goals, as the full-time screen shows them.
        await expect(page.locator('.strip:visible')).toContainText('0.11 – 0.22');
        expect(await hook(page, () => window.__touchline.report().next)).toEqual({ offered: true, ready: false });
        await expect(action(page)).toHaveText('New match');
        await expect(action(page)).toBeDisabled();
        for (const id of ['new', 'return']) {
          await expect(nextRow(page, id)).toBeDisabled();
        }
        await expect(button(page, 'Replay the whole match')).toBeDisabled();
        await expect(button(page, 'Save replay')).toBeDisabled();
        await expect(page.locator('.storing:visible')).toHaveText(/Getting the replay ready…/);
        await expect(page.getByRole('status').filter({ hasText: 'Getting the replay ready…' })).toHaveCount(1);
        await settled(page);
        await snap(page, `post-report-storing-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });
  });
}

test('full time after a resumed match', async ({ page }) => {
  test.setTimeout(8 * 60_000);
  const LEAVE_AT = 4_000;
  const engine = await frontDoor({ args: ['--seed', '11', '--minutes', '3'], fastForwardTo: LEAVE_AT });
  try {
    await liveFromStart(page, engine);
    await until(page, (t) => window.__touchline.history().newest_tick >= t, LEAVE_AT, 120_000);
    await page.keyboard.press('Escape');
    await page.locator('[data-item="return"]').click();
    await page.getByRole('button', { name: 'Save and leave' }).click();
    await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 15_000);
    await page.locator('[data-choice="resume"]').click();
    await until(page, () => window.__touchline.frontDoor().view === 'match', undefined, 15_000);
    await page.getByRole('group', { name: 'Playback' }).filter({ visible: true }).getByRole('button', { name: '8x', exact: true }).click();
    await playUntil(
      page,
      () => {
        const report = window.__touchline.report();
        return report.open && report.kind === 'full-time';
      },
      { timeout: 5 * 60_000 }
    );
    await until(page, () => window.__touchline.report().state === 'ready', undefined, 60_000);
    const whistle = await hook(page, () => window.__touchline.events().find((e) => e['event.type'] === 'full-time'));
    await button(page, 'Back to the match at full time').click();
    await expectFullTime(page, whistle.tick);
  } finally {
    engine.cleanUp();
  }
});

test('the reproduced path never shows LIVE or PAUSED again, with and without a start screen', async ({ page }, info) => {
  test.setTimeout(10 * 60_000);
  const record = {};
  const engine = await launch('broadcast-blue', SKIP_TICK);
  try {
    await liveFromStart(page, engine);
    await skipAt20(page);
    // The recorder's control: a planted LIVE date block must be recorded.
    await recordDateWords(page, { plant: true });
    await button(page, 'Back to the match at full time').click();
    await until(page, () => window.__touchline.view() === 'match', undefined, 5_000);
    const whistle = await hook(page, () => window.__touchline.events().find((e) => e['event.type'] === 'full-time'));
    await expectFullTime(page, whistle.tick);
    await action(page).click();
    await until(page, () => window.__touchline.report().open, undefined, 5_000);
    await button(page, 'Back to the match at full time').click();
    const row = page.getByRole('group', { name: 'Playback' }).filter({ visible: true });
    await row.getByRole('button', { name: '4x', exact: true }).click();
    await row.getByRole('button', { name: 'Back to kick-off', exact: true }).click();
    await until(page, () => window.__touchline.lastRenderedTick() <= 1, undefined, 5_000);
    await rewindTo(page, SKIP_TICK);
    await row.getByRole('button', { name: 'Rewind 10 seconds', exact: true }).click();
    await until(page, (t) => window.__touchline.lastRenderedTick() === t - 500, SKIP_TICK, 5_000);
    await row.getByRole('button', { name: 'Play the replay from here', exact: true }).click();
    await until(page, () => window.__touchline.view() === 'replay', undefined, 5_000);
    await until(page, (t) => window.__touchline.lastRenderedTick() > t, SKIP_TICK - 500, 15_000);
    await action(page).click();
    await until(page, () => window.__touchline.view() === 'match', undefined, 5_000);
    await expectFullTime(page, whistle.tick);
    record.front_door = await dateWords(page);
  } finally {
    engine.cleanUp();
  }

  // A served test page has no start screen: the report keeps CONTINUE and offers no Next list.
  const served = await startEngine({
    args: ['--seed', SEED, '--minutes', '1', '--web', VIEWER, ...fastForward(PAST_THE_END)],
  });
  try {
    await open(page, served.url);
    await until(page, () => window.__touchline.view() === 'tactics', undefined, 30_000);
    await action(page).click();
    await until(page, () => window.__touchline.view() === 'prematch', undefined, 5_000);
    await action(page).click();
    await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
    const whistle = await toFullTime(page);
    await until(page, () => window.__touchline.report().state === 'ready', undefined, 60_000);
    expect(await hook(page, () => window.__touchline.report().next)).toEqual({ offered: false, ready: true });
    await expect(page.getByRole('group', { name: 'Next' })).toHaveCount(0);
    await recordDateWords(page);
    await expect(action(page)).toHaveText('Continue');
    await action(page).click();
    await expectFullTime(page, whistle.tick);
    await action(page).click();
    await until(page, () => window.__touchline.report().open, undefined, 5_000);
    await action(page).click();
    await expectFullTime(page, whistle.tick);
    record.served = await dateWords(page);
  } finally {
    served.cleanUp();
  }
  fs.writeFileSync(evidence(info, 'date-words.json'), `${JSON.stringify(record, null, 2)}\n`);
  for (const words of Object.values(record)) {
    expect(words).toContain('FULL TIME');
    expect(words).not.toContain('LIVE');
    expect(words).not.toContain('PAUSED');
  }
});

test('Return to start leaves the launcher idle, with nothing to resume and the old worker gone', async ({ page }, info) => {
  test.setTimeout(8 * 60_000);
  const engine = await launch('broadcast-blue', SKIP_TICK);
  try {
    await liveFromStart(page, engine);
    const running = await status(page);
    await skipAt20(page);
    await until(page, () => window.__touchline.report().next.ready, undefined, 10_000);
    await nextRow(page, 'return').click();
    await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 20_000);
    const after = await status(page);
    fs.writeFileSync(evidence(info, 'next-steps-return.json'), `${JSON.stringify({ running, after }, null, 2)}\n`);
    expect(after['engine.state']).toBe('idle');
    expect(after.saved).toBeNull();
    expect(processAlive(running['engine.pid'])).toBe(false);
    await expect(page.locator('[data-choice="resume"]')).toBeDisabled();
    await expect(page.locator('[data-choice="new"]')).toBeEnabled();
  } finally {
    engine.cleanUp();
  }
});

test('New match opens match setup with the two clubs that played, and kicks off a fresh match', async ({ page }, info) => {
  test.setTimeout(8 * 60_000);
  const engine = await launch('broadcast-blue', SKIP_TICK);
  try {
    await liveFromStart(page, engine);
    const running = await status(page);
    const clubs = await hook(page, () => [
      ...new Set(window.__touchline.events().map((e) => e['team.id']).filter(Boolean)),
    ]);
    await skipAt20(page);
    await until(page, () => window.__touchline.report().next.ready, undefined, 10_000);
    await nextRow(page, 'new').click();
    await until(page, () => window.__touchline.frontDoor().view === 'setup', undefined, 20_000);
    await until(page, () => window.__touchline.frontDoor().round !== null, undefined, 10_000);
    const door = await hook(page, () => window.__touchline.frontDoor());
    expect([door.picks.home, door.picks.away].sort()).toEqual([...clubs].sort());
    const names = running.teams.filter((t) => t.id === door.picks.home || t.id === door.picks.away).map((t) => t.name);
    await page.locator('[data-kickoff]').click();
    await until(page, () => window.__touchline.screen() === 'kickoff', undefined, 30_000);
    const fresh = await status(page);
    fs.writeFileSync(
      evidence(info, 'next-steps-new.json'),
      `${JSON.stringify({ running, picks: door.picks, clubs, fresh }, null, 2)}\n`
    );
    expect(fresh['engine.state']).toBe('running');
    expect(fresh['match.id']).not.toBe(running['match.id']);
    expect(fresh['engine.pid']).not.toBe(running['engine.pid']);
    // The hello names both clubs: the Pre-match line-ups show them.
    await expect(action(page)).toHaveText('Continue');
    await action(page).click();
    await until(page, () => window.__touchline.view() === 'prematch', undefined, 5_000);
    for (const name of names) {
      await expect(page.locator('[data-screen="prematch"]')).toContainText(name);
    }
  } finally {
    engine.cleanUp();
  }
});

test('a Tab walk reaches every new control and no stub', { tag: '@sizes' }, async ({ page }, info) => {
  test.setTimeout(8 * 60_000);
  const engine = await launch('broadcast-blue', SKIP_TICK);
  try {
    await liveFromStart(page, engine);
    await skipAt20(page);
    const reportWalk = await tabWalk(page, 30);
    await button(page, 'Back to the match at full time').click();
    await until(page, () => window.__touchline.view() === 'match', undefined, 5_000);
    const matchWalk = await tabWalk(page, 40);
    fs.writeFileSync(evidence(info, 'post-tab-walk.json'), `${JSON.stringify({ match: matchWalk, report: reportWalk }, null, 2)}\n`);
    expectClean(matchWalk);
    expectClean(reportWalk);
    const names = (walk) => walk.map((s) => s.name);
    for (const name of ['Report', 'Back to kick-off', 'Rewind 10 seconds', 'Play the replay from here', '1x', '8x']) {
      expect(names(matchWalk)).toContain(name);
    }
    for (const name of ['New match', 'Replay the whole match', 'Save replay', 'Open a replay', 'Back to the match at full time']) {
      expect(names(reportWalk)).toContain(name);
    }
    expect(names(reportWalk).some((n) => /^New match ?Match setup/.test(n))).toBe(true);
    expect(names(reportWalk).some((n) => n.startsWith('Return to start'))).toBe(true);
  } finally {
    engine.cleanUp();
  }
});
