// The whole first match, lineup to full-time report, against the live engine: the twelve
// steps a manager takes, each with the checkpoint the manager sees.
import { existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { expect, test } from '@playwright/test';
import {
  generateLeague,
  readRecords,
  runEngine,
  startEngine,
  tempDir,
} from '../support/engine.mjs';
import {
  changeReaches,
  feedRows,
  hook,
  kickOff,
  kickOffButton,
  openMatch,
  playUntil,
  queueSubstitution,
  queued,
  renderedTick,
  scrubTo,
  setSpeed,
  until,
} from '../support/page.mjs';

// Positions travel in centimetres: ball x, y, height, then x and y for each of 22 players.
const HALF_LENGTH_CM = 5250;
const HALF_WIDTH_CM = 3400;
// A sent-off player stands beside the pitch at a parking spot, and is not on the pitch.
const isParked = (x, y) => Math.abs(y + 3700) <= 1 && Math.abs(x) >= 999 && Math.abs(x) <= 2001;

// The report rows, counted from the engine's own event records.
const ROWS = {
  goals: (e) => e['event.type'] === 'goal',
  yellow: (e) => e['event.type'] === 'card' && e['card.kind'] === 'yellow',
  red: (e) => e['event.type'] === 'card' && ['red', 'second-yellow'].includes(e['card.kind']),
  fouls: (e) => e['event.type'] === 'foul',
  corners: (e) => e['event.type'] === 'corner',
  offsides: (e) => e['event.type'] === 'offside',
  'throw-ins': (e) => e['event.type'] === 'throw-in',
  'goal-kicks': (e) => e['event.type'] === 'goal-kick',
  'free-kicks': (e) => e['event.type'] === 'free-kick',
  penalties: (e) => e['event.type'] === 'penalty',
};

function countsFrom(events, teamIds, uptoTick) {
  const upto = events.filter((e) => e.tick <= uptoTick);
  return Object.fromEntries(
    Object.entries(ROWS).map(([id, counts]) => [
      id,
      teamIds.map((team) => upto.filter((e) => e['team.id'] === team && counts(e)).length),
    ])
  );
}

async function shot(page, testInfo, step) {
  const name = `step-${String(step).padStart(2, '0')}.png`;
  await testInfo.attach(name, { body: await page.screenshot({ fullPage: true }), contentType: 'image/png' });
}

test('a manager plays a whole match, lineup to full-time report', async ({ page }, testInfo) => {
  const [teamA, teamB] = generateLeague(2026);
  const engine = await startEngine({
    command: 'serve',
    args: ['--seed', '42', '--web', 'web', '--team-a', teamA, '--team-b', teamB],
  });
  let chosenMentality = null;
  try {
    const seen = await openMatch(page, engine.url);

    await test.step('1. the header says the engine is connected, with its version', async () => {
      await until(page, () => window.__touchline.lineup().phase === 'pre-match', { timeout: 30_000 });
      expect(seen.hello).not.toBeNull();
      const version = seen.hello['engine.version'];
      await expect(page.locator('header')).toContainText(`Engine connected · v${version}`);
      await shot(page, testInfo, 1);
    });

    await test.step('2. an illegal lineup keeps kick-off disabled and says why', async () => {
      const legal = await hook(page, () => window.__touchline.lineup());
      expect(legal.legal).toBe(true);
      await page.getByRole('button', { name: /^Slot 4,/ }).click();
      await page.getByRole('button', { name: 'Empty the picked slot' }).click();
      const reason = page.getByTestId('lineup-reason');
      await expect(kickOffButton(page)).toBeDisabled();
      await expect(reason).not.toHaveText('The lineup is ready.');
      await expect(reason).not.toHaveText('');
      const illegal = await hook(page, () => window.__touchline.lineup());
      expect(illegal.legal).toBe(false);
      await shot(page, testInfo, 2);
      // Put the same player back: pick him in the squad, then the empty slot.
      await page.getByTestId(`squad-${legal.slots[3]}`).click();
      await page.getByRole('button', { name: /^Slot 4,/ }).click();
      await expect(reason).toHaveText('The lineup is ready.');
      await expect(kickOffButton(page)).toBeEnabled();
    });

    await test.step('3. the tactics show the chosen values', async () => {
      const mentality = page.getByLabel('Mentality');
      const count = await mentality.locator('option').count();
      const current = await mentality.evaluate((s) => s.selectedIndex);
      const next = current + 1 < count ? current + 1 : current - 1;
      await mentality.selectOption({ index: next });
      const pressing = page.getByLabel('Pressing');
      const levels = await pressing.locator('option').count();
      const pressNow = await pressing.evaluate((s) => s.selectedIndex);
      await pressing.selectOption({ index: pressNow + 1 < levels ? pressNow + 1 : pressNow - 1 });
      const tactics = await hook(page, () => window.__touchline.dugout().tactics);
      expect(tactics.mentality).toBe(next);
      chosenMentality = next;
      expect(await mentality.evaluate((s) => s.selectedIndex)).toBe(next);
      await shot(page, testInfo, 3);
    });

    await test.step('4. after kick-off the clock runs and all 22 players and the ball move on the pitch', async () => {
      await kickOff(page);
      const kickedOff = await hook(page, () => window.__touchline.dugout().tactics);
      await until(page, () => window.__touchline.lastRenderedTick() > 500, { timeout: 60_000 });
      const clockA = await page.getByLabel('Match clock').textContent();
      const a = await hook(page, () => window.__touchline.lastRendered());
      await page.waitForTimeout(3000);
      const b = await hook(page, () => window.__touchline.lastRendered());
      const clockB = await page.getByLabel('Match clock').textContent();
      expect(clockB).not.toBe(clockA);
      expect(a).toHaveLength(47);
      expect(a[0] !== b[0] || a[1] !== b[1]).toBe(true);
      let moved = 0;
      for (let p = 0; p < 22; p += 1) {
        const i = 3 + p * 2;
        if (a[i] !== b[i] || a[i + 1] !== b[i + 1]) {
          moved += 1;
        }
      }
      expect(moved).toBe(22);
      for (const frame of [a, b]) {
        for (let p = 0; p < 22; p += 1) {
          const [x, y] = [frame[3 + p * 2], frame[4 + p * 2]];
          if (!isParked(x, y)) {
            expect(Math.abs(x)).toBeLessThanOrEqual(HALF_LENGTH_CM);
            expect(Math.abs(y)).toBeLessThanOrEqual(HALF_WIDTH_CM);
          }
        }
      }
      // The tactics chosen before kick-off are the ones the engine confirmed.
      expect(kickedOff.mentality).toBe(chosenMentality);
      await shot(page, testInfo, 4);
    });

    await test.step('5. at 4x the clock runs four times faster, or the notice names the sustained speed', async () => {
      await setSpeed(page, 4);
      await page.waitForTimeout(1000);
      const t0 = await renderedTick(page);
      const w0 = Date.now();
      await page.waitForTimeout(5000);
      const t1 = await renderedTick(page);
      const seconds = (Date.now() - w0) / 1000;
      const rate = (t1 - t0) / seconds / 50;
      const notice = (await page.locator('#notice').textContent()).trim();
      const noticeShown = (await page.locator('#notice').getAttribute('data-shown')) === 'true';
      await shot(page, testInfo, 5);
      // Never faster than asked, whichever way the step goes.
      expect(rate).toBeLessThan(4 * 1.05);
      if (Math.abs(rate - 4) / 4 > 0.05) {
        // The engine could not keep up: a shown notice names the speed it sustained, below
        // the one asked for, and the clock ran at about that speed.
        expect(noticeShown, `notice: ${notice}`).toBe(true);
        const named = notice.match(/Playing at (\d+)x/);
        expect(named, `notice: ${notice}`).not.toBeNull();
        const sustained = Number(named[1]);
        expect(sustained).toBeLessThan(4);
        expect(Math.abs(rate - sustained), `measured ${rate}x, notice ${notice}`).toBeLessThanOrEqual(1);
      } else {
        // Playing at the speed asked for: no lag notice names a lower one.
        expect(noticeShown && /Playing at/.test(notice), `notice: ${notice}`).toBe(false);
      }
      await expect(page.getByLabel('Playback speed')).toHaveText('4x');
    });

    await test.step('6. a mentality change is queued, applies at the next dead ball, and the feed says so', async () => {
      const mentality = page.getByLabel('Mentality');
      const current = await mentality.evaluate((s) => s.selectedIndex);
      const count = await mentality.locator('option').count();
      const change = await queued(page, () =>
        mentality.selectOption({ index: current + 1 < count ? current + 1 : current - 1 })
      );
      expect(change.kind).toBe('tactics');
      await expect(page.locator('#chips .chip').filter({ hasText: 'Queued' })).toHaveCount(1);
      await shot(page, testInfo, 6);
      const applied = await changeReaches(page, change.queue_id, ['applied', 'rejected']);
      expect(applied.state).toBe('applied');
      // It applied at a stoppage after it was queued, not before.
      const stop = await hook(page, (t) => window.__touchline.stoppageAt(t), applied.queued_tick);
      expect(applied.applied_tick).toBeGreaterThanOrEqual(stop);
      await until(page, () =>
        [...document.querySelectorAll('#feed li')].some((li) => li.textContent.includes('Tactical change applied'))
      );
      // The chip clears once the change has applied.
      await until(page, (id) => !window.__touchline.pending().some((c) => c.queue_id === id && c.state !== 'applied'), {
        arg: change.queue_id,
      });
      await expect(page.locator('#chips .chip').filter({ hasText: 'Queued' })).toHaveCount(0, { timeout: 60_000 });
    });

    await test.step('7. a substitution applies at a dead ball, the players swap, and the count drops', async () => {
      const left = await hook(page, () => window.__touchline.dugout().subs_left);
      const offName = await page.getByLabel('Player coming off').evaluate((s) => s.options[5].text);
      const onName = await page.getByLabel('Substitute coming on').evaluate((s) => s.options[0].text);
      const change = await queueSubstitution(page, 5, 0);
      const applied = await changeReaches(page, change.queue_id, ['applied', 'rejected']);
      expect(applied.state).toBe('applied');
      await until(page, (n) => window.__touchline.dugout().subs_left === n, { arg: left - 1 });
      await expect(page.getByTestId('subs-left')).toContainText(String(left - 1));
      const labels = await hook(page, () => window.__touchline.matchDay().lineupLabels.map((r) => r.name));
      const surname = (text) => text.replace(/^\d+\s*/, '').split(/\s+/).pop();
      expect(labels.join(' | ')).toContain(surname(onName));
      await shot(page, testInfo, 7);
      testInfo.annotations.push({ type: 'substitution', description: `${offName} off, ${onName} on` });
    });

    let goal = null;
    await test.step('8. a goal updates the score, shows the banner, and the feed names scorer, minute, and score', async () => {
      await setSpeed(page, 8);
      goal = await playUntil(
        page,
        () => {
          const day = window.__touchline.matchDay();
          if (day.goalShownAtTick === null) {
            return null;
          }
          return window.__touchline.events().find((e) => e['event.type'] === 'goal') ?? null;
        },
        { timeout: 20 * 60_000 }
      );
      const moment = await hook(page, () =>
        window.__touchline.signals().find((s) => s.signal === 'viewer.goal_moment') ?? null
      );
      expect(moment).not.toBeNull();
      expect(moment.frame_delta).toBe(0);
      const day = await hook(page, () => window.__touchline.matchDay());
      expect(day.score[0] + day.score[1]).toBeGreaterThanOrEqual(1);
      const scoreText = await page.locator('#score-bug').getAttribute('aria-label');
      expect(scoreText).toMatch(/Score \d+ – \d+/);
      const banner = await hook(page, () => document.getElementById('goal-banner-text').textContent);
      expect(banner.length).toBeGreaterThan(0);
      const rows = await feedRows(page);
      const row = rows.find((r) => r.kind === 'goal' && r.tick === goal.tick);
      expect(row, 'a feed row for the goal').toBeTruthy();
      const scorer = seen.hello.teams
        .flatMap((t) => t.roster ?? [])
        .find((p) => p['player.id'] === goal['player.id']);
      const scorerSurname = scorer ? scorer['player.name'].split(/\s+/).pop() : '';
      expect(row.text).toContain(scorerSurname);
      expect(row.text).toMatch(new RegExp(`\\b${goal.minute}'?`));
      // The line is the commentary the engine chose for the goal's score state: an opener,
      // an equaliser, a lead extended, or a consolation.
      expect(goal.commentary.length).toBeGreaterThan(0);
      expect(row.text).toContain(goal.commentary);
      // The score bug changed in the frame that drew the goal, to the goal's score.
      expect([moment['home.score'], moment['away.score']]).toEqual([goal['home.score'], goal['away.score']]);
      await shot(page, testInfo, 8);
    });

    await test.step('9. rewinding to the goal draws the stored positions', async () => {
      await scrubTo(page, goal.tick);
      const rewind = await until(page, (t) => {
        const r = window.__touchline.lastRewind();
        return r && r.tick === t ? r : null;
      }, { arg: goal.tick });
      expect(rewind.exact).toBe(true);
      const stored = await hook(page, (t) => window.__touchline.tickAt(t), goal.tick);
      expect(rewind.drawn).toEqual(stored);
      await shot(page, testInfo, 9);
    });

    await test.step('10. the half-time report counts equal the event counts', async () => {
      const report = await playUntil(page, () => {
        const r = window.__touchline.report();
        return r.open && r.kind === 'half-time' ? r : null;
      }, { timeout: 20 * 60_000 });
      const teamIds = seen.hello.teams.map((t) => t['team.id']);
      const events = await hook(page, () => window.__touchline.events());
      expect(report.counts).toEqual(countsFrom(events, teamIds, report.tick));
      // The engine's own records agree with the page.
      const records = readRecords(engine.dataDir, seen.hello['match.id']);
      expect(records.events).not.toBeNull();
      expect(countsFrom(records.events, teamIds, report.tick)).toEqual(report.counts);
      const cells = await page.locator('#report-table tbody tr').evaluateAll((rows) =>
        rows.map((tr) => [
          tr.dataset.row,
          Number(tr.querySelector('td[data-side="home"]').textContent),
          Number(tr.querySelector('td[data-side="away"]').textContent),
        ])
      );
      for (const [id, home, away] of cells) {
        expect([home, away]).toEqual(report.counts[id]);
      }
      await shot(page, testInfo, 10);
      await page.getByRole('button', { name: 'Continue' }).click();
    });

    await test.step('11. the full-time report shows, the replay saves, and the records are written', async () => {
      await playUntil(page, () => {
        const r = window.__touchline.report();
        return r.open && r.kind === 'full-time';
      }, { timeout: 25 * 60_000 });
      await expect(page.getByRole('dialog')).toBeVisible();
      const download = page.waitForEvent('download');
      await page.getByRole('button', { name: 'Save replay' }).click();
      const file = await download;
      expect(file.suggestedFilename()).toBe(`touchline-${seen.hello['match.id']}.smfx`);
      const saved = path.join(tempDir('replay'), file.suggestedFilename());
      await file.saveAs(saved);
      expect(statSync(saved).size).toBeGreaterThan(0);
      const records = readRecords(engine.dataDir, seen.hello['match.id']);
      expect(existsSync(path.join(records.folder, 'stats.json'))).toBe(true);
      expect(existsSync(path.join(records.folder, 'events.jsonl'))).toBe(true);
      await shot(page, testInfo, 11);
    });

    await test.step('12. headless: an AI manager trailing after minute 70 makes a tactical change', async () => {
      // The engine names each club by the identifier in its team file.
      const homeId = JSON.parse(readFileSync(teamA, 'utf8')).club.id;
      const found = [];
      for (let seed = 1; seed <= 50 && found.length === 0; seed += 1) {
        const data = tempDir(`scan-${seed}`);
        const run = runEngine(
          ['simulate', '--seed', String(seed), '--team-a', teamA, '--team-b', teamB,
            '--ticks-out', path.join(data, 'match.ticks'), '--no-snapshot'],
          { dataDir: data }
        );
        expect(run.code).toBe(0);
        const id = JSON.parse(run.stdout.trim().split('\n').pop())['match.id'];
        const { events } = readRecords(data, id);
        const change = events.find((e) => {
          if (e['event.type'] !== 'tactics-change' || e.minute < 70 || e['change.state'] !== 'applied') {
            return false;
          }
          const [own, other] = e['team.id'] === homeId ? ['home.score', 'away.score'] : ['away.score', 'home.score'];
          return e[own] < e[other];
        });
        const fullTime = events.find((e) => e['event.type'] === 'full-time');
        if (change && fullTime && change.tick <= fullTime.tick) {
          found.push({ seed, change });
        }
      }
      expect(found, 'a seed from 1 to 50 where the trailing AI manager changes tactics after minute 70').toHaveLength(1);
      testInfo.annotations.push({ type: 'trailing-change', description: JSON.stringify(found[0]) });
    });
  } finally {
    engine.cleanUp();
  }
});
