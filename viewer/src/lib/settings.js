// The player's three settings, as the launcher keeps them in the data folder: the speed a
// match starts at, how the viewer moves, and whether the commentary shows. The launcher
// stores them because the page's port, and with it the browser's own storage, changes at
// every launch.

/// The speeds a match may start at.
export const SPEEDS = Object.freeze([1, 2, 4, 8]);

/// How the viewer moves: as Windows asks, always reduced, or always full.
export const MOTIONS = Object.freeze(['follow', 'reduce', 'full']);

/// The settings with no file: 1x, follow Windows, commentary on. A match with these plays as
/// a match played before the settings existed.
export const DEFAULTS = Object.freeze({ speed: 1, motion: 'follow', commentary: true });

/// The settings in `body` (the launcher's `settings` block), each value checked; a missing or
/// wrong value takes its default.
export function readSettings(body) {
  const value = body && typeof body === 'object' ? body : {};
  return {
    speed: SPEEDS.includes(value.speed) ? value.speed : DEFAULTS.speed,
    motion: MOTIONS.includes(value.motion) ? value.motion : DEFAULTS.motion,
    commentary: typeof value.commentary === 'boolean' ? value.commentary : DEFAULTS.commentary,
  };
}

/// The `data-motion` word for a motion setting and the system's preference: `reduce` or
/// `full`. `follow` follows the system.
export function motionWord(motion, systemReduces) {
  if (motion === 'reduce') {
    return 'reduce';
  }
  if (motion === 'full') {
    return 'full';
  }
  return systemReduces ? 'reduce' : 'full';
}
