// The page's side of the launcher: read the engine's state, and ask for a restart, to
// abandon the match, for a new match, for the saved match, to stop or quit, or to save the
// settings. Each request is a POST; the launcher checks that it comes from this page's own
// origin, and a body is JSON of at most 4 KiB.

/// Calls `fetch` on the global object, never as a detached function.
const defaultFetch = (...args) => globalThis.fetch(...args);

/// The engine's state, or null when nothing answers.
export async function fetchStatus(fetcher = defaultFetch) {
  try {
    const response = await fetcher('engine.json', { cache: 'no-store' });
    if (!response.ok) {
      return null;
    }
    return await response.json();
  } catch {
    return null;
  }
}

/// A POST: the new `engine.json` body, or null when the launcher refused or nothing answers.
/// On a refusal `onRefused` receives the launcher's plain-text reason.
async function act(path, fetcher, body = undefined, onRefused = undefined) {
  try {
    const init = { method: 'POST', cache: 'no-store' };
    if (body !== undefined) {
      init.body = JSON.stringify(body);
      init.headers = { 'Content-Type': 'application/json' };
    }
    const response = await fetcher(path, init);
    if (!response.ok) {
      const reason = typeof response.text === 'function' ? await response.text().catch(() => '') : '';
      onRefused?.(reason.trim());
      return null;
    }
    return await response.json();
  } catch {
    return null;
  }
}

/// Asks the launcher to start the engine again from the latest snapshot.
export const restart = (fetcher = defaultFetch) => act('engine/restart', fetcher);

/// Asks the launcher to stop the engine and give the match up.
export const abandon = (fetcher = defaultFetch) => act('engine/abandon', fetcher);

/// Asks the launcher for a fresh match once none is running: with the launch's seed and
/// teams, or, with `fixture` (`{ home, away }`, club ids), that fixture. On a refusal
/// `onRefused` receives the reason.
export const newMatch = (fetcher = defaultFetch, fixture = undefined, onRefused = undefined) =>
  act('engine/new-match', fetcher, fixture, onRefused);

/// Asks the launcher to continue the newest unfinished saved match. On a refusal `onRefused`
/// receives the reason.
export const resume = (fetcher = defaultFetch, onRefused = undefined) =>
  act('engine/resume', fetcher, undefined, onRefused);

/// The events a resumed match played before its save, in tick order; empty when the match
/// was not resumed or nothing answers.
export async function fetchEarlierEvents(fetcher = defaultFetch) {
  try {
    const response = await fetcher('engine/earlier-events', { cache: 'no-store' });
    if (!response.ok) {
      return [];
    }
    const rows = await response.json();
    return Array.isArray(rows) ? rows : [];
  } catch {
    return [];
  }
}

/// Asks the launcher to stop the match and keep its snapshot, for the start screen.
export const stop = (fetcher = defaultFetch) => act('engine/stop', fetcher);

/// Asks the launcher to stop the match, keep its snapshot and end. The answer carries the
/// facts the closed page shows.
export const quit = (fetcher = defaultFetch) => act('engine/quit', fetcher);

/// Saves the player's settings (`{ speed, motion, commentary }`).
export const saveSettings = (fetcher = defaultFetch, settings) => act('engine/settings', fetcher, settings);

/// The other fixtures of the round a match between two clubs would meet, or null.
export async function fetchRound(fetcher = defaultFetch, home, away) {
  try {
    const query = `home=${encodeURIComponent(home)}&away=${encodeURIComponent(away)}`;
    const response = await fetcher(`engine/round?${query}`, { cache: 'no-store' });
    if (!response.ok) {
      return null;
    }
    return await response.json();
  } catch {
    return null;
  }
}

/// Polls the state every `every` ms until `done(status)` holds or `timeoutMs` passes.
/// Returns the last status read, which may be null.
export async function poll(done, { every = 250, timeoutMs = 30_000, fetcher } = {}) {
  const deadline = Date.now() + timeoutMs;
  let status = await fetchStatus(fetcher);
  while (!done(status) && Date.now() < deadline) {
    await new Promise((resolve) => setTimeout(resolve, every));
    status = await fetchStatus(fetcher);
  }
  return status;
}
