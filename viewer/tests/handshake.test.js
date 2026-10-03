// The handshake page's log: every line is text, the tick frames are sampled as the former
// page sampled them, and the socket address carries the port and the protocol version.

import assert from 'node:assert/strict';
import { test } from 'vitest';

import {
  cannotOpenLine,
  checkLines,
  closeLine,
  facts,
  lineForText,
  lineForTick,
  noAnswerLine,
  noMatchLine,
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

test('a status with no socket port gives no address', () => {
  for (const port of [null, undefined, 0, -1, 1.5, '9001']) {
    assert.equal(socketAddress({ 'socket.port': port, 'protocol.version': 3 }), null, `port ${port}`);
  }
  assert.equal(socketAddress({ 'protocol.version': 3 }), null, 'no port key');
});

test('the no-match, no-answer and cannot-open lines say why and what to do, in the bad style', () => {
  assert.deepEqual(noMatchLine(), {
    text: 'no match is running, so the engine has no socket open. Start a match, then press Run again.',
    kind: 'bad',
  });
  assert.deepEqual(noAnswerLine(10), {
    text: 'no answer from the socket after 10 s. The engine serves one page at a time: close the match page, then press Run again.',
    kind: 'bad',
  });
  assert.deepEqual(cannotOpenLine('ws://127.0.0.1:1/?v=3', 'refused'), {
    text: 'cannot open ws://127.0.0.1:1/?v=3: refused',
    kind: 'bad',
  });
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
