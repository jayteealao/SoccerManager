// The read-only Pre-match line-ups: the pure half. Every row the page shows comes from the
// hello, so nothing here changes the lineup or the tactic. `PrematchScreen.svelte` and its
// components draw these rows.
//
// The parts the engine has no model for yet (condition between matches, match sharpness, the
// workload note, weather, crowd, the referee, what is at stake) are LATER stubs on the page and
// have no function here.

import { LENGTH, WIDTH } from './pitch.js';
import { words } from './tactics-panel.js';

export const STARTERS = 11;

/// The injury-risk word for an injury-resistance figure (0 to 100): the one figure the engine
/// holds that moves a player's chance of injury (injury chance scales with 1.5 less the
/// resistance). Each word carries its state token; the word, never the colour, is the state.
export function riskWord(injuryResistance) {
  const value = Number(injuryResistance ?? 0);
  if (value < 40) {
    return { word: 'High', tone: 'bad' };
  }
  if (value < 60) {
    return { word: 'Raised', tone: 'warn' };
  }
  return { word: 'Low', tone: 'good' };
}

/// Where each slot of a formation stands at kick-off, in percent of a pitch drawn with the
/// home side attacking to the right: `left` along the length, `top` across. The engine's rule
/// (`kick_off_position`): a slot whose depth from the halfway line is more than −1 m stands
/// 1 m inside its own half. The away side attacks to the left, so its length is mirrored; the
/// engine keeps the across value for both sides.
export function kickOffDots(slots, side) {
  const half = LENGTH / 2;
  return slots.map((slot) => {
    const depth = slot.x - half;
    const own = depth > -1 ? half - 1 : slot.x;
    const x = side === 0 ? own : LENGTH - own;
    return {
      left: (x / LENGTH) * 100,
      top: ((slot.y + WIDTH / 2) / WIDTH) * 100,
      position: slot.position,
    };
  });
}

/// The formation a hello team names, as a word for the sheet: the team's `formation` name,
/// or for the home team the setup's formation index when an earlier engine sent no name.
export function formationName(team, schema) {
  if (team?.formation) {
    return team.formation;
  }
  const index = team?.setup?.formation;
  return index !== undefined ? (schema?.formations?.[index]?.name ?? '') : '';
}

/// The formation's slots by name, or null when the tactics file holds no such formation.
export function formationSlots(schema, name) {
  return schema?.formations?.find((f) => f.name === name)?.slots ?? null;
}

/// The home sheet the manager set on Tactics: the eleven in slot order and the bench, as
/// squad indices into the hello's home `squad`. Each row carries the injury-risk word.
export function squadSheet(squad, lineup, bench) {
  const row = (i) => {
    const p = squad?.[i];
    if (!p) {
      return null;
    }
    return {
      id: p['player.id'],
      shirt: p['player.shirt'],
      name: p['player.name'],
      surname: surname(p['player.name']),
      position: p['player.position'],
      squad: i,
      risk: riskWord(p['player.injury_resistance']),
    };
  };
  return {
    eleven: (lineup ?? []).map(row).filter(Boolean),
    bench: (bench ?? []).map(row).filter(Boolean),
  };
}

/// The other team's sheet from its hello roster: the eleven in slot order, then the bench.
/// The engine sends no injury figure for the other team, so its rows carry no risk.
export function rosterSheet(team) {
  const row = (p) => ({
    id: p['player.id'],
    shirt: p['player.shirt'],
    name: p['player.name'],
    surname: surname(p['player.name']),
    position: p['player.position'],
    squad: p['player.squad_index'],
    risk: null,
  });
  const roster = team?.roster ?? [];
  return { eleven: roster.slice(0, STARTERS).map(row), bench: roster.slice(STARTERS).map(row) };
}

/// The last word of a name, as the pitch labels and the substitutes grid show it.
export function surname(name) {
  return String(name ?? '').split(' ').at(-1);
}

/// The kick-off pitch: both elevens at their kick-off places, each dot with the shirt and the
/// surname. `sides` holds each team's `{ formation, eleven }`, home first. A team whose
/// formation the tactics file does not hold, or whose eleven is short, has no dots.
export function kickOffSheet(schema, sides) {
  return sides.map(({ formation, eleven }, side) => {
    const slots = formationSlots(schema, formation);
    if (!slots || eleven.length < STARTERS) {
      return [];
    }
    return kickOffDots(slots, side).map((dot, slot) => ({
      ...dot,
      slot,
      shirt: eleven[slot].shirt,
      surname: eleven[slot].surname,
      keeper: slots[slot].position === 'GK',
    }));
  });
}

/// The rule-pack checks: the live rows from the hello's substitution rules, the bench size and
/// the knockout flag; then the rows the engine has no rule for yet, which the page shows as
/// LATER. Each live row is `{ mark, tone, text }`.
export function rulePackRows(hello) {
  const subs = hello?.substitutions ?? {};
  const limit = subs.limit ?? 0;
  const windows = subs.windows ?? 0;
  const exempt = subs.windows_exempt ?? [];
  const bench = hello?.tactics?.ai?.bench_size ?? null;
  const live = [];
  const halfTime = exempt.includes('half_time') ? '; half-time uses none' : '';
  live.push({
    mark: '✓',
    tone: 'good',
    text: `${count(limit, 'substitute')} in ${count(windows, 'window')}${halfTime}`,
  });
  const extraSubs = subs.extra_substitutions ?? 0;
  const extraWindows = subs.extra_windows ?? 0;
  if (extraSubs > 0 || extraWindows > 0) {
    live.push({
      mark: '✓',
      tone: 'good',
      text: `${extraParts(extraSubs, extraWindows)} in extra time`,
    });
  }
  if (bench !== null) {
    live.push({ mark: '✓', tone: 'good', text: `${bench} on the bench` });
  }
  return {
    live,
    level: hello?.knockout ? 'Extra time, then penalties' : 'The match ends level',
    later: [
      'Concussion substitutes',
      'Under-21 registration',
      'Cup yellow cards',
      'The video referee',
    ],
  };
}

function count(n, noun) {
  return `${n} ${noun}${n === 1 ? '' : 's'}`;
}

function extraParts(subs, windows) {
  if (subs > 0 && windows > 0) {
    return subs === 1 && windows === 1
      ? 'One more substitute and window'
      : `${count(subs, 'more substitute')} and ${count(windows, 'more window')}`;
  }
  return subs > 0 ? count(subs, 'more substitute') : count(windows, 'more window');
}

/// The strip's lead: the fixture and what a level score means. The other strip cells (league
/// places, weather, crowd, referee, importance) are LATER stubs.
export function prematchLead(hello) {
  const [home, away] = (hello?.teams ?? []).map((t) => t['team.name']);
  return {
    value: home && away ? `${home} v ${away}` : 'Pre-match',
    label: hello?.knockout ? 'Knockout: extra time and penalties if level' : 'One match: it can end level',
  };
}

/// The mentality name the tactics file gives index `i`, in words.
export function mentalityWord(schema, i) {
  return words(schema?.mentalities?.[i]?.name ?? '');
}
