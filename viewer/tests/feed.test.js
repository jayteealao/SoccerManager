// The feed at eight times speed: every event, in order, one insert per frame.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import fs from 'node:fs';
import path from 'node:path';

import {
  EMPTY_TEXT,
  FeedBatcher,
  FeedFollow,
  feedRow,
  instructionWords,
  minuteStamp,
  TRAIT_ACTS,
} from '../src/lib/feed.js';
import { KIND, MatchState } from '../src/lib/match-state.js';
import { eventMessage, REPO_ROOT } from './helpers.js';

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

test('a change row names its verdict, and a refusal carries the engine reason', () => {
  const clubs = new Map([['club-a', 'Oakmere Rangers']]);
  const row = (fields) =>
    feedRow(eventMessage(10, KIND.tacticsChange, { 'team.id': 'club-a', ...fields }), clubs).text;
  assert.equal(
    row({ 'change.kind': 'tactics', 'change.state': 'applied' }),
    'Tactical change applied — Oakmere Rangers'
  );
  assert.equal(
    row({ 'change.kind': 'substitution', 'change.state': 'applied' }),
    'Substitution applied — Oakmere Rangers'
  );
  assert.equal(
    row({
      'change.kind': 'substitution',
      'change.state': 'rejected',
      'change.rejected_reason': 'substitution limit reached (5 of 5)',
    }),
    'Change rejected: substitution limit reached (5 of 5) — Oakmere Rangers'
  );
  assert.equal(
    row({ 'change.kind': 'tactics', 'change.state': 'queued', 'team.id': undefined }),
    'Tactical change queued'
  );
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

test('the feed keeps following the newest row when a panel beside it shrinks it', () => {
  const follow = new FeedFollow();
  assert.equal(follow.following, true, 'a new feed follows');
  // The feed scrolled itself to the bottom: 437 high, 112 scrolled, 325 shown.
  follow.scrolledTo(112);
  follow.onScroll({ scrollHeight: 437, scrollTop: 112, clientHeight: 325 });
  assert.equal(follow.following, true);
  // The scroll event of that same scroll arrives after a chip appeared and the roles list
  // opened, which shrank the feed to 96 pixels. The list did not move, so it still follows;
  // the old check read 437 - 112 - 96 = 229 pixels from the bottom as scrolled up.
  follow.onScroll({ scrollHeight: 437, scrollTop: 112, clientHeight: 96 });
  assert.equal(follow.following, true, 'a shrink is not the manager scrolling up');
  // The manager scrolls up to read an older row, then back down to the newest.
  follow.scrolledTo(748);
  follow.onScroll({ scrollHeight: 844, scrollTop: 300, clientHeight: 96 });
  assert.equal(follow.following, false);
  follow.onScroll({ scrollHeight: 844, scrollTop: 748, clientHeight: 96 });
  assert.equal(follow.following, true);
});

test('a trait override names the player, what he did, the instruction and the cause, and is not read out', () => {
  const names = new Map([['p', 'Alvin Lomrick']]);
  const event = (fields) => eventMessage(300, KIND.traitOverride, { 'player.id': 'p', 'team.id': 't', ...fields });
  const held = feedRow(
    event({ 'trait.name': 'holds_up_ball', 'trait.cause': 'pressure', 'trait.instruction': 'tempo:faster' }),
    new Map(),
    names
  );
  assert.equal(held.kind, KIND.traitOverride);
  assert.equal(held.text, "Lomrick holds up the ball against 'Higher tempo' — strong trait, under pressure.");
  assert.equal(held.word, 'Trait');
  assert.equal(held.announce, false);
  assert.equal(held.player, 'p');
  assert.equal(held.example, "holds up the ball against 'Higher tempo', under pressure");
  const forward = feedRow(
    event({ 'trait.name': 'gets_forward', 'trait.cause': 'low-mood', 'trait.instruction': 'duty:defend' }),
    new Map(),
    names
  );
  assert.equal(forward.text, 'Lomrick gets forward against a defend duty — strong trait, in a low mood.');
  assert.equal(instructionWords('duty:attack'), 'an attack duty');
  assert.equal(instructionWords('mentality:defensive'), 'a defensive mentality');
  assert.equal(instructionWords('passing_directness:short'), "'Shorter passing'");
  assert.equal(instructionWords('time_wasting:on'), "'Time wasting: on'");
  const unknown = feedRow(event({ 'player.id': 'x', 'trait.name': 'new_habit', 'trait.instruction': 'width:wide' }), new Map(), names);
  assert.equal(unknown.text, "A player new habit against 'Wider shape' — strong trait.");
});

test('every trait of the catalogue has an act phrase', () => {
  const catalogue = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'content/traits.json'), 'utf8'));
  assert.deepEqual(Object.keys(TRAIT_ACTS).sort(), catalogue.traits.map((t) => t.name).sort());
});
