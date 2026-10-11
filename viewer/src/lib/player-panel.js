// The player panel: the pure half. One squad player of the hello as the panel shows him: his
// attributes by group as whole numbers 1 to 20 with their bands, his build, height and age
// in words, his nationality, the range his level plays in, his hidden values in words with how
// sure the club is, and his match ratings; his rating at each position, his feet, his
// personality as a summary word with the assistant manager's view, his mood today, his traits
// of play, his state today and his season. `PlayerPanelScreen.svelte` draws it. No DOM.
//
// No 1-20 value leaves here with a decimal; no hidden value, personality value, mood or form
// leaves as a number; a field the hello does not carry (an older protocol, a converted team
// file with no body) reads as not known, never as a guess. Match ratings keep their one
// decimal; the weaker foot is the one 0-100 value.

import { ATTRIBUTE_GROUPS, attributeName, ratingChip, ratingFigures } from './columns.js';
import {
  buildWord,
  hiddenWord,
  HIDDEN_NAMES,
  moodWord,
  NOT_YET_KNOWN,
  personalityKnown,
  standouts,
  summaryWord,
  sureness,
} from './hidden-words.js';
import { BAND_WORDS, bandOf, ratingText, wholeOf } from './scale.js';
import { footWord, formWord, goalText, progressWord } from './person-words.js';

export { footWord, formWord, goalText, progressWord };

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

/// The scale of the effective-ability graphic: 12 to 20, reaching lower in steps of four
/// when the lowest of `values` (whole numbers) sits below 12, so every player's range shows.
/// Returns the first number and the five ticks, evenly spaced up to 20.
export function abilityAxis(...values) {
  const known = values.filter((v) => typeof v === 'number' && Number.isFinite(v));
  const low = known.length ? Math.min(...known) : 12;
  const from = low >= 12 ? 12 : Math.max(0, 20 - 4 * Math.ceil((20 - low) / 4));
  const step = (20 - from) / 4;
  return { from, ticks: [0, 1, 2, 3, 4].map((i) => from + i * step) };
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

// ---- Positions and feet ---------------------------------------------------------------------

/// The ten positions in the order the screen-reader list names them.
export const POSITION_ORDER = Object.freeze(['GK', 'CB', 'LB', 'RB', 'DM', 'CM', 'AM', 'LW', 'RW', 'ST']);

/// Where each position's chip sits on the pitch, in percent of its width and height: the goal
/// on the left, the left flank at the top, as board 1 draws it.
export const PITCH_PLACES = Object.freeze({
  GK: { x: 7, y: 50 },
  CB: { x: 24, y: 50 },
  LB: { x: 28, y: 16 },
  RB: { x: 28, y: 84 },
  DM: { x: 42, y: 50 },
  CM: { x: 56, y: 50 },
  AM: { x: 70, y: 50 },
  LW: { x: 76, y: 16 },
  RW: { x: 76, y: 84 },
  ST: { x: 90, y: 50 },
});

/// The side letter the positions note adds to a central position: "AM (C)".
const CENTRAL = new Set(['CB', 'DM', 'CM', 'AM', 'ST']);

/// The chips of the positions pitch from the hello's `player.positions` (tenths by code),
/// natural position `natural` outlined. A whole rating of 1 is the converter's floor, not a
/// rating: it shows "—". Null when the hello carries no ratings (before protocol 7).
export function positionChips(positions, natural) {
  if (!positions || typeof positions !== 'object' || Object.keys(positions).length === 0) {
    return null;
  }
  const chips = POSITION_ORDER.map((code) => {
    const tenths = positions[code];
    const whole = typeof tenths === 'number' ? wholeOf(tenths) : 1;
    const rated = whole > 1;
    return {
      code,
      text: rated ? String(whole) : '—',
      whole: rated ? whole : null,
      band: rated ? bandOf(whole) : null,
      natural: code === natural,
      ...PITCH_PLACES[code],
      label: `${code} ${rated ? whole : 'not rated'}${code === natural ? ' (natural)' : ''}`,
    };
  });
  const side = CENTRAL.has(natural) ? ' (C)' : '';
  return {
    chips,
    note: natural
      ? `Natural: ${natural}${side}. Away from it he loses some familiarity; his versatility sets how much.`
      : null,
  };
}

/// The feet rows: the strong foot at 100 and the weaker foot as a percent with its word and the
/// band of the percent read as a 1-20 whole, and the note on when it costs him. Null when the
/// hello carries no feet (before protocol 7).
export function feetModel(strong, weaker) {
  if ((strong !== 'left' && strong !== 'right') || typeof weaker !== 'number') {
    return null;
  }
  const names = { left: 'Left', right: 'Right' };
  const other = strong === 'left' ? 'right' : 'left';
  const percent = Math.max(1, Math.min(100, Math.round(weaker)));
  const word = footWord(percent);
  return {
    rows: [
      { foot: strong, text: `${names[strong]} (strong)`, value: '100', width: 100, band: 'v4', strong: true },
      {
        foot: other,
        text: `${names[other]} · ${word}`,
        value: `${percent}%`,
        width: percent,
        band: bandOf(wholeOf(percent * 2)),
        strong: false,
      },
    ],
    word,
    note: `Costs him only on the ${other} side or with no time to shift it. Opponents show him ${other}.`,
  };
}

// ---- Personality, mood and form -----------------------------------------------------------

/// The personality block's words: the summary word, the standouts and how sure the staff are
/// in one line, and the token each is drawn in. Never a number for a value.
export function personalityModel(personality, matches) {
  const known = personalityKnown(personality);
  if (!known) {
    return {
      known: false,
      summary: NOT_YET_KNOWN,
      standouts: [],
      sureness: sureness(personality, matches),
      line: sureness(personality, matches),
      token: '--ink-3',
    };
  }
  const words = standouts(personality);
  const sure = sureness(personality, matches);
  return {
    known: true,
    summary: summaryWord(personality),
    standouts: words,
    sureness: sure,
    line: words.length ? `${words.join(', ')} · ${sure}` : sure,
    token: '--ink-2',
  };
}

/// The mood today from the hello's `player.mood` word key (`hidden-words.js` `moodWord`).
export function moodModel(mood, matches) {
  return moodWord(mood, matches);
}

/// The assistant manager's first sentence, by summary word.
const CHARACTER = Object.freeze({
  Driven: 'Model pro — first in, last out, and he wants to win things.',
  'Model professional': 'First in, last out — the kind of professional you build around.',
  'Natural leader': 'The others listen to him, and he keeps his head.',
  Volatile: 'A live wire — brilliant one day, trouble the next.',
  Fiery: 'Plays on the edge and can boil over.',
  Casual: 'Talented, but he needs pushing in training.',
  'Big-game player': 'The bigger the occasion, the better he plays.',
  Ambitious: 'Wants to move up, and he plays like it.',
  Loyal: 'Settled here and committed to the club.',
  Adaptable: 'Fits in wherever you put him.',
  Balanced: 'No worries about his character.',
});

/// The noun of each season goal kind, for the assistant's goal sentence.
const GOAL_NOUNS = Object.freeze({
  goals: 'goals',
  assists: 'assists',
  clean_sheets: 'clean sheets',
  starts: 'starts',
  average_rating: 'rating',
  no_own_fault_absence: 'discipline',
});

/// The assistant manager's view: up to four sentences from fixed templates over the known
/// personality words, the mood, the season goal's progress and money motivation; never a
/// digit. `season` is the player's season part (or null).
export function assistantView({ personality, mood, season, matches }) {
  const out = [];
  const key = (name) => {
    const v = personality?.[name];
    return v && v.confidence !== 'not_yet_known' ? (v.word ?? null) : null;
  };
  const known = personalityKnown(personality);
  if (known) {
    out.push(CHARACTER[summaryWord(personality)] ?? CHARACTER.Balanced);
    if (key('temperament') === 'high') {
      out.push('Doesn’t bite when fouled.');
    } else if (key('temperament') === 'low') {
      out.push('Can lose his head when fouled.');
    }
  } else {
    out.push('Too early to say what he is like.');
  }
  const m = moodWord(mood, matches);
  const moodLine = {
    settling: 'Quiet in his first week, a bit unsure of himself.',
    confident: 'Confident after his last match.',
    lifted: 'In good spirits after his last match.',
    low: 'Low after his last match.',
    rattled: 'Rattled after his last match.',
  }[m.key];
  if (moodLine) {
    out.push(moodLine);
  }
  if (known) {
    const word = progressWord(season?.progress);
    const noun = GOAL_NOUNS[season?.goal?.kind];
    if (word && noun) {
      out.push(word === 'behind' ? `Behind on his ${noun} target and knows it.` : `Chasing his ${noun} target.`);
    } else if (key('money_motivation') === 'low') {
      out.push('Money matters to him only a little.');
    } else if (key('money_motivation') === 'high') {
      out.push('Money matters to him.');
    }
  }
  return out.slice(0, 4);
}

// ---- Traits of play -----------------------------------------------------------------------

/// The trait catalogue's names in file order (`content/traits.json`); a test checks the copy.
export const TRAIT_ORDER = Object.freeze([
  'cuts_inside', 'hugs_touchline', 'shoots_from_distance', 'places_shots', 'powers_shots',
  'tries_lobs_and_chips', 'tries_killer_balls', 'plays_short_simple_passes', 'switches_play',
  'crosses_early', 'runs_with_ball_often', 'rarely_dribbles', 'tries_tricks', 'plays_one_twos',
  'holds_up_ball', 'releases_ball_early', 'dwells_on_ball', 'gets_forward', 'stays_back',
  'arrives_late_in_box', 'moves_into_channels', 'presses_relentlessly', 'dives_into_tackles',
  'stays_on_feet', 'goes_down_easily', 'argues_with_officials', 'rushes_out', 'stays_on_line',
  'punches_rather_than_catches', 'kicks_long', 'throws_short',
]);

/// The most trait rows the panel lists.
export const MAX_TRAIT_ROWS = 8;

/// A trait name in words: `tries_killer_balls` → "Tries killer balls".
export function traitWords(name) {
  const text = String(name ?? '').replaceAll('_', ' ').trim();
  return text ? text[0].toUpperCase() + text.slice(1) : '';
}

/// The traits of play rows from the hello's `player.traits`, in catalogue order, at most
/// eight: each with its words and its strength word ("Strong" or "Moderate").
export function traitRows(traits) {
  const at = (name) => {
    const i = TRAIT_ORDER.indexOf(name);
    return i < 0 ? TRAIT_ORDER.length : i;
  };
  return (Array.isArray(traits) ? traits : [])
    .filter((t) => t && typeof t.name === 'string')
    .sort((a, b) => at(a.name) - at(b.name))
    .slice(0, MAX_TRAIT_ROWS)
    .map((t) => ({
      name: t.name,
      words: traitWords(t.name),
      strength: t.strength === 'strong' ? 'Strong' : 'Moderate',
      strong: t.strength === 'strong',
    }));
}

/// The line under the traits naming the last match's overrides, or null when no count is
/// given or it is none. `example` is one override's words, when known.
export function overrideLine(count, example = null) {
  if (!Number.isInteger(count) || count <= 0) {
    return null;
  }
  const head = `${count} ${count === 1 ? 'time' : 'times'} last match a trait beat an instruction`;
  return { count, text: example ? `${head} — ${example}` : `${head}.` };
}

// ---- Season ---------------------------------------------------------------------------------

/// The last ratings the season line draws.
export const SEASON_LINE_POINTS = 6;

/// The season block from the player's part of the views answer's `season` (`p`, or null when
/// the answer carries no season): the last six ratings for the line, So far, Story, Goal and
/// Form. With no match yet it reads "No matches this season yet." with his goal at nothing.
export function seasonModel(p) {
  if (!p || typeof p !== 'object') {
    return { none: true, empty: true, text: 'No season record in this match’s data.', points: [], rows: [] };
  }
  const ratings = Array.isArray(p.ratings) ? p.ratings : [];
  const apps = ratings.length;
  const goal = goalText(p.goal, p);
  const progress = progressWord(p.progress);
  const form = formWord(p.form);
  const plural = (count, one, many) => `${count} ${count === 1 ? one : many}`;
  const goalRow = goal ? `${goal.aim} · ${goal.so}${progress ? `, ${progress}` : ''}` : null;
  if (apps === 0) {
    return {
      none: false,
      empty: true,
      text: 'No matches this season yet.',
      points: [],
      rows: [
        goalRow ? { key: 'goal', label: 'Goal', text: goalRow } : null,
        { key: 'form', label: 'Form', text: form.text, mark: form.mark, token: form.token, known: form.known },
      ].filter(Boolean),
    };
  }
  const values = ratings.map((r) => Number(r.rating));
  const avg = values.reduce((a, b) => a + b, 0) / apps;
  const last3 = values.slice(-3);
  let story;
  if (last3.length === 3 && last3.every((r) => r >= 7.0)) {
    story = 'Three good games in a row.';
  } else if (last3.length === 3 && last3.every((r) => r <= 6.3)) {
    story = 'A difficult run of three games.';
  } else {
    story = `${plural(p.starts ?? 0, 'start', 'starts')} and ${plural(p.minutes ?? 0, 'minute', 'minutes')} so far.`;
  }
  const soFar = [
    plural(apps, 'app', 'apps'),
    plural(p.goals ?? 0, 'goal', 'goals'),
    plural(p.assists ?? 0, 'assist', 'assists'),
    (p.clean_sheets ?? 0) > 0 ? plural(p.clean_sheets, 'clean sheet', 'clean sheets') : null,
    `avg ${ratingText(avg)}`,
  ].filter(Boolean);
  return {
    none: false,
    empty: false,
    text: null,
    points: values.slice(-SEASON_LINE_POINTS),
    rows: [
      { key: 'so-far', label: 'So far', text: soFar.join(' · ') },
      { key: 'story', label: 'Story', text: story },
      goalRow ? { key: 'goal', label: 'Goal', text: goalRow } : null,
      { key: 'form', label: 'Form', text: form.text, mark: form.mark, token: form.token, known: form.known },
    ].filter(Boolean),
  };
}

// ---- His state today ------------------------------------------------------------------------

/// The familiarity cost word of a gap between whole ratings: a little for 1 to 3, some for
/// 4 to 7, a lot from 8.
function costWord(gap) {
  if (gap >= 8) {
    return 'a lot';
  }
  return gap >= 4 ? 'some' : 'a little';
}

/// The four rows under Effective ability: Mind (mood and form), Body (condition and
/// sharpness), Familiarity (the slot he plays against his natural position) and Surroundings.
/// `played` is the position code of his slot today, or null when he has none.
export function stateRows({ mood, form, condition, sharpness, natural, played, positions }) {
  let body;
  if (typeof condition !== 'number') {
    body = NOT_YET_KNOWN;
  } else if (condition < 75) {
    body = 'Tired';
  } else if (typeof sharpness === 'number' && sharpness < 80) {
    body = 'Short of sharpness';
  } else if (condition >= 90) {
    body = 'Fresh and sharp';
  } else {
    body = 'A little tired';
  }
  let familiarity;
  let out = false;
  if (!played) {
    familiarity = 'Not in the eleven';
  } else if (played === natural) {
    familiarity = 'Natural position — knows the role';
  } else {
    out = true;
    const home = positions?.[natural];
    const there = positions?.[played];
    if (typeof home === 'number' && typeof there === 'number') {
      const gap = wholeOf(home) - wholeOf(there);
      familiarity =
        gap <= 0 ? `Plays ${played} as well as ${natural}` : `Out of position at ${played} — costs him ${costWord(gap)}`;
    } else {
      familiarity = `Out of position at ${played}`;
    }
  }
  return [
    { key: 'mind', label: 'Mind', mood, form },
    { key: 'body', label: 'Body', text: body },
    { key: 'familiarity', label: 'Familiarity', text: familiarity, out },
    { key: 'surroundings', label: 'Surroundings', text: 'No effect today', muted: true },
  ];
}

/// The whole panel of squad `player`. `ratings` is his stored and live match ratings, oldest
/// first (`[{ match, rating }]`). `season` is his part of the views answer's season (or
/// null); `played` the position code of his slot today (or null).
export function panelModel(player, ratings = [], { season = null, played = null, overrides = [] } = {}) {
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
    ...personPart(player, matches, season, played, overrides),
  };
}

/// The person's parts of the panel: positions, feet, personality with the assistant's view,
/// mood, traits, the override line (from `overrides`, the feed's trait override rows of his
/// last match, each with its `example` words), the season and the state rows.
function personPart(player, matches, season, played, overrides) {
  const positions = player['player.positions'] ?? null;
  const position = player['player.position'] ?? null;
  const personality = player['player.personality'] ?? null;
  const mood = moodModel(player['player.mood'], matches);
  const form = formWord(season?.form);
  return {
    positions: positionChips(positions, position),
    feet: feetModel(player['player.strong_foot'], player['player.weaker_foot']),
    personality: personalityModel(personality, matches),
    assistant: assistantView({ personality, mood: player['player.mood'], season, matches }),
    mood,
    traits: Array.isArray(player['player.traits']) || personality ? traitRows(player['player.traits']) : null,
    override: Array.isArray(overrides) && overrides.length > 0 ? overrideLine(overrides.length, overrides[0].example ?? null) : null,
    season: seasonModel(season),
    state: stateRows({
      mood,
      form,
      condition: player['player.condition'],
      sharpness: player['player.sharpness'],
      natural: position,
      played,
      positions,
    }),
  };
}
