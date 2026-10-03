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

/// Every message the page sends, `seen` and playback timing included: the Pre-match page's
/// read-only check needs all of them.
export const everyMessage = () => true;

/// A TCP relay between the page and the engine's socket that passes every server frame
/// through unchanged and lets a test send the page messages of its own on the same socket.
/// The page is pointed at the relay by its status (`socket.port`). The relay forwards whole
/// WebSocket frames only, so a frame the test sends never lands inside one of the engine's.
/// Playwright's own socket route would hold every frame of a long match in the test's memory;
/// the relay holds none. With `keep`, the relay also records the page's text messages that
/// `keep(message)` picks in `relay.sent`, as `recordClientMessages` does, so a drive that
/// fast-forwards a long match can still record what the page sent.
export async function injectingRelay(page, { keep = null } = {}) {
  const { createServer, connect } = await import('node:net');
  const relay = { enginePort: null, client: null, sent: [] };
  const server = createServer((client) => {
    const up = connect(relay.enginePort, '127.0.0.1');
    client.pipe(up);
    if (keep) {
      client.on('data', recordFrames(keep, relay.sent));
    }
    let buffer = Buffer.alloc(0);
    let upgraded = false;
    up.on('data', (chunk) => {
      buffer = Buffer.concat([buffer, chunk]);
      if (!upgraded) {
        const end = buffer.indexOf('\r\n\r\n');
        if (end < 0) {
          return;
        }
        client.write(buffer.subarray(0, end + 4));
        buffer = buffer.subarray(end + 4);
        upgraded = true;
        relay.client = client;
      }
      let sent = 0;
      for (;;) {
        const rest = buffer.length - sent;
        if (rest < 2) {
          break;
        }
        let length = buffer[sent + 1] & 0x7f;
        let at = 2;
        if (length === 126) {
          if (rest < 4) {
            break;
          }
          length = buffer.readUInt16BE(sent + 2);
          at = 4;
        } else if (length === 127) {
          if (rest < 10) {
            break;
          }
          length = Number(buffer.readBigUInt64BE(sent + 2));
          at = 10;
        }
        if (rest < at + length) {
          break;
        }
        sent += at + length;
      }
      if (sent > 0) {
        client.write(buffer.subarray(0, sent));
        buffer = buffer.subarray(sent);
      }
    });
    up.on('end', () => client.end());
    client.on('error', () => {});
    up.on('error', () => {});
    client.on('close', () => up.destroy());
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  await page.route('**/engine.json', async (route) => {
    const response = await route.fetch();
    const body = await response.json();
    if (body['socket.port']) {
      relay.enginePort = body['socket.port'];
      body['socket.port'] = server.address().port;
    }
    await route.fulfill({ response, json: body });
  });
  /// Sends `message` to the page as one unmasked text frame.
  relay.send = (message) => {
    const payload = Buffer.from(JSON.stringify(message), 'utf8');
    let head;
    if (payload.length < 126) {
      head = Buffer.from([0x81, payload.length]);
    } else if (payload.length < 65_536) {
      head = Buffer.alloc(4);
      head[0] = 0x81;
      head[1] = 126;
      head.writeUInt16BE(payload.length, 2);
    } else {
      head = Buffer.alloc(10);
      head[0] = 0x81;
      head[1] = 127;
      head.writeBigUInt64BE(BigInt(payload.length), 2);
    }
    relay.client.write(Buffer.concat([head, payload]));
  };
  relay.close = () => server.close();
  return relay;
}

/// A reader of the page's side of a socket: it skips the upgrade request, then unmasks each
/// whole text frame and records the JSON messages `keep` picks. It only reads; the bytes
/// travel on unchanged.
function recordFrames(keep, sent) {
  let buffer = Buffer.alloc(0);
  let upgraded = false;
  return (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    if (!upgraded) {
      const end = buffer.indexOf('\r\n\r\n');
      if (end < 0) {
        return;
      }
      buffer = buffer.subarray(end + 4);
      upgraded = true;
    }
    for (;;) {
      if (buffer.length < 2) {
        return;
      }
      const opcode = buffer[0] & 0x0f;
      const masked = (buffer[1] & 0x80) !== 0;
      let length = buffer[1] & 0x7f;
      let at = 2;
      if (length === 126) {
        if (buffer.length < 4) {
          return;
        }
        length = buffer.readUInt16BE(2);
        at = 4;
      } else if (length === 127) {
        if (buffer.length < 10) {
          return;
        }
        length = Number(buffer.readBigUInt64BE(2));
        at = 10;
      }
      const mask = masked ? buffer.subarray(at, at + 4) : null;
      at += masked ? 4 : 0;
      if (buffer.length < at + length) {
        return;
      }
      if (opcode === 0x1) {
        const payload = Buffer.from(buffer.subarray(at, at + length));
        if (mask) {
          for (let i = 0; i < payload.length; i += 1) {
            payload[i] ^= mask[i % 4];
          }
        }
        try {
          const parsed = JSON.parse(payload.toString('utf8'));
          if (keep(parsed)) {
            sent.push(parsed);
          }
        } catch {
          // Not JSON: forwarded, not recorded.
        }
      }
      buffer = buffer.subarray(at + length);
    }
  };
}
