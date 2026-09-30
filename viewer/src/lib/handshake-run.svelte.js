// The handshake page's one run: the page's checks, the engine status, the socket, and the
// log lines and facts they make. RUN AGAIN closes the socket, clears the log and runs the
// check again. The browser's fetch, socket and scope are passed in, so a test can drive it.

import {
  checkLines,
  checks,
  closeLine,
  errorLine,
  facts,
  lineForText,
  lineForTick,
  openingLine,
  sampled,
  socketAddress,
} from './handshake.js';

export class HandshakeRun {
  lines = $state.raw([]);
  ticks = $state(0);
  socketWord = $state('Connecting');
  socketBad = $state(false);
  running = $state(false);

  constructor({ scope = globalThis, fetcher = null, Socket = null } = {}) {
    this.scope = scope;
    this.fetcher = fetcher ?? ((url) => scope.fetch(url));
    this.Socket = Socket ?? scope.WebSocket;
    this.socket = null;
    this.checks = checks(scope);
  }

  /// The fact strip: isolation, the memory gauge, the tick frames counted and the socket.
  get facts() {
    return facts({
      isolated: this.checks.isolated,
      gauge: this.checks.gauge,
      ticks: this.ticks,
      socket: this.socketWord,
      socketBad: this.socketBad,
    });
  }

  say(line) {
    this.lines = [...this.lines, line];
  }

  /// Runs the check from the start: a socket still open is closed without a line, and the
  /// log starts again.
  async run() {
    this.close();
    this.lines = [{ text: 'connecting…', kind: 'plain' }];
    this.ticks = 0;
    this.socketWord = 'Connecting';
    this.socketBad = false;
    this.running = true;
    this.checks = checks(this.scope);
    for (const line of checkLines(this.checks)) {
      this.say(line);
    }
    let status;
    try {
      const response = await this.fetcher('engine.json');
      status = await response.json();
    } catch {
      this.say({ text: 'engine.json could not be read', kind: 'bad' });
      this.socketWord = 'Error';
      this.running = false;
      return;
    }
    const address = socketAddress(status);
    this.say(openingLine(address));
    const socket = new this.Socket(address);
    socket.binaryType = 'arraybuffer';
    this.socket = socket;
    socket.addEventListener('open', () => {
      if (this.socket === socket) {
        this.socketWord = 'Open';
      }
    });
    socket.addEventListener('message', (event) => {
      if (this.socket !== socket) {
        return;
      }
      if (typeof event.data === 'string') {
        this.say(lineForText(event.data));
        return;
      }
      this.ticks += 1;
      if (sampled(this.ticks)) {
        this.say(lineForTick(this.ticks, new Uint8Array(event.data)));
      }
    });
    socket.addEventListener('error', () => {
      if (this.socket === socket) {
        this.say(errorLine());
        this.socketWord = 'Error';
      }
    });
    socket.addEventListener('close', (event) => {
      if (this.socket !== socket) {
        return;
      }
      const line = closeLine(event.code, event.wasClean, this.ticks);
      this.say(event.wasClean ? line : { ...line, kind: 'bad' });
      this.socketWord = `closed · ${event.code}`;
      this.socketBad = !event.wasClean;
      this.socket = null;
      this.running = false;
    });
  }

  /// Closes the socket this run opened, if any, and forgets it: its late events print nothing.
  close() {
    const socket = this.socket;
    this.socket = null;
    if (socket && socket.readyState !== 3) {
      socket.close();
    }
  }
}
