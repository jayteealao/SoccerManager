// The match feed's logic: which rows are new this frame, the words of each row, and whether
// the list follows the newest row. Pure: the Commentary component renders what these return.
//
// At eight times speed a burst of events can land inside one frame. The batcher hands the
// renderer every entry released since the last frame, in tick order, as one batch, so the
// list changes once per frame however many rows arrive, and no row is dropped.

import { HIGHLIGHT_WORDS, KIND } from './match-state.js';
import { surname } from './prematch.js';
import { words } from './tactics-panel.js';

/// The empty state, before the first event is released.
export const EMPTY_TEXT = 'No events yet. The feed fills as the match plays.';

/// Only these kinds are announced to a screen reader. Announcing every row at 8x would
/// flood it.
export const ANNOUNCED = new Set([KIND.goal, KIND.card]);

export class FeedBatcher {
  constructor() {
    this.source = null;
    this.shown = 0;
  }

  /// The entries to insert this frame. `reset` is true when the rows already on screen no
  /// longer describe `entries` (a rewind), and the batch then holds every entry.
  take(entries) {
    if (entries !== this.source || entries.length < this.shown) {
      this.source = entries;
      this.shown = entries.length;
      return { reset: true, batch: entries.slice() };
    }
    const batch = entries.slice(this.shown);
    this.shown = entries.length;
    return { reset: false, batch };
  }
}

/// `23'`, or `45+2'` in added time.
export function minuteStamp(event) {
  const added = event['minute.added'];
  return added ? `${event.minute}+${added}'` : `${event.minute}'`;
}

/// What a player with each trait of play does when the trait beats an instruction, as the
/// feed line says it: "Lomrick holds up the ball against ...". One phrase per trait of the
/// catalogue (`content/traits.json`); a trait not listed reads as its name in words.
export const TRAIT_ACTS = Object.freeze({
  cuts_inside: 'cuts inside',
  hugs_touchline: 'hugs the touchline',
  shoots_from_distance: 'shoots from distance',
  places_shots: 'places his shot',
  powers_shots: 'powers his shot',
  tries_lobs_and_chips: 'tries a chip',
  tries_killer_balls: 'tries a killer ball',
  plays_short_simple_passes: 'plays it short and simple',
  switches_play: 'switches play',
  crosses_early: 'crosses early',
  runs_with_ball_often: 'runs with the ball',
  rarely_dribbles: 'passes rather than dribbles',
  tries_tricks: 'tries a trick',
  plays_one_twos: 'plays a one-two',
  holds_up_ball: 'holds up the ball',
  releases_ball_early: 'releases the ball early',
  dwells_on_ball: 'dwells on the ball',
  gets_forward: 'gets forward',
  stays_back: 'stays back',
  arrives_late_in_box: 'arrives late in the box',
  moves_into_channels: 'moves into the channel',
  presses_relentlessly: 'presses the carrier',
  dives_into_tackles: 'dives into a tackle',
  stays_on_feet: 'stays on his feet',
  goes_down_easily: 'goes down easily',
  argues_with_officials: 'argues with the referee',
  rushes_out: 'rushes out',
  stays_on_line: 'stays on his line',
  punches_rather_than_catches: 'punches rather than catches',
  kicks_long: 'kicks long',
  throws_short: 'throws short',
});

/// Why the trait beat the instruction, as the event's `trait.cause` names it.
export const CAUSE_WORDS = Object.freeze({
  pressure: 'under pressure',
  'low-mood': 'in a low mood',
});

/// The instruction levels a trait can go against, as the manager reads them.
const LEVEL_WORDS = Object.freeze({
  'pressing:low': 'Lower pressing',
  'width:narrow': 'Narrower shape',
  'width:wide': 'Wider shape',
  'tempo:slower': 'Lower tempo',
  'tempo:faster': 'Higher tempo',
  'line_height:deep': 'Deeper line',
  'line_height:high': 'Higher line',
  'passing_directness:short': 'Shorter passing',
  'passing_directness:direct': 'More direct passing',
});

/// The instruction a trait went against, from the event's `trait.instruction`: an instruction
/// level in quotes ("'Higher tempo'"), or a duty or a mentality ("a defend duty", "a
/// defensive mentality"). A level not listed reads as the tactics file's names in words.
export function instructionWords(code) {
  const text = String(code ?? '');
  const [name, level] = text.split(':');
  if (name === 'duty' && level) {
    return `${/^[aeiou]/.test(level) ? 'an' : 'a'} ${words(level).toLowerCase()} duty`;
  }
  if (name === 'mentality' && level) {
    return `a ${words(level).toLowerCase()} mentality`;
  }
  const known = LEVEL_WORDS[text];
  if (known) {
    return `'${known}'`;
  }
  return level ? `'${words(name)}: ${words(level).toLowerCase()}'` : 'an instruction';
}

/// The words of a trait override: `example`, what he did and why ("holds up the ball against
/// 'Higher tempo', under pressure"), which the panel's override line quotes, and `line`, the
/// feed line: "Lomrick holds up the ball against 'Higher tempo' — strong trait, under
/// pressure." Only a strong trait overrides, so the line always says so. `names` maps
/// `player.id` to the player's name.
export function overrideWords(event, names = new Map()) {
  const name = names.get(event['player.id']);
  const who = name ? surname(name) : 'A player';
  const trait = event['trait.name'];
  const act = TRAIT_ACTS[trait] ?? words(trait).toLowerCase();
  const cause = CAUSE_WORDS[event['trait.cause']];
  const against = instructionWords(event['trait.instruction']);
  return {
    example: `${act} against ${against}${cause ? `, ${cause}` : ''}`,
    line: `${who} ${act} against ${against} — strong trait${cause ? `, ${cause}` : ''}.`,
  };
}

/// The words of one feed row, from the event alone. `teamNames` maps `team.id` to the club
/// name, for the one kind that carries no commentary line; `names` maps `player.id` to the
/// player's name, for the trait override's line. A trait override is not read out.
export function feedRow(event, teamNames = new Map(), names = new Map()) {
  const kind = event['event.type'];
  if (kind === KIND.traitOverride) {
    const said = overrideWords(event, names);
    return {
      tick: event.tick,
      kind,
      minute: minuteStamp(event),
      text: said.line,
      word: 'Trait',
      announce: false,
      player: event['player.id'] ?? null,
      example: said.example,
    };
  }
  let text = event.commentary;
  if (!text) {
    const team = teamNames.get(event['team.id']);
    const words = kind === KIND.tacticsChange ? changeWords(event) : kindWords(kind);
    text = team ? `${words} — ${team}` : words;
  }
  return {
    tick: event.tick,
    kind,
    minute: minuteStamp(event),
    text,
    word: HIGHLIGHT_WORDS[kind] ?? null,
    announce: ANNOUNCED.has(kind),
  };
}

/// A change row names its verdict: applied, refused with the engine's reason, or queued.
function changeWords(event) {
  const substitution = event['change.kind'] === 'substitution';
  switch (event['change.state']) {
    case 'applied':
      return substitution ? 'Substitution applied' : 'Tactical change applied';
    case 'rejected': {
      const reason = event['change.rejected_reason'];
      return reason ? `Change rejected: ${reason}` : 'Change rejected';
    }
    case 'queued':
      return substitution ? 'Substitution queued' : 'Tactical change queued';
    default:
      return 'Tactics change';
  }
}

/// `free-kick` → `Free kick`.
function kindWords(kind) {
  const spaced = String(kind ?? 'event').replaceAll('-', ' ');
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

/// Pixels from the bottom within which the feed counts as reading the newest row.
export const FOLLOW_SLACK_PX = 24;

/// Whether the feed follows the newest row. Only the manager scrolling up stops it: a panel
/// that shrinks beside the feed moves the bottom away without moving the list, and leaves it
/// following. Pure: the caller reports where it scrolled the list, and the list's geometry on
/// each scroll event.
export class FeedFollow {
  constructor() {
    this.following = true;
    /// Where the feed last scrolled itself.
    this.autoTop = 0;
  }

  scrolledTo(top) {
    this.autoTop = top;
    this.following = true;
  }

  onScroll({ scrollHeight, scrollTop, clientHeight }) {
    const atBottom = scrollHeight - scrollTop - clientHeight < FOLLOW_SLACK_PX;
    this.following = atBottom || scrollTop >= this.autoTop;
  }
}
