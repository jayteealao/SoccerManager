// The rating scale. The engine holds every rating in tenths of 1 to 20 (10 to 200), and
// the screens show the whole number 1 to 20. Pure: no DOM.

/// The protocol version whose hello carries ratings in tenths.
export const TENTHS_PROTOCOL = 4;

/// The whole number 1 to 20 a screen shows for `tenths`: rounded, and kept to 1 to 20, so a
/// rating converted from an old team file (0.2 to 0.8) shows as 1.
export function wholeOf(tenths) {
  return Math.max(1, Math.min(20, Math.round(Number(tenths ?? 0) / 10)));
}

/// The hello with its ratings in tenths. A hello of protocol 3 or earlier carries ratings on
/// 1 to 100: every rating of the squad (`player.natural_fitness`,
/// `player.injury_resistance`, and each `role_fit`) is doubled into tenths, since a
/// protocol 3 rating `v` is `2v` tenths exactly. A later hello is returned as it is. The
/// hello is never changed; a lifted copy keeps its `protocol.version`, so a replay written
/// back still names the protocol of its frames.
export function liftHello(hello) {
  if (!(Number(hello?.['protocol.version']) < TENTHS_PROTOCOL)) {
    return hello;
  }
  const lift = (v) => (typeof v === 'number' ? v * 2 : v);
  const teams = (hello?.teams ?? []).map((team) => {
    if (!Array.isArray(team?.squad) || team.squad.length === 0) {
      return team;
    }
    return {
      ...team,
      squad: team.squad.map((p) => {
        const out = { ...p };
        for (const key of ['player.natural_fitness', 'player.injury_resistance']) {
          if (key in out) {
            out[key] = lift(out[key]);
          }
        }
        if (Array.isArray(out.role_fit)) {
          out.role_fit = out.role_fit.map(lift);
        }
        return out;
      }),
    };
  });
  return { ...hello, teams };
}
