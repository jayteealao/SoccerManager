// The two lineups: who is on the pitch, how tired they are, and what the referee showed them.
//
// Every state carries a word beside its colour: Fresh, Tiring, Exhausted, Yellow, Red,
// Injured, Sent off. A colour on its own never carries a state.

/// Energy bands, highest first. The word is the state; the token only reinforces it.
export const BANDS = Object.freeze([
  { min: 0.7, word: 'Fresh', token: '--tl-success' },
  { min: 0.4, word: 'Tiring', token: '--tl-warning' },
  { min: -Infinity, word: 'Exhausted', token: '--tl-danger' },
]);

export const PLAYERS_PER_TEAM = 11;

/// The band for an energy value from 0.0 (spent) to 1.0 (fresh). Before the first condition
/// message every player is fresh.
export function energyBand(energy) {
  const value = energy ?? 1;
  return BANDS.find((band) => value >= band.min);
}

/// The word beside a card chip.
export function cardWord(card) {
  if (card === 'red') {
    return 'Red';
  }
  return card === 'yellow' ? 'Yellow' : null;
}

/// The rows of both teams at the view `state` from `match-state.mjs`. `rosters` holds each
/// team's hello roster: the 11 starters in wire-slot order, then the named bench.
/// `teamIds` are the two `team.id` values, home first.
export function lineupModel(rosters, teamIds, state) {
  const slots = rosters.map((roster) => roster.slice(0, PLAYERS_PER_TEAM));
  for (const sub of state.substitutions) {
    let team = teamIds.indexOf(sub.team);
    if (team < 0) {
      team = slots.findIndex((s) => s.some((p) => p['player.id'] === sub.off));
    }
    if (team < 0) {
      continue;
    }
    const slot = slots[team].findIndex((p) => p['player.id'] === sub.off);
    const on = rosters[team].find((p) => p['player.id'] === sub.on);
    if (slot >= 0 && on) {
      slots[team][slot] = on;
    }
  }
  return slots.map((players, team) =>
    players.map((player, slot) => {
      const id = player['player.id'];
      const wire = team * PLAYERS_PER_TEAM + slot;
      const energy = state.energy ? state.energy[wire] : null;
      const band = energyBand(energy);
      const card = state.cards.get(id) ?? null;
      const sentOff = state.sentOff.has(id);
      const injured = state.injuries.has(id);
      let condition = band.word;
      if (sentOff) {
        condition = 'Sent off';
      } else if (injured) {
        condition = 'Injured';
      }
      return {
        wire,
        id,
        name: player['player.name'],
        shirt: player['player.shirt'],
        position: player['player.position'],
        energy: energy ?? 1,
        band: band.word,
        token: band.token,
        condition,
        card,
        cardWord: cardWord(card),
        injured,
        sentOff,
      };
    })
  );
}

/// Each team's named bench at the view `state`: the roster entries after the 11 starters,
/// less every player a substitution has brought on.
export function benchModel(rosters, state) {
  const on = new Set(state.substitutions.map((s) => s.on));
  return rosters.map((roster) =>
    roster.slice(PLAYERS_PER_TEAM).filter((p) => !on.has(p['player.id']))
  );
}

/// A cheap fingerprint of what a row shows, so a flush writes only the rows that changed.
function rowKey(row) {
  return `${row.id}|${Math.round(row.energy * 100)}|${row.condition}|${row.card}`;
}

/// The DOM side: two columns, home and away.
export class Lineups {
  constructor(root) {
    this.root = root;
    this.rosters = [[], []];
    this.teamIds = [null, null];
    this.rows = [[], []];
    this.keys = [[], []];
    this.model = [[], []];
    this.benches = [null, null];
    this.benchKey = '';
  }

  setTeams(teams) {
    const doc = this.root.ownerDocument;
    this.rosters = teams.map((t) => t.roster ?? []);
    this.teamIds = teams.map((t) => t['team.id']);
    this.root.replaceChildren();
    this.rows = [[], []];
    this.keys = [[], []];
    teams.forEach((team, t) => {
      const column = doc.createElement('section');
      column.className = 'lineup';
      column.setAttribute('aria-label', `${team['team.name']} lineup`);
      const title = doc.createElement('h3');
      title.className = 'lineup__team';
      title.textContent = team['team.name'];
      const list = doc.createElement('ol');
      list.className = 'lineup__list';
      for (let slot = 0; slot < PLAYERS_PER_TEAM; slot += 1) {
        const li = doc.createElement('li');
        li.className = 'lineup__row';
        li.innerHTML =
          '<span class="lineup__pos"></span><span class="lineup__shirt tl-num"></span>' +
          '<span class="lineup__name"></span><span class="lineup__card" hidden></span>' +
          '<span class="lineup__status">' +
          '<span class="lineup__bar" aria-hidden="true"><span class="lineup__fill"></span></span>' +
          '<span class="lineup__condition"></span></span>';
        list.append(li);
        this.rows[t].push(li);
        this.keys[t].push('');
      }
      const benchTitle = doc.createElement('h4');
      benchTitle.className = 'lineup__bench-title';
      benchTitle.textContent = 'Bench';
      const bench = doc.createElement('ul');
      bench.className = 'lineup__bench';
      this.benches[t] = bench;
      column.append(title, list, benchTitle, bench);
      this.root.append(column);
    });
    this.benchKey = '';
  }

  /// Writes the rows whose content changed. Returns the model.
  update(state) {
    this.model = lineupModel(this.rosters, this.teamIds, state);
    this.model.forEach((players, t) => {
      players.forEach((row, slot) => {
        const li = this.rows[t][slot];
        if (!li) {
          return;
        }
        const key = rowKey(row);
        if (this.keys[t][slot] === key) {
          return;
        }
        this.keys[t][slot] = key;
        li.dataset.band = row.band;
        li.dataset.condition = row.condition;
        li.dataset.playerId = row.id;
        li.querySelector('.lineup__pos').textContent = row.position ?? '';
        li.querySelector('.lineup__shirt').textContent = String(row.shirt ?? '');
        li.querySelector('.lineup__name').textContent = row.name ?? '';
        const fill = li.querySelector('.lineup__fill');
        fill.style.width = `${Math.round(Math.max(0, Math.min(1, row.energy)) * 100)}%`;
        fill.style.background = `var(${row.token})`;
        li.querySelector('.lineup__condition').textContent = row.condition;
        const card = li.querySelector('.lineup__card');
        card.hidden = !row.card;
        card.dataset.card = row.card ?? '';
        card.textContent = row.cardWord ?? '';
        li.setAttribute(
          'aria-label',
          `${row.position} ${row.shirt} ${row.name}, ${row.condition}` +
            (row.cardWord ? `, ${row.cardWord} card` : '')
        );
      });
    });
    const benches = benchModel(this.rosters, state);
    const benchKey = benches.map((b) => b.map((p) => p['player.id']).join(',')).join('|');
    if (benchKey !== this.benchKey) {
      this.benchKey = benchKey;
      benches.forEach((players, t) => {
        const list = this.benches[t];
        if (!list) {
          return;
        }
        const doc = list.ownerDocument;
        list.replaceChildren(
          ...players.map((p) => {
            const li = doc.createElement('li');
            li.className = 'lineup__bench-row';
            li.textContent = `${p['player.shirt']} ${p['player.name']}`;
            return li;
          })
        );
      });
    }
    return this.model;
  }

  /// Each on-pitch player's condition word, home then away, for the test hook.
  labels() {
    return this.model.flat().map((row) => ({
      id: row.id,
      name: row.name,
      condition: row.condition,
      band: row.band,
      energy: row.energy,
      card: row.cardWord,
    }));
  }
}
