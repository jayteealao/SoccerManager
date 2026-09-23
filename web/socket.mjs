// The socket client.
//
// `binaryType = 'arraybuffer'` matters more than it looks: the default hands every binary
// frame over as a Blob, whose contents can only be read asynchronously. At eight times
// speed that is four hundred asynchronous reads a second, each landing on a later turn
// than the frame that needs it.

import { signal } from './signal.mjs';

/// The address the engine prints, with the protocol version the server insists on.
export function socketAddress(port, version = 1) {
  return `ws://127.0.0.1:${port}/?v=${version}`;
}

export class MatchSocket {
  /// `onHello`, `onTick` and `onMessage` are called as frames arrive. `onTick` receives
  /// the raw `ArrayBuffer`, because the decoder writes straight out of it.
  constructor(address, { onHello, onTick, onMessage }) {
    this.address = address;
    this.matchId = null;
    this.tick = 0;
    this.socket = new WebSocket(address);
    this.socket.binaryType = 'arraybuffer';

    this.socket.addEventListener('message', (event) => {
      if (typeof event.data === 'string') {
        const message = JSON.parse(event.data);
        if (message.type === 'hello') {
          this.matchId = message['match.id'];
          signal('viewer.connected', {
            'protocol.version': message['protocol.version'],
            'engine.version': message['engine.version'],
            'match.id': message['match.id'],
            'owner.id': message['owner.id'],
            origin: globalThis.location ? globalThis.location.origin : null,
            ticks_expected: message.ticks_expected,
          });
          onHello(message);
          return;
        }
        onMessage(message);
        return;
      }
      onTick(event.data);
    });

    // A socket that closes mid-match leaves the page drawing its last frame for ever, so
    // a dead stream looks exactly like a frozen match. Naming it is the whole point.
    this.socket.addEventListener('close', (event) => {
      signal('viewer.socket_drops', {
        'match.id': this.matchId,
        tick: this.tick,
        code: event.code,
        wasClean: event.wasClean,
        reason: event.reason || null,
      });
      if (this.onClose) {
        this.onClose(event);
      }
    });

    this.socket.addEventListener('error', () => {
      signal('viewer.socket_drops', {
        'match.id': this.matchId,
        tick: this.tick,
        code: null,
        wasClean: false,
        reason: 'socket error',
      });
    });
  }

  /// The newest tick received, which the scheduler must never draw past.
  noteTick(tick) {
    this.tick = tick;
  }

  /// Sends one command as a JSON text frame. Returns `false` when the socket is not open, so
  /// a caller never waits for an answer that cannot come.
  send(command) {
    if (this.socket.readyState !== WebSocket.OPEN) {
      return false;
    }
    this.socket.send(JSON.stringify(command));
    return true;
  }

  close() {
    this.socket.close();
  }
}
