// A match played in the browser leaves its records in the data folder, as the engine wrote
// them: the event rows by each stoppage, and the statistics record at full time.
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { expect, test } from '@playwright/test';
import { REPO, readRecords, startEngine } from '../support/engine.mjs';
import { kickOff, openMatch, playUntil, setSpeed, until } from '../support/page.mjs';

const schema = (name) =>
  JSON.parse(readFileSync(path.join(REPO, 'schemas', 'observability', `${name}.schema.json`), 'utf8'));

/// Reads the records again until `accept` holds, for up to `ms`. A line being written can
/// fail to parse, so a failed read is tried again.
async function recordsWithin(dataDir, matchId, accept, ms) {
  const end = Date.now() + ms;
  let last = null;
  for (;;) {
    try {
      last = readRecords(dataDir, matchId);
      if (accept(last)) {
        return last;
      }
    } catch {
      // A half-written line; read again.
    }
    if (Date.now() > end) {
      return last;
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
}

test('the records of a browser-driven match reach the data folder in time', async ({ page }) => {
  const engine = await startEngine({ command: 'serve', args: ['--seed', '42', '--minutes', '10', '--web', 'web'] });
  try {
    const seen = await openMatch(page, engine.url);
    await kickOff(page);
    await setSpeed(page, 8);
    const hello = seen.hello;
    expect(hello).not.toBeNull();
    const matchId = hello['match.id'];

    // The first stoppage after kick-off: its event row is on disk by then.
    const first = await until(
      page,
      () => window.__touchline.events().find((e) => e.tick > 0 && e['event.type'] !== 'kick-off') ?? null,
      { timeout: 120_000 }
    );
    const byStoppage = await recordsWithin(
      engine.dataDir,
      matchId,
      (r) => r.events !== null && r.events.some((e) => e.tick >= first.tick),
      2000
    );
    expect(byStoppage.events, 'events.jsonl exists by the first stoppage').not.toBeNull();
    expect(byStoppage.events.some((e) => e.tick === first.tick && e['event.type'] === first['event.type'])).toBe(true);

    // Full time: the statistics record is written within 2 seconds of the page receiving it.
    await playUntil(page, () => window.__touchline.events().some((e) => e['event.type'] === 'full-time'), {
      timeout: 4 * 60_000,
    });
    const atFullTime = await recordsWithin(
      engine.dataDir,
      matchId,
      (r) =>
        r.stats !== null &&
        r.stats['record.kind'] === 'match-stats' &&
        r.events !== null &&
        r.events.some((e) => e['event.type'] === 'full-time'),
      2000
    );
    expect(atFullTime.stats, 'stats.json within 2 s of full time').not.toBeNull();
    const { stats, events } = atFullTime;

    // Untransformed records in the contract's shape.
    const eventSchema = schema('match-event');
    const statsSchema = schema('match-stats');
    expect(stats['record.kind']).toBe('match-stats');
    expect(stats['schema.version']).toBe(statsSchema.properties['schema.version'].const);
    for (const key of statsSchema.required) {
      expect(stats, `stats.json has ${key}`).toHaveProperty([key]);
    }
    expect(events.length).toBeGreaterThan(2);
    for (const event of events) {
      expect(event['record.kind']).toBe('match-event');
      expect(event['schema.version']).toBe(eventSchema.properties['schema.version'].const);
      for (const key of eventSchema.required) {
        expect(event, `an event row has ${key}`).toHaveProperty([key]);
      }
    }

    // The same build, owner, and match the page was shown.
    for (const record of [stats, ...events]) {
      expect(record.version).toBe(hello['engine.version']);
      expect(record['owner.id']).toBe(hello['owner.id']);
      expect(record['match.id']).toBe(matchId);
    }

    // The page's own event list is the engine's, row for row.
    const shown = await page.evaluate(() => window.__touchline.events());
    expect(shown.map((e) => [e.tick, e['event.type']])).toEqual(events.map((e) => [e.tick, e['event.type']]));
  } finally {
    engine.cleanUp();
  }
});
