// The charter scenario, steps 1 to 5, on the served viewer: one served match of seed 7 with its
// matchday, fast-forwarded to 30:00 so the page holds the first half hour at once.
//
// 1. The match screen opens in Broadcast Blue, and the other fixtures show 0-0 KO.
// 2. To minute 30 by seeks: at each of ten drawn ticks, every ground event shown is at or
//    before the drawn tick, and every event the engine computed at or before it is shown.
// 3. Skip to result opens the decision: the rest cannot be watched live, and the other grounds
//    finish too.
// 4. Confirm: the report shows the final score, the strip names how many grounds are final,
//    and the stored tick frames equal those of the same match played through with no skip.
// 5. Replay the whole match starts at kick-off and plays into the skipped part, drawing each
//    stored tick exactly as it arrived.
// Run with `--trace on` to keep the trace as evidence; each step also saves a screenshot.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { VIEWER, fastForward, startEngine, waitForGrounds } from '../support/engine.mjs';
import { playUntil } from '../support/page.mjs';

const SEED = 7;
/// Minute 30: the skip point.
const MINUTE_30 = 90_000;
/// Past the end of any match: a served match with this fast-forward plays straight through.
const PAST_THE_END = 1_000_000;
const MINUTE = 3_000;

const evidence = (info, name) => {
  const out = process.env.MATCH_EVIDENCE_DIR
    ? path.join(process.env.MATCH_EVIDENCE_DIR, name)
    : info.outputPath(name);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  return out;
};

const hook = (page, fn, arg) => page.evaluate(fn, arg);
const until = (page, fn, arg, timeout = 120_000) =>
  page.waitForFunction(fn, arg, { timeout, polling: 100 });
const action = (page) => page.locator('header button.cont:visible');
const playback = (page) => page.getByRole('group', { name: 'Playback' }).filter({ visible: true });
const button = (page, name) => page.getByRole('button', { name, exact: true }).filter({ visible: true });

async function open(page, url) {
  await page.goto(url);
  await expect(page.locator('html')).toHaveAttribute('data-ready', 'broadcast-blue', { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
}

async function rewindTo(page, tick) {
  await page.getByRole('slider', { name: 'Rewind to a tick' }).filter({ visible: true }).evaluate((scrub, value) => {
    scrub.value = String(value);
    scrub.dispatchEvent(new Event('input', { bubbles: true }));
    scrub.dispatchEvent(new Event('change', { bubbles: true }));
  }, tick);
  await until(page, (t) => window.__touchline.lastRenderedTick() === t, tick, 10_000);
}

/// Kicks off: from the Match view its action block reads KICK OFF; from Tactics, CONTINUE
/// opens the Pre-match line-ups, whose KICK OFF starts the match.
async function kickOff(page) {
  await until(page, () => ['tactics', 'match'].includes(window.__touchline.view()), undefined, 30_000);
  if ((await hook(page, () => window.__touchline.view())) === 'tactics') {
    await action(page).click();
    await until(page, () => window.__touchline.view() === 'prematch', undefined, 5_000);
  }
  await expect(action(page)).toHaveText('Kick off');
  await action(page).click();
  await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
}

/// The ground events shown at the drawn tick against those the engine computed.
const revealAt = (page) =>
  hook(page, () => {
    const day = window.__touchline.matchday();
    const tick = window.__touchline.lastRenderedTick();
    return {
      tick,
      rows: day.grounds.rows.map((row, i) => {
        const computed = day.events[i];
        const due = computed.filter((e) => e.tick <= tick);
        return {
          fixture: row.fixture,
          shown_score: row.score,
          minute: row.minute,
          computed: computed.map((e) => [e.tick, e.kind, e.score]),
          due_score: due.length ? due.at(-1).score : [0, 0],
          later: computed.filter((e) => e.tick > tick).length,
        };
      }),
    };
  });

test('the charter scenario to the replay of a skipped match', async ({ page }, info) => {
  test.setTimeout(20 * 60_000);
  let engine = await startEngine({ args: ['--seed', String(SEED), '--web', VIEWER, ...fastForward(MINUTE_30)] });
  const digests = {};
  try {
    // 1. Broadcast Blue, and the other fixtures at 0-0 KO before kick-off.
    await open(page, engine.url);
    await until(page, () => window.__touchline.screen() === 'kickoff', undefined, 30_000);
    await page.locator('nav.subnav:visible').getByRole('button', { name: 'Match', exact: true }).click();
    await waitForGrounds(page, 0);
    const font = await page.locator('.strip .score:visible').evaluate((el) => getComputedStyle(el).fontFamily);
    expect(font).toMatch(/Saira/);
    const before = await hook(page, () => window.__touchline.matchday());
    expect(before.fixtures).toHaveLength(4);
    for (const row of before.grounds.rows) {
      expect(row.score).toEqual([0, 0]);
      expect(row.minute).toBe('KO');
    }
    await page.screenshot({ path: evidence(info, 'charter-1-kickoff.png') });

    // 2. To minute 30 by seeks: nothing early, nothing missing.
    await kickOff(page);
    await playUntil(page, (t) => window.__touchline.history().newest_tick >= t, { arg: MINUTE_30, timeout: 5 * 60_000 });
    await action(page).click();
    await expect(action(page)).toHaveText('Resume');
    const samples = [];
    for (let i = 1; i <= 10; i += 1) {
      const tick = (MINUTE_30 / 10) * i;
      await rewindTo(page, tick);
      await waitForGrounds(page, tick);
      // The list is worked out once a simulated second, so read it after one has drawn.
      const sample = await revealAt(page);
      samples.push(sample);
      for (const row of sample.rows) {
        expect(row.shown_score, `fixture ${row.fixture} at ${tick}`).toEqual(row.due_score);
      }
    }
    fs.writeFileSync(evidence(info, 'reveal-samples.json'), `${JSON.stringify(samples, null, 2)}\n`);
    await page.screenshot({ path: evidence(info, 'charter-2-minute-30.png') });

    // 3. The decision: the rest cannot be watched live, and the other grounds finish too.
    expect(await hook(page, () => window.__touchline.canSkip())).toBe(true);
    await button(page, 'Skip to result').click();
    await until(page, () => window.__touchline.view() === 'skip', undefined, 5_000);
    const decision = page.locator('[data-screen="skip"]');
    await expect(decision).toContainText('Watching the rest of the match live.');
    await expect(decision).toContainText('They finish too, and their results show in the report.');
    await page.screenshot({ path: evidence(info, 'charter-3-decision.png') });

    // 4. Confirm: the final score, the grounds final, and the same match as one played through.
    await button(page, 'Confirm: skip to result').click();
    await until(page, () => window.__touchline.skip()?.state === 'ready', undefined, 5 * 60_000);
    await until(page, () => window.__touchline.report().state === 'ready', undefined, 10_000);
    await waitForGrounds(page, Number.MAX_SAFE_INTEGER, 180_000);
    const events = await hook(page, () => window.__touchline.events());
    const fullTime = events.find((e) => e['event.type'] === 'full-time');
    const report = await hook(page, () => window.__touchline.report());
    expect(report.score).toEqual([fullTime['home.score'], fullTime['away.score']]);
    await expect(page.locator('.strip .score:visible')).toHaveText(`${report.score[0]} – ${report.score[1]}`);
    await expect(page.locator('.strip:visible')).toContainText(/Other grounds: \d of 4 final/);
    await expect(page.locator('[data-screen="report"]')).toContainText('Matchday 1 · final');
    digests.skipped = {
      digest: await hook(page, () => window.__touchline.tickFrameDigest()),
      score: report.score,
      ticks: (await hook(page, () => window.__touchline.replay())).ticks,
    };
    await page.screenshot({ path: evidence(info, 'charter-4-report.png') });

    // 5. Replay the whole match: from kick-off into the skipped part, each tick as it arrived.
    await button(page, 'Replay the whole match').click();
    await until(page, () => window.__touchline.view() === 'replay', undefined, 5_000);
    expect((await hook(page, () => window.__touchline.lastRewind())).tick).toBeLessThanOrEqual(1);
    const group = playback(page);
    if (await group.getByRole('button', { name: 'Pause', exact: true }).count()) {
      await group.getByRole('button', { name: 'Pause', exact: true }).click();
    }
    const replayed = [];
    for (const tick of [MINUTE_30 + MINUTE, MINUTE_30 + 20 * MINUTE, MINUTE_30 + 40 * MINUTE, fullTime.tick - MINUTE]) {
      await rewindTo(page, tick);
      const rewind = await hook(page, () => window.__touchline.lastRewind());
      const stored = await hook(page, (t) => window.__touchline.tickAt(t), tick);
      replayed.push({ tick, exact: rewind.exact });
      expect(rewind.exact, `tick ${tick} drawn as stored`).toBe(true);
      expect(rewind.stored).toEqual(stored);
    }
    await rewindTo(page, MINUTE_30 - MINUTE / 6);
    await playback(page).getByRole('button', { name: '8x', exact: true }).click();
    await playback(page).getByRole('button', { name: 'Play', exact: true }).click();
    await until(page, (t) => window.__touchline.lastRenderedTick() > t, MINUTE_30 + MINUTE, 30_000);
    await expect(page.locator('[data-screen="replay"] .tagc')).toContainText('NOT WATCHED LIVE');
    await page.screenshot({ path: evidence(info, 'charter-5-replay.png') });
    engine.cleanUp();

    // The same seed played straight through with no skip.
    engine = await startEngine({ args: ['--seed', String(SEED), '--web', VIEWER, ...fastForward(PAST_THE_END)] });
    await open(page, engine.url);
    await kickOff(page);
    await playback(page).getByRole('button', { name: '8x', exact: true }).click();
    await playUntil(page, (t) => window.__touchline.history().newest_tick >= t, { arg: fullTime.tick, timeout: 5 * 60_000 });
    await until(page, () => /whole match is stored/.test(window.__touchline.notice().message ?? ''), undefined, 3 * 60_000);
    const watchedEnd = (await hook(page, () => window.__touchline.events())).find((e) => e['event.type'] === 'full-time');
    digests.watched = {
      digest: await hook(page, () => window.__touchline.tickFrameDigest()),
      score: [watchedEnd['home.score'], watchedEnd['away.score']],
      ticks: (await hook(page, () => window.__touchline.replay())).ticks,
    };
    fs.writeFileSync(evidence(info, 'charter-digests.json'), `${JSON.stringify({ ...digests, replayed }, null, 2)}\n`);
    expect(digests.skipped.ticks).toBe(digests.watched.ticks);
    expect(digests.skipped.digest).toBe(digests.watched.digest);
    expect(digests.skipped.score).toEqual(digests.watched.score);
  } finally {
    engine.cleanUp();
  }
});
