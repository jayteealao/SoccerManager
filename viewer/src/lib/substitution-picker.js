// The substitution picker's pure half: the count the rule pack leaves and the words beside it.
// The engine applies a substitution at the next stoppage that admits one and refuses it there
// with its own reason; the picker never predicts that verdict. `SubPicker.svelte` draws it.

/// Substitutions the home club has left: the rule pack's limit less the home substitutions
/// playback has reached.
export function remaining(limit, substitutions, homeTeamId) {
  const used = substitutions.filter((s) => s.team === homeTeamId).length;
  return Math.max(0, limit - used);
}

/// The words beside the count, so the figure is never read alone.
export function remainingText(left, limit) {
  return `${left} of ${limit} substitutions left`;
}

/// The picker's rows, as today's page lists them: the players on the pitch who may come off
/// (not sent off), and the bench players who may come on, each
/// `{ squad, shirt, name, position }`.
export function pickerPlayers(homeRows, benchRows) {
  return {
    off: homeRows
      .filter((r) => r.squad !== null && !r.sentOff)
      .map((r) => ({ squad: r.squad, shirt: r.shirt, name: r.name, position: r.position })),
    on: benchRows.map((p) => ({
        squad: p['player.squad_index'],
        shirt: p['player.shirt'],
        name: p['player.name'],
        position: p['player.position'],
      })),
  };
}

/// Why the picker cannot queue, or null when it can.
export function pickerBlock({ live, left, off, on }) {
  if (!live) {
    return 'Changes need a live match.';
  }
  if (left <= 0) {
    return 'No substitutions left.';
  }
  if (off.length === 0 || on.length === 0) {
    return 'No player to change.';
  }
  return null;
}
