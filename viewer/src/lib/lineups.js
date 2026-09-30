// The two lineups: who is on the pitch, how tired they are, and what the referee showed them.
// Pure: no DOM. The components read these rows.
//
// Every state carries a word beside its colour: Fresh, Tiring, Exhausted, Yellow, Red,
// Injured, Sent off. A colour on its own never carries a state.

/// Energy bands, highest first. The word is the state; the skin token only reinforces it.
export const BANDS = Object.freeze([
  { min: 0.7, word: 'Fresh', token: '--good' },
  { min: 0.4, word: 'Tiring', token: '--warn' },
  { min: -Infinity, word: 'Exhausted', token: '--bad' },
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

/// The rows of both teams at the view `state` from `match-state.js`. `rosters` holds each
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
        squad: player['player.squad_index'] ?? null,
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
