// The page's side of the launcher: read the engine's state, and ask for a restart or to
// abandon the match. Both requests are POST with no body; the launcher checks that they
// come from this page's own origin.

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

async function act(path, fetcher) {
  try {
    const response = await fetcher(path, { method: 'POST', cache: 'no-store' });
    if (!response.ok) {
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
