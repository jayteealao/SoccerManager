// One structured record per interesting thing the page does.
//
// The observability contract closed the engine's sink with a file plus the socket feed.
// Neither reaches a browser: a page cannot write a file, and the socket carries
// engine-to-page messages only. Until a browser transport exists, every page-side signal
// is one JSON Lines row on console.info plus a short ring a test or a drive can read.
// That is enough to verify a slice and not enough for a dashboard.

/// Rows kept in memory. A drive reads the ring; the console keeps the full history.
const RING_LIMIT = 256;

const ring = [];

/// The envelope every row carries, matching the engine's own record shape.
/// `record.kind` is a fifth kind the observability contract does not name; it is emitted
/// as an additive extra, exactly as `content.hash` and `change.queue_id` already are.
export function signal(name, fields = {}) {
  const row = {
    'record.kind': 'viewer-event',
    'schema.version': '1',
    service: 'touchline-viewer',
    operation: 'view',
    signal: name,
    ts: new Date().toISOString(),
    ...fields,
  };
  ring.push(row);
  if (ring.length > RING_LIMIT) {
    ring.shift();
  }
  console.info(JSON.stringify(row));
  return row;
}

/// A copy of the ring, newest last. The test hook exposes this.
export function signals() {
  return ring.slice();
}

/// Empties the ring. Tests use it; the page does not.
export function clearSignals() {
  ring.length = 0;
}

export { RING_LIMIT };
