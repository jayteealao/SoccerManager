// Game plans: a side's game-state rules as the Plans tab shows and edits them. A plan reads
// "If <every condition> → <every change>"; the plans are checked top to bottom and the higher
// one wins a clash. The rules stay in the tactics file's written form (names, not indices),
// the form the `game-state` change and the set-lineup carry. Pure: no DOM.

import { levelWord, primitiveWords } from './board-words.js';
import { levelNames, primitiveIndex, primitivesOf } from './primitives.js';

/// The most plans a side holds: the tactics file's limit.
export function maxPlans(schema) {
  return schema?.game_state?.max_rules ?? 6;
}

/// The condition kinds the editor offers, in the order the tactics file lists them, with the
/// words of each.
export const CONDITION_LABELS = Object.freeze({
  score: 'Score',
  time_left: 'Time left',
  red_cards_ours: 'Sent off (us)',
  red_cards_theirs: 'Sent off (them)',
  run_of_play: 'Run of play',
  team_energy: 'Energy',
});

export function conditionKinds(schema) {
  const listed = (schema?.game_state?.conditions ?? []).map((c) => c.kind);
  return listed.length ? listed : Object.keys(CONDITION_LABELS);
}

/// The range of condition kind `kind`: `{ min, max }`.
export function conditionRange(schema, kind) {
  const c = (schema?.game_state?.conditions ?? []).find((k) => k.kind === kind);
  return c ? { min: c.min, max: c.max } : { min: 0, max: 0 };
}

/// A new condition of `kind`, at a value inside its range.
export function newCondition(kind, schema) {
  const { min, max } = conditionRange(schema, kind);
  switch (kind) {
    case 'score':
      return { kind, state: 'trailing', by: 1 };
    case 'time_left':
      return { kind, minutes: Math.min(Math.max(15, min), max) };
    case 'red_cards_ours':
    case 'red_cards_theirs':
      return { kind, at_least: 1 };
    case 'run_of_play':
      return { kind, below: 0 };
    default:
      return { kind, below: Math.min(Math.max(0.5, min), max) };
  }
}

const plural = (n, one, many) => (n === 1 ? one : many);

/// A condition in a player's words, with its tone: `''`, `red` (a sending-off) or `en`
/// (energy).
export function conditionWords(c, schema) {
  const window = schema?.game_state?.run_of_play_window_min ?? 15;
  switch (c.kind) {
    case 'score':
      if (c.state === 'level') {
        return { text: 'Drawing', tone: '' };
      }
      return { text: `${c.state === 'leading' ? 'Winning' : 'Losing'} by ${c.by ?? 1}`, tone: '' };
    case 'time_left':
      return { text: `${c.minutes} min or less left`, tone: '' };
    case 'red_cards_ours': {
      const n = c.at_least ?? 1;
      return { text: n === 1 ? 'We have a player sent off' : `We have ${n} players sent off`, tone: 'red' };
    }
    case 'red_cards_theirs': {
      const n = c.at_least ?? 1;
      return { text: n === 1 ? 'They have a player sent off' : `They have ${n} players sent off`, tone: 'red' };
    }
    case 'run_of_play':
      return c.above !== undefined
        ? { text: `On top · last ${window} min`, tone: '' }
        : { text: `Not on top · last ${window} min`, tone: '' };
    case 'team_energy':
      return { text: `Energy below ${Math.round((c.below ?? 0) * 100)}%`, tone: 'en' };
    default:
      return { text: String(c.kind), tone: '' };
  }
}

/// A change in a player's words: `{ label, word, waits }`; a formation waits for a stoppage.
export function changeWords(a, schema) {
  if (a.formation !== undefined) {
    return { label: 'Formation', word: a.formation, waits: true };
  }
  const p = primitivesOf(schema)[primitiveIndex(schema, a.primitive)];
  const names = levelNames(schema, p);
  return {
    label: primitiveWords(a.primitive, names).full,
    word: levelWord(a.primitive, names.indexOf(a.level), names),
    waits: false,
  };
}

/// The plans of `gameState` as the list shows them.
export function planRows(gameState, schema) {
  return (gameState?.rules ?? []).map((rule, i) => ({
    n: i + 1,
    when: rule.when.map((c) => conditionWords(c, schema)),
    set: rule.set.map((a) => changeWords(a, schema)),
  }));
}

/// The plans with plan `i` moved by `delta` (-1 up, +1 down); unchanged at either end.
export function moved(rules, i, delta) {
  const j = i + delta;
  if (j < 0 || j >= rules.length) {
    return rules;
  }
  const out = [...rules];
  [out[i], out[j]] = [out[j], out[i]];
  return out;
}

/// Why `rule` cannot be saved, or null: it needs a condition and a change, and names no
/// condition kind, primitive or formation twice.
export function ruleProblem(rule) {
  if (!rule.when.length) {
    return 'Add a condition.';
  }
  if (!rule.set.length) {
    return 'Add a change.';
  }
  const kinds = rule.when.map((c) => c.kind);
  if (new Set(kinds).size !== kinds.length) {
    return 'Each condition can be used once in a plan.';
  }
  const things = rule.set.map((a) => a.primitive ?? 'formation');
  if (new Set(things).size !== things.length) {
    return 'Each change can be used once in a plan.';
  }
  return null;
}

/// When each plan can switch on: a lane per plan with a time condition, from its minute to
/// the end of the 90, and the plans that can switch on at any time.
export function timeline(gameState, schema) {
  const lanes = [];
  const anyTime = [];
  planRows(gameState, schema).forEach((row, i) => {
    const time = gameState.rules[i].when.find((c) => c.kind === 'time_left');
    const others = row.when.filter((_, k) => gameState.rules[i].when[k].kind !== 'time_left');
    const what = row.set.map((s) => `${s.label} ${s.word}`).join(', ');
    if (time) {
      const reason = others.map((o) => o.text.toLowerCase()).join(', ');
      lanes.push({
        n: row.n,
        from: Math.max(0, 90 - time.minutes),
        tone: others.find((o) => o.tone)?.tone ?? '',
        text: `${row.n} · ${reason ? `${reason} → ` : ''}${what}`,
      });
    } else {
      anyTime.push({ n: row.n, text: `${row.n} ${row.when.map((w) => w.text.toLowerCase()).join(', ')}` });
    }
  });
  return { lanes, anyTime };
}

/// `count` words: "6 of 6".
export function planCount(gameState, schema) {
  const n = gameState?.rules?.length ?? 0;
  return `${n} of ${maxPlans(schema)}`;
}

/// The words for a plans list that is full.
export function fullWords(schema) {
  const max = maxPlans(schema);
  return `${max} of ${max} ${plural(max, 'plan', 'plans')} · remove one to add another`;
}
