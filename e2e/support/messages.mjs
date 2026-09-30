// Records the text messages a page sends to the engine, on the page's own socket, while the
// messages still reach the real engine: the route connects to the server, forwards every
// client message itself, and leaves the server's messages to flow on untouched.

/// Starts recording before the page opens. `keep(message)` picks the messages to record;
/// the returned list fills as the page sends.
export async function recordClientMessages(page, keep = () => true) {
  const sent = [];
  await page.routeWebSocket(/^ws:\/\/127\.0\.0\.1:\d+\//, (ws) => {
    const server = ws.connectToServer();
    ws.onMessage((message) => {
      if (typeof message === 'string') {
        try {
          const parsed = JSON.parse(message);
          if (keep(parsed)) {
            sent.push(parsed);
          }
        } catch {
          // Not JSON: forwarded, not recorded.
        }
      }
      server.send(message);
    });
    ws.onClose((code, reason) => server.close({ code, reason }));
  });
  return sent;
}

/// The manager's own commands: the lineup, each queued change and each withdrawal. `seen`,
/// `start`, `pause` and `resume` follow playback timing, not the manager's choices.
export const MANAGER_COMMANDS = new Set(['set-lineup', 'queue-change', 'cancel-change']);

export const managerCommand = (message) => MANAGER_COMMANDS.has(message.type);
