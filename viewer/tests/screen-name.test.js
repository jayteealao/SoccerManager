// The name of each screen for the tab title and the screen-change announcement, and the
// failure line the start screen and match setup show.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import { failure } from '../src/lib/front-door.svelte.js';
import { screenName } from '../src/lib/screen-name.js';

test('every front-door screen and every match view has a name', () => {
  assert.equal(screenName('start'), 'Start');
  assert.equal(screenName('setup'), 'Match setup');
  assert.equal(screenName('match', 'tactics'), 'Tactics');
  assert.equal(screenName('match', 'replay'), 'Replay');
  assert.equal(screenName('match', undefined), 'Match');
  assert.equal(screenName('unknown'), '');
});

test('a failure line names what failed, the reason when there is one, and what to do', () => {
  assert.equal(
    failure('The engine did not start the match', 'a match is already running.', 'Choose KICK OFF again.'),
    'The engine did not start the match: a match is already running. Choose KICK OFF again.',
  );
  assert.equal(failure('The saved match could not be resumed', '', 'Choose Resume again.'), 'The saved match could not be resumed. Choose Resume again.');
});
