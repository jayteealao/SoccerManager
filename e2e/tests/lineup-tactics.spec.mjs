// The lineup before kick-off and the changes during play: each illegal lineup blocks
// kick-off with its reason, a change waits for a stoppage, a sixth substitution is refused
// with the engine's reason, and every control works from the keyboard.
import { expect, test } from '@playwright/test';
import { startEngine } from '../support/engine.mjs';
import {
  changeReaches,
  chooseIndex,
  kickOff,
  kickOffButton,
  openMatch,
  queueSubstitution,
  queued,
  setSpeed,
  until,
} from '../support/page.mjs';

const lineup = (page) => page.evaluate(() => window.__touchline.lineup());
const slot = (page, n) => page.getByRole('button', { name: new RegExp(`^Slot ${n + 1},`) });

async function serve(page, args = []) {
  const engine = await startEngine({ command: 'serve', args: ['--seed', '42', '--web', 'web', ...args] });
  await openMatch(page, engine.url);
  await until(page, () => window.__touchline.lineup().phase === 'pre-match', { timeout: 30_000 });
  return engine;
}

async function expectBlocked(page) {
  await expect(kickOffButton(page)).toBeDisabled();
  const reason = page.getByTestId('lineup-reason');
  await expect(reason).not.toHaveText('The lineup is ready.');
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
    await page.getByTestId(`squad-${start.slots[3]}`).click();
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
    await page.getByTestId(`squad-${start.slots[1]}`).click();
    await slot(page, 2).click();
    reasons.push(await expectBlocked(page));
    // A click on the disabled control does not kick off.
    await kickOffButton(page).click({ force: true });
    expect((await lineup(page)).phase).toBe('pre-match');

    expect(new Set(reasons).size).toBe(3);
    await page.getByTestId(`squad-${start.slots[2]}`).click();
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
    const mentality = page.getByTestId('mentality');
    const now = await mentality.evaluate((s) => s.selectedIndex);
    const count = await mentality.locator('option').count();
    const change = await queued(page, () => chooseIndex(mentality, now + 1 < count ? now + 1 : now - 1));
    await expect(page.locator('#chips .chip').filter({ hasText: 'Queued' })).toHaveCount(1);
    const applied = await changeReaches(page, change.queue_id, ['applied', 'rejected']);
    expect(applied.state).toBe('applied');
    expect(applied.applied_tick).toBeGreaterThanOrEqual(
      await page.evaluate((t) => window.__touchline.stoppageAt(t), applied.queued_tick)
    );
    await until(page, () =>
      [...document.querySelectorAll('#feed li')].some((li) => li.textContent.includes('Tactical change applied'))
    );

    // One substitution: the players swap and the count drops.
    const left = await page.evaluate(() => window.__touchline.dugout().subs_left);
    const onText = await page.getByLabel('Substitute coming on').evaluate((s) => s.options[0].text);
    const sub = await queueSubstitution(page, 5, 0);
    expect((await changeReaches(page, sub.queue_id, ['applied', 'rejected'])).state).toBe('applied');
    await until(page, (n) => window.__touchline.dugout().subs_left === n, { arg: left - 1 });
    const names = await page.evaluate(() => window.__touchline.matchDay().lineupLabels.slice(0, 11).map((r) => r.name));
    expect(names.join('|')).toContain(onText.replace(/^\d+\s*/, '').split(/\s+/).pop());

    // Four more at one stoppage, queued while playback is paused.
    await page.getByRole('button', { name: 'Pause' }).click();
    const four = [];
    for (const [off, on] of [[1, 0], [2, 1], [3, 2], [4, 3]]) {
      four.push(await queueSubstitution(page, off, on));
    }
    await page.getByRole('button', { name: 'Play' }).click();
    for (const s of four) {
      expect((await changeReaches(page, s.queue_id, ['applied', 'rejected'])).state).toBe('applied');
    }
    await until(page, () => window.__touchline.dugout().subs_left === 0);

    // A sixth: the engine refuses it, and the chip shows the engine's reason word for word.
    const sixth = await queueSubstitution(page, 6, 0);
    const refused = await changeReaches(page, sixth.queue_id, ['applied', 'rejected']);
    expect(refused.state).toBe('rejected');
    expect(refused.reason).toBeTruthy();
    await expect(page.locator('#chips .chip').filter({ hasText: 'Rejected' }).first()).toContainText(refused.reason);
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
    await page.getByRole('button', { name: 'Pause' }).click();
    const pausedAt = await page.evaluate(() => window.__touchline.lastRenderedTick());
    await page.getByText('Roles and duties').click();
    const slotIndex = await page.evaluate(() => {
      for (let s = 1; s < 11; s += 1) {
        if (document.querySelector(`[data-testid="role-${s}"]`).options.length > 1) {
          return s;
        }
      }
      return -1;
    });
    expect(slotIndex).toBeGreaterThan(0);
    const role = page.getByTestId(`role-${slotIndex}`);
    const index = await role.evaluate((s) => s.selectedIndex);
    const optionCount = await role.locator('option').count();
    const change = await queued(page, () => chooseIndex(role, index + 1 < optionCount ? index + 1 : index - 1));
    await page.waitForTimeout(3000);
    const whilePaused = await page.evaluate((id) => window.__touchline.pending().find((c) => c.queue_id === id), change.queue_id);
    expect(whilePaused.state).not.toBe('applied');
    await page.getByRole('button', { name: 'Play' }).click();
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

test('every slot, control, and picker is reached with Tab and shows a focus ring', async ({ page }) => {
  const engine = await serve(page, ['--minutes', '10']);
  try {
    const focusOrder = async (limit) => {
      await page.evaluate(() => document.activeElement?.blur());
      const seen = [];
      for (let i = 0; i < limit; i += 1) {
        await page.keyboard.press('Tab');
        const focus = await page.evaluate(() => {
          const a = document.activeElement;
          if (!a || a === document.body) {
            return null;
          }
          const style = getComputedStyle(a);
          return {
            key: a.dataset.testid ?? a.id ?? a.tagName,
            outline: style.outlineStyle,
            visible: a.matches(':focus-visible'),
          };
        });
        if (!focus || seen.some((s) => s.key === focus.key)) {
          break;
        }
        seen.push(focus);
      }
      return seen;
    };
    const wanted = () =>
      page.evaluate(() =>
        [...new Set(
          [...document.querySelectorAll('button, select, input, summary, a[href], [tabindex="0"]')]
            .filter((e) => e.checkVisibility() && !e.disabled && !e.closest('dialog') && e.type !== 'file')
            .map((e) => e.dataset.testid ?? e.id ?? e.tagName)
        )]
      );

    const pre = await focusOrder(120);
    const preKeys = pre.map((f) => f.key);
    for (const key of await wanted()) {
      expect(preKeys, `Tab reaches ${key} before kick-off`).toContain(key);
    }
    for (const f of pre) {
      expect(f.visible && f.outline !== 'none', `${f.key} shows a focus ring`).toBe(true);
    }

    // Kick off from the keyboard.
    await kickOffButton(page).focus();
    await page.keyboard.press('Enter');
    await until(page, () => window.__touchline.lineup().phase === 'live', { timeout: 15_000 });
    await until(page, () => !document.querySelector('[data-testid="sub-queue"]').disabled, { timeout: 30_000 });

    const live = await focusOrder(120);
    const liveKeys = live.map((f) => f.key);
    for (const key of await wanted()) {
      expect(liveKeys, `Tab reaches ${key} during play`).toContain(key);
    }
    for (const f of live) {
      expect(f.visible && f.outline !== 'none', `${f.key} shows a focus ring`).toBe(true);
    }

    // Queue a substitution with the keyboard alone.
    await page.getByTestId('sub-queue').focus();
    const change = await queued(page, () => page.keyboard.press('Enter'));
    expect(change.kind).toBe('substitution');
  } finally {
    engine.cleanUp();
  }
});
