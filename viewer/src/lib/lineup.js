// Lineup legality, as the kick-off control reads it. Pure: no DOM.
//
// The page checks a lineup before it sends one, so the manager sees the reason beside the
// editor instead of after a round trip. The engine checks the same structure again when the
// lineup arrives, and its word is final.

export const STARTERS = 11;

const NUMBER_WORDS = [
  'No',
  'One',
  'Two',
  'Three',
  'Four',
  'Five',
  'Six',
  'Seven',
  'Eight',
  'Nine',
  'Ten',
  'Eleven',
];

function countWord(n) {
  return NUMBER_WORDS[n] ?? String(n);
}

/// `slots` holds eleven squad indices in slot order, `null` for an empty slot; slot 0 is the
/// goalkeeper's. `bench` holds squad indices, `null` for an empty bench place. `squad` is the
/// hello's squad in file order. The first failure is reported, in a fixed order: too few
/// starters, no goalkeeper in slot 0, a player placed twice, too many substitutes.
export function checkLineup({ slots, bench = [], squad, benchSize }) {
  const starters = slots.filter((s) => s !== null && s !== undefined).length;
  if (starters < STARTERS) {
    const noun = starters === 1 ? 'starter' : 'starters';
    return { legal: false, reason: `${countWord(starters)} ${noun}; a match needs eleven.` };
  }
  const keeper = squad[slots[0]];
  if (!keeper || keeper['player.position'] !== 'GK') {
    const name = keeper ? keeper['player.name'] : 'Nobody';
    return {
      legal: false,
      reason: `The goalkeeper's slot holds ${name}, who is not a goalkeeper.`,
    };
  }
  const seen = new Set();
  for (const index of [...slots, ...bench]) {
    if (index === null || index === undefined) {
      continue;
    }
    if (seen.has(index)) {
      const name = squad[index] ? squad[index]['player.name'] : `Player ${index}`;
      return { legal: false, reason: `${name} is placed twice.` };
    }
    seen.add(index);
  }
  const named = bench.filter((s) => s !== null && s !== undefined).length;
  if (named > benchSize) {
    return {
      legal: false,
      reason: `${countWord(named)} substitutes; the bench holds ${benchSize}.`,
    };
  }
  return { legal: true, reason: null };
}

/// The `set-lineup` fields from an editor state: the eleven slots and the named bench.
export function lineupMessage(slots, bench) {
  return {
    lineup: slots.slice(0, STARTERS),
    bench: bench.filter((s) => s !== null && s !== undefined),
  };
}

/// Word bands beside every 0 to 100 figure, so a number is never read by colour.
export function fitWord(fit) {
  if (fit >= 75) {
    return 'Strong';
  }
  if (fit >= 60) {
    return 'Good';
  }
  return fit >= 45 ? 'Fair' : 'Poor';
}

export function fitnessWord(fitness) {
  if (fitness >= 70) {
    return 'High';
  }
  return fitness >= 50 ? 'Medium' : 'Low';
}
