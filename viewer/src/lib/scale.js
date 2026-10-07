// The rating scale. The engine holds every rating in tenths of 1 to 20 (10 to 200), and
// the screens show the whole number 1 to 20. Pure: no DOM.

/// The protocol version whose hello carries ratings in tenths.
export const TENTHS_PROTOCOL = 4;

/// The whole number 1 to 20 a screen shows for `tenths`: rounded, and kept to 1 to 20, so a
/// rating converted from an old team file (0.2 to 0.8) shows as 1.
export function wholeOf(tenths) {
  return Math.max(1, Math.min(20, Math.round(Number(tenths ?? 0) / 10)));
}

/// The protocol version whose hello carries the hidden values as words.
export const WORDS_PROTOCOL = 5;

/// The hidden values a squad entry carries as a word and a confidence, never a number.
export const HIDDEN_KEYS = Object.freeze(['player.consistency', 'player.injury_proneness']);

/// The hello as this page reads it. A hello of protocol 3 or earlier carries ratings on
/// 1 to 100: every rating of the squad (`player.natural_fitness` and each `role_fit`) is
/// doubled into tenths, since a protocol 3 rating `v` is `2v` tenths exactly. A hello before
/// protocol 5 carries `player.injury_resistance` as a figure and no hidden value: the figure
/// is dropped and both hidden values read as not yet known, since an old figure is never
/// turned into a word. A later hello is returned as it is. The hello is never changed; a
/// lifted copy keeps its `protocol.version`, so a replay written back still names the
/// protocol of its frames.
export function liftHello(hello) {
  const version = Number(hello?.['protocol.version']);
  if (!(version < WORDS_PROTOCOL)) {
    return hello;
  }
  const tenths = version < TENTHS_PROTOCOL;
  const lift = (v) => (tenths && typeof v === 'number' ? v * 2 : v);
  const teams = (hello?.teams ?? []).map((team) => {
    if (!Array.isArray(team?.squad) || team.squad.length === 0) {
      return team;
    }
    return {
      ...team,
      squad: team.squad.map((p) => {
        const out = { ...p };
        if ('player.natural_fitness' in out) {
          out['player.natural_fitness'] = lift(out['player.natural_fitness']);
        }
        if (Array.isArray(out.role_fit)) {
          out.role_fit = out.role_fit.map(lift);
        }
        delete out['player.injury_resistance'];
        for (const key of HIDDEN_KEYS) {
          out[key] = { confidence: 'not_yet_known' };
        }
        return out;
      }),
    };
  });
  return { ...hello, teams };
}
