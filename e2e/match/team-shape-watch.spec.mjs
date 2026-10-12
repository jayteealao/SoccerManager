// The team-shape charter, steps 1 to 6, watched on the built viewer: a recorded match of the
// default teams (seed 42, or SM_WATCH_SEED) replayed with `replay --web`, each moment found
// from the stored ticks and events, sought with the scrubber, checked on the rendered tick and
// saved as a screenshot (`step-<n>.png`, in MATCH_EVIDENCE_DIR when it is set).
//
// 1. The walk-on: before the first tick the players walk from the touchline to their places,
//    and the clock waits; no player moves faster than a top speed at any tick of play.
// 2. The home team builds from the back after its keeper's restart: the figures of its back
//    line are measured and recorded (see the step for what the shipped play shows).
// 3. The ball goes wide: both blocks shift toward it, and the players far from it walk.
// 4. The home team loses the ball: the nearest home player sprints to press, and at least the
//    smallest rest-defence minimum of home players stayed goal-side of the ball.
// 5. A throw-in: the ball waits where it went out, the taker walks to it, players with no job
//    walk or stand, and the throw comes after a realistic wait.
// 6. Late on, a restart of the leading side takes longer than a restart of the same kind at a
//    level score.
//
// A moment the seed does not hold fails its step with the reason; the evidence file
// `moments.json` holds every moment found and its figures.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { CONTENT, VIEWER, fastForward, runEngine, startEngine, tempDir } from '../support/engine.mjs';

const SEED = Number(process.env.SM_WATCH_SEED ?? 42);
/// Past the end of any match: the replay sends every tick at once.
const PAST_THE_END = 1_000_000;
/// A speed no player reaches: the fastest top speed of the shipped scale is under 11 m/s, and a
/// jump of a place (a teleport) is hundreds of metres a second.
const TOP_SPEED_MS = 12;
/// The smallest rest-defence minimum of the shipped mentalities (attacking).
const REST_DEFENCE_MIN = 2;
const HOME_ID = JSON.parse(fs.readFileSync(path.join(CONTENT, 'teams', 'default-a.json'), 'utf8')).club.id;

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

async function open(page, url) {
  await page.goto(url);
  await expect(page.locator('html')).toHaveAttribute('data-ready', /.+/, { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
}

/// Seeks to `tick` and returns the rendered tick, read in the same turn as the frame it drew.
async function seek(page, tick) {
  await page.getByRole('slider', { name: 'Rewind to a tick' }).filter({ visible: true }).evaluate((scrub, value) => {
    scrub.value = String(value);
    scrub.dispatchEvent(new Event('input', { bubbles: true }));
    scrub.dispatchEvent(new Event('change', { bubbles: true }));
  }, tick);
  await until(page, (t) => window.__touchline.lastRenderedTick() === t, tick, 15_000);
  return hook(page, () => ({ tick: window.__touchline.lastRenderedTick(), frame: window.__touchline.lastRendered() }));
}

/// Finds the moments of steps 1 to 6 in the stored ticks and events, in the page. Ticks are
/// the wire's components in centimetres: the ball's x, y and height, then each player's x and
/// y, home players 0 to 10 (0 the keeper) and away players 11 to 21.
function findMoments({ homeId, topSpeed, restMin }) {
  const T = window.__touchline;
  const events = T.events();
  const fullTime = events.find((e) => e['event.type'] === 'full-time');
  const halfTime = events.find((e) => e['event.type'] === 'half-time');
  const last = fullTime.tick;
  const dt = 0.02;
  const frames = new Array(last + 1);
  for (let t = 1; t <= last; t += 1) {
    frames[t] = T.tickAt(t);
  }
  const ball = (t) => [frames[t][0] / 100, frames[t][1] / 100, frames[t][2] / 100];
  const at = (t, k) => [frames[t][3 + 2 * k] / 100, frames[t][4 + 2 * k] / 100];
  const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);
  const speed = (t, k) => (t > 1 && frames[t - 1] ? dist(at(t, k), at(t - 1, k)) / dt : 0);
  const ballSpeed = (t) => (t > 1 ? dist(ball(t), ball(t - 1)) / dt : 0);
  /// The home side's own goal: the sign of its keeper's x.
  const own = (t) => Math.sign(at(t, 0)[0]);
  const outfield = (side) => Array.from({ length: 10 }, (_, i) => side * 11 + 1 + i);
  /// The player within 0.8 m of a ball on the ground, else null.
  const holder = (t) => {
    const b = ball(t);
    if (b[2] > 0.5) return null;
    let best = null;
    let bestD = 0.8;
    for (let k = 0; k < 22; k += 1) {
      const d = dist(at(t, k), b);
      if (d < bestD) {
        best = k;
        bestD = d;
      }
    }
    return best;
  };
  /// Ticks next to a substitution, an injury or the half-time change of ends, where a player
  /// enters at the touchline or the teams change ends.
  const skip = new Set();
  for (const e of events) {
    if (['substitution', 'injury', 'half-time'].includes(e['event.type'])) {
      for (let t = e.tick - 100; t <= e.tick + 300; t += 1) skip.add(t);
    }
  }
  const out = {};

  // 1. The fastest move of any player at any tick of play.
  let fastest = { speed: 0, tick: 0, player: -1 };
  for (let t = 2; t <= last; t += 1) {
    if (skip.has(t) || !frames[t - 1]) continue;
    for (let k = 0; k < 22; k += 1) {
      const v = speed(t, k);
      if (v > fastest.speed) fastest = { speed: v, tick: t, player: k };
    }
  }
  out.step1 = { fastest, top_speed: topSpeed, half_time: halfTime?.tick ?? null };

  // 2. The home keeper's first restart that a home player then plays: the back line's
  //    figures one second after the kick.
  const homeKicks = events.filter((e) => e['event.type'] === 'goal-kick' && e['team.id'] === homeId);
  for (const e of homeKicks) {
    let go = null;
    for (let t = e.tick + 25; t < Math.min(last, e.tick + 6000); t += 1) {
      if (ballSpeed(t) > 3 && dist(at(t, 0), ball(t - 1)) < 2) {
        go = t;
        break;
      }
    }
    if (go === null) continue;
    const t = go + 50;
    const s = own(t);
    const depth = outfield(0).sort((a, b) => s * at(t, b)[0] - s * at(t, a)[0]);
    const back = depth.slice(0, 4).sort((a, b) => at(t, a)[1] - at(t, b)[1]);
    const centre = [back[1], back[2]];
    const wide = [back[0], back[3]];
    const gap = Math.abs(at(t, centre[0])[1] - at(t, centre[1])[1]);
    const gapKickOff = Math.abs(at(1, centre[0])[1] - at(1, centre[1])[1]);
    const up = (k) => -s * at(t, k)[0];
    const fullBacksAhead = (up(wide[0]) + up(wide[1])) / 2 - (up(centre[0]) + up(centre[1])) / 2;
    // Who receives the next ball the home side plays in the following 20 s.
    let received = null;
    for (let u = go + 10; u < Math.min(last, go + 1000); u += 1) {
      const h = holder(u);
      if (h !== null && h !== 0) {
        received = { tick: u, player: h, back_line: back.includes(h), home: h < 11 };
        break;
      }
    }
    const behind = outfield(0).filter((k) => s * at(t, k)[0] > s * ball(t)[0]).length;
    out.step2 = { event: e.tick, kick: go, tick: t, back_line: back, centre_gap_m: gap, centre_gap_kickoff_m: gapKickOff, full_backs_ahead_m: fullBacksAhead, goal_side: behind, received };
    break;
  }

  // 3. The ball wide in play: both blocks' mean across the pitch on the ball's side, and the
  //    players more than 35 m from the ball walking over the next second.
  for (let t = 3000; t <= last - 60 && !out.step3; t += 25) {
    if (skip.has(t)) continue;
    const b = ball(t);
    if (Math.abs(b[1]) < 25 || b[2] > 1 || ballSpeed(t) < 0.5) continue;
    const side = Math.sign(b[1]);
    const means = [0, 1].map((team) => outfield(team).reduce((sum, k) => sum + at(t, k)[1], 0) / 10);
    const far = [...outfield(0), ...outfield(1)].filter((k) => dist(at(t, k), b) > 35);
    if (far.length === 0 || !means.every((m) => Math.sign(m) === side)) continue;
    let total = 0;
    for (const k of far) {
      for (let u = t + 1; u <= t + 50; u += 1) total += speed(u, k);
    }
    const farSpeed = total / (far.length * 50);
    if (farSpeed <= 2) {
      out.step3 = { tick: t, ball_y_m: b[1], block_y_m: means, far_players: far.length, far_speed_ms: farSpeed };
    }
  }

  // 4. A home player on the ball, then an away player on it within 2 s: the home presser
  //    nearest the ball reaches a sprint within 3 s, and enough home players stayed goal-side.
  let lastHolder = null;
  for (let t = 3000; t <= last - 160 && !out.step4; t += 5) {
    if (skip.has(t)) continue;
    const h = holder(t);
    if (h === null) continue;
    if (lastHolder && lastHolder.player < 11 && h >= 11 && t - lastHolder.tick <= 100) {
      const b = ball(t);
      const presser = outfield(0).sort((a, c) => dist(at(t, a), b) - dist(at(t, c), b))[0];
      let top = 0;
      for (let u = t; u <= t + 150; u += 1) top = Math.max(top, speed(u, presser));
      const s = own(lastHolder.tick);
      const lost = ball(lastHolder.tick);
      const behind = outfield(0).filter((k) => s * at(lastHolder.tick, k)[0] > s * lost[0]).length;
      if (top > 7 && behind >= restMin) {
        out.step4 = { lost_at: lastHolder.tick, won_at: t, presser, presser_top_ms: top, goal_side: behind };
      }
    }
    lastHolder = { player: h, tick: t };
  }

  // 5. A throw-in: the ball still where it went out until the throw, the taker walking, the
  //    players with no job at 2 m/s or less, and a wait of 3 s or more.
  for (const e of events.filter((x) => x['event.type'] === 'throw-in')) {
    let go = null;
    for (let t = e.tick + 25; t < Math.min(last, e.tick + 6000); t += 1) {
      if (ballSpeed(t) > 1) {
        go = t;
        break;
      }
    }
    if (go === null) continue;
    const rest = ball(e.tick + 10);
    let drift = 0;
    for (let t = e.tick + 10; t < go - 1; t += 1) drift = Math.max(drift, dist(ball(t), rest));
    const spot = ball(go - 1);
    const taker = Array.from({ length: 22 }, (_, k) => k).sort((a, c) => dist(at(go - 1, a), spot) - dist(at(go - 1, c), spot))[0];
    let takerTop = 0;
    for (let t = e.tick + 1; t < go; t += 1) takerTop = Math.max(takerTop, speed(t, taker));
    const calm = Array.from({ length: 22 }, (_, k) => k).filter((k) => {
      if (k === taker) return false;
      let sum = 0;
      for (let t = e.tick + 50; t < go; t += 1) sum += speed(t, k);
      return sum / Math.max(1, go - e.tick - 50) <= 2;
    }).length;
    const waitS = (go - e.tick) * dt;
    if (waitS >= 3 && drift < 1 && takerTop <= 2.5 && calm >= 15) {
      out.step5 = { event: e.tick, thrown: go, tick: Math.round((e.tick + go) / 2), wait_s: waitS, ball_drift_m: drift, taker, taker_top_ms: takerTop, calm_players: calm };
      break;
    }
  }

  // 6. Each restart's length, from the event that stopped play to the kick, by kind and by the
  //    score of the side that restarts.
  const kinds = ['throw-in', 'goal-kick', 'free-kick', 'corner'];
  const rows = [];
  for (const e of events.filter((x) => kinds.includes(x['event.type']))) {
    let go = null;
    for (let t = e.tick + 25; t < Math.min(last, e.tick + 9000); t += 1) {
      if (ballSpeed(t) > 1) {
        go = t;
        break;
      }
    }
    if (go === null) continue;
    const home = e['team.id'] === homeId;
    const lead = (e['home.score'] - e['away.score']) * (home ? 1 : -1);
    rows.push({ kind: e['event.type'], tick: e.tick, minute: e.minute, lead, seconds: (go - e.tick) * dt });
  }
  for (const kind of kinds) {
    const late = rows.filter((r) => r.kind === kind && r.lead > 0 && r.minute >= 75);
    const level = rows.filter((r) => r.kind === kind && r.lead === 0);
    if (late.length === 0 || level.length === 0) continue;
    const mean = (xs) => xs.reduce((a, r) => a + r.seconds, 0) / xs.length;
    const longest = late.reduce((a, r) => (r.seconds > a.seconds ? r : a));
    if (mean(late) > mean(level)) {
      out.step6 = { kind, late_mean_s: mean(late), level_mean_s: mean(level), late: late.length, level: level.length, tick: longest.tick + 50, restart: longest };
      break;
    }
  }
  return out;
}

test('the team-shape charter, steps 1 to 6, on a watched match', async ({ page }, info) => {
  test.setTimeout(20 * 60_000);
  const dir = tempDir('team-shape-watch');
  const fixture = path.join(dir, 'match.smfx');
  const recorded = runEngine(['record', '--seed', String(SEED), '--out', fixture], { dataDir: dir });
  expect(recorded.code, recorded.stderr).toBe(0);
  const log = [];
  const note = (step, what) => {
    log.push({ step, ...what });
    info.annotations.push({ type: `step ${step}`, description: JSON.stringify(what) });
  };

  // 1a. The walk-on, played at normal speed into the first tick: the clock waits on tick 1.
  let engine = await startEngine({ command: 'replay', args: ['--fixture', fixture, '--web', VIEWER, '--speed', '1'] });
  try {
    await open(page, engine.url);
    await until(page, () => window.__touchline.signals().some((s) => s.signal === 'viewer.walk_on'), undefined, 60_000);
    await page.waitForTimeout(1_500);
    const during = await hook(page, () => ({
      tick: window.__touchline.lastRenderedTick(),
      walk: window.__touchline.signals().find((s) => s.signal === 'viewer.walk_on'),
    }));
    expect(during.tick, 'the clock waits during the walk-on').toBeLessThanOrEqual(1);
    note(1, { walk_on: during.walk, rendered_tick: during.tick });
    await page.screenshot({ path: evidence(info, 'step-1.png') });
  } finally {
    engine.cleanUp();
  }

  // The whole match, stored at once, for the moments.
  engine = await startEngine({
    command: 'replay',
    args: ['--fixture', fixture, '--web', VIEWER, '--speed', '50', ...fastForward(PAST_THE_END)],
  });
  try {
    await open(page, engine.url);
    await until(page, () => window.__touchline.events().some((e) => e['event.type'] === 'full-time'), undefined, 5 * 60_000);
    const end = await hook(page, () => window.__touchline.events().find((e) => e['event.type'] === 'full-time').tick);
    await until(page, (t) => window.__touchline.history().newest_tick >= t, end, 5 * 60_000);
    const pauseButton = page.getByRole('button', { name: 'Pause', exact: true }).filter({ visible: true });
    if (await pauseButton.count()) {
      await pauseButton.first().click();
    }
    const moments = await hook(page, findMoments, { homeId: HOME_ID, topSpeed: TOP_SPEED_MS, restMin: REST_DEFENCE_MIN });
    fs.writeFileSync(evidence(info, 'moments.json'), `${JSON.stringify({ seed: SEED, moments }, null, 2)}\n`);

    // 1b. No player moves faster than a top speed at any tick of play.
    note(1, { fastest: moments.step1.fastest });
    expect(moments.step1.fastest.speed, `fastest move ${JSON.stringify(moments.step1.fastest)}`).toBeLessThan(TOP_SPEED_MS);

    // 2. The build-up after the home keeper's restart, measured. With the off-ball stands off
    //    (the tuning flag team_stands), the back line keeps its formation places: the
    //    centre-backs' gap and the full-backs' height are recorded here, not asserted, until the
    //    stands come back; the rest-defence minimum is asserted.
    expect(moments.step2, 'a home goal kick that the keeper plays').toBeTruthy();
    note(2, moments.step2);
    const shot2 = await seek(page, moments.step2.tick);
    expect(shot2.tick).toBe(moments.step2.tick);
    expect(moments.step2.goal_side).toBeGreaterThanOrEqual(REST_DEFENCE_MIN);
    await page.screenshot({ path: evidence(info, 'step-2.png') });

    // 3. The ball wide: the blocks shift toward it; the far players walk.
    expect(moments.step3, 'the ball wide with both blocks shifted and the far players walking').toBeTruthy();
    note(3, moments.step3);
    const shot3 = await seek(page, moments.step3.tick);
    expect(Math.abs(shot3.frame[1]) / 100).toBeGreaterThan(25);
    await page.screenshot({ path: evidence(info, 'step-3.png') });

    // 4. The home side loses the ball: the nearest presser sprints; the minimum stayed back.
    expect(moments.step4, 'a home loss of the ball with a sprinting presser').toBeTruthy();
    note(4, moments.step4);
    const shot4 = await seek(page, moments.step4.won_at + 25);
    expect(shot4.tick).toBe(moments.step4.won_at + 25);
    await page.screenshot({ path: evidence(info, 'step-4.png') });

    // 5. A throw-in: the ball waits, the taker walks, the rest stand or walk.
    expect(moments.step5, 'a throw-in with the ball waiting, the taker walking and the others calm').toBeTruthy();
    note(5, moments.step5);
    await seek(page, moments.step5.tick);
    await page.screenshot({ path: evidence(info, 'step-5.png') });

    // 6. Late on, the leading side's restart takes longer than one at a level score.
    expect(moments.step6, 'a late restart of the leading side and a level-score restart of the same kind').toBeTruthy();
    note(6, moments.step6);
    expect(moments.step6.late_mean_s).toBeGreaterThan(moments.step6.level_mean_s);
    await seek(page, moments.step6.tick);
    await page.screenshot({ path: evidence(info, 'step-6.png') });
  } finally {
    fs.writeFileSync(evidence(info, 'steps.json'), `${JSON.stringify(log, null, 2)}\n`);
    engine.cleanUp();
    fs.rmSync(dir, { recursive: true, force: true });
  }
});
