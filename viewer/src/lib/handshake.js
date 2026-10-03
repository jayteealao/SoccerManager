// The handshake page's pure parts: the socket address, the checks, and every log line. The
// page opens the socket, prints the hello and a sample of the tick frames, and stops, so a
// fault it shows is a fault in the handshake, the origin, the version query or the isolation
// headers, never in the renderer. Every line is text: the socket's own words reach the page
// from the wire and are never parsed as markup.

/// The first tick frames always get a line; after them, every 500th.
export const FIRST_FRAMES = 3;
export const FRAME_EVERY = 500;

/// How long the page waits for the socket to open. A free engine accepts at once; an engine
/// that already serves the match page leaves a second page's connection waiting unanswered.
export const ANSWER_WAIT_MS = 10_000;

/// The socket address the engine status names: the loopback socket port and the protocol
/// version as the query. Null when the status names no port: before kick-off and after full
/// time the launcher has no socket open.
export function socketAddress(status) {
  const port = status['socket.port'];
  if (!Number.isInteger(port) || port <= 0) {
    return null;
  }
  return `ws://127.0.0.1:${port}/?v=${status['protocol.version']}`;
}

/// The page's own checks, as the former page printed them: the origin, whether the page is
/// cross-origin isolated, and whether the memory gauge exists.
export function checks(scope) {
  const isolated = scope.crossOriginIsolated === true;
  const gauge = typeof scope.performance?.measureUserAgentSpecificMemory === 'function';
  return { origin: scope.location?.origin ?? '', isolated, gauge };
}

/// The first log lines, before the socket opens. `kind` is `ok`, `bad` or `plain`; every
/// kind also says its state in words, so colour is never the only signal.
export function checkLines({ origin, isolated, gauge }) {
  return [
    { text: `origin ${origin}`, kind: 'plain' },
    { text: `crossOriginIsolated ${isolated}`, kind: isolated ? 'ok' : 'bad' },
    { text: `memory gauge ${gauge}`, kind: gauge ? 'ok' : 'bad' },
  ];
}

export function openingLine(address) {
  return { text: `opening ${address}`, kind: 'plain' };
}

/// A text message from the socket, printed whole after its type. The hello is the one line
/// marked `ok`. A message that cannot be read is printed as it came.
export function lineForText(data) {
  let type = null;
  try {
    type = JSON.parse(data)?.type ?? null;
  } catch {
    // Not JSON: printed as it came.
  }
  return { text: `${type ?? 'text'}: ${data}`, kind: type === 'hello' ? 'ok' : 'plain' };
}

/// Whether tick frame number `count` (from 1) gets a line.
export function sampled(count) {
  return count <= FIRST_FRAMES || count % FRAME_EVERY === 0;
}

/// The line for tick frame number `count`: its length and its kind byte.
export function lineForTick(count, bytes) {
  return {
    text: `tick frame ${count}: ${bytes.length} bytes, kind 0x0${bytes[0]}`,
    kind: 'plain',
  };
}

export function closeLine(code, clean, ticks) {
  return { text: `closed code=${code} clean=${clean} after ${ticks} tick frames`, kind: 'plain' };
}

export function errorLine() {
  return { text: 'socket error', kind: 'bad' };
}

export function noMatchLine() {
  return {
    text: 'no match is running, so the engine has no socket open. Start a match, then press Run again.',
    kind: 'bad',
  };
}

export function noAnswerLine(seconds) {
  return {
    text: `no answer from the socket after ${seconds} s. The engine serves one page at a time: close the match page, then press Run again.`,
    kind: 'bad',
  };
}

/// The browser refused to make the socket: the reason is its own message, printed as text.
export function cannotOpenLine(address, reason) {
  return { text: `cannot open ${address}: ${reason}`, kind: 'bad' };
}

/// The fact strip: each fact carries a word, never a colour alone. `socketBad` marks a
/// socket that closed uncleanly.
export function facts({ isolated, gauge, ticks, socket, socketBad = false }) {
  return [
    { label: 'crossOriginIsolated', value: isolated ? 'Yes' : 'No', kind: isolated ? 'ok' : 'bad' },
    { label: 'Memory gauge', value: gauge ? 'Present' : 'Missing', kind: gauge ? 'ok' : 'bad' },
    { label: 'Tick frames', value: String(ticks), kind: 'plain' },
    { label: 'Socket', value: socket, kind: socketBad || socket === 'Error' ? 'bad' : 'plain' },
  ];
}
