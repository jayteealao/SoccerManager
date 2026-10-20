// Shared helpers of the browser tests of the player as a person: content copies whose home
// team is the version 3 fixture (position ratings, feet, traits, personality), the copy whose
// strikers are relisted so the shipped pick plays someone out of position, a copy that makes a
// trait override certain, and the check that no mood, form or personality text holds a digit.
// Every copy is a new folder under the system temporary folder; the caller removes it with
// `removeDir` in `finally`.
import fs from 'node:fs';
import path from 'node:path';

import { expect } from '@playwright/test';

import { CONTENT, REPO, tempDir } from './engine.mjs';

/// A version 3 team file, from the engine's test fixtures: every player with ratings by
/// position, feet, traits and every personality value.
export const V3_HOME = path.join(REPO, 'crates', 'engine', 'tests', 'fixtures', 'teams', 'v3-good.json');

/// A copy of the content folder whose home team file is the version 3 fixture, under the
/// shipped home club's name and id, with `edit` applied to it. Returns the folder and the
/// team as written.
export function contentWithPersonHome(edit = () => {}) {
  const dir = tempDir('content-person-home');
  fs.cpSync(CONTENT, dir, { recursive: true });
  const team = JSON.parse(fs.readFileSync(V3_HOME, 'utf8'));
  // The shipped home club's identity, so it stays the home side (clubs order by id).
  team.club = JSON.parse(fs.readFileSync(path.join(CONTENT, 'teams', 'default-a.json'), 'utf8')).club;
  edit(team);
  fs.writeFileSync(path.join(dir, 'teams', 'default-a.json'), `${JSON.stringify(team, null, 2)}\n`);
  return { dir, team };
}

/// Relists the team's strikers as attacking midfielders, rated highest there, so the shipped
/// pick puts a player listed elsewhere in a striker's slot.
export function relistStrikers(team) {
  for (const p of team.players.filter((q) => q.position === 'ST')) {
    p.position = 'AM';
    p.positions.AM = Math.min(20, Math.max(...Object.values(p.positions)) + 0.5);
  }
}

/// Sets the chance that a strong `trait` beats an instruction it goes against to `chance` in
/// the content copy `dir`'s tuning, so a short run shows an override. The event, the feed row
/// and the panel's count are the real ones; only the wait is shorter.
export function plantOverride(dir, trait, chance = 1) {
  const file = path.join(dir, 'tuning.json');
  const tuning = JSON.parse(fs.readFileSync(file, 'utf8'));
  const table = tuning.engine.contract.traits.override_chance;
  if (!(trait in table)) {
    throw new Error(`no override chance for ${trait}`);
  }
  table[trait] = chance;
  fs.writeFileSync(file, `${JSON.stringify(tuning, null, 2)}\n`);
}

/// Gives every outfield player of `team` holding up the ball as a strong trait (it has no
/// opposite), so whichever eleven the shipped pick starts, a planted override chance shows.
export function holdUpForAll(team) {
  for (const p of team.players.filter((q) => q.position !== 'GK')) {
    p.traits = { ...(p.traits ?? {}), holds_up_ball: 'strong' };
  }
}

/// The home player of the match's first hold-up override, from the stored events: `{ id,
/// index, count }`, with `index` his squad index and `count` all his overrides of the match
/// (the number his panel shows after it); null when no home player held up the ball.
export function firstHolder(page) {
  return page.evaluate(() => {
    const ids = window.__touchline.squadIds();
    const overrides = window.__touchline.events().filter((e) => e['event.type'] === 'trait-override');
    const first = overrides.find((e) => e['trait.name'] === 'holds_up_ball' && ids.includes(e['player.id']));
    if (!first) {
      return null;
    }
    const id = first['player.id'];
    return { id, index: ids.indexOf(id), count: overrides.filter((e) => e['player.id'] === id).length };
  });
}

/// From the player panel back through the Squad screen to the view the Squad screen opened
/// from.
export async function leavePanel(page) {
  await tab(page, 'Squad').click();
  await until(page, () => window.__touchline.view() === 'squad', undefined, 10_000);
  await page.locator('nav.subnav:visible button').first().click();
  await until(page, () => !['squad', 'player'].includes(window.__touchline.view()), undefined, 10_000);
}

/// Removes a folder a test made; a file an exiting process still holds stays behind.
export function removeDir(dir) {
  try {
    fs.rmSync(dir, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  } catch {
    // A file still held open by an exiting process; the system temporary folder keeps it.
  }
}

/// No element under `selector` holds a digit in its text or in an aria label inside it.
export async function noDigitsIn(page, selector) {
  const texts = await page.evaluate((sel) => {
    const out = [];
    for (const node of document.querySelectorAll(sel)) {
      if (!node.checkVisibility?.() && node.offsetParent === null) {
        continue;
      }
      const labels = [node, ...node.querySelectorAll('*')].map((n) => n.getAttribute('aria-label') ?? '').join(' ');
      out.push(`${node.textContent} ${labels}`);
    }
    return out;
  }, selector);
  expect(texts.length, `${selector} is on screen`).toBeGreaterThan(0);
  for (const text of texts) {
    expect(text, selector).not.toMatch(/\d/);
  }
}

const until = (page, fn, arg, timeout = 120_000) => page.waitForFunction(fn, arg, { timeout, polling: 100 });

/// Where a test keeps a screenshot or a report: the evidence folder when set, else the test's
/// own output folder.
export function evidence(info, name) {
  const out = process.env.MATCH_EVIDENCE_DIR ? path.join(process.env.MATCH_EVIDENCE_DIR, name) : info.outputPath(name);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  return out;
}

/// A sub-navigation tab of the view on show.
export const tab = (page, name) => page.locator('nav.subnav:visible').getByRole('button', { name, exact: true });

/// From the match setup screen (a new match) to the Tactics screen, before kick-off.
export async function setupToTactics(page) {
  await until(page, () => window.__touchline.frontDoor().round !== null, undefined, 10_000);
  await page.locator('[data-kickoff]').click();
  await until(page, () => window.__touchline.view() === 'tactics', undefined, 30_000);
}

/// From the Tactics screen through Pre-match to kick-off; returns when the match is live.
export async function kickOffFromTactics(page) {
  const action = page.locator('header button.cont:visible');
  await expect(action).toHaveText('Continue');
  await action.click();
  await until(page, () => window.__touchline.view() === 'prematch', undefined, 5_000);
  await expect(action).toHaveText('Kick off');
  await action.click();
  await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
}

/// Pauses playback when it plays.
export async function pauseIfPlaying(page) {
  const playback = page.getByRole('group', { name: 'Playback' }).filter({ visible: true });
  const pause = playback.getByRole('button', { name: 'Pause', exact: true });
  if (await pause.isVisible()) {
    await pause.click();
  }
}

/// The starters of `team` (the content copy's home team, as written) who play a formation
/// slot away from their listed position and are rated lower there: `{ squad, name, plays,
/// natural }` each, from the kicked-off lineup and the tactics file.
export async function outOfPositionStarters(page, team, tactics) {
  const lineup = await page.evaluate(() => window.__touchline.lineup());
  const eleven = lineup.kicked_off ?? lineup.slots;
  const slots = tactics.formations[lineup.formation].slots;
  return eleven
    .map((s, n) => ({ squad: s, player: s === null ? null : team.players[s], plays: slots[n].position }))
    .filter(
      ({ player, plays }) =>
        player && player.position !== plays && (player.positions?.[plays] ?? 1) < (player.positions?.[player.position] ?? 1)
    )
    .map(({ squad, player, plays }) => ({ squad, name: player.name, plays, natural: player.position }));
}

/// The Touchline's player state as drawn, with the hello's kick-off mood of the home squad,
/// read in one turn: `{ tick, rows: [{ id, mood, text, label, level, levelText, name }],
/// hello, note }`.
export function readTouchline(page) {
  return page.evaluate(() => {
    const root = document.querySelector('[data-screen="touchline"]') ?? document;
    const rows = [...root.querySelectorAll('section[aria-label="Player state"] tbody tr[data-player]')]
      .filter((tr) => tr.offsetParent !== null)
      .map((tr) => ({
        id: tr.dataset.player,
        mood: tr.querySelector('td.mood')?.dataset.mood ?? null,
        text: tr.querySelector('td.mood')?.textContent.replace(/\s+/g, ' ').trim() ?? '',
        label: tr.querySelector('td.mood')?.getAttribute('aria-label') ?? '',
        level: tr.querySelector('td.lv')?.dataset.level ?? null,
        levelText: tr.querySelector('td.lv')?.textContent.replace(/\s+/g, ' ').trim() ?? '',
        name: tr.querySelector('td.nm')?.textContent.trim() ?? '',
      }));
    const note = root.querySelector('[data-out-of-position]');
    return {
      tick: window.__touchline.lastRenderedTick(),
      rows,
      hello: window.__touchline.squadMood(),
      note: note ? { ids: note.dataset.outOfPosition, text: note.textContent.replace(/\s+/g, ' ').trim() } : null,
    };
  });
}

/// The five mood words with their marks, as the Mood column draws them.
export const MOOD_CELL = /^(▲ Confident|▲ Lifted|= Steady|▼ Low|▼ Rattled)$/;
