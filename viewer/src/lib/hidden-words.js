// The words the page shows for the engine's word keys: the hidden values (consistency and
// injury proneness), each with how sure the club is of it, and the build word. Pure: no DOM.
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
