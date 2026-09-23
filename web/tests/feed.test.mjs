// The feed at eight times speed: every event, in order, one insert per frame.

import assert from 'node:assert/strict';
import test from 'node:test';

import { EMPTY_TEXT, FeedBatcher, feedRow, minuteStamp } from '../feed.mjs';
import { KIND, MatchState } from '../match-state.mjs';
import { eventMessage } from './helpers.mjs';

test('a burst of 40 events inside one simulated second at 8x all appear, in order, none twice', () => {
  const state = new MatchState();
  // 40 events inside ticks 1000 to 1049: far more than 10 a second.
  const kinds = [KIND.goal, 'foul', KIND.card, 'free-kick', 'throw-in'];
  for (let i = 0; i < 40; i += 1) {
    state.add(eventMessage(1000 + Math.floor(i * 1.25), kinds[i % kinds.length]));
  }
  const batcher = new FeedBatcher();
  const inserted = [];
  let batches = 0;
  // 8x on a 60 Hz panel: 400 ticks a second over 60 frames, about 6.7 ticks a frame.
  for (let frame = 0; frame < 40; frame += 1) {
    const tick = 990 + Math.floor((frame * 400) / 60);
    const { reset, batch } = batcher.take(state.at(tick).entries);
    if (frame > 0) {
      assert.equal(reset, false, `frame ${frame} must not re-render`);
    }
    if (batch.length > 0) {
      batches += 1;
      inserted.push(...batch);
    }
  }
  assert.equal(inserted.length, 40);
  assert.deepEqual(
    inserted.map((e) => e.tick),
    [...inserted.map((e) => e.tick)].sort((a, b) => a - b)
  );
  assert.equal(new Set(inserted).size, 40, 'no event inserted twice');
  assert.ok(batches > 1 && batches < 40, `${batches} batches: several events share a frame`);
});

test('a rewind hands the renderer every entry again as one reset batch', () => {
  const state = new MatchState();
  for (const tick of [10, 20, 30]) {
    state.add(eventMessage(tick, 'corner'));
  }
  const batcher = new FeedBatcher();
  batcher.take(state.at(30).entries);
  const back = batcher.take(state.at(15).entries);
  assert.equal(back.reset, true);
  assert.equal(back.batch.length, 1);
  const forward = batcher.take(state.at(30).entries);
  assert.equal(forward.reset, false);
  assert.equal(forward.batch.length, 2);
});

test('a row shows the minute stamp and the commentary line', () => {
  const row = feedRow(
    eventMessage(4000, KIND.goal, { minute: 45, 'minute.added': 2, commentary: 'In it goes!' })
  );
  assert.equal(row.minute, "45+2'");
  assert.equal(row.text, 'In it goes!');
  assert.equal(row.word, 'Goal');
  assert.equal(row.announce, true);
  assert.equal(minuteStamp({ minute: 23 }), "23'");
});

test('a tactics change, which has no commentary, is named in words with the club', () => {
  const row = feedRow(
    eventMessage(10, KIND.tacticsChange, { 'team.id': 'club-a' }),
    new Map([['club-a', 'Oakmere Rangers']])
  );
  assert.equal(row.text, 'Tactics change — Oakmere Rangers');
  assert.equal(row.word, null);
  assert.equal(row.announce, false);
});

test('goals, cards, substitutions, and injuries carry a word; only goals and cards are announced', () => {
  for (const kind of [KIND.goal, KIND.card, KIND.substitution, KIND.injury]) {
    assert.ok(feedRow(eventMessage(1, kind)).word, kind);
  }
  assert.equal(feedRow(eventMessage(1, KIND.substitution)).announce, false);
  assert.equal(feedRow(eventMessage(1, KIND.injury)).announce, false);
  assert.equal(feedRow(eventMessage(1, 'throw-in')).word, null);
});

test('the empty state names what fills the feed', () => {
  assert.equal(EMPTY_TEXT, 'No events yet. The feed fills as the match plays.');
});
