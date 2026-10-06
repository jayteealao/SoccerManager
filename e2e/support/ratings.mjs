// Checks that the screens show ratings on the 1 to 20 scale: every rating a page shows, in a
// cell or in a control's name, is a whole number from 1 to 20, and no star label has a
// decimal. The engine sends ratings in tenths; the page must never show a tenth or a value
// of the old 1 to 100 scale.
import { expect } from '@playwright/test';

/// The rating numbers in `names` that follow "role fit" or "fitness".
function ratingsIn(names) {
  const out = [];
  for (const name of names) {
    for (const m of name.matchAll(/\b(?:role fit|fitness) (\d+(?:\.\d+)?)/g)) out.push(m[1]);
  }
  return out;
}

/// Every Fit cell of the squad list, and every rating in a squad row's or a pitch slot's
/// name, is a whole number from 1 to 20.
export async function expectRatingsOnTwenty(page) {
  const cells = await page.locator('button.row[data-squad] > b.num.r').allTextContents();
  const names = await page.locator('[aria-label*="fitness "]').evaluateAll((els) =>
    els.map((e) => e.getAttribute('aria-label')),
  );
  const values = [...cells.map((c) => c.trim()), ...ratingsIn(names)];
  expect(cells.length, 'the squad list shows a Fit cell per player').toBeGreaterThan(0);
  expect(ratingsIn(names).length, 'the rows and slots name their ratings').toBeGreaterThan(0);
  for (const v of values) {
    expect(v, `a rating shows as ${v}`).toMatch(/^\d+$/);
    expect(Number(v)).toBeGreaterThanOrEqual(1);
    expect(Number(v)).toBeLessThanOrEqual(20);
  }
  console.log(`ratings on 1 to 20: ${values.length} values, ${[...new Set(values)].sort((a, b) => a - b).join(' ')}`);
}

/// Every club's stars label says whole or half stars of 5, with no decimal.
export async function expectStarLabels(page) {
  const labels = await page.locator('.stars[role="img"]').evaluateAll((els) =>
    els.map((e) => e.getAttribute('aria-label')),
  );
  expect(labels.length).toBeGreaterThan(0);
  for (const label of labels) {
    expect(label).toMatch(/^(?:Half a star of 5|\d+(?: and a half)? of 5 stars)$/);
    expect(label).not.toMatch(/\d\.\d/);
  }
  console.log(`star labels: ${[...new Set(labels)].join('; ')}`);
}

/// Every Risk cell of the home line-up holds one of the three words.
export async function expectRiskWords(page) {
  const words = (await page.locator('td.risk').allTextContents()).map((w) => w.trim());
  expect(words.length).toBeGreaterThan(0);
  for (const w of words) expect(['Low', 'Raised', 'High']).toContain(w);
  console.log(`risk words: ${words.join(' ')}`);
}
