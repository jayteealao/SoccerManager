// The handshake page's log: every line is text, the tick frames are sampled as the former
// page sampled them, and the socket address carries the port and the protocol version.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import {
  checkLines,
  closeLine,
  facts,
  lineForText,
  lineForTick,
  sampled,
  socketAddress,
} from '../src/lib/handshake.js';

test('a text message holding markup stays text, and the hello is the one ok line', () => {
  const hello = lineForText('{"type":"hello","name":"<b>Home</b>"}');
  assert.equal(hello.text, 'hello: {"type":"hello","name":"<b>Home</b>"}');
  assert.equal(hello.kind, 'ok');
  const other = lineForText('{"type":"event","commentary":"<img src=x onerror=alert(1)>"}');
  assert.equal(other.kind, 'plain');
  assert.ok(other.text.includes('<img src=x onerror=alert(1)>'), 'the markup is kept as characters');
  assert.equal(lineForText('not json').text, 'text: not json');
});

test('frames 1 to 3 and every 500th make a line', () => {
  const lines = [];
  for (let n = 1; n <= 1500; n += 1) {
    if (sampled(n)) {
      lines.push(n);
    }
  }
  assert.deepEqual(lines, [1, 2, 3, 500, 1000, 1500]);
  assert.equal(lineForTick(2, new Uint8Array([2, 0, 0, 0])).text, 'tick frame 2: 4 bytes, kind 0x02');
});

test('the socket address carries the port and the protocol version', () => {
  assert.equal(
    socketAddress({ 'socket.port': 49152, 'protocol.version': 4 }),
    'ws://127.0.0.1:49152/?v=4'
  );
});

test('the close line names the code, the clean flag and the count, and each fact says its state in words', () => {
  assert.equal(closeLine(1006, false, 3).text, 'closed code=1006 clean=false after 3 tick frames');
  const [origin, isolated, gauge] = checkLines({ origin: 'http://127.0.0.1:1', isolated: false, gauge: true });
  assert.equal(origin.text, 'origin http://127.0.0.1:1');
  assert.deepEqual([isolated.text, isolated.kind], ['crossOriginIsolated false', 'bad']);
  assert.deepEqual([gauge.text, gauge.kind], ['memory gauge true', 'ok']);
  const strip = facts({ isolated: true, gauge: false, ticks: 12, socket: 'Closed' });
  assert.deepEqual(
    strip.map((f) => f.value),
    ['Yes', 'Missing', '12', 'Closed']
  );
});
