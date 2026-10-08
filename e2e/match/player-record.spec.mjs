// The player record on the built viewer, end to end: the states the boards draw, each driven
// the way a player reaches it, served by the release program's `launch` or `replay`.
//
// 1. The scenario on the shipped teams (version 1 files), seed 7, the whole match sent at once:
//    match setup names the converted home club; the Squad screen shows whole numbers 1 to 20
//    in the four bands, and Height is removed, added back, moved by drag and sorted both ways;
//    a player's panel shows his attributes, his build word, the line that the file holds no
//    height or age, and "Not yet known" for both hidden values; at minute 67 the Touchline
//    shows a tired player's level below his fresh level ("N of M"); after full time the panel
//    shows his match rating.
// 2. A version 2 home team whose first three players have 0, 9 and 31 matches at the club:
//    setup shows no notice; the panel shows height, age and nationality, and reads not yet
//    known, tentative and firm.
// 3. The committed old replay (release 0.1.0, a protocol 3 hello), streamed by `replay`: no
//    notice and no value off the 1 to 20 scale on the match, Tactics and Squad screens.
//
// Screenshots of each state are kept as evidence.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { CONTENT, REPO, VIEWER, frontDoor, startEngine, tempDir } from '../support/engine.mjs';
import { kickOffFromStart, open, pastSplash, rewindTo, settled, toFullTime } from '../support/front.mjs';

const SEED = 7;
/// Past the end of any match: the engine sends the whole match at once.
const PAST_THE_END = 1_000_000;
/// Minute 67 at 50 ticks a second.
const MINUTE_67 = 201_000;
/// The committed replay recorded by release 0.1.0: a protocol 3 hello on 1 to 100.
const OLD_REPLAY = path.join(REPO, 'viewer', 'tests', 'data', 'one-minute-v4.smfx');
/// A version 2 team file of the home club, from the engine's test fixtures.
const V2_HOME = path.join(REPO, 'crates', 'engine', 'tests', 'fixtures', 'teams', 'condition-good.json');

const until = (page, fn, arg, timeout = 120_000) => page.waitForFunction(fn, arg, { timeout, polling: 100 });
const hook = (page, fn, arg) => page.evaluate(fn, arg);

const evidence = (info, name) => {
  const out = process.env.MATCH_EVIDENCE_DIR ? path.join(process.env.MATCH_EVIDENCE_DIR, name) : info.outputPath(name);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  return out;
};

const tab = (page, name) => page.locator('nav.subnav:visible').getByRole('button', { name, exact: true });
const squad = (page) => page.locator('[data-screen="squad"]:visible');
const panel = (page) => page.locator('[data-screen="player"]:visible');
const shownColumns = (menu) => menu.locator('li[data-shown]').evaluateAll((lis) => lis.map((li) => li.dataset.shown));
const headers = (page) =>
  squad(page)
    .locator('[data-squad-table] thead th[data-col]')
    .evaluateAll((ths) => ths.map((th) => th.dataset.col));

/// Every attribute and level cell of the Squad table holds a whole number 1 to 20 with a band
/// class; returns the values read.
async function scaleScan(page) {
  const found = await squad(page).evaluate((root) => {
    const values = [];
    const bad = [];
    for (const td of root.querySelectorAll('td[data-cell^="attr:"], td[data-cell="level"]')) {
      const text = td.textContent.trim();
      if (text === '—') continue;
      values.push(Number(text));
      const banded = td.querySelector('.v1, .v2, .v3, .v4') ?? (td.matches('.v1, .v2, .v3, .v4') ? td : null);
      if (!/^\d+$/.test(text) || Number(text) < 1 || Number(text) > 20) bad.push(`${td.dataset.cell}: ${text}`);
      if (!banded) bad.push(`${td.dataset.cell}: no band class`);
    }
    // No decimal and no value over 20 anywhere in the table's text outside the rating chips.
    for (const td of root.querySelectorAll('td[data-cell]:not([data-cell="rating"])')) {
      if (/\b\d+\.\d\b/.test(td.textContent)) bad.push(`decimal in ${td.dataset.cell}: ${td.textContent.trim()}`);
    }
    return { values, bad };
  });
  expect(found.bad, found.bad.join('\n')).toEqual([]);
  expect(found.values.length).toBeGreaterThan(50);
  return found.values;
}

/// No converted-file notice anywhere on the page.
async function noNotice(page) {
  await expect(page.locator('[data-converted]')).toHaveCount(0);
}

/// Opens the Squad screen from the sub-navigation of the view on show.
async function openSquad(page) {
  await tab(page, 'Squad').click();
  await until(page, () => window.__touchline.view() === 'squad', undefined, 10_000);
  await expect(squad(page).locator('[data-squad-table] tbody tr[data-row]').first()).toBeVisible();
  await settled(page);
}

/// Opens the panel of the player in squad row `index`.
async function openPanel(page, index) {
  await squad(page).locator(`tbody tr[data-row="${index}"] button.who`).click();
  await until(page, () => window.__touchline.view() === 'player', undefined, 10_000);
  await expect(panel(page)).toBeVisible();
  await settled(page);
}

/// Back from the panel to the Squad screen.
async function backToSquad(page) {
  await tab(page, 'Squad').click();
  await until(page, () => window.__touchline.view() === 'squad', undefined, 10_000);
}

/// A copy of the content folder whose home team file is the version 2 fixture, with its first
/// three players at 0, 9 and 31 matches at the club.
function contentWithV2Home() {
  const dir = tempDir('content-v2-home');
  fs.cpSync(CONTENT, dir, { recursive: true });
  const team = JSON.parse(fs.readFileSync(V2_HOME, 'utf8'));
  for (const [i, matches] of [0, 9, 31].entries()) {
    team.players[i].condition = { ...(team.players[i].condition ?? {}), matches_at_club: matches };
  }
  fs.writeFileSync(path.join(dir, 'teams', 'default-a.json'), `${JSON.stringify(team, null, 2)}\n`);
  return { dir, names: team.players.slice(0, 3).map((p) => p.name), nations: team.players.slice(0, 3).map((p) => p.nationality) };
}

test('the scenario on the shipped teams: notice, squad columns, panel, a tired player at 67:00 and the match rating', async ({
  page,
}, info) => {
  test.setTimeout(10 * 60_000);
  const dataDir = tempDir('player-record');
  const engine = await frontDoor({ args: ['--seed', String(SEED)], dataDir, fastForwardTo: PAST_THE_END });
  try {
    // Step 1: the shipped version 1 files load; setup names the converted home club.
    await open(page, engine.url);
    await pastSplash(page);
    await page.locator('[data-choice="new"]').click();
    await until(page, () => window.__touchline.frontDoor().round !== null, undefined, 10_000);
    const notice = page.locator('[data-converted]');
    await expect(notice).toHaveCount(1);
    await expect(notice).toHaveAttribute('data-converted', 'Oakmere Rangers');
    await expect(notice).toContainText('converted to the 1-20 scale');
    await settled(page);
    await page.screenshot({ path: evidence(info, 'step1-setup-converted.png') });

    // Step 3: Tactics, then the Squad screen on 1 to 20.
    await page.locator('[data-kickoff]').click();
    await until(page, () => window.__touchline.view() === 'tactics', undefined, 30_000);
    await openSquad(page);
    const values = await scaleScan(page);
    console.log(`squad: ${values.length} values, ${Math.min(...values)} to ${Math.max(...values)}`);
    await page.screenshot({ path: evidence(info, 'step3-squad-default.png') });

    // Remove Height, add it back, then move it by drag before Pace.
    await squad(page).locator('[data-columns-chip]').click();
    const menu = squad(page).locator('[data-column-menu]');
    await expect(menu).toBeVisible();
    await menu.locator('li[data-shown="height"] [data-control="remove"]').click();
    expect(await shownColumns(menu)).not.toContain('height');
    await menu.locator('button[data-add="height"]').click();
    let shown = await shownColumns(menu);
    expect(shown.at(-1)).toBe('height');
    await menu.locator('li[data-shown="height"]').dragTo(menu.locator('li[data-shown="attr:pace"]'));
    shown = await shownColumns(menu);
    expect(shown.indexOf('height'), shown.join(',')).toBeLessThan(shown.indexOf('attr:pace'));
    expect(shown.indexOf('height')).toBe(shown.indexOf('attr:pace') - 1);
    await settled(page);
    await page.screenshot({ path: evidence(info, 'step3-menu-dragged.png') });
    await menu.locator('[data-done]').click();
    await expect(menu).toHaveCount(0);
    const cols = await headers(page);
    expect(cols.indexOf('height')).toBe(cols.indexOf('attr:pace') - 1);

    // Sort by Height, then reverse; the rows follow.
    const table = squad(page).locator('[data-squad-table]');
    const heights = () =>
      table.locator('td[data-cell="height"]').evaluateAll((tds) => tds.map((td) => Number.parseInt(td.textContent, 10)));
    await table.locator('th[data-col="height"] button').click();
    await expect(table).toHaveAttribute('data-sort', 'height:down');
    await expect(table.locator('th[data-col="height"]')).toHaveAttribute('aria-sort', 'descending');
    const down = await heights();
    expect(down).toEqual([...down].sort((a, b) => b - a));
    await table.locator('th[data-col="height"] button').click();
    await expect(table).toHaveAttribute('data-sort', 'height:up');
    const up = await heights();
    expect(up).toEqual([...up].sort((a, b) => a - b));
    await page.screenshot({ path: evidence(info, 'step3-squad-sorted-up.png') });

    // Step 4: a player's panel.
    await table.locator('th[data-col="height"] button').click();
    const first = await squad(page).locator('tbody tr[data-row]').first().getAttribute('data-row');
    await openPanel(page, first);
    const p = panel(page);
    const attrs = await p.locator('[data-attr] dd').allTextContents();
    expect(attrs.length).toBeGreaterThan(20);
    for (const v of attrs) {
      expect(v.trim()).toMatch(/^\d+$/);
      expect(Number(v)).toBeGreaterThanOrEqual(1);
      expect(Number(v)).toBeLessThanOrEqual(20);
    }
    // The shipped files are version 1: they carry no height, age or nationality, and the panel
    // says so rather than inventing them. The build word derives from the attributes.
    const body = (await p.locator('[data-body]').innerText()).trim();
    expect(body).toMatch(/\b(slight|athletic|powerful)\b/i);
    expect(body).toContain('Height and age are not in this team file.');
    const whole = await p.innerText();
    for (const name of ['consistency', 'injury_proneness']) {
      await expect(p.locator(`[data-hidden="${name}"]`)).toHaveAttribute('data-confidence', 'not_yet_known');
      await expect(p.locator(`[data-hidden="${name}"]`)).toContainText('Not yet known');
    }
    expect(whole).not.toMatch(/consistency\s*\d|injury[ _]proneness\s*\d/i);
    await page.screenshot({ path: evidence(info, 'step4-panel-new.png') });

    // Step 5: kick off; the whole match arrives; at 67:00 a tired player is below his fresh
    // level on the Touchline.
    await backToSquad(page);
    await tab(page, 'Tactics').click();
    await until(page, () => window.__touchline.view() === 'tactics', undefined, 10_000);
    const action = page.locator('header button.cont:visible');
    await expect(action).toHaveText('Continue');
    await action.click();
    await until(page, () => window.__touchline.view() === 'prematch', undefined, 5_000);
    await action.click();
    await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
    await until(page, () => window.__touchline.events().some((e) => e['event.type'] === 'full-time'), undefined, 180_000);
    const playback = page.getByRole('group', { name: 'Playback' }).filter({ visible: true });
    if (await playback.getByRole('button', { name: 'Pause', exact: true }).isVisible()) {
      await playback.getByRole('button', { name: 'Pause', exact: true }).click();
    }
    await rewindTo(page, MINUTE_67);
    await tab(page, 'Touchline').click();
    await until(page, () => window.__touchline.view() === 'touchline', undefined, 5_000);
    await settled(page);
    const read = await page.evaluate(() => ({
      tick: window.__touchline.lastRenderedTick(),
      rows: Array.from(document.querySelectorAll('[data-screen="touchline"] td[data-level], td[data-level]'))
        .filter((td) => td.offsetParent !== null)
        .map((td) => ({ state: td.dataset.level, text: td.textContent.replace(/\s+/g, ' ').trim(), label: td.getAttribute('aria-label') })),
    }));
    fs.writeFileSync(evidence(info, 'step5-touchline-levels.json'), `${JSON.stringify(read, null, 2)}\n`);
    expect(read.tick).toBe(MINUTE_67);
    expect(read.rows.length).toBeGreaterThanOrEqual(11);
    const below = read.rows.filter((r) => r.state === 'below');
    expect(below.length, JSON.stringify(read.rows)).toBeGreaterThan(0);
    for (const r of below) {
      const m = r.text.match(/^(\d+) of (\d+)$/);
      expect(m, r.text).not.toBeNull();
      expect(Number(m[1])).toBeLessThan(Number(m[2]));
      expect(Number(m[2])).toBeLessThanOrEqual(20);
    }
    for (const r of read.rows.filter((x) => x.state === 'equal')) expect(r.text).toBe('=');
    await page.screenshot({ path: evidence(info, 'step5-touchline-67.png') });

    // Step 6: to full time; the panel shows the match rating with one decimal.
    await tab(page, 'Match').click();
    await until(page, () => window.__touchline.view() === 'match', undefined, 5_000);
    await toFullTime(page);
    await openSquad(page);
    await openPanel(page, first);
    await expect(panel(page).locator('[data-ratings]')).toHaveAttribute('data-ratings', 'some');
    const chips = await panel(page).locator('[data-ratings] .rt').allTextContents();
    expect(chips.length).toBe(1);
    expect(chips[0].trim()).toMatch(/^\d{1,2}\.\d$/);
    expect(Number(chips[0])).toBeGreaterThanOrEqual(1);
    expect(Number(chips[0])).toBeLessThanOrEqual(10);
    await page.screenshot({ path: evidence(info, 'step6-panel-rating.png') });
  } finally {
    engine.cleanUp();
  }
});

test('a version 2 home team: no notice, and the words read not yet known, tentative and firm', async ({ page }, info) => {
  test.setTimeout(4 * 60_000);
  const content = contentWithV2Home();
  const engine = await frontDoor({ env: { SM_CONTENT_DIR: content.dir } });
  try {
    await open(page, engine.url);
    await pastSplash(page);
    await page.locator('[data-choice="new"]').click();
    await until(page, () => window.__touchline.frontDoor().round !== null, undefined, 10_000);
    await settled(page);
    await noNotice(page);
    await page.screenshot({ path: evidence(info, 'v2-setup-no-notice.png') });
    await page.locator('[data-kickoff]').click();
    await until(page, () => window.__touchline.view() === 'tactics', undefined, 30_000);
    await openSquad(page);
    await scaleScan(page);
    const expected = [
      ['not_yet_known', /Not yet known/],
      ['tentative', /9 matches here, tentative/],
      ['firm', /31 matches here, sure/],
    ];
    for (const [i, [confidence, line]] of expected.entries()) {
      const row = squad(page).locator('tbody tr[data-row]').filter({ hasText: content.names[i] });
      const index = await row.getAttribute('data-row');
      await openPanel(page, index);
      // A version 2 file carries the body: height, age and nationality show.
      const body = (await panel(page).locator('[data-body]').innerText()).trim();
      expect(body).toMatch(/\b(slight|athletic|powerful)\b/i);
      expect(body).toMatch(/\b1\d{2} cm\b/);
      expect(body).toMatch(/\bAge \d{2}\b/);
      expect(await panel(page).innerText()).toContain(content.nations[i]);
      for (const name of ['consistency', 'injury_proneness']) {
        const cell = panel(page).locator(`[data-hidden="${name}"]`);
        await expect(cell).toHaveAttribute('data-confidence', confidence);
        await expect(cell).toContainText(line);
        expect(await cell.innerText()).not.toMatch(/\d+\.\d|\b(1[0-9]|20|[1-9])\b(?! matches)/);
      }
      await page.screenshot({ path: evidence(info, `v2-panel-${confidence}.png`) });
      await backToSquad(page);
    }
  } finally {
    engine.cleanUp();
    fs.rmSync(content.dir, { recursive: true, force: true });
  }
});

test('the committed old replay: no notice, and no value off the 1 to 20 scale on any screen', async ({ page }, info) => {
  test.setTimeout(4 * 60_000);
  const engine = await startEngine({
    command: 'replay',
    args: ['--fixture', OLD_REPLAY, '--web', VIEWER, '--speed', '1'],
  });
  /// No converted notice, and no decimal or number over 20 in the screen's rating cells.
  const clean = async (where) => {
    await noNotice(page);
    const bad = await page.evaluate(() =>
      Array.from(document.querySelectorAll('td[data-cell^="attr:"], td[data-cell="level"], [data-attr] dd'))
        .filter((el) => el.offsetParent !== null)
        .map((el) => el.textContent.trim())
        .filter((t) => t !== '—' && (!/^\d+$/.test(t) || Number(t) > 20))
    );
    expect(bad, where).toEqual([]);
  };
  try {
    await open(page, engine.url);
    await until(page, () => ['tactics', 'match'].includes(window.__touchline.view()), undefined, 30_000);
    await clean('match');
    await page.screenshot({ path: evidence(info, 'old-replay-match.png') });
    if ((await hook(page, () => window.__touchline.view())) !== 'tactics') {
      await tab(page, 'Tactics').click();
      await until(page, () => window.__touchline.view() === 'tactics', undefined, 10_000);
    }
    await clean('tactics');
    await page.screenshot({ path: evidence(info, 'old-replay-tactics.png') });
    // The recording (release 0.1.0, a match no page managed) carries no squad: the Squad
    // screen opens with no rows and no notice.
    await tab(page, 'Squad').click();
    await until(page, () => window.__touchline.view() === 'squad', undefined, 10_000);
    await settled(page);
    const rows = await squad(page).locator('[data-squad-table] tbody tr[data-row]').count();
    console.log(`old replay squad rows: ${rows}`);
    if (rows > 0) {
      await scaleScan(page);
    }
    await clean('squad');
    await page.screenshot({ path: evidence(info, 'old-replay-squad.png') });
  } finally {
    engine.cleanUp();
  }
});
