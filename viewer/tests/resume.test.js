// The Resume a saved match screen's model: what the header, the fact strip and the alert say
// for each kind of save no engine of this version can finish.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { resumeModel, savedBy, savedDay, savedMatch } from '../src/lib/resume.js';

/// The launcher's `resume` block for the committed save stamped as release 0.1.0.
const OLDER = {
  kind: 'older',
  'saved.version': '0.1.0',
  'saved.build': '3ba8fed',
  'saved.tick': 156_500,
  'saved.teams': ['Ashford Rovers', 'Port Varrow'],
  'saved.score': [1, 0],
  'saved.millis': 1_700_000_000_000,
  engines: ['0.3.0', '0.2.0'],
  reason: 'this match was saved by Touchline 0.1.0, two or more versions back; ...',
};

test('a save two or more versions back reads as the board draws it', () => {
  const model = resumeModel(OLDER);
  assert.equal(model.title, 'Resume a saved match');
  assert.equal(model.subtitle, 'Touchline 0.3.0 · finishes matches saved by 0.3.0 and 0.2.0');
  assert.equal(model.date, 'TUE 14 NOVEMBER');
  assert.equal(model.dateSub, 'Saved match');
  assert.deepEqual(
    model.facts.map((f) => [f.value, f.label]),
    [
      ['Saved match', 'Ashford Rovers 1–0 Port Varrow · 52:10'],
      ['Touchline 0.1.0', 'Saved by'],
      ['Touchline 0.3.0', 'This version'],
      ['0.3.0 and 0.2.0', 'Engines in this version'],
      ['Cannot resume', 'Two or more versions back'],
    ]
  );
  // The state carries a word as well as its colour.
  assert.equal(model.facts[4].tone, 'warn');
  assert.equal(model.heading, 'This match was saved by Touchline 0.1.0');
  assert.match(model.body, /^This version finishes matches saved by 0\.3\.0 and 0\.2\.0\. A match saved two or more versions back/);
  assert.equal(model.hint, 'To finish it, open the save in Touchline 0.1.0.');
  assert.equal(model.note, 'The save stays on disk. Nothing is deleted.');
});

test('every kind of refusal names the save and says why', () => {
  const words = {
    older: 'Two or more versions back',
    newer: 'A newer version',
    other: 'Not carried by this version',
    unreleased: 'An unreleased build',
    'previous-missing': 'Its engine is missing',
  };
  for (const [kind, word] of Object.entries(words)) {
    const model = resumeModel({ ...OLDER, kind });
    assert.equal(model.facts[4].label, word, kind);
    assert.ok(model.body.length > 0, kind);
  }
  const unreleased = resumeModel({ ...OLDER, kind: 'unreleased', 'saved.version': null, 'saved.build': '1234567' });
  assert.equal(unreleased.heading, 'This match was saved by an unreleased build');
  assert.equal(unreleased.facts[1].value, 'Unreleased build 1234567');
  assert.equal(unreleased.hint, null);
  assert.equal(resumeModel(null), null);
});

test('an older save without a summary names only its clock, and no stamp keeps the label', () => {
  assert.equal(savedMatch({ 'saved.tick': 156_500, 'saved.teams': null, 'saved.score': null }), 'A match at 52:10');
  assert.equal(savedMatch({}), 'A saved match');
  assert.equal(savedBy({ 'saved.version': '0.2.0-beta.1' }), 'Touchline 0.2.0-beta.1');
  assert.equal(savedDay(null), null);
  assert.equal(savedDay(0), null);
  assert.equal(resumeModel({ ...OLDER, 'saved.millis': undefined }).date, 'SAVED MATCH');
});
