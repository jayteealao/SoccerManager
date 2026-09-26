// The header: two club crests, the team names, the score bug, the clock, and the speed.
//
// Crests are the touchline mark drawn with each club's kit colours in place of the turf, so
// a crest and that club's markers on the pitch cannot drift apart. No image is loaded.

import { safeKit } from './colour.mjs';
import { drawMark } from './mark.mjs';

/// The score as the bug writes it: an en dash between the two numbers.
export function scoreText(home, away) {
  return `${home} – ${away}`;
}

export class Scoreboard {
  /// `nodes` holds the header elements by role; see `web/index.html`.
  constructor(nodes) {
    this.nodes = nodes;
    this.score = [null, null];
  }

  /// Names and crests from the hello.
  setTeams(teams, band) {
    teams.forEach((team, t) => {
      this.nodes.names[t].textContent = team['team.name'];
      const kit = safeKit({
        primary: team['team.kit.primary'],
        secondary: team['team.kit.secondary'],
      });
      const canvas = this.nodes.crests[t];
      const ratio = Math.max(1, Math.min(3, globalThis.devicePixelRatio || 1));
      const size = canvas.clientWidth || 28;
      canvas.width = Math.round(size * ratio);
      canvas.height = Math.round(size * ratio);
      const ctx = canvas.getContext('2d');
      ctx.scale(ratio, ratio);
      drawMark(ctx, 0, 0, size, { field: kit.fill, line: kit.ring, band });
      canvas.setAttribute('aria-label', `${team['team.name']} crest`);
    });
  }

  /// Writes the score when it changed. Returns true when it did.
  setScore(home, away) {
    if (this.score[0] === home && this.score[1] === away) {
      return false;
    }
    this.score = [home, away];
    this.nodes.home.textContent = String(home);
    this.nodes.away.textContent = String(away);
    this.nodes.bug.setAttribute('aria-label', `Score ${scoreText(home, away)}`);
    return true;
  }

  setClock(text) {
    if (this.nodes.clock.textContent !== text) {
      this.nodes.clock.textContent = text;
    }
  }

  setSpeed(effective) {
    this.nodes.speed.textContent = `${effective}x`;
  }
}
