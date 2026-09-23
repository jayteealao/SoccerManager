// The page pauses a live engine that runs too far ahead of playback and restarts it when
// playback catches up, so a queued change reaches the engine before the stoppage it names.

import assert from 'node:assert/strict';
import test from 'node:test';

import { LeadControl, PAUSE_AFTER_S, RESUME_BELOW_S, SEEN_EVERY_MS, SeenReport } from '../lead.mjs';

test('a lead past the bound pauses once, and a caught-up lead restarts once', () => {
  const lead = new LeadControl();
  assert.equal(lead.next(100), null);
  assert.equal(lead.next(PAUSE_AFTER_S * 50 + 1), 'pause');
  assert.equal(lead.next(900), null, 'no second pause while holding');
  assert.equal(lead.next(RESUME_BELOW_S * 50 + 1), null, 'between the two bounds nothing changes');
  assert.equal(lead.next(RESUME_BELOW_S * 50 - 1), 'start');
  assert.equal(lead.next(10), null);
  assert.equal(lead.pauses, 1);
});

test('the bounds scale with the playback speed', () => {
  const lead = new LeadControl();
  assert.equal(lead.next(PAUSE_AFTER_S * 50 * 4, 8), null, 'twenty seconds at 1x is 2.5 at 8x');
  assert.equal(lead.next(PAUSE_AFTER_S * 50 * 8 + 1, 8), 'pause');
  assert.equal(lead.next(RESUME_BELOW_S * 50 * 8 - 1, 8), 'start');
});

test('after full time nothing is sent', () => {
  const lead = new LeadControl();
  lead.next(10_000);
  assert.equal(lead.next(10_000, 1, true), null);
  assert.equal(lead.holding, false);
});

test('the drawn tick is reported at most once per interval, and only when it moved', () => {
  const seen = new SeenReport();
  assert.equal(seen.next(0, 0), 0, 'the first drawn tick is reported at once');
  assert.equal(seen.next(40, SEEN_EVERY_MS - 1), null, 'too soon after the last report');
  assert.equal(seen.next(40, SEEN_EVERY_MS), 40);
  assert.equal(seen.next(40, SEEN_EVERY_MS * 5), null, 'a paused pitch is not reported again');
  assert.equal(seen.next(12, SEEN_EVERY_MS * 6), 12, 'a rewind reports the lower tick');
});
