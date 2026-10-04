// Resuming a saved match from the launcher, on the new viewer served by the release engine
// with `--web viewer/dist`.
//
// Screenshots, each in both skins at 1280 by 800, against their baselines:
// - the Resume a saved match screen: `launch --resume` with the committed save stamped as
//   release 0.1.0, which no engine of this version finishes.
// The light skin is chosen by configuration only (a content copy's slots.json).
//
// Drives: the refusal shows before any engine starts and names the save's version; a Tab walk
// never lands in a stub, and the rendered contrast check passes; Start a new match asks the
// launcher for a fresh match. With SM_PREVIOUS_ENGINE_PATH set, a save of the previous release
// opens straight on the live match view on that release's engine, watched for its first frames
// only.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { contrastReport } from '../support/contrast.mjs';
import { REPO, VIEWER, contentWithSkin, startEngine, tempDir } from '../support/engine.mjs';
import { clickClear, snap } from '../support/page.mjs';

const SKINS = ['broadcast-blue', 'interim-light'];
/// The committed save: format 8, stamped as Touchline 0.1.0, stopped at 52:10.
const OLD_SAVE = path.join(REPO, 'e2e', 'match', 'fixtures', 'saved-0.1.0.smsn');
const PREVIOUS = process.env.SM_PREVIOUS_ENGINE_PATH ? path.resolve(process.env.SM_PREVIOUS_ENGINE_PATH) : null;

const fixture = {};

test.beforeAll(() => {
  fixture.light = contentWithSkin('interim-light');
});

const evidence = (info, name) => {
  const out = process.env.MATCH_EVIDENCE_DIR
    ? path.join(process.env.MATCH_EVIDENCE_DIR, name)
    : info.outputPath(name);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  return out;
};

const hook = (page, fn, arg) => page.evaluate(fn, arg);
const until = (page, fn, arg, timeout = 30_000) => page.waitForFunction(fn, arg, { timeout, polling: 100 });

async function open(page, url, skin = 'broadcast-blue') {
  await page.goto(url);
  await expect(page.locator('html')).toHaveAttribute('data-ready', skin, { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
}

/// The launcher with the old save to resume.
async function launchOld(skin = 'broadcast-blue') {
  return startEngine({
    command: 'launch',
    args: ['--resume', OLD_SAVE, '--web', VIEWER],
    env: skin === 'broadcast-blue' ? {} : { SM_CONTENT_DIR: fixture.light },
  });
}

const status = (launcher) => fetch(`${launcher.url}engine.json`).then((r) => r.json());

/// Opens the page and waits for the Resume a saved match screen.
async function openRefusal(page, launcher, skin) {
  await open(page, launcher.url, skin);
  await until(page, () => window.__touchline.resume().shown);
  await expect(page.locator('[data-screen="resume"]')).toBeVisible();
}

/// Presses Tab `presses` times and records where focus lands.
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

for (const skin of SKINS) {
  test.describe(`${skin} skin`, () => {
    test('the Resume a saved match screen for a save two or more versions back', { tag: '@sizes' }, async ({ page }) => {
      const launcher = await launchOld(skin);
      try {
        await openRefusal(page, launcher, skin);
        await snap(page, `resume-refused-${skin}.png`);
      } finally {
        launcher.cleanUp();
      }
    });
  });
}

test('an old save is refused before any engine starts, and the alert names its version', async ({ page }, info) => {
  const launcher = await launchOld();
  try {
    const body = await status(launcher);
    fs.writeFileSync(evidence(info, 'resume-engine.json'), `${JSON.stringify(body, null, 2)}\n`);
    expect(body['engine.state']).toBe('refused');
    expect(body['engine.pid'] ?? null).toBeNull();
    expect(body.resume).toMatchObject({ kind: 'older', 'saved.version': '0.1.0', 'saved.tick': 156_500 });

    await openRefusal(page, launcher, 'broadcast-blue');
    expect(await hook(page, () => window.__touchline.resume())).toMatchObject({
      shown: true,
      kind: 'older',
      saved_version: '0.1.0',
      saved_tick: 156_500,
      engine_pid: null,
    });
    const alert = page.getByRole('alert');
    await expect(alert).toContainText('Cannot resume');
    await expect(alert).toContainText('This match was saved by Touchline 0.1.0');
    await expect(page.locator('[data-screen="resume"]')).toContainText('The save stays on disk. Nothing is deleted.');
    // The page asked nothing of the launcher: still no worker, and the save is untouched.
    expect((await status(launcher))['engine.pid'] ?? null).toBeNull();
    expect(fs.existsSync(OLD_SAVE)).toBe(true);
  } finally {
    launcher.cleanUp();
  }
});

test('a Tab walk on the resume screen never lands in a stub in either skin, and it meets WCAG AA contrast', { tag: '@sizes' }, async ({ page }, info) => {
  const walks = {};
  const reports = {};
  for (const skin of SKINS) {
    const launcher = await launchOld(skin);
    try {
      await openRefusal(page, launcher, skin);
      walks[skin] = await tabWalk(page, 8);
      reports[skin] = await contrastReport(page);
    } finally {
      launcher.cleanUp();
    }
  }
  fs.writeFileSync(evidence(info, 'resume-tab-walk.json'), `${JSON.stringify(walks, null, 2)}\n`);
  fs.writeFileSync(evidence(info, 'resume-contrast-report.json'), `${JSON.stringify(reports, null, 2)}\n`);
  for (const skin of SKINS) {
    for (const step of walks[skin]) {
      expect(step.stub, `${skin} press ${step.press}: ${step.name}`).toBeNull();
      expect(step.inert, `${skin} press ${step.press}: ${step.name}`).toBe(false);
      expect(step.hidden, `${skin} press ${step.press}: ${step.name}`).toBe(false);
    }
    const names = walks[skin].map((s) => s.name);
    for (const name of ['New match', 'Saved matches', 'Start a new match', 'Open a replay']) {
      expect(names, skin).toContain(name);
    }
  }
  // The default skin carries the WCAG AA commitment, as on every other screen; the interim
  // light skin's report is kept as evidence (its fact strip predates this screen).
  expect(reports['broadcast-blue'].checked).toBeGreaterThan(10);
  expect(reports['broadcast-blue'].failures, JSON.stringify(reports['broadcast-blue'].failures, null, 2)).toEqual([]);
});

test('Start a new match asks the launcher for a fresh match and leaves the resume screen', async ({ page }) => {
  const launcher = await launchOld();
  try {
    await openRefusal(page, launcher, 'broadcast-blue');
    await page.getByRole('button', { name: 'Start a new match', exact: true }).click();
    await until(page, () => ['tactics', 'match'].includes(window.__touchline.view()));
    await expect(page.locator('[data-screen="resume"]')).toHaveCount(0);
    const body = await status(launcher);
    expect(body['engine.state']).toBe('running');
    expect(body.resume ?? null).toBeNull();
  } finally {
    launcher.cleanUp();
  }
});

test.describe('a save of the previous release', () => {
  test.skip(!PREVIOUS, 'needs SM_PREVIOUS_ENGINE_PATH, the previous release engine built from its tag');

  test('opens straight on the live match view on its own engine', async ({ page }, info) => {
    test.setTimeout(3 * 60_000);
    const content = path.join(path.dirname(PREVIOUS), 'content');
    const data = tempDir('previous-save');
    // The previous program plays a whole match and leaves its last snapshot behind.
    const run = spawnSync(PREVIOUS, ['--content-dir', content, 'simulate', '--seed', '42', '--ticks-out', path.join(data, 'whole.ticks')], {
      env: { ...process.env, SM_DATA_DIR: data },
      encoding: 'utf8',
      maxBuffer: 64 * 1024 * 1024,
    });
    expect(run.status, run.stderr).toBe(0);
    const id = JSON.parse(run.stdout)['match.id'];
    const save = path.join(data, 'matches', id, 'snapshot.smsn');
    const launcher = await startEngine({
      command: 'launch',
      args: ['--resume', save, '--previous', PREVIOUS, '--web', VIEWER],
    });
    try {
      await open(page, launcher.url, 'broadcast-blue');
      // The first frames only: the screen opens live, and the engine draws a tick.
      await until(page, () => window.__touchline.view() === 'match' && window.__touchline.screen() === 'live', undefined, 60_000);
      await until(page, () => window.__touchline.lastRenderedTick() > 0, undefined, 30_000);
      const version = await hook(page, () => window.__touchline.engineVersion());
      fs.writeFileSync(evidence(info, 'resume-previous-engine.json'), `${JSON.stringify(version, null, 2)}\n`);
      expect(version.engine).toBe('0.2.0-beta.1');
      expect(version.resumed_from).not.toBeNull();
      expect(version.steps.join(' ')).toContain('0.2.0-beta.1');
      expect(await hook(page, () => window.__touchline.resume().shown)).toBe(false);
      await page.screenshot({ path: evidence(info, 'drive-previous-live.png') });
    } finally {
      launcher.cleanUp();
      fs.rmSync(data, { recursive: true, force: true });
    }
  });
});
