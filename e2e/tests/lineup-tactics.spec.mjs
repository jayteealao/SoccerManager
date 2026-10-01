// The lineup before kick-off and the changes during play: each illegal lineup blocks
// kick-off with its reason, a change waits for a stoppage, a sixth substitution is refused
// with the engine's reason, and every control works from the keyboard.
import { expect, test } from '@playwright/test';
import { WEB, startEngine } from '../support/engine.mjs';
import {
  changeReaches,
  chipList,
  chooseIndex,
  comingOn,
  kickOff,
  kickOffButton,
  openMatch,
  pause,
  play,
  queueSubstitution,
  queued,
  setSpeed,
  showTactics,
  until,
} from '../support/page.mjs';

const lineup = (page) => page.evaluate(() => window.__touchline.lineup());
const slot = (page, n) => page.getByRole('button', { name: new RegExp(`^Slot ${n + 1},`) });
const squadRow = (page, i) => page.locator(`button[data-squad="${i}"]`);
/// The lineup's verdict on Tactics: its reason, or the sentence a legal lineup reads.
const lineupReason = (page) => page.locator('.verdict .why');
const READY = 'The lineup is legal.';
/// The picker's option text is "shirt name · position"; the name is between.
const optionName = (text) => text.replace(/^\d+\s*/, '').replace(/\s*·.*$/, '');

async function serve(page, args = []) {
  const engine = await startEngine({ command: 'serve', args: ['--seed', '42', '--web', WEB, ...args] });
  await openMatch(page, engine.url);
  await until(page, () => window.__touchline.lineup().phase === 'pre-match', { timeout: 30_000 });
  return engine;
}

async function expectBlocked(page) {
  await expect(kickOffButton(page)).toBeDisabled();
  const reason = lineupReason(page);
  await expect(reason).not.toHaveText(READY);
  const view = await lineup(page);
  expect(view.legal).toBe(false);
  await expect(reason).toHaveText(view.reason);
  return view.reason;
}

test('each illegal lineup keeps kick-off disabled and names its reason', async ({ page }) => {
  const engine = await serve(page);
  try {
    const start = await lineup(page);
    expect(start.legal).toBe(true);
    const reasons = [];

    // Ten starters.
    await slot(page, 3).click();
    await page.getByRole('button', { name: 'Empty the picked slot' }).click();
    reasons.push(await expectBlocked(page));
    await squadRow(page, start.slots[3]).click();
    await slot(page, 3).click();
    await expect(kickOffButton(page)).toBeEnabled();

    // No goalkeeper: the keeper's slot and an outfield slot swap.
    await slot(page, 0).click();
    await slot(page, 1).click();
    reasons.push(await expectBlocked(page));
    await slot(page, 0).click();
    await slot(page, 1).click();
    await expect(kickOffButton(page)).toBeEnabled();

    // One player in two slots.
    await squadRow(page, start.slots[1]).click();
    await slot(page, 2).click();
    reasons.push(await expectBlocked(page));
    // A click on the disabled control does not kick off.
    await kickOffButton(page).click({ force: true });
    expect((await lineup(page)).phase).toBe('pre-match');

    expect(new Set(reasons).size).toBe(3);
    await squadRow(page, start.slots[2]).click();
    await slot(page, 2).click();
    await expect(kickOffButton(page)).toBeEnabled();
  } finally {
    engine.cleanUp();
  }
});

test('changes during play wait for a stoppage, and a sixth substitution is refused', async ({ page }) => {
  test.setTimeout(10 * 60_000);
  const engine = await serve(page, ['--minutes', '45']);
  try {
    await kickOff(page);
    await setSpeed(page, 8);
    await until(page, () => window.__touchline.lastRenderedTick() > 1500, { timeout: 60_000 });

    // Mentality: Queued, then Applied at a stoppage, and the feed says so.
    await showTactics(page);
    // The select only: once the change is queued, its Edit and Cancel buttons are labelled
    // "Edit: Mentality: …" and "Cancel: Mentality: …" and match the label as well.
    const mentality = page.getByLabel('Mentality').and(page.locator('select'));
    const now = await mentality.evaluate((s) => s.selectedIndex);
    const count = await mentality.locator('option').count();
    const change = await queued(page, () => chooseIndex(mentality, now + 1 < count ? now + 1 : now - 1));
    await expect(chipList(page).filter({ hasText: 'Queued' })).toHaveCount(1);
    const applied = await changeReaches(page, change.queue_id, ['applied', 'rejected']);
    expect(applied.state).toBe('applied');
    expect(applied.applied_tick).toBeGreaterThanOrEqual(
      await page.evaluate((t) => window.__touchline.stoppageAt(t), applied.queued_tick)
    );
    await until(page, () =>
      [...document.querySelectorAll('section[aria-label="Commentary"] li')].some((li) =>
        li.textContent.includes('Tactical change applied')
      )
    );

    // One substitution: the players swap and the count drops.
    const left = await page.evaluate(() => window.__touchline.dugout().subs_left);
    const onText = await comingOn(page).evaluate((s) => s.options[0].text);
    const sub = await queueSubstitution(page, 5, 0);
    expect((await changeReaches(page, sub.queue_id, ['applied', 'rejected'])).state).toBe('applied');
    await until(page, (n) => window.__touchline.dugout().subs_left === n, { arg: left - 1 });
    const names = await page.evaluate(() => window.__touchline.matchDay().lineupLabels.slice(0, 11).map((r) => r.name));
    expect(names.join('|')).toContain(optionName(onText).split(/\s+/).pop());

    // Four more at one stoppage, queued while playback is paused. The viewer's picker closes
    // once five have applied, so the sixth is queued in the same pause, behind the four, for
    // the engine to refuse.
    await pause(page);
    const four = [];
    for (const [off, on] of [[1, 0], [2, 1], [3, 2], [4, 3]]) {
      four.push(await queueSubstitution(page, off, on));
    }
    // A sixth: the engine refuses it, and the chip shows the engine's reason word for word.
    const sixth = await queueSubstitution(page, 6, 4);
    await play(page);
    for (const s of four) {
      expect((await changeReaches(page, s.queue_id, ['applied', 'rejected'])).state).toBe('applied');
    }
    await until(page, () => window.__touchline.dugout().subs_left === 0);

    const refused = await changeReaches(page, sixth.queue_id, ['applied', 'rejected']);
    expect(refused.state).toBe('rejected');
    expect(refused.reason).toBeTruthy();
    await showTactics(page);
    await expect(chipList(page).filter({ hasText: 'Rejected' }).first()).toContainText(refused.reason);
  } finally {
    engine.cleanUp();
  }
});

test('a role change made while paused applies at the next stoppage, not at resume', async ({ page }) => {
  test.setTimeout(6 * 60_000);
  const engine = await serve(page, ['--minutes', '20']);
  try {
    await kickOff(page);
    await setSpeed(page, 4);
    await until(page, () => window.__touchline.lastRenderedTick() > 1000, { timeout: 60_000 });
    await pause(page);
    const pausedAt = await page.evaluate(() => window.__touchline.lastRenderedTick());
    await showTactics(page);
    await page.getByText('Roles and duties').click();
    // The roles table lists the eleven in slot order, each with a role and a duty select.
    const slotIndex = await page.evaluate(() => {
      const roles = document.querySelectorAll('select[aria-label$=": role"]');
      for (let s = 1; s < 11; s += 1) {
        if (roles[s].options.length > 1) {
          return s;
        }
      }
      return -1;
    });
    expect(slotIndex).toBeGreaterThan(0);
    const role = page.locator('select[aria-label$=": role"]').nth(slotIndex);
    const index = await role.evaluate((s) => s.selectedIndex);
    const optionCount = await role.locator('option').count();
    const change = await queued(page, () => chooseIndex(role, index + 1 < optionCount ? index + 1 : index - 1));
    await page.waitForTimeout(3000);
    const whilePaused = await page.evaluate((id) => window.__touchline.pending().find((c) => c.queue_id === id), change.queue_id);
    expect(whilePaused.state).not.toBe('applied');
    await play(page);
    const resumeTick = await page.evaluate(() => window.__touchline.lastRenderedTick());
    const applied = await changeReaches(page, change.queue_id, ['applied', 'rejected']);
    expect(applied.state).toBe('applied');
    const stoppage = await page.evaluate((t) => window.__touchline.stoppageAt(t), applied.queued_tick);
    expect(applied.applied_tick).toBe(stoppage);
    expect(applied.applied_tick).toBeGreaterThan(pausedAt);
    expect(resumeTick).toBeLessThanOrEqual(pausedAt + 1);
  } finally {
    engine.cleanUp();
  }
});

/// A control's key in the focus walk: its id, its accessible name, the label around it, or
/// its squad row. Passed into the page as source.
const KEY = `(a) =>
  a.dataset.testid ||
  a.id ||
  a.getAttribute('aria-label') ||
  (a.dataset.squad !== undefined ? 'squad-' + a.dataset.squad : '') ||
  (a.closest('label')?.firstChild?.textContent ?? '').trim() ||
  a.textContent.trim().replace(/\\s+/g, ' ') ||
  a.tagName`;

test('every slot, control, and picker is reached with Tab and shows a focus ring', async ({ page }) => {
  const engine = await serve(page, ['--minutes', '10']);
  try {
    const focusOrder = async (limit) => {
      await page.evaluate(() => document.activeElement?.blur());
      const seen = [];
      for (let i = 0; i < limit; i += 1) {
        await page.keyboard.press('Tab');
        const focus = await page.evaluate((source) => {
          // eslint-disable-next-line no-new-func
          const key = new Function(`return ${source}`)();
          const a = document.activeElement;
          if (!a || a === document.body) {
            return null;
          }
          const style = getComputedStyle(a);
          return {
            key: key(a),
            outline: style.outlineStyle,
            visible: a.matches(':focus-visible'),
          };
        }, KEY);
        if (!focus || seen.some((s) => s.key === focus.key)) {
          break;
        }
        seen.push(focus);
      }
      return seen;
    };
    const wanted = () =>
      page.evaluate((source) => {
        // eslint-disable-next-line no-new-func
        const key = new Function(`return ${source}`)();
        return [...new Set(
          [...document.querySelectorAll('button, select, input, summary, a[href], [tabindex="0"]')]
            .filter((e) => e.checkVisibility() && !e.disabled && !e.closest('dialog') && !e.closest('[inert]') && e.type !== 'file')
            .map((e) => key(e))
        )];
      }, KEY);

    const pre = await focusOrder(120);
    const preKeys = pre.map((f) => f.key);
    // The walk keys each control by its name, so it covers the slots and the squad rows.
    expect(pre.length).toBeGreaterThan(30);
    for (const key of await wanted()) {
      expect(preKeys, `Tab reaches ${key} before kick-off`).toContain(key);
    }
    for (const f of pre) {
      expect(f.visible && f.outline !== 'none', `${f.key} shows a focus ring`).toBe(true);
    }

    // Kick off from the keyboard: CONTINUE opens the Pre-match line-ups, and KICK OFF there.
    await kickOffButton(page).focus();
    await page.keyboard.press('Enter');
    await until(page, () => window.__touchline.view() === 'prematch', { timeout: 5_000 });
    await kickOffButton(page).focus();
    await page.keyboard.press('Enter');
    await until(page, () => window.__touchline.lineup().phase === 'live', { timeout: 15_000 });
    // During play the substitution picker is on Tactics.
    await showTactics(page);
    const subQueue = page.getByRole('button', { name: 'Queue substitution' });
    await expect(subQueue).toBeEnabled({ timeout: 30_000 });

    const live = await focusOrder(120);
    const liveKeys = live.map((f) => f.key);
    expect(live.length).toBeGreaterThan(20);
    for (const key of await wanted()) {
      expect(liveKeys, `Tab reaches ${key} during play`).toContain(key);
    }
    for (const f of live) {
      expect(f.visible && f.outline !== 'none', `${f.key} shows a focus ring`).toBe(true);
    }

    // Queue a substitution with the keyboard alone.
    await subQueue.focus();
    const change = await queued(page, () => page.keyboard.press('Enter'));
    expect(change.kind).toBe('substitution');
  } finally {
    engine.cleanUp();
  }
});
