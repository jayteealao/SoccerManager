// The page pauses a live engine that runs too far ahead of playback and restarts it when
// playback catches up, so a queued change reaches the engine before the stoppage it names.

import assert from 'node:assert/strict';
import test from 'node:test';

import { LeadControl, PAUSE_AFTER_S, RESUME_BELOW_S } from '../lead.mjs';

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
