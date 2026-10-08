// The player panel: the pure half. One squad player of the hello as the panel shows him: his
// attributes by group as whole numbers 1 to 20 with their bands, his build, height and age
// in words, his nationality, the range his level plays in, his hidden values in words with how
// sure the club is, and his match ratings. `PlayerPanelScreen.svelte` draws it. No DOM.
//
// No 1-20 value leaves here with a decimal; no hidden value leaves as a number; a field the
// hello does not carry (an older protocol, a converted team file with no body) reads as not
// known, never as a guess. Match ratings keep their one decimal.

import { ATTRIBUTE_GROUPS, attributeName, ratingChip, ratingFigures } from './columns.js';
import { buildWord, hiddenWord, HIDDEN_NAMES } from './hidden-words.js';
import { BAND_WORDS, bandOf, ratingText, wholeOf } from './scale.js';

/// The engine's body references (`content/tuning.json` → `engine.contract.body`): the height
/// and the age at which the body jobs have no effect. A test checks them against the file.
export const BODY_REFERENCE = Object.freeze({ heightCm: 181, age: 28 });

/// The position words of the header.
export const POSITION_WORDS = Object.freeze({
  GK: 'Goalkeeper',
  CB: 'Centre back',
  LB: 'Left back',
  RB: 'Right back',
  DM: 'Defensive midfielder',
  CM: 'Centre midfielder',
  AM: 'Attacking midfielder',
  LW: 'Left winger',
  RW: 'Right winger',
  ST: 'Striker',
});

/// The groups the panel shows, in the sketch's column order: an outfield player's technical,
/// mental and physical groups; a keeper's goalkeeping group in place of the technical one.
const OUTFIELD_GROUPS = Object.freeze([
  { id: 'technical', label: 'Technical' },
  { id: 'mental', label: 'Mental' },
  { id: 'physical', label: 'Physical' },
]);
const KEEPER_GROUPS = Object.freeze([
  { id: 'goalkeeping', label: 'Goalkeeping' },
  { id: 'mental', label: 'Mental' },
  { id: 'physical', label: 'Physical' },
]);

/// The height in words: reach and turning against the reference height.
export function heightWords(cm) {
  if (typeof cm !== 'number') {
    return null;
  }
  const gap = cm - BODY_REFERENCE.heightCm;
  if (gap >= 4) {
    return `${cm} cm: reaches higher in the air than most, turns a little slower`;
  }
  if (gap <= -4) {
    return `${cm} cm: turns a little quicker than most, reaches lower in the air`;
  }
  return `${cm} cm: an average reach in the air and an average turn`;
}

/// The age in words: recovery between matches and the late-match fade.
export function ageWords(age) {
  if (typeof age !== 'number') {
    return null;
  }
  if (age <= BODY_REFERENCE.age) {
    return `Age ${age}: recovers at full speed; no late-match fade from age yet`;
  }
  const years = age - BODY_REFERENCE.age;
  return years >= 4
    ? `Age ${age}: recovers more slowly between matches and fades late in a match`
    : `Age ${age}: recovers a little more slowly and fades a little late in a match`;
}

/// The range his level plays in, as whole numbers: "Plays between 16 and 18", or "Plays at
/// 17" when both round to one number. Null with no levels in the hello.
export function playsBetween(range) {
  if (!Array.isArray(range) || range.length !== 2) {
    return null;
  }
  const lo = wholeOf(Math.min(range[0], range[1]));
  const hi = wholeOf(Math.max(range[0], range[1]));
  return {
    lo,
    hi,
    text: lo === hi ? `Plays at ${hi}` : `Plays between ${lo} and ${hi}`,
  };
}

/// The match-rating lines of the panel: the chips of the newest matches, the last, and the
/// averages of the last 3 and the last 10 with how many matches each covers.
export function ratingLines(list) {
  const figures = ratingFigures(list);
  if (!figures) {
    return { none: true, text: 'No match rating yet', chips: [], summary: 'no match rating yet' };
  }
  const covers = (n) => `${n} ${n === 1 ? 'match' : 'matches'}`;
  return {
    none: false,
    chips: (list ?? []).slice(-5).map((r) => ratingChip(r.rating)),
    last: ratingText(figures.last),
    avg3: { text: ratingText(figures.avg3.value), of: covers(figures.avg3.of) },
    avg10: { text: ratingText(figures.avg10.value), of: covers(figures.avg10.of) },
    summary: `${covers(figures.count)} · rating avg ${ratingText(figures.avg10.value)}, last ${ratingText(figures.last)}`,
  };
}

/// The whole panel of squad `player`. `ratings` is his stored and live match ratings, oldest
/// first (`[{ match, rating }]`).
export function panelModel(player, ratings = []) {
  if (!player) {
    return null;
  }
  const attributes = player['player.attributes'] ?? {};
  const keeper = player['player.position'] === 'GK';
  const groups = (keeper ? KEEPER_GROUPS : OUTFIELD_GROUPS).map((g) => ({
    ...g,
    rows: ATTRIBUTE_GROUPS[g.id]
      .filter((name) => typeof attributes[name] === 'number')
      .map((name) => {
        const whole = wholeOf(attributes[name]);
        const band = bandOf(whole);
        return { name, label: attributeName(name), value: whole, band, word: BAND_WORDS[band] };
      }),
  })).filter((g) => g.rows.length > 0);
  const matches = Number.isInteger(player['player.matches_at_club']) ? player['player.matches_at_club'] : null;
  const hidden = ['consistency', 'injury_proneness'].map((name) => ({
    name,
    label: HIDDEN_NAMES[name],
    ...hiddenWord(name, player[`player.${name}`], matches),
  }));
  const build = buildWord(player['player.build']);
  const position = player['player.position'] ?? '';
  const age = player['player.age'];
  const height = player['player.height'];
  const nation = player['player.nationality'] ?? null;
  const newSigning = matches === 0;
  const lines = ratingLines(ratings);
  return {
    id: player['player.id'],
    name: player['player.name'],
    shirt: player['player.shirt'],
    position,
    positionWords: POSITION_WORDS[position] ?? position,
    age: typeof age === 'number' ? `${age} years old` : 'Age not known',
    facts: [typeof height === 'number' ? `${height} cm` : null, build ? `${build} build` : null].filter(Boolean),
    sub: `${position}${newSigning ? ' · signed recently' : ''} · ${lines.none ? 'no match rating yet' : lines.summary}`,
    nation: {
      value: newSigning ? 'New at the club' : matches === null ? 'Not known' : 'Settled',
      label: [nation, matches === null ? null : newSigning ? 'no match for us yet' : `${matches} ${matches === 1 ? 'match' : 'matches'} here`]
        .filter(Boolean)
        .join(' · '),
    },
    groups,
    body: {
      build: build ? `Build: ${build}, from his strength and balance.` : null,
      height: heightWords(height),
      age: ageWords(age),
    },
    plays: playsBetween(player['player.plays_between']),
    level: typeof player['player.level'] === 'number' ? wholeOf(player['player.level']) : null,
    hidden,
    ratings: lines,
  };
}
