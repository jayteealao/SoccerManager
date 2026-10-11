// The player panel on the built viewer, served by the release program's `launch`.
//
// 1. After full time: a fast-forwarded match of seed 7 to its full-time report, then the
//    report's Squad tab and a player's name. The panel shows his attributes by group as whole
//    numbers 1 to 20, the hidden values in words (the shipped teams are version 1 files, so
//    every player reads "Not yet known"), "Plays between" in whole numbers, and this match's
//    rating in one decimal. On the next launch on the same data folder, the stored rating is
//    read back, and the season block shows the match: one app, one point on the line, his
//    goal with a progress word and a form word.
// 2. The panel before any match, @sizes: a Tab walk never lands in a stub, the rendered
//    contrast passes, and the panel is compared with its baseline at each window size.
// 3. A known player of a version 3 home team: ten position chips with his natural one
//    highest, both feet, the summary word "Driven" with its standouts and "sure · 31 matches
//    here", the assistant's view, the mood today and three traits with their strengths.
// 4. The attribute hover: pointer over Crossing, then Tab to it, list the same actions as the
//    attribute catalogue's stage tables give, with no minor weight; Escape closes it.
// 5. The design states: not yet known; converted, with no traits and an empty season; out of
//    position at a striker's slot.
// On every state, no personality, mood or form text carries a digit. Screenshots are kept as
// evidence; each test removes the content copies and data folders it made.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { contrastReport } from '../support/contrast.mjs';
import { CONTENT, REPO, frontDoor, tempDir } from '../support/engine.mjs';
import { kickOffFromStart, open, settled, toFullTime } from '../support/front.mjs';
import { clearPointer, snap } from '../support/page.mjs';

/// Past the end of any match: a match with this fast-forward plays straight through.
const PAST_THE_END = 1_000_000;
/// A version 3 team file, from the engine's test fixtures: every player with ratings by
/// position, feet, traits and every personality value.
const V3_HOME = path.join(REPO, 'crates', 'engine', 'tests', 'fixtures', 'teams', 'v3-good.json');
const CATALOGUE = JSON.parse(fs.readFileSync(path.join(CONTENT, 'attributes.json'), 'utf8'));
const TACTICS = JSON.parse(fs.readFileSync(path.join(CONTENT, 'tactics.json'), 'utf8'));
const PERSONALITY = CATALOGUE.attributes.filter((a) => a.group === 'personality').map((a) => a.name);

const until = (page, fn, arg, timeout = 120_000) => page.waitForFunction(fn, arg, { timeout, polling: 100 });

const evidence = (info, name) => {
  const out = process.env.MATCH_EVIDENCE_DIR ? path.join(process.env.MATCH_EVIDENCE_DIR, name) : info.outputPath(name);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  return out;
};

const tab = (page, name) => page.locator('nav.subnav:visible').getByRole('button', { name, exact: true });
const panel = (page) => page.locator('[data-screen="player"]:visible');

async function toReport(page, url) {
  await open(page, url);
  await until(page, () => window.__touchline.frontDoor().answered);
  await page.keyboard.press('Enter');
  await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 10_000);
  await kickOffFromStart(page);
  await toFullTime(page);
}

/// From the splash to the Tactics screen of a new match, before kick-off.
async function toTactics(page, url) {
  await open(page, url);
  await until(page, () => window.__touchline.frontDoor().answered);
  await page.keyboard.press('Enter');
  await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 10_000);
  await page.locator('[data-choice="new"]').click();
  await until(page, () => window.__touchline.frontDoor().round !== null, undefined, 10_000);
  await page.locator('[data-kickoff]').click();
  await until(page, () => window.__touchline.view() === 'tactics', undefined, 30_000);
}

/// The panel of the player in squad row `index` (the first starter by default), from the
/// Squad screen.
async function openPanel(page, index = 0) {
  if ((await page.evaluate(() => window.__touchline.view())) !== 'squad') {
    await tab(page, 'Squad').click();
    await until(page, () => window.__touchline.view() === 'squad', undefined, 10_000);
  }
  const row = page.locator(`[data-screen="squad"]:visible tbody tr[data-row="${index}"]`);
  await row.locator('button.who').click();
  await until(page, () => window.__touchline.view() === 'player', undefined, 10_000);
  await expect(panel(page)).toBeVisible();
  await settled(page);
}

/// A copy of the content folder whose home team file is the version 3 fixture, under the
/// shipped home club's name and id, with `edit` applied to it. Returns the folder and the team as written.
function contentWithPersonHome(edit = () => {}) {
  const dir = tempDir('content-person-home');
  fs.cpSync(CONTENT, dir, { recursive: true });
  const team = JSON.parse(fs.readFileSync(V3_HOME, 'utf8'));
  // The shipped home club's identity, so it stays the home side (clubs order by id).
  team.club = JSON.parse(fs.readFileSync(path.join(CONTENT, 'teams', 'default-a.json'), 'utf8')).club;
  edit(team);
  fs.writeFileSync(path.join(dir, 'teams', 'default-a.json'), `${JSON.stringify(team, null, 2)}\n`);
  return { dir, team };
}

const removeDir = (dir) => {
  try {
    fs.rmSync(dir, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  } catch {
    // A file still held open by an exiting process; the system temporary folder keeps it.
  }
};

/// Player `p` of a version 3 team at `matches` matches at the club.
const atClub = (p, matches) => {
  p.condition = { ...(p.condition ?? {}), matches_at_club: matches };
};

/// The known player: professional, ambitious and calm, the other values middling, two strong
/// traits and one moderate, 31 matches at the club.
function known(p) {
  atClub(p, 31);
  for (const name of PERSONALITY) {
    p.attributes[name] = 10;
  }
  Object.assign(p.attributes, { professionalism: 16, ambition: 16, temperament: 15 });
  p.traits = { tries_killer_balls: 'strong', switches_play: 'strong', tries_tricks: 'moderate' };
}

/// AC-30 on screen: the panel's personality (but the matches-here count), mood, mind and form
/// texts hold no digit, nor any aria label in them.
async function noDigitsInPerson(page) {
  const texts = await panel(page).evaluate((el) => {
    const read = (sel) => {
      const node = el.querySelector(sel);
      if (!node) {
        return '';
      }
      const labels = [node, ...node.querySelectorAll('*')].map((n) => n.getAttribute('aria-label') ?? '').join(' ');
      return `${node.textContent} ${labels}`;
    };
    const block = el.querySelector('[data-section="personality"]');
    const sure = el.querySelector('[data-sureness]')?.textContent ?? '';
    return {
      personality: (block?.textContent ?? '').replace(sure, ''),
      summary: read('[data-summary]'),
      assistant: read('[data-assistant]'),
      mood: read('[data-mood]'),
      mind: read('[data-state="mind"]'),
      form: read('[data-season-row="form"]'),
    };
  });
  for (const [part, text] of Object.entries(texts)) {
    expect(text, part).not.toMatch(/\d/);
  }
  expect(await panel(page).innerText()).not.toMatch(/Thirteen personality attributes/);
}

test('after full time the panel shows whole numbers, words and the match rating, then the season', async ({ page }, info) => {
  test.setTimeout(8 * 60_000);
  const dataDir = tempDir('player-panel');
  let engine = await frontDoor({ args: ['--seed', '7'], dataDir, fastForwardTo: PAST_THE_END });
  try {
    await toReport(page, engine.url);
    await openPanel(page);
    const p = panel(page);
    const values = await p.locator('[data-attr] dd').allTextContents();
    expect(values.length).toBeGreaterThan(20);
    for (const v of values) {
      expect(v.trim()).toMatch(/^\d+$/);
      expect(Number(v)).toBeGreaterThanOrEqual(1);
      expect(Number(v)).toBeLessThanOrEqual(20);
    }
    for (const name of ['consistency', 'injury_proneness']) {
      await expect(p.locator(`[data-hidden="${name}"]`)).toContainText('Not yet known — no match for us yet');
    }
    await expect(p.locator('[data-plays]')).toHaveText(/^\s*Plays (between \d+ and \d+|at \d+)\s*$/);
    await expect(p.locator('[data-ratings]')).toHaveAttribute('data-ratings', 'some');
    const chips = await p.locator('[data-ratings] .rt').allTextContents();
    expect(chips.length).toBe(1);
    expect(chips[0]).toMatch(/^\d{1,2}\.\d$/);
    // No hidden value as a number anywhere on the panel.
    const text = await p.evaluate((el) => el.innerText);
    expect(text).not.toMatch(/consistency\s*\d|injury[ _]proneness\s*\d/i);
    await noDigitsInPerson(page);
    await page.screenshot({ path: evidence(info, 'panel-after-full-time.png') });

    await tab(page, 'Squad').click();
    await until(page, () => window.__touchline.view() === 'squad', undefined, 10_000);

    // The next launch on the same data folder reads the stored rating and the season back.
    engine.kill();
    await engine.exited;
    expect(fs.existsSync(path.join(dataDir, 'views.json')), 'the launcher stored the ratings').toBe(true);
    const stored = JSON.parse(fs.readFileSync(path.join(dataDir, 'views.json'), 'utf8'));
    const ratings = Object.values(stored.clubs).flatMap((c) => Object.values(c.ratings ?? {}));
    expect(ratings.length).toBeGreaterThan(10);
    engine = await frontDoor({ args: ['--seed', '7'], dataDir });
    await toTactics(page, engine.url);
    await openPanel(page);
    const q = panel(page);
    await expect(q.locator('[data-ratings]')).toHaveAttribute('data-ratings', 'some');
    await expect(q.locator('[data-season]')).toHaveAttribute('data-season', 'some', { timeout: 10_000 });
    await expect(q.locator('[data-season-line]')).toHaveAttribute('data-season-line', '1');
    await expect(q.locator('[data-season-row="so-far"]')).toContainText('1 app');
    await expect(q.locator('[data-season-row="goal"]')).toContainText(/, (ahead|on track|behind)$/);
    await expect(q.locator('[data-season-row="form"]')).toContainText(/In form|Steady form|Out of form/);
    await noDigitsInPerson(page);
    await clearPointer(page);
    await page.screenshot({ path: evidence(info, 'panel-season.png') });
  } finally {
    engine.cleanUp();
  }
});

test('the panel: a Tab walk never lands in a stub, and the rendered contrast passes', { tag: '@sizes' }, async ({ page }, info) => {
  test.setTimeout(4 * 60_000);
  const engine = await frontDoor();
  try {
    await toTactics(page, engine.url);
    await openPanel(page);
    await expect(panel(page).locator('[data-ratings]')).toHaveAttribute('data-ratings', 'none');
    await page.screenshot({ path: evidence(info, 'panel-new-signing.png') });
    // The panel before any match, against its baseline at each window size.
    await clearPointer(page);
    await snap(page, 'panel-new-signing.png');
    const walk = [];
    for (let n = 0; n < 40; n += 1) {
      await page.keyboard.press('Tab');
      walk.push(await page.evaluate(() => Boolean(document.activeElement?.closest('[data-stub]'))));
    }
    expect(walk.filter(Boolean)).toEqual([]);
    const report = await contrastReport(page);
    fs.writeFileSync(evidence(info, 'panel-contrast-report.json'), `${JSON.stringify(report, null, 2)}\n`);
    expect(report.checked).toBeGreaterThan(30);
    expect(report.failures, JSON.stringify(report.failures, null, 2)).toEqual([]);
  } finally {
    engine.cleanUp();
  }
});

test('a known player: positions, feet, the personality summary with the assistant, mood and traits', async ({ page }, info) => {
  test.setTimeout(4 * 60_000);
  const { dir, team } = contentWithPersonHome((t) => known(t.players[1]));
  const engine = await frontDoor({ env: { SM_CONTENT_DIR: dir } });
  try {
    await toTactics(page, engine.url);
    await openPanel(page, 1);
    const p = panel(page);
    const player = team.players[1];
    // Ten chips, each a whole number 1 to 20 or "—"; his natural position the highest.
    const chips = await p.locator('[data-chip]').evaluateAll((els) =>
      els.map((el) => ({ code: el.dataset.chip, rating: el.dataset.rating, natural: el.dataset.natural === 'true', text: el.textContent.trim() }))
    );
    expect(chips.length).toBe(10);
    for (const c of chips) {
      expect(c.text).toMatch(new RegExp(`^${c.code}(\\d{1,2}|—)$`));
      if (c.rating !== 'none') {
        expect(Number(c.rating)).toBeGreaterThanOrEqual(2);
        expect(Number(c.rating)).toBeLessThanOrEqual(20);
      }
    }
    const natural = chips.find((c) => c.natural);
    expect(natural.code).toBe(player.position);
    for (const c of chips.filter((c) => c.rating !== 'none')) {
      expect(Number(c.rating)).toBeLessThanOrEqual(Number(natural.rating));
    }
    await expect(p.locator('[data-pitch-list] li')).toHaveCount(10);
    // Both feet: the strong at 100 and the weaker as a percent with its word.
    await expect(p.locator('[data-foot="strong"]')).toContainText(/(Left|Right) \(strong\)\s*100/);
    await expect(p.locator('[data-foot="weaker"]')).toContainText(new RegExp(`(one-footed|decent|two-footed)\\s*${player.foot.weaker}%`));
    // Personality: the summary word, the standouts with how sure, the assistant and the mood.
    await expect(p.locator('[data-summary]')).toHaveText('Driven');
    await expect(p.locator('[data-sureness]')).toHaveText('professional, ambitious, calm · sure · 31 matches here');
    await expect(p.locator('[data-assistant]')).toContainText('Assistant manager:');
    await expect(p.locator('[data-assistant]')).toContainText('Model pro');
    await expect(p.locator('[data-mood]')).toContainText(/Mood today:\s*(▲|▼|=)?\s*(Confident|Lifted|Steady|Low|Rattled)/);
    // Traits in catalogue order with their strengths.
    const traits = await p.locator('[data-trait]').evaluateAll((els) => els.map((el) => el.lastElementChild.textContent.trim()));
    expect(traits).toEqual(['Strong', 'Strong', 'Moderate']);
    await expect(p.locator('[data-trait="tries_killer_balls"]')).toContainText('Tries killer balls');
    await expect(p.locator('[data-season]')).toHaveAttribute('data-season', 'empty');
    await noDigitsInPerson(page);
    await clearPointer(page);
    await settled(page);
    await page.screenshot({ path: evidence(info, 'panel-known.png') });
  } finally {
    engine.cleanUp();
    removeDir(dir);
  }
});

test('the attribute hover lists the actions on pointer and on focus, with no minor weight', async ({ page }, info) => {
  test.setTimeout(4 * 60_000);
  const { dir } = contentWithPersonHome((t) => known(t.players[1]));
  const engine = await frontDoor({ env: { SM_CONTENT_DIR: dir } });
  // The expected list, from the catalogue: main first, then the shown supports.
  const tables = (action) => Object.values(CATALOGUE.actions[action]).filter((t) => t && typeof t === 'object' && t.main);
  const main = Object.keys(CATALOGUE.actions).filter((a) => tables(a).some((t) => t.main.attribute === 'crossing'));
  const supports = Object.keys(CATALOGUE.actions).filter(
    (a) => !main.includes(a) && tables(a).some((t) => (t.supports ?? []).some((s) => s.attribute === 'crossing'))
  );
  const expected = [...main.map((a) => [a, 'main']), ...supports.map((a) => [a, 'support'])];
  const minors = Object.entries(CATALOGUE.actions)
    .filter(([, table]) => Object.values(table).some((t) => (t?.minor ?? []).some((m) => m.attribute === 'crossing')))
    .map(([a]) => a)
    .filter((a) => !main.includes(a) && !supports.includes(a));
  try {
    await toTactics(page, engine.url);
    await openPanel(page, 1);
    const p = panel(page);
    const row = p.locator('[data-attr="crossing"]');
    const pop = p.locator('[data-hover="crossing"]');
    const listed = () => pop.locator('li').evaluateAll((els) => els.map((el) => [el.dataset.action, el.dataset.role]));
    // Pointer: the popover shows in the same step as the move.
    await row.hover();
    await expect(pop).toBeVisible();
    await expect(pop).toContainText('Crossing drives');
    await expect(pop).toHaveAttribute('role', 'tooltip');
    const byPointer = await listed();
    expect(byPointer).toEqual(expected);
    for (const a of minors) {
      expect(byPointer.map(([action]) => action)).not.toContain(a);
    }
    await settled(page);
    await page.screenshot({ path: evidence(info, 'hover-pointer.png') });
    await clearPointer(page);
    await expect(pop).toHaveCount(0);
    // Keyboard: Tab until the Crossing row has focus.
    let reached = false;
    for (let n = 0; n < 80 && !reached; n += 1) {
      await page.keyboard.press('Tab');
      reached = await row.evaluate((el) => el === document.activeElement);
    }
    expect(reached, 'Tab reaches the Crossing row').toBe(true);
    await expect(pop).toBeVisible();
    expect(await listed()).toEqual(byPointer);
    await expect(row).toHaveAttribute('aria-describedby', await pop.getAttribute('id'));
    await settled(page);
    await page.screenshot({ path: evidence(info, 'hover-focus.png') });
    await page.keyboard.press('Escape');
    await expect(pop).toHaveCount(0);
    expect(await row.evaluate((el) => el === document.activeElement), 'the row keeps focus').toBe(true);
  } finally {
    engine.cleanUp();
    removeDir(dir);
  }
});

test('the design states: not yet known, converted with no traits and an empty season, out of position', async ({ page }, info) => {
  test.setTimeout(8 * 60_000);
  // Not yet known: a version 3 player with no match at the club.
  {
    const { dir } = contentWithPersonHome((t) => atClub(t.players[2], 0));
    const engine = await frontDoor({ env: { SM_CONTENT_DIR: dir } });
    try {
      await toTactics(page, engine.url);
      await openPanel(page, 2);
      const p = panel(page);
      await expect(p.locator('[data-summary]')).toHaveText('Not yet known');
      await expect(p.locator('[data-sureness]')).toHaveText('no match for us yet');
      await expect(p.locator('[data-assistant]')).toContainText('Too early to say what he is like.');
      await expect(p.locator('[data-mood]')).toContainText('Settling in');
      await noDigitsInPerson(page);
      await clearPointer(page);
      await settled(page);
      await page.screenshot({ path: evidence(info, 'panel-not-yet-known.png') });
    } finally {
      engine.cleanUp();
      removeDir(dir);
    }
  }
  // Converted, no traits and an empty season: the shipped version 1 teams on a fresh folder.
  {
    const engine = await frontDoor();
    try {
      await toTactics(page, engine.url);
      await openPanel(page, 1);
      const p = panel(page);
      const rated = await p.locator('[data-chip]').evaluateAll((els) => els.filter((el) => el.dataset.rating !== 'none').map((el) => el.dataset.chip));
      expect(rated.length).toBe(1);
      expect(await p.locator('[data-chip][data-natural="true"]').getAttribute('data-chip')).toBe(rated[0]);
      await expect(p.locator('[data-chip][data-rating="none"]').first()).toContainText('—');
      await expect(p.locator('[data-foot="weaker"]')).toContainText(/decent\s*50%/);
      await expect(p.locator('[data-traits]')).toHaveText('No traits of play.');
      await expect(p.locator('[data-season]')).toHaveAttribute('data-season', 'empty');
      await expect(p.locator('[data-season-empty]')).toHaveText('No matches this season yet.');
      await expect(p.locator('[data-season-row="goal"]')).toContainText(/ · /);
      await expect(p.locator('[data-season-row="form"]')).toContainText('Not yet known');
      await noDigitsInPerson(page);
      await clearPointer(page);
      await settled(page);
      await page.screenshot({ path: evidence(info, 'panel-converted-no-traits-empty-season.png') });
    } finally {
      engine.cleanUp();
    }
  }
  // Out of position: the version 3 strikers relisted as attacking midfielders (rated highest
  // there), so the shipped pick puts a player listed elsewhere in a striker's slot.
  {
    const { dir, team } = contentWithPersonHome((t) => {
      for (const p of t.players.filter((q) => q.position === 'ST')) {
        p.position = 'AM';
        p.positions.AM = Math.min(20, Math.max(...Object.values(p.positions)) + 0.5);
      }
    });
    const engine = await frontDoor({ env: { SM_CONTENT_DIR: dir } });
    try {
      await toTactics(page, engine.url);
      const lineup = await page.evaluate(() => window.__touchline.lineup());
      const slots = TACTICS.formations[lineup.formation].slots;
      const n = lineup.slots.findIndex((s, i) => s !== null && team.players[s].position !== slots[i].position);
      expect(n, 'a starter plays away from his listed position').toBeGreaterThanOrEqual(0);
      const at = slots[n].position;
      await openPanel(page, lineup.slots[n]);
      const row = panel(page).locator('[data-state="familiarity"]');
      await expect(row).toHaveAttribute('data-out', 'true');
      await expect(row).toContainText(`Out of position at ${at}`);
      await noDigitsInPerson(page);
      await clearPointer(page);
      await settled(page);
      await page.screenshot({ path: evidence(info, 'panel-out-of-position.png') });
    } finally {
      engine.cleanUp();
      removeDir(dir);
    }
  }
});
