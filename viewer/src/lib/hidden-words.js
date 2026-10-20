// The words the page shows for the engine's word keys: the hidden values (consistency and
// injury proneness), each with how sure the club is of it, the build word, the personality
// summary and the mood. Pure: no DOM.
//
// A hidden value reaches the page only as a word key and a confidence, never as a number, and
// no function here can print one: a key with no text shows the key in words. Each result
// carries the state token it is drawn in (`--ink-2` known, `--ink-3` not yet known), never a
// band colour: words never colour.

/// The display text of each word key, per hidden value, best first. The engine bands them
/// (`content/tuning.json` → `hidden.words`); the page owns the text.
export const HIDDEN_TEXT = Object.freeze({
  consistency: Object.freeze({
    rarely_off: 'Rarely has an off day',
    steady: 'Steady',
    has_off_days: 'Has off days',
    erratic: 'Blows hot and cold',
  }),
  injury_proneness: Object.freeze({
    hardly_ever_injured: 'Hardly ever injured',
    rarely_injured: 'Rarely injured',
    picks_up_knocks: 'Picks up knocks',
    injury_prone: 'Injury prone',
  }),
});

/// The name of each hidden value, as a label.
export const HIDDEN_NAMES = Object.freeze({
  consistency: 'Consistency',
  injury_proneness: 'Injury proneness',
});

/// The build words, derived by the engine from strength and balance and never stored.
export const BUILD_TEXT = Object.freeze({
  slight: 'slight',
  athletic: 'athletic',
  powerful: 'powerful',
});

/// The words "not yet known" for a hidden value no match has shown.
export const NOT_YET_KNOWN = 'Not yet known';

/// A word key in words when the page has no text for it: `has_off_days` → "Has off days".
export function keyWords(key) {
  const text = String(key ?? '').replaceAll('_', ' ').trim();
  return text ? text[0].toUpperCase() + text.slice(1) : '';
}

/// The confidence line under a hidden word: "Not yet known — no match for us yet"; with the
/// matches seen at the club, "9 matches here, tentative" or "31 matches here, sure".
export function confidenceLine(confidence, matches) {
  const n = Number.isInteger(matches) ? matches : null;
  const here = n === null ? '' : `${n} ${n === 1 ? 'match' : 'matches'} here, `;
  switch (confidence) {
    case 'firm':
      return `${here}sure`;
    case 'tentative':
      return `${here}tentative`;
    default:
      return 'no match for us yet';
  }
}

/// One hidden value as the page shows it. `name` is `consistency` or `injury_proneness`;
/// `value` the hello's `{ word?, confidence }`; `matches` the player's matches at the club.
/// Returns `{ known, text, confidence, line, full, token, order }`: `text` the word (or "Not
/// yet known"), `line` the confidence line, `full` both in one phrase, `token` the ink it is
/// drawn in, and `order` its place for sorting (1 for the worst word up to 4 for the best;
/// null when not yet known, which sorts last).
export function hiddenWord(name, value, matches) {
  const confidence = value?.confidence ?? 'not_yet_known';
  const words = HIDDEN_TEXT[name] ?? {};
  const keys = Object.keys(words);
  const word = confidence === 'not_yet_known' ? null : (value?.word ?? null);
  if (word === null) {
    return {
      known: false,
      text: NOT_YET_KNOWN,
      confidence: 'not_yet_known',
      line: confidenceLine('not_yet_known'),
      full: `${NOT_YET_KNOWN} — ${confidenceLine('not_yet_known')}`,
      token: '--ink-3',
      order: null,
    };
  }
  const text = words[word] ?? keyWords(word);
  const at = keys.indexOf(word);
  const line = confidenceLine(confidence, matches);
  return {
    known: true,
    text,
    confidence,
    line,
    full: `“${text}” — ${line}`,
    token: '--ink-2',
    order: at < 0 ? 0 : keys.length - at,
  };
}

/// The build word of a build key, or null with none.
export function buildWord(key) {
  if (!key) {
    return null;
  }
  return BUILD_TEXT[key] ?? keyWords(key).toLowerCase();
}

// ---- Personality and mood -------------------------------------------------------------------
//
// The eleven personality values reach the page as word keys (`high`, `middling`, `low`) with
// a confidence, never as numbers. The panel never lists them: it composes one summary word,
// up to three standout words and how sure the staff are. The kick-off mood reaches it as a
// word key too.

/// The personality values in the order their standout words are picked.
export const PERSONALITY_NAMES = Object.freeze([
  'professionalism',
  'ambition',
  'temperament',
  'leadership',
  'pressure_handling',
  'sportsmanship',
  'loyalty',
  'adaptability',
  'versatility',
  'money_motivation',
  'controversy',
]);

/// The standout word of each personality value at `high` and at `low`. A value at
/// `middling` has no standout word.
export const PERSONALITY_TEXT = Object.freeze({
  professionalism: Object.freeze({ high: 'professional', low: 'unprofessional' }),
  ambition: Object.freeze({ high: 'ambitious', low: 'content' }),
  temperament: Object.freeze({ high: 'calm', low: 'hot-headed' }),
  leadership: Object.freeze({ high: 'a leader', low: 'quiet' }),
  pressure_handling: Object.freeze({ high: 'thrives under pressure', low: 'nervous under pressure' }),
  sportsmanship: Object.freeze({ high: 'sporting', low: 'a gamesman' }),
  loyalty: Object.freeze({ high: 'loyal', low: 'restless' }),
  adaptability: Object.freeze({ high: 'adapts quickly', low: 'slow to settle' }),
  versatility: Object.freeze({ high: 'versatile', low: 'one-position' }),
  money_motivation: Object.freeze({ high: 'money-minded', low: 'not money-minded' }),
  controversy: Object.freeze({ high: 'controversial', low: 'low-profile' }),
});

/// The word key of personality value `name`, or null while it is not yet known.
function personalityKey(personality, name) {
  const value = personality?.[name];
  if (!value || value.confidence === 'not_yet_known' || typeof value.word !== 'string') {
    return null;
  }
  return value.word;
}

/// Whether any personality value of `personality` is known.
export function personalityKnown(personality) {
  return PERSONALITY_NAMES.some((name) => personalityKey(personality, name) !== null);
}

/// The summary rule: the first row whose word keys all hold gives the word.
const SUMMARY_RULES = Object.freeze([
  { word: 'Driven', when: { professionalism: 'high', ambition: 'high' } },
  { word: 'Model professional', when: { professionalism: 'high', temperament: 'high' } },
  { word: 'Natural leader', when: { leadership: 'high', temperament: 'high' } },
  { word: 'Volatile', when: { temperament: 'low', controversy: 'high' } },
  { word: 'Fiery', when: { temperament: 'low' } },
  { word: 'Casual', when: { professionalism: 'low' } },
  { word: 'Big-game player', when: { pressure_handling: 'high' } },
  { word: 'Ambitious', when: { ambition: 'high' } },
  { word: 'Loyal', when: { loyalty: 'high' } },
  { word: 'Adaptable', when: { adaptability: 'high' } },
  { word: 'Adaptable', when: { versatility: 'high' } },
]);

/// The summary word of `personality` (the hello's `player.personality`): the first rule that
/// holds, "Balanced" when none does, "Not yet known" when no value is known.
export function summaryWord(personality) {
  if (!personalityKnown(personality)) {
    return NOT_YET_KNOWN;
  }
  const rule = SUMMARY_RULES.find((r) =>
    Object.entries(r.when).every(([name, key]) => personalityKey(personality, name) === key)
  );
  return rule ? rule.word : 'Balanced';
}

/// At most three standout words, in the fixed order of `PERSONALITY_NAMES`: each value at
/// `high` or `low` gives its word.
export function standouts(personality) {
  const words = [];
  for (const name of PERSONALITY_NAMES) {
    const key = personalityKey(personality, name);
    const word = key === 'high' || key === 'low' ? PERSONALITY_TEXT[name][key] : null;
    if (word) {
      words.push(word);
    }
    if (words.length === 3) {
      break;
    }
  }
  return words;
}

/// How sure the staff are of `personality`: "sure · 31 matches here", "tentative · 9 matches
/// here", or "no match for us yet" while nothing is known.
export function sureness(personality, matches) {
  const known = PERSONALITY_NAMES.map((name) => personality?.[name]).find(
    (v) => v && v.confidence !== 'not_yet_known' && typeof v.word === 'string'
  );
  if (!known) {
    return confidenceLine('not_yet_known');
  }
  const word = known.confidence === 'firm' ? 'sure' : 'tentative';
  const n = Number.isInteger(matches) ? matches : null;
  return n === null ? word : `${word} · ${n} ${n === 1 ? 'match' : 'matches'} here`;
}

/// The text, mark and mark token of each mood word key. Only the mark takes a colour; the
/// word is drawn in `--ink-2`.
export const MOOD_TEXT = Object.freeze({
  confident: Object.freeze({ text: 'Confident', mark: '▲', token: '--good' }),
  lifted: Object.freeze({ text: 'Lifted', mark: '▲', token: '--good' }),
  steady: Object.freeze({ text: 'Steady', mark: '=', token: null }),
  low: Object.freeze({ text: 'Low', mark: '▼', token: '--warn' }),
  rattled: Object.freeze({ text: 'Rattled', mark: '▼', token: '--warn' }),
});

/// The mood of word key `key` as the page shows it: `{ known, key, text, mark, token }`. A
/// steady player with no match at the club yet reads "Settling in"; a missing key (an older
/// hello) reads "Not yet known" with no mark.
export function moodWord(key, matches) {
  const mood = MOOD_TEXT[key];
  if (!mood) {
    return { known: false, key: null, text: NOT_YET_KNOWN, mark: null, token: null };
  }
  if (key === 'steady' && matches === 0) {
    return { known: true, key: 'settling', text: 'Settling in', mark: '=', token: null };
  }
  return { known: true, key, ...mood };
}
