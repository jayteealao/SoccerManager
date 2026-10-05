// The score strip's words: the score as the bug writes it, the fixture as the header names
// it, and each side's scorers under its name. Pure: the ScoreStrip component renders them.
//
// Crests are drawn from each club's kit colours, made safe by `safeKit`, so a crest and that
// club's markers on the pitch cannot drift apart. No image is loaded.

import { minuteStamp } from './feed.js';

/// The score as the bug writes it: an en dash between the two numbers.
export function scoreText(home, away) {
  return `${home} – ${away}`;
}

/// `Home v Away` before a score counts, `Home 1–0 Away` once the match is under way.
export function fixtureTitle(names, score = null) {
  if (!names) {
    return 'Touchline';
  }
  return score ? `${names[0]} ${score[0]}–${score[1]} ${names[1]}` : `${names[0]} v ${names[1]}`;
}

/// The two, three or four capitals a crest carries: the first letter of each word.
export function initials(name) {
  const words = String(name ?? '')
    .split(/\s+/)
    .filter(Boolean);
  return words
    .slice(0, 3)
    .map((w) => w[0].toUpperCase())
    .join('');
}

/// The scorer lines under each team's name, for the goals released at the rendered tick:
/// `Oduya 23', Hask 58'`. A scorer is named by the surname on the hello roster; a goal whose
/// scorer is not on it shows its minute alone.
export function scorerLines(goals, teams) {
  const lines = ['', ''];
  if (!teams) {
    return lines;
  }
  const names = new Map();
  for (const team of teams) {
    for (const player of team.roster ?? []) {
      names.set(player['player.id'], String(player['player.name'] ?? '').split(/\s+/).pop());
    }
  }
  const parts = [[], []];
  for (const goal of goals) {
    const side = teams.findIndex((t) => t['team.id'] === goal['team.id']);
    if (side < 0) {
      continue;
    }
    const name = names.get(goal['player.id']);
    parts[side].push(name ? `${name} ${minuteStamp(goal)}` : minuteStamp(goal));
  }
  return parts.map((p) => p.join(', '));
}
